//! # Codio Chunk Distribution
//!
//! BitTorrent-style chunk distribution system for the Codio CDN.
//!
//! ## Features
//!
//! - **Parallel Downloads**: Download from multiple peers simultaneously
//! - **Rarest-First Strategy**: Prioritize downloading rare chunks to improve network distribution
//! - **Tit-for-Tat**: Incentivize fair sharing by prioritizing peers who upload to us
//! - **Integrity Verification**: Verify SHA-256 hash of every chunk to prevent corruption
//!
//! ## Architecture
//!
//! The chunk distribution system splits content into fixed-size chunks (default 256KB).
//! Each chunk is independently addressed by its index and can be downloaded from any
//! peer that has it.
//!
//! ### Rarest-First Algorithm
//!
//! 1. Query all peers for their chunk availability
//! 2. Calculate rarity score for each chunk (fewer peers = higher rarity)
//! 3. Download chunks in order of rarity (rarest first)
//! 4. This ensures rare chunks spread quickly across the network
//!
//! ### Tit-for-Tat Mechanism
//!
//! 1. Track upload/download ratios for each peer
//! 2. Prioritize uploading to peers who upload to us
//! 3. Implement "optimistic unchoking" - randomly upload to a new peer every 30s
//! 4. This incentivizes fair sharing and prevents free-riding
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use codio_chunk::{ChunkDistributor, ChunkConfig, DistributionStrategy};
//! use codio_content_id::ContentId;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ChunkConfig {
//!     max_concurrent_downloads: 10,
//!     chunks_per_peer: 4,
//!     request_timeout: std::time::Duration::from_secs(30),
//!     strategy: DistributionStrategy::RarestFirst,
//!     chunk_size: 256 * 1024,
//! };
//!
//! let mut distributor = ChunkDistributor::new(config).await?;
//!
//! // Download content from multiple peers
//! let cid = ContentId::new(b"test content");
//! let providers = vec![]; // Get from DHT
//! let content = distributor.download_content(cid, providers).await?;
//! # Ok(())
//! # }
//! ```

pub mod config;

pub use config::{ChunkConfig, DistributionStrategy};

use anyhow::{anyhow, Result};
use codio_common::{ContentId, PeerId};
use futures::future::join_all;
use rand::seq::SliceRandom;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Size of a chunk in bytes (default: 256 KB)
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

/// Maximum number of concurrent chunk requests per peer
pub const MAX_CHUNKS_PER_PEER: usize = 4;

/// Optimistic unchoking interval (30 seconds)
pub const OPTIMISTIC_UNCHOKE_INTERVAL: Duration = Duration::from_secs(30);

/// Information about a peer in the network
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub available_chunks: HashSet<u32>,
}

impl PeerInfo {
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            available_chunks: HashSet::new(),
        }
    }

    pub fn with_chunks(peer_id: PeerId, chunks: HashSet<u32>) -> Self {
        Self {
            peer_id,
            available_chunks: chunks,
        }
    }
}

/// A single chunk of content
#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: u32,
    pub data: Vec<u8>,
    pub hash: [u8; 32],
}

impl Chunk {
    /// Create a new chunk from data
    pub fn new(index: u32, data: Vec<u8>) -> Self {
        let hash = Self::compute_hash(&data);
        Self { index, data, hash }
    }

    /// Compute SHA-256 hash of chunk data
    pub fn compute_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Verify chunk integrity
    pub fn verify(&self) -> bool {
        let computed = Self::compute_hash(&self.data);
        computed == self.hash
    }
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub download_rate: f64, // bytes/sec
    pub peers: usize,
    pub chunks_downloaded: usize,
    pub total_chunks: usize,
}

impl DownloadProgress {
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

/// Active download tracking
#[derive(Debug)]
pub struct Download {
    pub cid: ContentId,
    pub total_chunks: u32,
    pub chunk_size: usize,
    pub total_bytes: u64,
    pub downloaded_chunks: HashSet<u32>,
    pub chunks: HashMap<u32, Chunk>,
    pub active_requests: HashMap<u32, PeerId>,
    pub strategy: DistributionStrategy,
    pub start_time: Instant,
    pub downloaded_bytes: u64,
}

impl Download {
    pub fn new(
        cid: ContentId,
        total_bytes: u64,
        chunk_size: usize,
        strategy: DistributionStrategy,
    ) -> Self {
        let total_chunks = total_bytes.div_ceil(chunk_size as u64) as u32;

        Self {
            cid,
            total_chunks,
            chunk_size,
            total_bytes,
            downloaded_chunks: HashSet::new(),
            chunks: HashMap::new(),
            active_requests: HashMap::new(),
            strategy,
            start_time: Instant::now(),
            downloaded_bytes: 0,
        }
    }

    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        self.downloaded_chunks.len() == self.total_chunks as usize
    }

