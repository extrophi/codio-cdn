//! # Codio Availability Tracker
//!
//! This crate provides peer availability tracking, reputation management, and load balancing
//! for the Codio decentralized CDN.
//!
//! ## Features
//!
//! - **Peer Metrics Tracking**: Monitor uptime, bandwidth, and success rates
//! - **Reputation System**: Score peers based on performance and reliability
//! - **Load Balancing**: Select optimal peers for content delivery
//! - **Gossip Protocol**: Share peer metrics across the network
//!
//! ## Reputation Algorithm
//!
//! The reputation score is calculated as:
//! ```text
//! reputation = (success_rate * success_weight) + (uptime_factor * uptime_weight)
//! where:
//!   success_rate = chunks_served / (chunks_served + chunks_failed)
//!   uptime_factor = min(uptime_seconds / 3600, 1.0)  // capped at 1 hour
//! ```
//!
//! Peers are classified as:
//! - **Reliable**: reputation > 0.8
//! - **Unreliable**: reputation < 0.3
//!
//! ## Example Usage
//!
//! ```no_run
//! use codio_tracker::{AvailabilityTracker, TrackerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = TrackerConfig::default();
//!     let tracker = AvailabilityTracker::new(config).await?;
//!
//!     // Track peer coming online
//!     let peer_id = codio_common::PeerId("peer123".to_string());
//!     tracker.record_peer_online(peer_id.clone()).await;
//!
//!     // Record successful download
//!     tracker.record_download_speed(peer_id.clone(), 1_000_000).await; // 1 MB/s
//!     tracker.record_upload_success(peer_id.clone()).await;
//!
//!     // Get peer metrics
//!     if let Some(metrics) = tracker.get_peer_metrics(&peer_id).await {
//!         println!("Reputation: {}", metrics.reputation_score);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod config;

pub use config::TrackerConfig;

use codio_common::{ContentId, PeerId};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Main availability tracker for managing peer metrics and content availability
pub struct AvailabilityTracker {
    /// Metrics for each tracked peer
    peer_metrics: RwLock<HashMap<PeerId, PeerMetrics>>,
    /// Map of content to available peers
    content_availability: RwLock<HashMap<ContentId, ContentAvailability>>,
    /// Configuration
    config: TrackerConfig,
    /// Last gossip update time
    last_gossip: RwLock<SystemTime>,
}

impl AvailabilityTracker {
    /// Create a new availability tracker
    pub async fn new(config: TrackerConfig) -> anyhow::Result<Self> {
        config.validate()?;

        Ok(Self {
            peer_metrics: RwLock::new(HashMap::new()),
            content_availability: RwLock::new(HashMap::new()),
            config,
            last_gossip: RwLock::new(SystemTime::now()),
        })
    }

    /// Record that a peer has come online
    pub async fn record_peer_online(&self, peer: PeerId) {
        let mut metrics = self.peer_metrics.write().await;

        if let Some(existing) = metrics.get_mut(&peer) {
            existing.last_seen = SystemTime::now();
            debug!("Peer {} updated as online", peer.0);
        } else {
            let new_metrics = PeerMetrics::new(peer.clone());
            metrics.insert(peer.clone(), new_metrics);
            info!("New peer {} tracked as online", peer.0);
        }
    }

    /// Record that a peer has gone offline
    pub async fn record_peer_offline(&self, peer: PeerId) {
        let mut metrics = self.peer_metrics.write().await;

        if let Some(existing) = metrics.get_mut(&peer) {
            existing.is_online = false;
            debug!("Peer {} marked as offline", peer.0);
        }
    }

    /// Get metrics for a specific peer
    pub async fn get_peer_metrics(&self, peer: &PeerId) -> Option<PeerMetrics> {
        let metrics = self.peer_metrics.read().await;
        metrics.get(peer).cloned()
    }

    /// Get all peer metrics
    pub async fn get_all_peer_metrics(&self) -> Vec<PeerMetrics> {
        let metrics = self.peer_metrics.read().await;
        metrics.values().cloned().collect()
    }

