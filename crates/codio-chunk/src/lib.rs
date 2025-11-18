//! BitTorrent-style chunk distribution for Codio CDN
//!
//! This crate implements parallel chunk downloads with:
//! - Rarest-first strategy (download rare chunks first)
//! - Tit-for-tat mechanism (prefer peers who upload to us)
//! - SHA-256 chunk verification
//! - Automatic peer failover
//!
//! # Example
//!
//! ```no_run
//! use codio_chunk::{ChunkDistributor, ChunkConfig};
//! use codio_content_id::ContentId;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ChunkConfig::default();
//! let mut distributor = ChunkDistributor::new(config).await?;
//!
//! let cid = ContentId::new(b"test content");
//! let providers = vec![]; // Get from DHT
//! let content = distributor.download_content(cid.clone(), providers).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{anyhow, Result};
use codio_common::PeerId;
use codio_content_id::ContentId;
use rand::seq::SliceRandom;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub mod config;
pub use config::{ChunkConfig, DistributionStrategy};

/// Standard chunk size (256KB, like BitTorrent)
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

/// Chunk data with metadata
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk index in the file
    pub index: u32,
    /// Raw chunk data
    pub data: Vec<u8>,
    /// SHA-256 hash of the chunk
    pub hash: [u8; 32],
}

impl Chunk {
    /// Create a new chunk with automatic hash calculation
    pub fn new(index: u32, data: Vec<u8>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash: [u8; 32] = hasher.finalize().into();

        Self { index, data, hash }
    }

    /// Verify chunk integrity
    pub fn verify(&self) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        let computed_hash: [u8; 32] = hasher.finalize().into();
        computed_hash == self.hash
    }

    /// Get chunk size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Peer information for chunk transfers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub address: String,
}

impl PeerInfo {
    pub fn new(peer_id: PeerId, address: String) -> Self {
        Self { peer_id, address }
    }
}

/// Download state for a specific content
#[derive(Debug, Clone)]
pub struct Download {
    /// Content being downloaded
    pub cid: ContentId,
    /// Total number of chunks
    pub total_chunks: u32,
    /// Total content size in bytes
    pub total_size: u64,
    /// Successfully downloaded chunks
    pub downloaded_chunks: HashSet<u32>,
    /// Chunks currently being requested (chunk_index -> peer)
    pub active_requests: HashMap<u32, PeerInfo>,
    /// Available peers
    pub available_peers: Vec<PeerInfo>,
    /// Chunk availability map (chunk_index -> count of peers having it)
    pub chunk_availability: HashMap<u32, usize>,
    /// Download strategy
    pub strategy: DistributionStrategy,
    /// Start time
    pub start_time: Instant,
    /// Downloaded bytes
    pub downloaded_bytes: u64,
    /// Chunk data storage
    pub chunks: HashMap<u32, Chunk>,
}