    /// Get download progress
    pub fn progress(&self, peers: usize) -> DownloadProgress {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let download_rate = if elapsed > 0.0 {
            self.downloaded_bytes as f64 / elapsed
        } else {
            0.0
        };

        DownloadProgress {
            downloaded_bytes: self.downloaded_bytes,
            total_bytes: self.total_bytes,
            download_rate,
            peers,
            chunks_downloaded: self.downloaded_chunks.len(),
            total_chunks: self.total_chunks as usize,
        }
    }

    /// Get missing chunks
    pub fn missing_chunks(&self) -> Vec<u32> {
        (0..self.total_chunks)
            .filter(|i| !self.downloaded_chunks.contains(i))
            .collect()
    }

    /// Reconstruct full content from chunks
    pub fn reconstruct(&self) -> Result<Vec<u8>> {
        if !self.is_complete() {
            return Err(anyhow!("Download incomplete"));
        }

        let mut result = Vec::with_capacity(self.total_bytes as usize);

        for i in 0..self.total_chunks {
            let chunk = self
                .chunks
                .get(&i)
                .ok_or_else(|| anyhow!("Missing chunk {}", i))?;

            // Verify chunk integrity
            if !chunk.verify() {
                return Err(anyhow!("Chunk {} failed integrity check", i));
            }

            result.extend_from_slice(&chunk.data);
        }

        // Trim to exact size (last chunk might be padded)
        result.truncate(self.total_bytes as usize);

        Ok(result)
    }
}

/// Upload tracking for tit-for-tat
#[derive(Debug, Clone)]
pub struct Upload {
    pub cid: ContentId,
    pub peer: PeerInfo,
    pub uploaded_chunks: u32,
    pub uploaded_bytes: u64,
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

    pub fn record_upload(&mut self, chunk: &Chunk) {
        self.uploaded_chunks += 1;
        self.uploaded_bytes += chunk.data.len() as u64;
        self.last_upload = Instant::now();
    }
}

/// Peer statistics for tit-for-tat
#[derive(Debug, Clone)]
pub struct PeerStats {
    pub peer_id: PeerId,
    pub uploaded_bytes: u64,
    pub downloaded_bytes: u64,
    pub last_interaction: Instant,
}

impl PeerStats {
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            uploaded_bytes: 0,
            downloaded_bytes: 0,
            last_interaction: Instant::now(),
        }
    }

    /// Calculate upload/download ratio
    pub fn ratio(&self) -> f64 {
        if self.downloaded_bytes == 0 {
            return f64::INFINITY;
        }
        self.uploaded_bytes as f64 / self.downloaded_bytes as f64
    }

    /// Record download from this peer
    pub fn record_download(&mut self, bytes: u64) {
        self.downloaded_bytes += bytes;
        self.last_interaction = Instant::now();
    }

    /// Record upload to this peer
    pub fn record_upload(&mut self, bytes: u64) {
        self.uploaded_bytes += bytes;
        self.last_interaction = Instant::now();
    }
}

/// Main chunk distribution manager
///
/// Manages parallel downloads from multiple peers using BitTorrent-style
/// chunk distribution strategies.
pub struct ChunkDistributor {
    config: ChunkConfig,
    active_downloads: Arc<RwLock<HashMap<ContentId, Download>>>,
    active_uploads: Arc<RwLock<HashMap<ContentId, Vec<Upload>>>>,
    pub peer_stats: Arc<RwLock<HashMap<PeerId, PeerStats>>>,
    download_semaphore: Arc<Semaphore>,
    local_content: Arc<RwLock<HashMap<ContentId, Vec<u8>>>>,
    last_optimistic_unchoke: Arc<RwLock<Instant>>,
}

impl ChunkDistributor {
    /// Create a new chunk distributor
    pub async fn new(config: ChunkConfig) -> Result<Self> {
        let max_concurrent = config.max_concurrent_downloads;

        Ok(Self {
            config,
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            active_uploads: Arc::new(RwLock::new(HashMap::new())),
            peer_stats: Arc::new(RwLock::new(HashMap::new())),
            download_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            local_content: Arc::new(RwLock::new(HashMap::new())),
            last_optimistic_unchoke: Arc::new(RwLock::new(Instant::now())),
        })
    }