    /// Record that content is available from a specific peer
    pub async fn record_content_available(&self, cid: ContentId, peer: PeerId) {
        let mut availability = self.content_availability.write().await;

        availability
            .entry(cid.clone())
            .or_insert_with(|| ContentAvailability::new(cid.clone()))
            .add_peer(peer.clone());

        debug!("Content {} now available from peer {}", cid, peer.0);
    }

    /// Get list of peers that have specific content
    pub async fn get_available_peers(&self, cid: &ContentId) -> Vec<PeerId> {
        let availability = self.content_availability.read().await;

        availability
            .get(cid)
            .map(|ca| ca.peers.iter().map(|pi| pi.peer_id.clone()).collect())
            .unwrap_or_default()
    }

    /// Record download speed for a peer (used for rolling average)
    pub async fn record_download_speed(&self, peer: PeerId, bytes_per_sec: u64) {
        let mut metrics = self.peer_metrics.write().await;

        if let Some(existing) = metrics.get_mut(&peer) {
            existing.add_bandwidth_sample(bytes_per_sec, self.config.max_bandwidth_samples);
            debug!(
                "Peer {} download speed: {} bytes/sec (avg: {})",
                peer.0, bytes_per_sec, existing.download_speed
            );
        } else {
            // Create new peer if not exists
            let mut new_metrics = PeerMetrics::new(peer.clone());
            new_metrics.add_bandwidth_sample(bytes_per_sec, self.config.max_bandwidth_samples);
            metrics.insert(peer, new_metrics);
        }
    }

    /// Record successful upload/chunk served
    pub async fn record_upload_success(&self, peer: PeerId) {
        let mut metrics = self.peer_metrics.write().await;

        if let Some(existing) = metrics.get_mut(&peer) {
            existing.chunks_served += 1;
            existing.last_seen = SystemTime::now();
            existing.update_reputation(&self.config);
            debug!(
                "Peer {} upload success (total: {})",
                peer.0, existing.chunks_served
            );
        }
    }

    /// Record failed upload/chunk
    pub async fn record_upload_failure(&self, peer: PeerId) {
        let mut metrics = self.peer_metrics.write().await;

        if let Some(existing) = metrics.get_mut(&peer) {
            existing.chunks_failed += 1;
            existing.last_seen = SystemTime::now();
            existing.update_reputation(&self.config);
            warn!(
                "Peer {} upload failure (total: {})",
                peer.0, existing.chunks_failed
            );
        }
    }

    /// Select the best N peers for downloading content
    ///
    /// Selection criteria (in order of priority):
    /// 1. Peers must be online
    /// 2. Peers must have reputation >= min_reputation
    /// 3. Sort by reputation score (highest first)
    /// 4. Sort by bandwidth (highest first) as tiebreaker
    pub async fn select_best_peers(&self, cid: &ContentId, count: usize) -> Vec<PeerInfo> {
        let availability = self.content_availability.read().await;
        let metrics = self.peer_metrics.read().await;

        let available_peers = match availability.get(cid) {
            Some(ca) => ca.peers.clone(),
            None => return Vec::new(),
        };

        let mut scored_peers: Vec<(PeerInfo, PeerMetrics)> = available_peers
            .into_iter()
            .filter_map(|peer_info| {
                metrics
                    .get(&peer_info.peer_id)
                    .filter(|m| m.is_online && m.reputation_score >= self.config.min_reputation)
                    .map(|m| (peer_info, m.clone()))
            })
            .collect();

        // Sort by reputation (descending), then by bandwidth (descending)
        scored_peers.sort_by(|a, b| {
            b.1.reputation_score
                .partial_cmp(&a.1.reputation_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.1.download_speed.cmp(&a.1.download_speed))
        });

        scored_peers
            .into_iter()
            .take(count)
            .map(|(peer_info, _)| peer_info)
            .collect()
    }

    /// Clean up stale peers (those that haven't been seen within timeout period)
    pub async fn cleanup_stale_peers(&self) {
        let mut metrics = self.peer_metrics.write().await;
        let now = SystemTime::now();

        let stale_peers: Vec<PeerId> = metrics
            .iter()
            .filter_map(|(peer_id, metric)| {
                if let Ok(elapsed) = now.duration_since(metric.last_seen) {
                    if elapsed > self.config.peer_timeout {
                        return Some(peer_id.clone());
                    }
                }
                None
            })
            .collect();

        for peer in stale_peers {
            metrics.remove(&peer);
            info!("Removed stale peer: {}", peer.0);
        }
    }