impl Download {
    /// Create a new download
    pub fn new(
        cid: ContentId,
        total_chunks: u32,
        total_size: u64,
        available_peers: Vec<PeerInfo>,
        strategy: DistributionStrategy,
    ) -> Self {
        Self {
            cid,
            total_chunks,
            total_size,
            downloaded_chunks: HashSet::new(),
            active_requests: HashMap::new(),
            available_peers,
            chunk_availability: HashMap::new(),
            strategy,
            start_time: Instant::now(),
            downloaded_bytes: 0,
            chunks: HashMap::new(),
        }
    }

    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        self.downloaded_chunks.len() == self.total_chunks as usize
    }

    /// Get download progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.total_chunks == 0 {
            return 1.0;
        }
        self.downloaded_chunks.len() as f64 / self.total_chunks as f64
    }

    /// Get download rate in bytes/sec
    pub fn download_rate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.downloaded_bytes as f64 / elapsed
    }

    /// Get next chunk to download based on strategy
    pub fn next_chunk(&mut self) -> Option<u32> {
        let mut candidates: Vec<u32> = (0..self.total_chunks)
            .filter(|i| {
                !self.downloaded_chunks.contains(i) && !self.active_requests.contains_key(i)
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        match self.strategy {
            DistributionStrategy::RarestFirst => {
                // Sort by rarity (ascending availability)
                candidates.sort_by_key(|i| self.chunk_availability.get(i).copied().unwrap_or(0));
                candidates.first().copied()
            }
            DistributionStrategy::Sequential => {
                // Return lowest index
                candidates.into_iter().min()
            }
            DistributionStrategy::RandomOrder => {
                // Random selection
                let mut rng = rand::thread_rng();
                candidates.choose(&mut rng).copied()
            }
        }
    }

    /// Mark chunk as downloaded
    pub fn mark_downloaded(&mut self, chunk: Chunk) {
        self.downloaded_chunks.insert(chunk.index);
        self.active_requests.remove(&chunk.index);
        self.downloaded_bytes += chunk.size() as u64;
        self.chunks.insert(chunk.index, chunk);
    }

    /// Mark chunk request as failed
    pub fn mark_failed(&mut self, chunk_index: u32) {
        self.active_requests.remove(&chunk_index);
    }

    /// Assemble complete content from chunks
    pub fn assemble(&self) -> Result<Vec<u8>> {
        if !self.is_complete() {
            return Err(anyhow!(
                "Download incomplete: {}/{} chunks",
                self.downloaded_chunks.len(),
                self.total_chunks
            ));
        }

        let mut result = Vec::with_capacity(self.total_size as usize);
        for i in 0..self.total_chunks {
            let chunk = self
                .chunks
                .get(&i)
                .ok_or_else(|| anyhow!("Missing chunk {}", i))?;
            result.extend_from_slice(&chunk.data);
        }

        Ok(result)
    }
}

/// Upload tracking for a peer
#[derive(Debug, Clone)]
pub struct Upload {
    /// Content being uploaded
    pub cid: ContentId,
    /// Peer receiving the upload
    pub peer: PeerInfo,
    /// Number of chunks uploaded to this peer
    pub uploaded_chunks: u32,
    /// Total bytes uploaded
    pub uploaded_bytes: u64,
    /// Last upload time
    pub last_upload: Instant,
}

impl Upload {
    pub fn new(cid: ContentId, peer: PeerInfo) -> Self {
        Self {
            cid,
            peer,
            uploaded_chunks: 0,
            uploaded_bytes: 0,
            last_upload: Instant::now(),
        }
    }

    /// Record a chunk upload
    pub fn record_upload(&mut self, chunk_size: usize) {
        self.uploaded_chunks += 1;
        self.uploaded_bytes += chunk_size as u64;
        self.last_upload = Instant::now();
    }
}

/// Peer statistics for tit-for-tat
#[derive(Debug, Clone)]
pub struct PeerStats {
    pub peer: PeerInfo,
    /// Bytes downloaded from this peer
    pub downloaded: u64,
    /// Bytes uploaded to this peer
    pub uploaded: u64,
    /// Last interaction time
    pub last_seen: Instant,
}

impl PeerStats {
    pub fn new(peer: PeerInfo) -> Self {
        Self {
            peer,
            downloaded: 0,
            uploaded: 0,
            last_seen: Instant::now(),
        }
    }

    /// Get upload/download ratio
    pub fn ratio(&self) -> f64 {
        if self.downloaded == 0 {
            return f64::INFINITY;
        }
        self.uploaded as f64 / self.downloaded as f64
    }

    /// Record download
    pub fn record_download(&mut self, bytes: u64) {
        self.downloaded += bytes;
        self.last_seen = Instant::now();
    }

    /// Record upload
    pub fn record_upload(&mut self, bytes: u64) {
        self.uploaded += bytes;
        self.last_seen = Instant::now();
    }
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub download_rate: f64,
    pub peers: usize,
    pub progress: f64,
    pub eta: Option<Duration>,
}

impl DownloadProgress {
    pub fn from_download(download: &Download) -> Self {
        let progress = download.progress();
        let download_rate = download.download_rate();

        let eta = if download_rate > 0.0 {
            let remaining_bytes = download.total_size - download.downloaded_bytes;
            let seconds = remaining_bytes as f64 / download_rate;
            Some(Duration::from_secs_f64(seconds))
        } else {
            None
        };

        Self {
            downloaded_bytes: download.downloaded_bytes,
            total_bytes: download.total_size,
            download_rate,
            peers: download.available_peers.len(),
            progress,
            eta,
        }
    }
}

/// Main chunk distribution manager
pub struct ChunkDistributor {
    /// Configuration
    config: ChunkConfig,
    /// Active downloads
    active_downloads: Arc<RwLock<HashMap<ContentId, Download>>>,
    /// Active uploads
    active_uploads: Arc<RwLock<HashMap<ContentId, Vec<Upload>>>>,
    /// Peer statistics for tit-for-tat
    peer_stats: Arc<RwLock<HashMap<PeerInfo, PeerStats>>>,
    /// Local content storage (for serving chunks)
    local_content: Arc<RwLock<HashMap<ContentId, Vec<u8>>>>,
    /// Last optimistic unchoke time
    last_optimistic_unchoke: Arc<RwLock<Instant>>,
}

impl ChunkDistributor {
    /// Create a new chunk distributor
    pub async fn new(config: ChunkConfig) -> Result<Self> {
        info!(
            "Initializing chunk distributor with strategy: {:?}",
            config.strategy
        );

        Ok(Self {
            config,
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            active_uploads: Arc::new(RwLock::new(HashMap::new())),
            peer_stats: Arc::new(RwLock::new(HashMap::new())),
            local_content: Arc::new(RwLock::new(HashMap::new())),
            last_optimistic_unchoke: Arc::new(RwLock::new(Instant::now())),
        })
    }

    /// Store content locally (for serving to other peers)
    pub async fn store_content(&self, cid: ContentId, content: Vec<u8>) -> Result<()> {
        info!("Storing content locally: {}", cid);
        let mut local = self.local_content.write().await;
        local.insert(cid, content);
        Ok(())
    }

    /// Download content from multiple peers
    pub async fn download_content(
        &mut self,
        cid: ContentId,
        providers: Vec<PeerInfo>,
    ) -> Result<Vec<u8>> {
        if providers.is_empty() {
            return Err(anyhow!("No providers available for content {}", cid));
        }

        info!(
            "Starting download for {} from {} providers",
            cid,
            providers.len()
        );

        // Check if already downloading
        {
            let downloads = self.active_downloads.read().await;
            if downloads.contains_key(&cid) {
                return Err(anyhow!("Download already in progress for {}", cid));
            }
        }

        // For this implementation, we'll simulate the download
        // In a real implementation, this would query peers for chunk availability
        // and download chunks in parallel

        let content = self.download_from_providers(&cid, &providers).await?;

        // Store locally after successful download
        self.store_content(cid.clone(), content.clone()).await?;

        Ok(content)
    }

    /// Internal download implementation
    async fn download_from_providers(
        &self,
        cid: &ContentId,
        providers: &[PeerInfo],
    ) -> Result<Vec<u8>> {
        // Query first provider for content metadata
        let (total_size, total_chunks) = self.query_content_metadata(cid, &providers[0]).await?;

        debug!(
            "Content metadata: {} bytes, {} chunks",
            total_size, total_chunks
        );

        // Create download state
        let mut download = Download::new(
            cid.clone(),
            total_chunks,
            total_size,
            providers.to_vec(),
            self.config.strategy,
        );

        // Query chunk availability from all peers
        for peer in providers {
            let available_chunks = self.query_peer_chunks(cid, peer).await?;
            for chunk_idx in available_chunks {
                *download.chunk_availability.entry(chunk_idx).or_insert(0) += 1;
            }
        }

        // Store download state
        {
            let mut downloads = self.active_downloads.write().await;
            downloads.insert(cid.clone(), download);
        }

        // Download chunks in parallel
        self.download_chunks_parallel(cid).await?;

        // Get final download state and assemble content
        let downloads = self.active_downloads.read().await;
        let download = downloads
            .get(cid)
            .ok_or_else(|| anyhow!("Download state lost"))?;

        let content = download.assemble()?;

        // Verify content CID
        let computed_cid = ContentId::new(&content);
        if computed_cid != *cid {
            return Err(anyhow!("Content verification failed: CID mismatch"));
        }

        // Clean up download state
        drop(downloads);
        let mut downloads = self.active_downloads.write().await;
        downloads.remove(cid);

        info!("Download complete: {} ({} bytes)", cid, content.len());

        Ok(content)
    }

    /// Download chunks in parallel from multiple peers
    async fn download_chunks_parallel(&self, cid: &ContentId) -> Result<()> {
        let max_concurrent =
            self.config.chunks_per_peer * self.get_download(cid).await?.available_peers.len();

        let mut tasks: Vec<tokio::task::JoinHandle<Result<(u32, Option<Chunk>, PeerInfo)>>> =
            Vec::new();

        loop {
            // Get next chunks to download
            let mut downloads = self.active_downloads.write().await;
            let download = downloads
                .get_mut(cid)
                .ok_or_else(|| anyhow!("Download not found"))?;

            if download.is_complete() {
                break;
            }

            // Start new downloads up to max concurrent
            while tasks.len() < max_concurrent {
                if let Some(chunk_idx) = download.next_chunk() {
                    // Select best peer for this chunk
                    let peer = self.select_peer_for_chunk(download, chunk_idx).await?;

                    download.active_requests.insert(chunk_idx, peer.clone());

                    // Spawn download task
                    let cid = cid.clone();
                    let peer_clone = peer.clone();
                    let distributor = self.clone_for_task();
                    let timeout = self.config.request_timeout;

                    let task = tokio::spawn(async move {
                        match tokio::time::timeout(
                            timeout,
                            distributor.download_chunk(&cid, chunk_idx, &peer_clone),
                        )
                        .await
                        {
                            Ok(Ok(chunk)) => Ok((chunk_idx, Some(chunk), peer_clone)),
                            Ok(Err(e)) => {
                                warn!("Chunk {} download failed: {}", chunk_idx, e);
                                Ok((chunk_idx, None, peer_clone))
                            }
                            Err(_) => {
                                warn!("Chunk {} download timeout", chunk_idx);
                                Ok((chunk_idx, None, peer_clone))
                            }
                        }
                    });

                    tasks.push(task);
                } else {
                    break;
                }
            }

            drop(downloads);

            if tasks.is_empty() {
                break;
            }

            // Wait for any task to complete
            let (result, _idx, remaining) = futures::future::select_all(tasks).await;
            tasks = remaining;

            match result {
                Ok(Ok((chunk_idx, Some(chunk), peer))) => {
                    // Verify chunk
                    if !chunk.verify() {
                        error!("Chunk {} verification failed, re-requesting", chunk_idx);
                        let mut downloads = self.active_downloads.write().await;
                        if let Some(download) = downloads.get_mut(cid) {
                            download.mark_failed(chunk_idx);
                        }
                        continue;
                    }

                    // Mark as downloaded
                    let mut downloads = self.active_downloads.write().await;
                    if let Some(download) = downloads.get_mut(cid) {
                        download.mark_downloaded(chunk);

                        // Update peer stats
                        let chunk_size = download
                            .chunks
                            .get(&chunk_idx)
                            .map(|c| c.size())
                            .unwrap_or(0);
                        drop(downloads);

                        let mut stats = self.peer_stats.write().await;
                        stats
                            .entry(peer)
                            .or_insert_with_key(|p| PeerStats::new(p.clone()))
                            .record_download(chunk_size as u64);
                    }
                }
                Ok(Ok((chunk_idx, None, _peer))) => {
                    // Failed download, mark for retry
                    let mut downloads = self.active_downloads.write().await;
                    if let Some(download) = downloads.get_mut(cid) {
                        download.mark_failed(chunk_idx);
                    }
                }
                Ok(Err(e)) => {
                    error!("Task error: {}", e);
                }
                Err(e) => {
                    error!("Task panic: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Select best peer for downloading a chunk (tit-for-tat)
    async fn select_peer_for_chunk(
        &self,
        download: &Download,
        _chunk_idx: u32,
    ) -> Result<PeerInfo> {
        let stats = self.peer_stats.read().await;

        // Optimistic unchoking: occasionally give random peers a chance
        let mut last_unchoke = self.last_optimistic_unchoke.write().await;
        if last_unchoke.elapsed() > self.config.optimistic_unchoke_interval {
            *last_unchoke = Instant::now();
            if let Some(peer) = download.available_peers.choose(&mut rand::thread_rng()) {
                debug!("Optimistic unchoke: selecting random peer");
                return Ok(peer.clone());
            }
        }

        // Select peer with best upload ratio
        let mut best_peer = None;
        let mut best_ratio = 0.0;

        for peer in &download.available_peers {
            let ratio = stats.get(peer).map(|s| s.ratio()).unwrap_or(f64::INFINITY);

            if ratio > best_ratio {
                best_ratio = ratio;
                best_peer = Some(peer.clone());
            }
        }

        best_peer.ok_or_else(|| anyhow!("No suitable peer found"))
    }

    /// Download a single chunk from a peer
    async fn download_chunk(
        &self,
        cid: &ContentId,
        chunk_idx: u32,
        peer: &PeerInfo,
    ) -> Result<Chunk> {
        debug!(
            "Downloading chunk {} from peer {:?}",
            chunk_idx, peer.peer_id
        );

        // In a real implementation, this would make a network request
        // For now, we'll simulate by reading from local storage
        let local = self.local_content.read().await;
        let content = local
            .get(cid)
            .ok_or_else(|| anyhow!("Content not found locally (simulation)"))?;

        let chunk_size = self.config.chunk_size;
        let start = (chunk_idx as usize) * chunk_size;
        let end = std::cmp::min(start + chunk_size, content.len());

        if start >= content.len() {
            return Err(anyhow!("Chunk index out of bounds"));
        }

        let chunk_data = content[start..end].to_vec();
        let chunk = Chunk::new(chunk_idx, chunk_data);

        Ok(chunk)
    }

    /// Query content metadata from a peer
    async fn query_content_metadata(
        &self,
        cid: &ContentId,
        _peer: &PeerInfo,
    ) -> Result<(u64, u32)> {
        // In a real implementation, this would query the peer
        // For now, read from local storage if available
        let local = self.local_content.read().await;
        let content = local
            .get(cid)
            .ok_or_else(|| anyhow!("Content not found for metadata query"))?;

        let total_size = content.len() as u64;
        let total_chunks =
            ((total_size as usize + self.config.chunk_size - 1) / self.config.chunk_size) as u32;

        Ok((total_size, total_chunks))
    }

    /// Query which chunks a peer has available
    async fn query_peer_chunks(&self, cid: &ContentId, _peer: &PeerInfo) -> Result<Vec<u32>> {
        // In a real implementation, this would query the peer
        // For simulation, assume peer has all chunks if content is local
        let local = self.local_content.read().await;
        if let Some(content) = local.get(cid) {
            let total_chunks =
                ((content.len() + self.config.chunk_size - 1) / self.config.chunk_size) as u32;
            Ok((0..total_chunks).collect())
        } else {
            Ok(vec![])
        }
    }

    /// Get download progress
    pub async fn download_progress(&self, cid: &ContentId) -> Option<DownloadProgress> {
        let downloads = self.active_downloads.read().await;
        downloads.get(cid).map(DownloadProgress::from_download)
    }

    /// Serve a chunk to a peer (upload)
    pub async fn serve_chunk(
        &mut self,
        cid: ContentId,
        chunk_index: u32,
        peer: PeerInfo,
    ) -> Result<Chunk> {
        debug!("Serving chunk {} to peer {:?}", chunk_index, peer.peer_id);

        // Get content from local storage
        let local = self.local_content.read().await;
        let content = local
            .get(&cid)
            .ok_or_else(|| anyhow!("Content not available locally"))?;

        let chunk_size = self.config.chunk_size;
        let start = (chunk_index as usize) * chunk_size;
        let end = std::cmp::min(start + chunk_size, content.len());

        if start >= content.len() {
            return Err(anyhow!("Chunk index out of bounds"));
        }

        let chunk_data = content[start..end].to_vec();
        let chunk = Chunk::new(chunk_index, chunk_data);

        // Record upload
        let mut uploads = self.active_uploads.write().await;
        let upload_list = uploads.entry(cid.clone()).or_insert_with(Vec::new);

        if let Some(upload) = upload_list.iter_mut().find(|u| u.peer == peer) {
            upload.record_upload(chunk.size());
        } else {
            let mut upload = Upload::new(cid.clone(), peer.clone());
            upload.record_upload(chunk.size());
            upload_list.push(upload);
        }

        // Update peer stats
        let mut stats = self.peer_stats.write().await;
        stats
            .entry(peer)
            .or_insert_with_key(|p| PeerStats::new(p.clone()))
            .record_upload(chunk.size() as u64);

        Ok(chunk)
    }

    /// Get current download state (for testing)
    async fn get_download(&self, cid: &ContentId) -> Result<Download> {
        let downloads = self.active_downloads.read().await;
        downloads
            .get(cid)
            .cloned()
            .ok_or_else(|| anyhow!("Download not found"))
    }

    /// Clone distributor for async tasks (simplified)
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_downloads: Arc::clone(&self.active_downloads),
            active_uploads: Arc::clone(&self.active_uploads),
            peer_stats: Arc::clone(&self.peer_stats),
            local_content: Arc::clone(&self.local_content),
            last_optimistic_unchoke: Arc::clone(&self.last_optimistic_unchoke),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let chunk = Chunk::new(0, data.clone());

        assert_eq!(chunk.index, 0);
        assert_eq!(chunk.data, data);
        assert!(chunk.verify());
    }

    #[test]
    fn test_chunk_verification() {
        let chunk = Chunk::new(0, vec![1, 2, 3]);
        assert!(chunk.verify());

        // Corrupt the data
        let mut corrupted = chunk.clone();
        corrupted.data[0] = 99;
        assert!(!corrupted.verify());
    }

    #[test]
    fn test_download_progress() {
        let cid = ContentId::new(b"test");
        let peers = vec![];
        let mut download = Download::new(cid, 10, 1000, peers, DistributionStrategy::RarestFirst);

        assert_eq!(download.progress(), 0.0);
        assert!(!download.is_complete());

        // Download some chunks
        for i in 0..5 {
            download.mark_downloaded(Chunk::new(i, vec![0; 100]));
        }

        assert_eq!(download.progress(), 0.5);
        assert!(!download.is_complete());

        // Download remaining chunks
        for i in 5..10 {
            download.mark_downloaded(Chunk::new(i, vec![0; 100]));
        }

        assert_eq!(download.progress(), 1.0);
        assert!(download.is_complete());
    }

    #[test]
    fn test_peer_stats() {
        let peer = PeerInfo::new(PeerId("peer1".to_string()), "addr1".to_string());
        let mut stats = PeerStats::new(peer);

        stats.record_download(1000);
        stats.record_upload(800);

        assert_eq!(stats.downloaded, 1000);
        assert_eq!(stats.uploaded, 800);
        assert_eq!(stats.ratio(), 0.8);
    }

    #[tokio::test]
    async fn test_distributor_creation() {
        let config = ChunkConfig::default();
        let distributor = ChunkDistributor::new(config).await.unwrap();

        assert!(distributor.active_downloads.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_store_and_serve() {
        let config = ChunkConfig::default();
        let mut distributor = ChunkDistributor::new(config).await.unwrap();

        let content = b"Hello, distributed world!".to_vec();
        let cid = ContentId::new(&content);

        // Store content
        distributor
            .store_content(cid.clone(), content.clone())
            .await
            .unwrap();

        // Serve a chunk
        let peer = PeerInfo::new(PeerId("peer1".to_string()), "addr1".to_string());
        let chunk = distributor.serve_chunk(cid, 0, peer).await.unwrap();

        assert_eq!(chunk.index, 0);
        assert!(chunk.verify());
    }
}