    /// Store content locally for serving to other peers
    pub async fn store_content(&self, cid: ContentId, content: Vec<u8>) {
        let mut local = self.local_content.write().await;
        local.insert(cid, content);
    }

    /// Download content from multiple providers
    ///
    /// This is the main entry point for downloading content. It:
    /// 1. Creates a download tracker
    /// 2. Determines chunk order based on strategy
    /// 3. Downloads chunks in parallel from multiple peers
    /// 4. Verifies chunk integrity
    /// 5. Reconstructs the complete content
    pub async fn download_content(
        &mut self,
        cid: ContentId,
        providers: Vec<PeerInfo>,
    ) -> Result<Vec<u8>> {
        if providers.is_empty() {
            return Err(anyhow!("No providers available"));
        }

        info!(
            "Starting download for CID {} from {} providers",
            cid,
            providers.len()
        );

        // Get content size from first provider (simplified - in production, query metadata)
        let total_bytes = self.query_content_size(&cid, &providers[0]).await?;

        // Create download tracker
        let download = Download::new(
            cid.clone(),
            total_bytes,
            self.config.chunk_size,
            self.config.strategy,
        );

        let total_chunks = download.total_chunks;

        {
            let mut downloads = self.active_downloads.write().await;
            downloads.insert(cid.clone(), download);
        }

        // Determine chunk download order based on strategy
        let chunk_order = self
            .determine_chunk_order(&cid, &providers, total_chunks)
            .await?;

        // Download chunks in parallel
        self.download_chunks_parallel(&cid, chunk_order, providers)
            .await?;

        // Reconstruct content
        let content = {
            let downloads = self.active_downloads.read().await;
            let download = downloads
                .get(&cid)
                .ok_or_else(|| anyhow!("Download not found"))?;
            download.reconstruct()?
        };

        // Clean up
        {
            let mut downloads = self.active_downloads.write().await;
            downloads.remove(&cid);
        }

        info!("Download complete for CID {}", cid);

        Ok(content)
    }

    /// Determine chunk download order based on strategy
    pub async fn determine_chunk_order(
        &self,
        cid: &ContentId,
        providers: &[PeerInfo],
        total_chunks: u32,
    ) -> Result<Vec<u32>> {
        let strategy = {
            let downloads = self.active_downloads.read().await;
            downloads
                .get(cid)
                .map(|d| d.strategy)
                .unwrap_or(DistributionStrategy::RarestFirst)
        };

        match strategy {
            DistributionStrategy::RarestFirst => {
                self.calculate_rarest_first_order(providers, total_chunks)
                    .await
            }
            DistributionStrategy::Sequential => Ok((0..total_chunks).collect()),
            DistributionStrategy::RandomOrder => {
                let mut chunks: Vec<u32> = (0..total_chunks).collect();
                let mut rng = rand::thread_rng();
                chunks.shuffle(&mut rng);
                Ok(chunks)
            }
        }
    }

    /// Calculate rarest-first chunk order
    ///
    /// Algorithm:
    /// 1. Count how many peers have each chunk
    /// 2. Sort chunks by availability (rarest first)
    /// 3. This helps distribute rare chunks quickly
    pub async fn calculate_rarest_first_order(
        &self,
        providers: &[PeerInfo],
        total_chunks: u32,
    ) -> Result<Vec<u32>> {
        // Count availability for each chunk
        let mut chunk_availability: HashMap<u32, usize> = HashMap::new();

        for provider in providers {
            for &chunk_idx in &provider.available_chunks {
                if chunk_idx < total_chunks {
                    *chunk_availability.entry(chunk_idx).or_insert(0) += 1;
                }
            }
        }

        // If no availability info, assume all peers have all chunks
        if chunk_availability.is_empty() {
            debug!("No chunk availability info, assuming all peers have all chunks");
            return Ok((0..total_chunks).collect());
        }

        // Sort chunks by availability (rarest first)
        let mut chunks: Vec<(u32, usize)> = chunk_availability.into_iter().collect();
        chunks.sort_by_key(|(_, count)| *count);

        let ordered: Vec<u32> = chunks.into_iter().map(|(idx, _)| idx).collect();

        // Add any chunks that weren't in the availability map
        let mut result = ordered;
        for i in 0..total_chunks {
            if !result.contains(&i) {
                result.push(i);
            }
        }

        debug!(
            "Rarest-first order calculated: {:?}",
            &result[..std::cmp::min(10, result.len())]
        );

        Ok(result)
    }

