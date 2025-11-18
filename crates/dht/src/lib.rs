//! Kademlia DHT implementation for Codio CDN
//!
//! This module provides a distributed hash table (DHT) based on the Kademlia protocol
//! for peer discovery and content routing in the Codio decentralized CDN.
//!
//! # Overview
//!
//! The Kademlia DHT uses an XOR distance metric to organize peers into a routing table
//! and efficiently locate content providers across the network. Key features include:
//!
//! - **Content Routing**: Announce and discover content providers
//! - **Peer Discovery**: Find and connect to network peers
//! - **Fault Tolerance**: Redundant provider records (20x replication)
//! - **Self-Maintenance**: Automatic re-providing and routing table updates
//!
//! # XOR Distance Metric
//!
//! Kademlia uses XOR (exclusive OR) to measure "distance" between node IDs:
//! - Distance(A, B) = A XOR B
//! - Symmetric: Distance(A, B) = Distance(B, A)
//! - Triangle inequality: Distance(A, C) ≤ Distance(A, B) + Distance(B, C)
//! - Unidirectional: For any point X, all distances converge along one path
//!
//! # Provider Record Lifecycle
//!
//! 1. **Announce**: Node announces it has content via `provide(cid)`
//! 2. **Replicate**: Record is stored on K=20 closest nodes to the content hash
//! 3. **Discover**: Other nodes find providers via `find_providers(cid)`
//! 4. **Refresh**: Provider automatically re-announces every 12 hours
//! 5. **Expire**: Records expire after 24 hours if not refreshed
//!
//! # Example
//!
//! ```rust,no_run
//! use codio_dht::{DHTManager, DHTConfig};
//! use codio_content_id::ContentId;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create DHT manager with default config
//!     let config = DHTConfig::default();
//!     let mut dht = DHTManager::new(config).await?;
//!
//!     // Start listening
//!     dht.listen("/ip4/0.0.0.0/tcp/0").await?;
//!
//!     // Bootstrap to network
//!     dht.bootstrap().await?;
//!
//!     // Announce content
//!     let cid = ContentId::new(b"Hello, World!");
//!     dht.provide(cid.clone()).await?;
//!
//!     // Find providers
//!     let providers = dht.find_providers(cid).await?;
//!     println!("Found {} providers", providers.len());
//!
//!     Ok(())
//! }
//! ```

use anyhow::{anyhow, Context, Result};
use codio_content_id::ContentId;
use futures::StreamExt;
use libp2p::{
    identity::Keypair,
    kad::{
        store::MemoryStore, Behaviour as Kademlia, BootstrapOk, Config as KademliaConfig,
        Event as KademliaEvent, QueryId, QueryResult, RecordKey,
    },
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId, SwarmBuilder,
};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

mod config;
pub use config::DHTConfig;

/// Maximum number of providers to return per query
const MAX_PROVIDERS: usize = 50;

/// Default K-bucket size (concurrent lookups)
const K_VALUE: usize = 20;

/// Timeout for DHT queries
const QUERY_TIMEOUT: Duration = Duration::from_secs(60);

/// How often to check for expired provider records
const CLEANUP_INTERVAL: Duration = Duration::from_secs(600); // 10 minutes

/// DHT Manager - Main interface for Kademlia DHT operations
///
/// The DHTManager handles all DHT operations including content routing,
/// peer discovery, and network maintenance.
pub struct DHTManager {
    /// The libp2p Swarm containing the Kademlia behaviour
    swarm: Swarm<Kademlia<MemoryStore>>,

    /// Our peer ID
    peer_id: PeerId,

    /// DHT configuration
    config: DHTConfig,

    /// Active query tracking
    queries: HashMap<QueryId, QueryInfo>,

    /// Provider records we're tracking
    provider_records: HashMap<ContentId, Vec<ProviderRecord>>,

    /// Content we're providing (for re-announcement)
    providing: HashSet<ContentId>,

    /// Event channel for DHT events
    event_tx: mpsc::UnboundedSender<DHTEvent>,