    /// Get gossip update for sharing with neighbors
    ///
    /// Returns a summary of peer metrics to share with other nodes
    pub async fn get_gossip_update(&self) -> GossipUpdate {
        let metrics = self.peer_metrics.read().await;
        let mut last_gossip = self.last_gossip.write().await;

        let peer_summaries: Vec<PeerSummary> = metrics
            .values()
            .filter(|m| m.is_online)
            .map(|m| PeerSummary {
                peer_id: m.peer_id.clone(),
                reputation_score: m.reputation_score,
                download_speed: m.download_speed,
                last_seen: m.last_seen,
            })
            .collect();

        *last_gossip = SystemTime::now();

        GossipUpdate {
            peer_summaries,
            timestamp: SystemTime::now(),
        }
    }

    /// Apply gossip update received from another node
    pub async fn apply_gossip_update(&self, update: GossipUpdate) {
        let mut metrics = self.peer_metrics.write().await;

        for summary in update.peer_summaries {
            if let Some(existing) = metrics.get_mut(&summary.peer_id) {
                // Update only if the gossip data is more recent
                if summary.last_seen > existing.last_seen {
                    existing.last_seen = summary.last_seen;
                    // We keep our own measurements for reputation and speed
                    // but we can use gossip to discover new peers
                }
            } else {
                // New peer discovered via gossip
                let new_metrics = PeerMetrics {
                    peer_id: summary.peer_id.clone(),
                    uptime: Duration::from_secs(0),
                    last_seen: summary.last_seen,
                    download_speed: summary.download_speed,
                    bandwidth_samples: VecDeque::from([summary.download_speed]),
                    chunks_served: 0,
                    chunks_failed: 0,
                    reputation_score: summary.reputation_score,
                    is_online: true,
                    first_seen: SystemTime::now(),
                };
                metrics.insert(summary.peer_id.clone(), new_metrics);
                info!("Discovered new peer via gossip: {}", summary.peer_id.0);
            }
        }
    }

    /// Get statistics about the tracker
    pub async fn get_stats(&self) -> TrackerStats {
        let metrics = self.peer_metrics.read().await;
        let availability = self.content_availability.read().await;

        let total_peers = metrics.len();
        let online_peers = metrics.values().filter(|m| m.is_online).count();
        let reliable_peers = metrics.values().filter(|m| m.is_reliable()).count();
        let total_content = availability.len();

        let avg_reputation = if total_peers > 0 {
            metrics.values().map(|m| m.reputation_score).sum::<f64>() / total_peers as f64
        } else {
            0.0
        };

        TrackerStats {
            total_peers,
            online_peers,
            reliable_peers,
            total_content,
            avg_reputation,
        }
    }
}

/// Metrics for an individual peer
#[derive(Debug, Clone)]
pub struct PeerMetrics {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Total uptime duration
    pub uptime: Duration,
    /// Last time peer was seen
    pub last_seen: SystemTime,
    /// Current download speed (rolling average in bytes/sec)
    pub download_speed: u64,
    /// Recent bandwidth samples for rolling average
    bandwidth_samples: VecDeque<u64>,
    /// Number of chunks successfully served
    pub chunks_served: u64,
    /// Number of chunks that failed
    pub chunks_failed: u64,
    /// Current reputation score (0.0 to 1.0)
    pub reputation_score: f64,
    /// Whether peer is currently online
    pub is_online: bool,
    /// First time peer was seen
    first_seen: SystemTime,
}

impl PeerMetrics {
    /// Create new peer metrics
    pub fn new(peer_id: PeerId) -> Self {
        let now = SystemTime::now();
        Self {
            peer_id,
            uptime: Duration::from_secs(0),
            last_seen: now,
            download_speed: 0,
            bandwidth_samples: VecDeque::new(),
            chunks_served: 0,
            chunks_failed: 0,
            reputation_score: 0.5, // Start with neutral reputation
            is_online: true,
            first_seen: now,
        }
    }