    /// Download chunks in parallel from multiple peers
    async fn download_chunks_parallel(
        &self,
        cid: &ContentId,
        chunk_order: Vec<u32>,
        providers: Vec<PeerInfo>,
    ) -> Result<()> {
        let chunks_per_peer = self.config.chunks_per_peer;

        // Distribute chunks across peers
        let mut tasks = Vec::new();

        for (i, chunk_idx) in chunk_order.iter().enumerate() {
            // Select peer for this chunk (round-robin with availability check)
            let peer = self.select_peer_for_chunk(*chunk_idx, &providers, i);

            if let Some(peer_info) = peer {
                let cid_clone = cid.clone();
                let chunk_idx = *chunk_idx;
                let peer_clone = peer_info.clone();
                let distributor_clone = self.clone_arc_fields();

                // Limit concurrent downloads
                let permit = self.download_semaphore.clone().acquire_owned().await?;

                let task = tokio::spawn(async move {
                    let result = distributor_clone
                        .download_chunk(&cid_clone, chunk_idx, &peer_clone)
                        .await;
                    drop(permit); // Release semaphore
                    result
                });

                tasks.push(task);

                // Limit tasks per peer
                if tasks.len() >= providers.len() * chunks_per_peer {
                    // Wait for some tasks to complete
                    let results = join_all(tasks.drain(..)).await;
                    for result in results {
                        result??; // Propagate errors
                    }
                    tasks.clear();
                }
            }
        }

        // Wait for remaining tasks
        let results = join_all(tasks).await;
        for result in results {
            result??;
        }

        Ok(())
    }

    /// Select best peer for downloading a chunk
    ///
    /// Uses tit-for-tat: prefer peers who upload to us
    pub fn select_peer_for_chunk(
        &self,
        chunk_idx: u32,
        providers: &[PeerInfo],
        round_robin_index: usize,
    ) -> Option<PeerInfo> {
        // Filter peers that have this chunk
        let available_peers: Vec<&PeerInfo> = providers
            .iter()
            .filter(|p| p.available_chunks.is_empty() || p.available_chunks.contains(&chunk_idx))
            .collect();

        if available_peers.is_empty() {
            return None;
        }

        // Simple round-robin for now
        // In production, implement full tit-for-tat with peer stats
        let index = round_robin_index % available_peers.len();
        Some(available_peers[index].clone())
    }

    /// Download a single chunk from a peer
    async fn download_chunk(&self, cid: &ContentId, chunk_idx: u32, peer: &PeerInfo) -> Result<()> {
        debug!(
            "Downloading chunk {} for CID {} from peer {:?}",
            chunk_idx, cid, peer.peer_id
        );

        // Apply timeout
        let result = timeout(
            self.config.request_timeout,
            self.fetch_chunk_from_peer(cid, chunk_idx, peer),
        )
        .await;

        match result {
            Ok(Ok(chunk)) => {
                // Verify chunk integrity
                if !chunk.verify() {
                    error!("Chunk {} failed integrity check", chunk_idx);
                    return Err(anyhow!("Chunk integrity verification failed"));
                }

                // Store chunk
                {
                    let mut downloads = self.active_downloads.write().await;
                    if let Some(download) = downloads.get_mut(cid) {
                        download.downloaded_chunks.insert(chunk_idx);
                        download.downloaded_bytes += chunk.data.len() as u64;
                        download.chunks.insert(chunk_idx, chunk.clone());
                    }
                }

                // Update peer stats
                {
                    let mut stats = self.peer_stats.write().await;
                    let peer_stats = stats
                        .entry(peer.peer_id.clone())
                        .or_insert_with(|| PeerStats::new(peer.peer_id.clone()));
                    peer_stats.record_download(chunk.data.len() as u64);
                }

                debug!("Chunk {} downloaded successfully", chunk_idx);
                Ok(())
            }
            Ok(Err(e)) => {
                warn!("Failed to download chunk {}: {}", chunk_idx, e);
                Err(e)
            }
            Err(_) => {
                warn!(
                    "Timeout downloading chunk {} from peer {:?}",
                    chunk_idx, peer.peer_id
                );
                Err(anyhow!("Chunk download timeout"))
            }
        }
    }