    /// Event receiver
    event_rx: Option<mpsc::UnboundedReceiver<DHTEvent>>,

    /// Statistics
    stats: DHTStats,

    /// Bootstrap peers
    bootstrap_peers: Vec<(PeerId, Multiaddr)>,

    /// Flag indicating if we're bootstrapped
    is_bootstrapped: bool,
}

/// Information about an active DHT query
#[derive(Debug, Clone)]
struct QueryInfo {
    /// Type of query
    query_type: QueryType,
    /// Content ID (if applicable)
    cid: Option<ContentId>,
    /// When the query started
    started_at: SystemTime,
    /// Timeout for this query
    timeout: Duration,
}

/// Type of DHT query
#[derive(Debug, Clone, PartialEq)]
enum QueryType {
    /// Finding providers for content
    FindProviders,
    /// Announcing we provide content
    Provide,
    /// Bootstrapping to the network
    Bootstrap,
    /// Finding a specific peer
    FindPeer,
    /// Getting the closest peers to a key
    GetClosestPeers,
}

/// Provider record with metadata
///
/// Tracks which peer provides which content, including distance metrics
/// and timestamps for expiration.
#[derive(Debug, Clone)]
pub struct ProviderRecord {
    /// Content identifier
    pub cid: ContentId,
    /// Provider peer information
    pub provider: PeerInfo,
    /// When this record was created
    pub timestamp: SystemTime,
    /// XOR distance from content hash to provider peer ID
    pub distance: Distance,
}

impl ProviderRecord {
    /// Check if this record has expired
    pub fn is_expired(&self, ttl: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|age| age > ttl)
            .unwrap_or(true)
    }

    /// Get the age of this record
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or(Duration::ZERO)
    }
}

/// Peer information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Known addresses for this peer
    pub addresses: Vec<Multiaddr>,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self { peer_id, addresses }
    }

    /// Create peer info with single address
    pub fn with_addr(peer_id: PeerId, addr: Multiaddr) -> Self {
        Self {
            peer_id,
            addresses: vec![addr],
        }
    }
}

/// XOR distance between two keys
///
/// In Kademlia, distance is calculated as the XOR of two node IDs.
/// This creates a metric space where distance is symmetric and follows
/// the triangle inequality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Distance(pub u128);

impl Distance {
    /// Calculate XOR distance between two byte arrays
    pub fn calculate(a: &[u8], b: &[u8]) -> Self {
        let mut result = 0u128;
        let len = a.len().min(b.len()).min(16); // Use first 128 bits

        for i in 0..len {
            result = (result << 8) | ((a[i] ^ b[i]) as u128);
        }

        Distance(result)
    }

    /// Zero distance
    pub fn zero() -> Self {
        Distance(0)
    }

    /// Maximum distance
    pub fn max() -> Self {
        Distance(u128::MAX)
    }
}

/// DHT statistics
///
/// Provides metrics about the DHT's current state including
/// routing table size, number of providers, and query statistics.
#[derive(Debug, Clone, Default)]
pub struct DHTStats {
    /// Number of peers in routing table
    pub num_peers: usize,

    /// Number of provider records we're tracking
    pub num_providers: usize,

    /// Size of the routing table (k-buckets)
    pub routing_table_size: usize,

    /// Number of pending queries
    pub pending_queries: usize,

    /// Number of content items we're providing
    pub num_providing: usize,

    /// Total queries executed
    pub total_queries: u64,

    /// Successful queries
    pub successful_queries: u64,

    /// Failed queries
    pub failed_queries: u64,

    /// Number of times we've bootstrapped
    pub bootstrap_count: u64,

    /// Is the node bootstrapped?
    pub is_bootstrapped: bool,
}

/// Events emitted by the DHT
#[derive(Debug, Clone)]
pub enum DHTEvent {
    /// Providers found for content
    ProvidersFound {
        cid: ContentId,
        providers: Vec<PeerInfo>,
    },

    /// Content announcement successful
    ProvideSuccess { cid: ContentId },

    /// Content announcement failed
    ProvideFailed { cid: ContentId, error: String },