    /// Calculate and update reputation score
    ///
    /// Formula: reputation = (success_rate * success_weight) + (uptime_factor * uptime_weight)
    pub fn update_reputation(&mut self, config: &TrackerConfig) {
        self.reputation_score = self.calculate_reputation(config);
    }

    /// Calculate reputation score based on current metrics
    pub fn calculate_reputation(&self, config: &TrackerConfig) -> f64 {
        // Calculate success rate
        let total_chunks = self.chunks_served + self.chunks_failed;
        let success_rate = if total_chunks > 0 {
            self.chunks_served as f64 / total_chunks as f64
        } else {
            0.5 // Neutral score if no data
        };

        // Calculate uptime factor (capped at 1 hour = 1.0)
        let uptime_secs = if let Ok(elapsed) = SystemTime::now().duration_since(self.first_seen) {
            elapsed.as_secs()
        } else {
            0
        };
        let uptime_factor = (uptime_secs as f64 / 3600.0).min(1.0);

        // Weighted combination
        let reputation =
            (success_rate * config.success_weight) + (uptime_factor * config.uptime_weight);

        // Ensure score is in valid range
        reputation.clamp(0.0, 1.0)
    }

    /// Check if peer is considered reliable (reputation > 0.8)
    pub fn is_reliable(&self) -> bool {
        self.reputation_score > 0.8
    }

    /// Check if peer is considered unreliable (reputation < 0.3)
    pub fn is_unreliable(&self) -> bool {
        self.reputation_score < 0.3
    }

    /// Add a new bandwidth sample and update rolling average
    fn add_bandwidth_sample(&mut self, bytes_per_sec: u64, max_samples: usize) {
        self.bandwidth_samples.push_back(bytes_per_sec);

        // Keep only the last N samples
        while self.bandwidth_samples.len() > max_samples {
            self.bandwidth_samples.pop_front();
        }

        // Calculate rolling average
        if !self.bandwidth_samples.is_empty() {
            let sum: u64 = self.bandwidth_samples.iter().sum();
            self.download_speed = sum / self.bandwidth_samples.len() as u64;
        }
    }
}

/// Information about a peer that has content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Network address (optional)
    pub address: Option<String>,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            address: None,
        }
    }

    /// Create peer info with address
    pub fn with_address(peer_id: PeerId, address: String) -> Self {
        Self {
            peer_id,
            address: Some(address),
        }
    }
}

/// Content availability information
#[derive(Debug, Clone)]
pub struct ContentAvailability {
    /// Content identifier
    pub cid: ContentId,
    /// List of peers that have this content
    pub peers: Vec<PeerInfo>,
    /// Desired replication factor
    pub replication_factor: usize,
    /// Last time this was updated
    pub last_updated: SystemTime,
}

impl ContentAvailability {
    /// Create new content availability record
    pub fn new(cid: ContentId) -> Self {
        Self {
            cid,
            peers: Vec::new(),
            replication_factor: 3, // Default replication factor
            last_updated: SystemTime::now(),
        }
    }

    /// Add a peer that has this content
    pub fn add_peer(&mut self, peer_id: PeerId) {
        // Only add if not already present
        if !self.peers.iter().any(|p| p.peer_id == peer_id) {
            self.peers.push(PeerInfo::new(peer_id));
            self.last_updated = SystemTime::now();
        }
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.retain(|p| &p.peer_id != peer_id);
        self.last_updated = SystemTime::now();
    }

    /// Check if content is well-replicated
    pub fn is_well_replicated(&self) -> bool {
        self.peers.len() >= self.replication_factor
    }

    /// Get current replication count
    pub fn replication_count(&self) -> usize {
        self.peers.len()
    }
}

/// Summary of peer metrics for gossip protocol
#[derive(Debug, Clone)]
pub struct PeerSummary {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Reputation score
    pub reputation_score: f64,
    /// Download speed
    pub download_speed: u64,
    /// Last seen timestamp
    pub last_seen: SystemTime,
}

/// Gossip update message
#[derive(Debug, Clone)]
pub struct GossipUpdate {
    /// List of peer summaries
    pub peer_summaries: Vec<PeerSummary>,
    /// Timestamp of the gossip
    pub timestamp: SystemTime,
}