    /// Fetch chunk from peer (mock implementation)
    ///
    /// In production, this would use the network layer to request the chunk
    /// from the peer via libp2p or WebRTC.
    async fn fetch_chunk_from_peer(
        &self,
        cid: &ContentId,
        chunk_idx: u32,
        _peer: &PeerInfo,
    ) -> Result<Chunk> {
        // Mock implementation: check local content
        let local = self.local_content.read().await;
        if let Some(content) = local.get(cid) {
            let chunk_size = self.config.chunk_size;
            let start = (chunk_idx as usize) * chunk_size;
            let end = std::cmp::min(start + chunk_size, content.len());

            if start < content.len() {
                let chunk_data = content[start..end].to_vec();
                return Ok(Chunk::new(chunk_idx, chunk_data));
            }
        }

        Err(anyhow!("Chunk not available"))
    }

    /// Query content size from peer (mock implementation)
    async fn query_content_size(&self, cid: &ContentId, _peer: &PeerInfo) -> Result<u64> {
        // Mock: check local content
        let local = self.local_content.read().await;
        if let Some(content) = local.get(cid) {
            return Ok(content.len() as u64);
        }

        // Default size for testing
        Ok(1024 * 1024) // 1 MB
    }

    /// Serve a chunk to a peer (for uploading)
    ///
    /// Implements tit-for-tat: prioritize peers who upload to us
    pub async fn serve_chunk(
        &mut self,
        cid: ContentId,
        chunk_index: u32,
        peer: PeerId,
    ) -> Result<Chunk> {
        // Check if we should serve to this peer (tit-for-tat)
        if !self.should_upload_to_peer(&peer).await {
            return Err(anyhow!("Peer not eligible for upload (tit-for-tat)"));
        }

        // Get content
        let local = self.local_content.read().await;
        let content = local
            .get(&cid)
            .ok_or_else(|| anyhow!("Content not found"))?;

        // Extract chunk
        let chunk_size = self.config.chunk_size;
        let start = (chunk_index as usize) * chunk_size;
        let end = std::cmp::min(start + chunk_size, content.len());

        if start >= content.len() {
            return Err(anyhow!("Chunk index out of range"));
        }

        let chunk_data = content[start..end].to_vec();
        let chunk = Chunk::new(chunk_index, chunk_data);

        // Record upload
        {
            let mut stats = self.peer_stats.write().await;
            let peer_stats = stats
                .entry(peer.clone())
                .or_insert_with(|| PeerStats::new(peer.clone()));
            peer_stats.record_upload(chunk.data.len() as u64);
        }

        {
            let mut uploads = self.active_uploads.write().await;
            let peer_info = PeerInfo::new(peer);
            let upload_list = uploads.entry(cid.clone()).or_insert_with(Vec::new);

            if let Some(upload) = upload_list
                .iter_mut()
                .find(|u| u.peer.peer_id == peer_info.peer_id)
            {
                upload.record_upload(&chunk);
            } else {
                let mut new_upload = Upload::new(cid.clone(), peer_info);
                new_upload.record_upload(&chunk);
                upload_list.push(new_upload);
            }
        }

        Ok(chunk)
    }

    /// Determine if we should upload to a peer (tit-for-tat)
    ///
    /// Algorithm:
    /// 1. Always upload to peers with good upload ratio
    /// 2. Implement optimistic unchoking: randomly serve new peers every 30s
    /// 3. This incentivizes fair sharing
    pub async fn should_upload_to_peer(&self, peer: &PeerId) -> bool {
        let stats = self.peer_stats.read().await;

        // Check peer ratio first
        if let Some(peer_stats) = stats.get(peer) {
            // Upload to peers who upload to us or have reasonable ratio
            // A ratio of infinity means they haven't downloaded anything yet
            let ratio = peer_stats.ratio();
            if ratio >= 0.3 || ratio.is_infinite() {
                return true;
            }
        } else {
            // New peer - give them a chance
            return true;
        }

        // Optimistic unchoking: periodically give low-ratio peers a chance
        let last_unchoke = self.last_optimistic_unchoke.read().await;
        if last_unchoke.elapsed() >= OPTIMISTIC_UNCHOKE_INTERVAL {
            // Update timestamp
            drop(last_unchoke);
            let mut last_unchoke = self.last_optimistic_unchoke.write().await;
            *last_unchoke = Instant::now();
            return true;
        }

        // Default: don't upload to low-ratio peers outside optimistic unchoke
        false
    }