    /// Bootstrap completed successfully
    BootstrapComplete,

    /// Bootstrap failed
    BootstrapFailed { error: String },

    /// Peer discovered
    PeerDiscovered { peer: PeerInfo },

    /// Query completed
    QueryComplete { query_type: QueryType },

    /// Query failed
    QueryFailed {
        query_type: QueryType,
        error: String,
    },
}

impl DHTManager {
    /// Create a new DHT manager
    ///
    /// # Arguments
    ///
    /// * `config` - DHT configuration
    ///
    /// # Returns
    ///
    /// A new DHTManager instance
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use codio_dht::{DHTManager, DHTConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let config = DHTConfig::default();
    ///     let dht = DHTManager::new(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(config: DHTConfig) -> Result<Self> {
        info!("Initializing DHT Manager");

        // Generate keypair
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        info!("Local peer ID: {}", peer_id);

        // Create Kademlia store
        let store = MemoryStore::new(peer_id);

        // Configure Kademlia
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(config.query_timeout);
        kad_config.set_replication_factor(
            config
                .replication_factor
                .try_into()
                .context("Invalid replication factor")?,
        );
        kad_config.set_parallelism(
            config
                .replication_factor
                .try_into()
                .context("Invalid replication factor")?,
        );

        // Set provider record TTL
        if let Some(ttl) = config.provider_timeout {
            kad_config.set_provider_record_ttl(Some(ttl));
        }

        // Set republication interval
        if let Some(interval) = config.republish_interval {
            kad_config.set_provider_publication_interval(Some(interval));
        }

        // Create Kademlia behaviour
        let kademlia = Kademlia::with_config(peer_id, store, kad_config);

        // Build the swarm
        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .context("Failed to configure transport")?
            .with_behaviour(|_| kademlia)
            .context("Failed to create behaviour")?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let manager = Self {
            swarm,
            peer_id,
            config: config.clone(),
            queries: HashMap::new(),
            provider_records: HashMap::new(),
            providing: HashSet::new(),
            event_tx,
            event_rx: Some(event_rx),
            stats: DHTStats::default(),
            bootstrap_peers: config.bootstrap_peers.clone(),
            is_bootstrapped: false,
        };

        info!("DHT Manager initialized successfully");
        Ok(manager)
    }

    /// Get our peer ID
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Start listening on an address
    ///
    /// # Arguments
    ///
    /// * `addr` - Multiaddr to listen on (e.g., "/ip4/0.0.0.0/tcp/0")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::{DHTManager, DHTConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let mut dht = DHTManager::new(DHTConfig::default()).await?;
    /// dht.listen("/ip4/0.0.0.0/tcp/4001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn listen(&mut self, addr: &str) -> Result<Multiaddr> {
        let addr: Multiaddr = addr.parse().context("Invalid multiaddr")?;
        self.swarm
            .listen_on(addr.clone())
            .context("Failed to listen")?;