/// Tracker statistics
#[derive(Debug, Clone)]
pub struct TrackerStats {
    /// Total number of peers
    pub total_peers: usize,
    /// Number of online peers
    pub online_peers: usize,
    /// Number of reliable peers
    pub reliable_peers: usize,
    /// Total content items tracked
    pub total_content: usize,
    /// Average reputation across all peers
    pub avg_reputation: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_metrics_new() {
        let peer_id = PeerId("test_peer".to_string());
        let metrics = PeerMetrics::new(peer_id.clone());

        assert_eq!(metrics.peer_id, peer_id);
        assert_eq!(metrics.chunks_served, 0);
        assert_eq!(metrics.chunks_failed, 0);
        assert_eq!(metrics.reputation_score, 0.5);
        assert!(metrics.is_online);
    }

    #[test]
    fn test_reputation_calculation() {
        let config = TrackerConfig::default();
        let peer_id = PeerId("test_peer".to_string());
        let mut metrics = PeerMetrics::new(peer_id);

        // High success rate
        metrics.chunks_served = 100;
        metrics.chunks_failed = 10;
        metrics.update_reputation(&config);

        // Should have good reputation (success_rate = 100/110 = 0.909)
        // With default weights (0.7 success, 0.3 uptime)
        // reputation = 0.909 * 0.7 + uptime_factor * 0.3
        assert!(metrics.reputation_score > 0.6);
    }

    #[test]
    fn test_is_reliable() {
        let config = TrackerConfig::default();
        let peer_id = PeerId("test_peer".to_string());
        let mut metrics = PeerMetrics::new(peer_id);

        // Set first_seen to 1 hour ago to simulate uptime
        metrics.first_seen = SystemTime::now() - Duration::from_secs(3600);
        metrics.chunks_served = 95;
        metrics.chunks_failed = 5;
        metrics.update_reputation(&config);

        assert!(metrics.is_reliable());
    }

    #[test]
    fn test_is_unreliable() {
        let config = TrackerConfig::default();
        let peer_id = PeerId("test_peer".to_string());
        let mut metrics = PeerMetrics::new(peer_id);

        metrics.chunks_served = 10;
        metrics.chunks_failed = 90;
        metrics.update_reputation(&config);

        assert!(metrics.is_unreliable());
    }

    #[test]
    fn test_bandwidth_rolling_average() {
        let peer_id = PeerId("test_peer".to_string());
        let mut metrics = PeerMetrics::new(peer_id);

        metrics.add_bandwidth_sample(100, 3);
        assert_eq!(metrics.download_speed, 100);

        metrics.add_bandwidth_sample(200, 3);
        assert_eq!(metrics.download_speed, 150);

        metrics.add_bandwidth_sample(300, 3);
        assert_eq!(metrics.download_speed, 200);

        // Should evict first sample (100)
        metrics.add_bandwidth_sample(400, 3);
        assert_eq!(metrics.download_speed, 300); // (200 + 300 + 400) / 3
    }