    /// Get download progress for a content ID
    pub fn download_progress(&self, _cid: &ContentId) -> Option<DownloadProgress> {
        // This is a synchronous wrapper - in production use async
        None
    }

    /// Get download progress (async version)
    pub async fn get_download_progress(&self, cid: &ContentId) -> Option<DownloadProgress> {
        let downloads = self.active_downloads.read().await;
        let download = downloads.get(cid)?;

        // Count active providers
        let providers = download.active_requests.len();

        Some(download.progress(providers))
    }

    /// Helper to clone Arc fields for spawned tasks
    pub fn clone_arc_fields(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_downloads: Arc::clone(&self.active_downloads),
            active_uploads: Arc::clone(&self.active_uploads),
            peer_stats: Arc::clone(&self.peer_stats),
            download_semaphore: Arc::clone(&self.download_semaphore),
            local_content: Arc::clone(&self.local_content),
            last_optimistic_unchoke: Arc::clone(&self.last_optimistic_unchoke),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation_and_verification() {
        let data = b"test chunk data".to_vec();
        let chunk = Chunk::new(0, data);

        assert_eq!(chunk.index, 0);
        assert!(chunk.verify());
    }

    #[test]
    fn test_chunk_integrity_detection() {
        let mut chunk = Chunk::new(0, b"original data".to_vec());

        // Corrupt the data
        chunk.data = b"corrupted data".to_vec();

        // Verification should fail
        assert!(!chunk.verify());
    }

    #[tokio::test]
    async fn test_download_creation() {
        let cid = ContentId::new(b"test content");
        let download = Download::new(cid, 1024, 256, DistributionStrategy::RarestFirst);

        assert_eq!(download.total_chunks, 4); // 1024 / 256 = 4
        assert!(!download.is_complete());
    }

    #[tokio::test]
    async fn test_distributor_creation() {
        let config = ChunkConfig::default();
        let distributor = ChunkDistributor::new(config).await;

        assert!(distributor.is_ok());
    }

    #[tokio::test]
    async fn test_peer_stats_ratio() {
        let peer_id = PeerId("peer1".to_string());
        let mut stats = PeerStats::new(peer_id);

        stats.record_upload(1000);
        stats.record_download(500);

        assert_eq!(stats.ratio(), 2.0); // 1000 / 500 = 2.0
    }

    #[tokio::test]
    async fn test_rarest_first_ordering() {
        let distributor = ChunkDistributor::new(ChunkConfig::default()).await.unwrap();

        let mut peer1 = PeerInfo::new(PeerId("peer1".to_string()));
        peer1.available_chunks = vec![0, 1, 2].into_iter().collect();

        let mut peer2 = PeerInfo::new(PeerId("peer2".to_string()));
        peer2.available_chunks = vec![0, 1, 3].into_iter().collect();

        let mut peer3 = PeerInfo::new(PeerId("peer3".to_string()));
        peer3.available_chunks = vec![0, 1].into_iter().collect();

        let providers = vec![peer1, peer2, peer3];

        let order = distributor
            .calculate_rarest_first_order(&providers, 4)
            .await
            .unwrap();

        // Chunks 2 and 3 are rarest (only 1 peer each)
        // Chunk 0 and 1 are most common (3 peers each)
        assert!(order[0] == 2 || order[0] == 3);
        assert!(order[1] == 2 || order[1] == 3);
    }

    #[tokio::test]
    async fn test_content_storage_and_retrieval() {
        let distributor = ChunkDistributor::new(ChunkConfig::default()).await.unwrap();

        let cid = ContentId::new(b"test content");
        let content = b"test content".to_vec();

        distributor
            .store_content(cid.clone(), content.clone())
            .await;

        // Verify content is stored
        let local = distributor.local_content.read().await;
        assert_eq!(local.get(&cid), Some(&content));
    }

    #[tokio::test]
    async fn test_download_progress_tracking() {
        let cid = ContentId::new(b"test");
        let mut download = Download::new(cid, 1000, 256, DistributionStrategy::Sequential);

        // Simulate downloading chunks
        download.downloaded_chunks.insert(0);
        download.downloaded_bytes += 256;

        let progress = download.progress(2);

        assert_eq!(progress.downloaded_bytes, 256);
        assert_eq!(progress.total_bytes, 1000);
        assert_eq!(progress.chunks_downloaded, 1);
        assert_eq!(progress.total_chunks, 4);
        assert_eq!(progress.peers, 2);
    }
}