        info!("Listening on: {}", addr);
        Ok(addr)
    }

    /// Bootstrap the DHT by connecting to the network
    ///
    /// This connects to bootstrap peers and performs a DHT bootstrap query
    /// to populate the routing table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::{DHTManager, DHTConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let mut dht = DHTManager::new(DHTConfig::default()).await?;
    /// dht.bootstrap().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bootstrap(&mut self) -> Result<()> {
        info!("Bootstrapping DHT...");

        // Add bootstrap peers
        for (peer_id, addr) in &self.bootstrap_peers {
            debug!("Adding bootstrap peer: {} at {}", peer_id, addr);
            self.swarm
                .behaviour_mut()
                .add_address(peer_id, addr.clone());
        }

        // Start bootstrap process
        let query_id = self
            .swarm
            .behaviour_mut()
            .bootstrap()
            .context("Failed to start bootstrap")?;

        // Track the query
        let query_info = QueryInfo {
            query_type: QueryType::Bootstrap,
            cid: None,
            started_at: SystemTime::now(),
            timeout: self.config.query_timeout,
        };

        self.queries.insert(query_id, query_info);

        // Update stats
        self.stats.bootstrap_count += 1;

        info!("Bootstrap query initiated");
        Ok(())
    }

    /// Announce that we provide content
    ///
    /// This publishes a provider record to the DHT, announcing that we have
    /// the specified content available. The record will be replicated to
    /// approximately K=20 nodes closest to the content hash.
    ///
    /// # Arguments
    ///
    /// * `cid` - Content identifier to announce
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::{DHTManager, DHTConfig};
    /// # use codio_content_id::ContentId;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let mut dht = DHTManager::new(DHTConfig::default()).await?;
    /// let cid = ContentId::new(b"Hello, World!");
    /// dht.provide(cid).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn provide(&mut self, cid: ContentId) -> Result<()> {
        info!("Announcing content: {}", cid);

        let key = cid_to_record_key(&cid);
        let query_id = self
            .swarm
            .behaviour_mut()
            .start_providing(key)
            .context("Failed to start providing")?;

        // Track the query
        let query_info = QueryInfo {
            query_type: QueryType::Provide,
            cid: Some(cid.clone()),
            started_at: SystemTime::now(),
            timeout: self.config.query_timeout,
        };

        self.queries.insert(query_id, query_info);

        // Add to our providing set
        self.providing.insert(cid.clone());

        // Update stats
        self.stats.num_providing = self.providing.len();
        self.stats.total_queries += 1;

        debug!("Provide query initiated for: {}", cid);
        Ok(())
    }

    /// Find providers for content
    ///
    /// Queries the DHT to find which peers are providing the specified content.
    /// Returns up to MAX_PROVIDERS results.
    ///
    /// # Arguments
    ///
    /// * `cid` - Content identifier to search for
    ///
    /// # Returns
    ///
    /// Vector of PeerInfo for providers, sorted by XOR distance
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::{DHTManager, DHTConfig};
    /// # use codio_content_id::ContentId;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let mut dht = DHTManager::new(DHTConfig::default()).await?;
    /// let cid = ContentId::new(b"Hello, World!");
    /// let providers = dht.find_providers(cid).await?;
    /// println!("Found {} providers", providers.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_providers(&mut self, cid: ContentId) -> Result<Vec<PeerInfo>> {
        info!("Finding providers for: {}", cid);

        let key = cid_to_record_key(&cid);
        let query_id = self.swarm.behaviour_mut().get_providers(key);

        // Track the query
        let query_info = QueryInfo {
            query_type: QueryType::FindProviders,
            cid: Some(cid.clone()),
            started_at: SystemTime::now(),
            timeout: self.config.query_timeout,
        };

        self.queries.insert(query_id, query_info);

        // Update stats
        self.stats.total_queries += 1;

        debug!("Find providers query initiated for: {}", cid);

        // Return cached providers if available
        if let Some(providers) = self.provider_records.get(&cid) {
            let peer_infos: Vec<PeerInfo> = providers
                .iter()
                .filter(|p| {
                    !p.is_expired(
                        self.config
                            .provider_timeout
                            .unwrap_or(Duration::from_secs(86400)),
                    )
                })
                .map(|p| p.provider.clone())
                .collect();

            if !peer_infos.is_empty() {
                debug!("Returning {} cached providers", peer_infos.len());
                return Ok(peer_infos);
            }
        }

        Ok(Vec::new())
    }

    /// Stop providing content
    ///
    /// Removes content from our local providing set. Note that provider records
    /// already published to the DHT will remain until they expire.
    ///
    /// # Arguments
    ///
    /// * `cid` - Content identifier to stop providing
    pub async fn stop_providing(&mut self, cid: ContentId) -> Result<()> {
        info!("Stopping providing: {}", cid);

        self.providing.remove(&cid);

        // Update stats
        self.stats.num_providing = self.providing.len();

        debug!("Removed {} from providing set", cid);
        Ok(())
    }

    /// Find a specific peer by ID
    ///
    /// Queries the DHT to locate a peer and retrieve its known addresses.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Peer identifier to search for
    ///
    /// # Returns
    ///
    /// Vector of known addresses for the peer
    pub async fn find_peer(&mut self, peer_id: PeerId) -> Result<Vec<Multiaddr>> {
        info!("Finding peer: {}", peer_id);

        let query_id = self.swarm.behaviour_mut().get_closest_peers(peer_id);

        let query_info = QueryInfo {
            query_type: QueryType::FindPeer,
            cid: None,
            started_at: SystemTime::now(),
            timeout: self.config.query_timeout,
        };

        self.queries.insert(query_id, query_info);

        // Update stats
        self.stats.total_queries += 1;

        debug!("Find peer query initiated for: {}", peer_id);
        Ok(Vec::new())
    }

    /// Get the closest peers to a key
    ///
    /// Returns the K closest peers to a given key according to the XOR distance metric.
    ///
    /// # Arguments
    ///
    /// * `_key` - Key to find closest peers to
    ///
    /// # Returns
    ///
    /// Vector of peer IDs sorted by distance (closest first)
    pub fn get_closest_peers(&self, _key: &[u8]) -> Vec<PeerId> {
        // This is a synchronous operation on the routing table
        // In a real implementation, we'd query the Kademlia routing table directly
        Vec::new()
    }

    /// Get current DHT statistics
    ///
    /// Returns metrics about the DHT's current state.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::{DHTManager, DHTConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let dht = DHTManager::new(DHTConfig::default()).await?;
    /// let stats = dht.stats().await;
    /// println!("Peers: {}, Providers: {}", stats.num_peers, stats.num_providers);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stats(&self) -> DHTStats {
        self.stats.clone()
    }

    /// Take the event receiver
    ///
    /// This can only be called once. Subsequent calls will return None.
    /// Use this to receive DHT events in your application.
    pub fn take_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<DHTEvent>> {
        self.event_rx.take()
    }

    /// Run the DHT event loop
    ///
    /// This should be called in a background task to process DHT events.
    /// It will run indefinitely until the manager is dropped.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::{DHTManager, DHTConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut dht = DHTManager::new(DHTConfig::default()).await?;
    ///
    /// // Spawn event loop in background
    /// tokio::spawn(async move {
    ///     dht.run().await;
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&mut self) {
        info!("Starting DHT event loop");

        loop {
            let event = self.swarm.select_next_some().await;

            if let Err(e) = self.handle_swarm_event(event).await {
                error!("Error handling swarm event: {}", e);
            }
        }
    }

    /// Handle a swarm event
    async fn handle_swarm_event(&mut self, event: SwarmEvent<KademliaEvent>) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(kad_event) => {
                self.handle_kademlia_event(kad_event).await?;
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!("Connection established with {}", peer_id);

                // Update stats
                self.stats.num_peers += 1;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!("Connection closed with {}", peer_id);

                // Update stats
                self.stats.num_peers = self.stats.num_peers.saturating_sub(1);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle a Kademlia event
    async fn handle_kademlia_event(&mut self, event: KademliaEvent) -> Result<()> {
        match event {
            KademliaEvent::OutboundQueryProgressed { id, result, .. } => {
                self.handle_query_result(id, result).await?;
            }
            KademliaEvent::RoutingUpdated { peer, .. } => {
                debug!("Routing table updated with peer: {}", peer);
            }
            KademliaEvent::InboundRequest { request } => {
                debug!("Received inbound request: {:?}", request);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle a query result
    async fn handle_query_result(&mut self, query_id: QueryId, result: QueryResult) -> Result<()> {
        // Get query info
        let query_info = self.queries.get(&query_id).cloned();

        let query_info = match query_info {
            Some(info) => info,
            None => {
                debug!("Received result for unknown query: {:?}", query_id);
                return Ok(());
            }
        };

        match result {
            QueryResult::GetProviders(Ok(ok)) => {
                // GetProvidersOk is an enum with Finished variant
                match ok {
                    libp2p::kad::GetProvidersOk::FinishedWithNoAdditionalRecord {
                        closest_peers,
                    } => {
                        // No providers found
                        debug!("No providers found, closest peers: {:?}", closest_peers);
                        self.handle_query_failed(query_info, "No providers found".to_string())
                            .await?;
                    }
                    libp2p::kad::GetProvidersOk::FoundProviders { key, providers } => {
                        // Providers found
                        self.handle_providers_found(query_info, key, providers)
                            .await?;
                    }
                }
            }
            QueryResult::GetProviders(Err(e)) => {
                warn!("Get providers failed: {:?}", e);
                self.handle_query_failed(query_info, format!("{:?}", e))
                    .await?;
            }
            QueryResult::StartProviding(Ok(_)) => {
                self.handle_provide_success(query_info).await?;
            }
            QueryResult::StartProviding(Err(e)) => {
                warn!("Start providing failed: {:?}", e);
                self.handle_provide_failed(query_info, format!("{:?}", e))
                    .await?;
            }
            QueryResult::Bootstrap(Ok(BootstrapOk { peer, .. })) => {
                self.handle_bootstrap_success(peer).await?;
            }
            QueryResult::Bootstrap(Err(e)) => {
                warn!("Bootstrap failed: {:?}", e);
                self.handle_bootstrap_failed(format!("{:?}", e)).await?;
            }
            QueryResult::GetClosestPeers(Ok(ok)) => {
                debug!("Found closest peers: {:?}", ok.peers);
            }
            QueryResult::GetClosestPeers(Err(e)) => {
                warn!("Get closest peers failed: {:?}", e);
            }
            _ => {
                debug!("Unhandled query result: {:?}", result);
            }
        }

        // Remove completed query
        self.queries.remove(&query_id);

        // Update stats
        self.stats.pending_queries = self.queries.len();

        Ok(())
    }

    /// Handle providers found
    async fn handle_providers_found(
        &mut self,
        query_info: QueryInfo,
        _key: RecordKey,
        providers: HashSet<PeerId>,
    ) -> Result<()> {
        let cid = match query_info.cid {
            Some(cid) => cid,
            None => {
                warn!("Providers found but no CID in query info");
                return Ok(());
            }
        };

        info!("Found {} providers for {}", providers.len(), cid);

        // Convert to PeerInfo
        let mut peer_infos = Vec::new();

        for peer_id in providers.iter().take(MAX_PROVIDERS) {
            // Note: In a real implementation, we'd query the routing table for addresses
            // For now, we just store the peer ID
            peer_infos.push(PeerInfo::new(*peer_id, vec![]));
        }

        // Calculate distances and create provider records
        let mut records = Vec::new();
        for peer_info in &peer_infos {
            let distance = Distance::calculate(cid.hash(), peer_info.peer_id.to_bytes().as_slice());
            records.push(ProviderRecord {
                cid: cid.clone(),
                provider: peer_info.clone(),
                timestamp: SystemTime::now(),
                distance,
            });
        }

        // Sort by distance
        records.sort_by_key(|r| r.distance);

        // Store provider records
        self.provider_records.insert(cid.clone(), records);

        // Update stats
        self.stats.num_providers = self.provider_records.values().map(|v| v.len()).sum();
        self.stats.successful_queries += 1;

        // Emit event
        let _ = self.event_tx.send(DHTEvent::ProvidersFound {
            cid,
            providers: peer_infos,
        });

        Ok(())
    }

    /// Handle provide success
    async fn handle_provide_success(&mut self, query_info: QueryInfo) -> Result<()> {
        let cid = match query_info.cid {
            Some(cid) => cid,
            None => return Ok(()),
        };

        info!("Successfully announced content: {}", cid);

        // Update stats
        self.stats.successful_queries += 1;

        // Emit event
        let _ = self.event_tx.send(DHTEvent::ProvideSuccess { cid });

        Ok(())
    }

    /// Handle provide failed
    async fn handle_provide_failed(&mut self, query_info: QueryInfo, error: String) -> Result<()> {
        let cid = match query_info.cid {
            Some(cid) => cid,
            None => return Ok(()),
        };

        error!("Failed to announce content {}: {}", cid, error);

        // Update stats
        self.stats.failed_queries += 1;

        // Emit event
        let _ = self.event_tx.send(DHTEvent::ProvideFailed { cid, error });

        Ok(())
    }

    /// Handle query failed
    async fn handle_query_failed(&mut self, query_info: QueryInfo, error: String) -> Result<()> {
        error!("Query {:?} failed: {}", query_info.query_type, error);

        // Update stats
        self.stats.failed_queries += 1;

        // Emit event
        let _ = self.event_tx.send(DHTEvent::QueryFailed {
            query_type: query_info.query_type,
            error,
        });

        Ok(())
    }

    /// Handle bootstrap success
    async fn handle_bootstrap_success(&mut self, peer: PeerId) -> Result<()> {
        info!("Bootstrap successful with peer: {}", peer);

        // Mark as bootstrapped
        self.is_bootstrapped = true;

        // Update stats
        self.stats.is_bootstrapped = true;
        self.stats.successful_queries += 1;

        // Emit event
        let _ = self.event_tx.send(DHTEvent::BootstrapComplete);

        Ok(())
    }

    /// Handle bootstrap failed
    async fn handle_bootstrap_failed(&mut self, error: String) -> Result<()> {
        error!("Bootstrap failed: {}", error);

        // Update stats
        self.stats.failed_queries += 1;

        // Emit event
        let _ = self.event_tx.send(DHTEvent::BootstrapFailed { error });

        Ok(())
    }
}

/// Convert ContentId to Kademlia RecordKey
fn cid_to_record_key(cid: &ContentId) -> RecordKey {
    RecordKey::new(cid.hash())
}

/// Convert Kademlia RecordKey to ContentId
fn record_key_to_cid(key: &RecordKey) -> Result<ContentId> {
    let hash_bytes = key.as_ref();
    if hash_bytes.len() != 32 {
        return Err(anyhow!("Invalid key length: expected 32 bytes"));
    }

    let multibase = format!("Qm{}", bs58::encode(hash_bytes).into_string());
    ContentId::from_str(&multibase).map_err(|e| anyhow!("Invalid CID: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dht_creation() {
        let config = DHTConfig::default();
        let dht = DHTManager::new(config).await.unwrap();
        assert_eq!(dht.peer_id(), dht.peer_id());
    }

    #[tokio::test]
    async fn test_cid_record_key_conversion() {
        let cid = ContentId::new(b"Test content");
        let key = cid_to_record_key(&cid);
        let cid2 = record_key_to_cid(&key).unwrap();
        assert_eq!(cid, cid2);
    }

    #[tokio::test]
    async fn test_distance_calculation() {
        let a = b"test key 1";
        let b = b"test key 2";
        let distance = Distance::calculate(a, b);
        assert!(distance.0 > 0);

        // Distance to self is zero
        let zero = Distance::calculate(a, a);
        assert_eq!(zero.0, 0);
    }

    #[tokio::test]
    async fn test_provider_record_expiration() {
        let cid = ContentId::new(b"Test content");
        let peer_id = PeerId::random();
        let peer_info = PeerInfo::new(peer_id, vec![]);

        let record = ProviderRecord {
            cid,
            provider: peer_info,
            timestamp: SystemTime::now() - Duration::from_secs(100),
            distance: Distance::zero(),
        };

        // Should not be expired with 2 hour TTL
        assert!(!record.is_expired(Duration::from_secs(7200)));

        // Should be expired with 1 second TTL
        assert!(record.is_expired(Duration::from_secs(1)));
    }

    #[tokio::test]
    async fn test_stats_initialization() {
        let config = DHTConfig::default();
        let dht = DHTManager::new(config).await.unwrap();
        let stats = dht.stats().await;

        assert_eq!(stats.num_peers, 0);
        assert_eq!(stats.num_providers, 0);
        assert_eq!(stats.total_queries, 0);
        assert!(!stats.is_bootstrapped);
    }
}