    #[tokio::test]
    async fn test_tracker_creation() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await;
        assert!(tracker.is_ok());
    }

    #[tokio::test]
    async fn test_record_peer_online() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let peer_id = PeerId("test_peer".to_string());

        tracker.record_peer_online(peer_id.clone()).await;

        let metrics = tracker.get_peer_metrics(&peer_id).await;
        assert!(metrics.is_some());
        assert!(metrics.unwrap().is_online);
    }

    #[tokio::test]
    async fn test_record_peer_offline() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let peer_id = PeerId("test_peer".to_string());

        tracker.record_peer_online(peer_id.clone()).await;
        tracker.record_peer_offline(peer_id.clone()).await;

        let metrics = tracker.get_peer_metrics(&peer_id).await;
        assert!(metrics.is_some());
        assert!(!metrics.unwrap().is_online);
    }

    #[tokio::test]
    async fn test_content_availability() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let cid = ContentId::new(b"test content");
        let peer_id = PeerId("test_peer".to_string());

        tracker
            .record_content_available(cid.clone(), peer_id.clone())
            .await;

        let peers = tracker.get_available_peers(&cid).await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], peer_id);
    }

    #[tokio::test]
    async fn test_download_speed_tracking() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let peer_id = PeerId("test_peer".to_string());

        tracker.record_download_speed(peer_id.clone(), 1000).await;
        tracker.record_download_speed(peer_id.clone(), 2000).await;

        let metrics = tracker.get_peer_metrics(&peer_id).await.unwrap();
        assert_eq!(metrics.download_speed, 1500); // Average of 1000 and 2000
    }

    #[tokio::test]
    async fn test_upload_success_and_failure() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let peer_id = PeerId("test_peer".to_string());

        tracker.record_peer_online(peer_id.clone()).await;
        tracker.record_upload_success(peer_id.clone()).await;
        tracker.record_upload_success(peer_id.clone()).await;
        tracker.record_upload_failure(peer_id.clone()).await;

        let metrics = tracker.get_peer_metrics(&peer_id).await.unwrap();
        assert_eq!(metrics.chunks_served, 2);
        assert_eq!(metrics.chunks_failed, 1);
    }

    #[tokio::test]
    async fn test_select_best_peers() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let cid = ContentId::new(b"test content");

        // Create peers with different reputations
        for i in 0..5 {
            let peer_id = PeerId(format!("peer_{}", i));
            tracker.record_peer_online(peer_id.clone()).await;
            tracker
                .record_content_available(cid.clone(), peer_id.clone())
                .await;

            // Give different success rates
            for _ in 0..(10 - i) {
                tracker.record_upload_success(peer_id.clone()).await;
            }
            for _ in 0..i {
                tracker.record_upload_failure(peer_id.clone()).await;
            }
        }

        let best_peers = tracker.select_best_peers(&cid, 3).await;
        assert_eq!(best_peers.len(), 3);

        // Best peer should be peer_0 (highest success rate)
        assert_eq!(best_peers[0].peer_id.0, "peer_0");
    }

    #[tokio::test]
    async fn test_gossip_update() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let peer_id = PeerId("test_peer".to_string());

        tracker.record_peer_online(peer_id.clone()).await;
        tracker.record_upload_success(peer_id.clone()).await;

        let gossip = tracker.get_gossip_update().await;
        assert!(!gossip.peer_summaries.is_empty());
    }

    #[tokio::test]
    async fn test_apply_gossip_update() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();

        let peer_id = PeerId("remote_peer".to_string());
        let summary = PeerSummary {
            peer_id: peer_id.clone(),
            reputation_score: 0.9,
            download_speed: 1000000,
            last_seen: SystemTime::now(),
        };

        let gossip = GossipUpdate {
            peer_summaries: vec![summary],
            timestamp: SystemTime::now(),
        };

        tracker.apply_gossip_update(gossip).await;

        let metrics = tracker.get_peer_metrics(&peer_id).await;
        assert!(metrics.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_stale_peers() {
        let mut config = TrackerConfig::default();
        config.peer_timeout = Duration::from_millis(100);

        let tracker = AvailabilityTracker::new(config).await.unwrap();
        let peer_id = PeerId("test_peer".to_string());

        tracker.record_peer_online(peer_id.clone()).await;

        // Wait for peer to become stale
        tokio::time::sleep(Duration::from_millis(150)).await;

        tracker.cleanup_stale_peers().await;

        let metrics = tracker.get_peer_metrics(&peer_id).await;
        assert!(metrics.is_none());
    }

    #[tokio::test]
    async fn test_tracker_stats() {
        let config = TrackerConfig::default();
        let tracker = AvailabilityTracker::new(config).await.unwrap();

        let peer1 = PeerId("peer1".to_string());
        let peer2 = PeerId("peer2".to_string());
        let cid = ContentId::new(b"test content");

        tracker.record_peer_online(peer1.clone()).await;
        tracker.record_peer_online(peer2.clone()).await;
        tracker.record_content_available(cid, peer1).await;

        let stats = tracker.get_stats().await;
        assert_eq!(stats.total_peers, 2);
        assert_eq!(stats.online_peers, 2);
        assert_eq!(stats.total_content, 1);
    }
}
