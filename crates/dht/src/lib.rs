use codio_content_id::ContentId;
use libp2p::kad::Behaviour as Kademlia;
use libp2p::{
    kad::{store::MemoryStore, Config as KademliaConfig, Event as KademliaEvent, QueryResult},
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId, SwarmBuilder,
};
use std::time::Duration;
use tokio::sync::mpsc;

pub mod config;
pub use config::{ConfigError, DHTConfig};

/// DHT Manager for content routing and peer discovery
///
/// This is the main entry point for DHT operations. It manages:
/// - Kademlia DHT for peer routing
/// - Provider record tracking and expiration
/// - Content announcement and discovery
/// - Peer discovery and routing table management
pub struct DHTManager {
    /// The underlying Kademlia swarm
    swarm: Swarm<Kademlia<MemoryStore>>,

    /// Configuration
    config: DHTConfig,

    /// Local peer ID
    peer_id: PeerId,

    /// Event channel for DHT events
    event_tx: mpsc::UnboundedSender<DHTEvent>,

    /// Event receiver (external)
    event_rx: Option<mpsc::UnboundedReceiver<DHTEvent>>,

    /// Provider records we're tracking
    provider_records: Arc<Mutex<HashMap<ContentId, Vec<ProviderRecord>>>>,

    /// Content we're providing
    local_providers: Arc<Mutex<HashSet<ContentId>>>,

    /// Active queries
    active_queries: Arc<Mutex<HashMap<libp2p::kad::QueryId, QueryInfo>>>,

    /// Statistics
    stats: Arc<Mutex<DHTStats>>,

    /// Last republish time
    last_republish: Arc<Mutex<SystemTime>>,
}

/// Information about a peer in the network
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: PeerId,

    /// Known addresses for this peer
    pub addresses: Vec<Multiaddr>,

    /// When we last saw this peer
    pub last_seen: SystemTime,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            addresses: Vec::new(),
            last_seen: SystemTime::now(),
        }
    }

    /// Create peer info with addresses
    pub fn with_addresses(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self {
            peer_id,
            addresses,
            last_seen: SystemTime::now(),
        }
    }

    /// Add an address to this peer
    pub fn add_address(&mut self, addr: Multiaddr) {
        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }
        self.last_seen = SystemTime::now();
    }
}

/// Provider record for content
///
/// Tracks which peers have announced they have specific content.
#[derive(Debug, Clone)]
pub struct ProviderRecord {
    /// Content ID
    pub cid: ContentId,

    /// Peer providing the content
    pub provider: PeerInfo,

    /// When this record was created/updated
    pub timestamp: SystemTime,

    /// XOR distance from our node to the content key
    pub distance: Distance,
}

impl ProviderRecord {
    /// Create a new provider record
    pub fn new(cid: ContentId, provider: PeerInfo) -> Self {
        Self {
            cid,
            provider,
            timestamp: SystemTime::now(),
            distance: Distance::zero(),
        }
    }

    /// Check if this record has expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|age| age > timeout)
            .unwrap_or(false)
    }

    /// Update the timestamp
    pub fn refresh(&mut self) {
        self.timestamp = SystemTime::now();
    }
}

/// XOR distance between two keys in the DHT
///
/// Kademlia uses XOR distance as a metric for determining which nodes
/// are "close" to each other or to a piece of content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Distance([u8; 32]);

impl Distance {
    /// Zero distance
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Calculate XOR distance between two keys
    pub fn between(a: &[u8], b: &[u8]) -> Self {
        let mut result = [0u8; 32];
        let len = a.len().min(b.len()).min(32);

        for i in 0..len {
            result[i] = a[i] ^ b[i];
        }

        Self(result)
    }

    /// Get the distance as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Get the number of leading zero bits (used for bucket index calculation)
    pub fn leading_zeros(&self) -> u32 {
        let mut zeros = 0;
        for byte in &self.0 {
            let byte_zeros = byte.leading_zeros();
            zeros += byte_zeros;
            if byte_zeros < 8 {
                break;
            }
        }
        zeros
    }
}

/// DHT statistics
///
/// Provides insight into the state and health of the DHT.
#[derive(Debug, Clone, Default)]
pub struct DHTStats {
    /// Number of peers in the routing table
    pub num_peers: usize,

    /// Number of provider records we're tracking
    pub num_providers: usize,

    /// Size of the routing table (number of k-buckets)
    pub routing_table_size: usize,

    /// Number of pending queries
    pub pending_queries: usize,

    /// Number of content items we're providing
    pub local_content_count: usize,

    /// Total number of queries executed
    pub total_queries: u64,

    /// Number of successful queries
    pub successful_queries: u64,

    /// Number of failed queries
    pub failed_queries: u64,

    /// Last bootstrap time
    pub last_bootstrap: Option<SystemTime>,

    /// Is the node bootstrapped?
    pub is_bootstrapped: bool,
}

impl DHTStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            (self.successful_queries as f64) / (self.total_queries as f64)
        }
    }
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
    /// Bootstrap completed
    BootstrapComplete,
}

impl DhtNode {
    /// Create new DHT node
    pub async fn new(
        config: DhtConfig,
    ) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<DhtEvent>)> {
        // Generate keypair
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("Creating DHT manager with peer ID: {}", local_peer_id);

        // Create Kademlia store
        let store = MemoryStore::new(local_peer_id);

        // Configure Kademlia
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(config.query_timeout);
        kad_config.set_provider_record_ttl(Some(config.provider_timeout));
        kad_config.set_record_ttl(Some(config.record_ttl));
        kad_config.set_replication_factor(
            std::num::NonZeroUsize::new(config.replication_factor)
                .context("Replication factor must be non-zero")?,
        );
        kad_config.set_parallelism(
            std::num::NonZeroUsize::new(config.parallelism)
                .context("Parallelism must be non-zero")?,
        );

        // Create Kademlia behaviour
        let kademlia = Kademlia::with_config(local_peer_id, store, kad_config);

        // Create swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .context("Failed to configure TCP transport")?
            .with_behaviour(|_| kademlia)
            .context("Failed to create Kademlia behaviour")?
            .build();

        // Event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok((DhtNode { swarm, event_tx }, event_rx))
    }

    /// Get the local peer ID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// Get a clone of the configuration
    pub fn config(&self) -> &DHTConfig {
        &self.config
    }

    /// Take the event receiver
    ///
    /// This can only be called once. Subsequent calls will return None.
    pub fn take_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<DHTEvent>> {
        self.event_rx.take()
    }

    /// Start listening on an address
    ///
    /// # Arguments
    ///
    /// * `addr` - The multiaddr to listen on (e.g., "/ip4/0.0.0.0/tcp/0")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::DHTManager;
    /// # async fn example(mut dht: DHTManager) -> anyhow::Result<()> {
    /// dht.listen("/ip4/0.0.0.0/tcp/4001".parse()?).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn listen(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm
            .listen_on(addr.clone())
            .context("Failed to start listening")?;

        tracing::info!("Listening on {}", addr);
        Ok(())
    }

    /// Bootstrap the DHT by connecting to known peers
    ///
    /// This initiates the bootstrap process which will:
    /// 1. Connect to provided bootstrap peers
    /// 2. Perform a random walk to populate the routing table
    /// 3. Make the node discoverable to others
    ///
    /// # Arguments
    ///
    /// * `bootstrap_peers` - List of known peer addresses to bootstrap from
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::DHTManager;
    /// # use libp2p::Multiaddr;
    /// # async fn example(mut dht: DHTManager) -> anyhow::Result<()> {
    /// let bootstrap_addrs = vec![
    ///     "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooW...".parse()?,
    /// ];
    /// dht.bootstrap(bootstrap_addrs).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bootstrap(&mut self, bootstrap_peers: Vec<Multiaddr>) -> Result<()> {
        tracing::info!("Bootstrapping DHT with {} peers", bootstrap_peers.len());

        // Add bootstrap peers to the routing table
        for addr in bootstrap_peers {
            // Extract peer ID from multiaddr if present
            if let Some(libp2p::multiaddr::Protocol::P2p(peer_id_multihash)) = addr.iter().last() {
                if let Ok(peer_id) = PeerId::from_multihash(peer_id_multihash.into()) {
                    tracing::debug!("Adding bootstrap peer: {}", peer_id);
                    self.swarm
                        .behaviour_mut()
                        .add_address(&peer_id, addr.clone());
                }
            }
        }

        // Start bootstrap process
        match self.swarm.behaviour_mut().bootstrap() {
            Ok(query_id) => {
                tracing::info!("Bootstrap initiated with query ID: {:?}", query_id);

                // Track the query
                let mut queries = self.active_queries.lock().unwrap();
                queries.insert(
                    query_id,
                    QueryInfo {
                        query_type: QueryType::Bootstrap,
                        started_at: SystemTime::now(),
                        response_tx: None,
                    },
                );

                Ok(())
            }
            Err(e) => {
                tracing::error!("Bootstrap failed: {:?}", e);
                Err(anyhow!("Bootstrap failed: {:?}", e))
            }
        }
    }

    /// Announce that we have content (become a provider)
    ///
    /// This will:
    /// 1. Add a provider record to the DHT
    /// 2. Replicate the record to the k closest nodes
    /// 3. Periodically republish the record (if auto-republish is enabled)
    ///
    /// # Arguments
    ///
    /// * `cid` - Content ID to provide
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::DHTManager;
    /// # use codio_content_id::ContentId;
    /// # async fn example(mut dht: DHTManager) -> anyhow::Result<()> {
    /// let cid = ContentId::new(b"Hello, world!");
    /// dht.provide(cid).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn provide(&mut self, cid: ContentId) -> Result<()> {
        tracing::info!("Announcing content: {}", cid);

        let key = cid_to_kad_key(&cid);

        match self.swarm.behaviour_mut().start_providing(key) {
            Ok(query_id) => {
                // Track locally
                let mut local_providers = self.local_providers.lock().unwrap();
                local_providers.insert(cid.clone());
                drop(local_providers);

                // Update stats
                let mut stats = self.stats.lock().unwrap();
                stats.local_content_count += 1;
                stats.total_queries += 1;
                drop(stats);

                // Track the query
                let mut queries = self.active_queries.lock().unwrap();
                queries.insert(
                    query_id,
                    QueryInfo {
                        query_type: QueryType::StartProviding { cid: cid.clone() },
                        started_at: SystemTime::now(),
                        response_tx: None,
                    },
                );

                tracing::debug!("Provide query started: {:?}", query_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to start providing: {:?}", e);

                let mut stats = self.stats.lock().unwrap();
                stats.failed_queries += 1;

                Err(anyhow!("Failed to start providing: {:?}", e))
            }
        }
    }

    /// Find providers for content
    ///
    /// Queries the DHT to find peers that have announced they have the specified content.
    ///
    /// # Arguments
    ///
    /// * `cid` - Content ID to search for
    ///
    /// # Returns
    ///
    /// A vector of peer information for providers of the content
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use codio_dht::DHTManager;
    /// # use codio_content_id::ContentId;
    /// # async fn example(mut dht: DHTManager) -> anyhow::Result<()> {
    /// let cid = ContentId::new(b"Hello, world!");
    /// let providers = dht.find_providers(cid).await?;
    /// println!("Found {} providers", providers.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_providers(&mut self, cid: ContentId) -> Result<Vec<PeerInfo>> {
        tracing::info!("Finding providers for: {}", cid);

        let key = cid_to_kad_key(&cid);
        let query_id = self.swarm.behaviour_mut().get_providers(key);

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Track the query
        {
            let mut queries = self.active_queries.lock().unwrap();
            queries.insert(
                query_id,
                QueryInfo {
                    query_type: QueryType::GetProviders { cid: cid.clone() },
                    started_at: SystemTime::now(),
                    response_tx: Some(tx),
                },
            );
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_queries += 1;
        }

        tracing::debug!("Get providers query started: {:?}", query_id);

        // Wait for response (with timeout)
        match tokio::time::timeout(self.config.query_timeout, rx).await {
            Ok(Ok(QueryResponse::Providers(providers))) => {
                tracing::info!("Found {} providers for {}", providers.len(), cid);

                let mut stats = self.stats.lock().unwrap();
                stats.successful_queries += 1;

                Ok(providers)
            }
            Ok(Ok(QueryResponse::Error(e))) => {
                tracing::error!("Provider query failed: {}", e);

                let mut stats = self.stats.lock().unwrap();
                stats.failed_queries += 1;

                Err(anyhow!("Provider query failed: {}", e))
            }
            Ok(Ok(_)) => {
                let mut stats = self.stats.lock().unwrap();
                stats.failed_queries += 1;

                Err(anyhow!("Unexpected query response type"))
            }
            Ok(Err(_)) => {
                let mut stats = self.stats.lock().unwrap();
                stats.failed_queries += 1;

                Err(anyhow!("Query response channel closed"))
            }
            Err(_) => {
                let mut stats = self.stats.lock().unwrap();
                stats.failed_queries += 1;

                Err(anyhow!("Query timeout"))
            }
        }
    }

    /// Stop providing content
    ///
    /// Removes our provider record for the specified content from the DHT.
    ///
    /// # Arguments
    ///
    /// * `cid` - Content ID to stop providing
    pub async fn stop_providing(&mut self, cid: &ContentId) -> Result<()> {
        tracing::info!("Stopping provision of: {}", cid);

        let key = cid_to_kad_key(cid);
        self.swarm.behaviour_mut().stop_providing(&key);

        // Remove from local tracking
        let mut local_providers = self.local_providers.lock().unwrap();
        local_providers.remove(cid);
        drop(local_providers);

        // Update stats
        let mut stats = self.stats.lock().unwrap();
        stats.local_content_count = stats.local_content_count.saturating_sub(1);

        Ok(())
    }

    /// Find a specific peer by ID
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Peer ID to search for
    ///
    /// # Returns
    ///
    /// A vector of known addresses for the peer
    pub async fn find_peer(&mut self, peer_id: PeerId) -> Result<Vec<Multiaddr>> {
        tracing::info!("Finding peer: {}", peer_id);

        let query_id = self.swarm.behaviour_mut().get_closest_peers(peer_id);

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Track the query
        {
            let mut queries = self.active_queries.lock().unwrap();
            queries.insert(
                query_id,
                QueryInfo {
                    query_type: QueryType::FindPeer { peer_id },
                    started_at: SystemTime::now(),
                    response_tx: Some(tx),
                },
            );
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_queries += 1;
        }

        // Wait for response
        match tokio::time::timeout(self.config.query_timeout, rx).await {
            Ok(Ok(QueryResponse::PeerFound(peer_info))) => {
                let mut stats = self.stats.lock().unwrap();
                stats.successful_queries += 1;

                Ok(peer_info.addresses)
            }
            _ => {
                let mut stats = self.stats.lock().unwrap();
                stats.failed_queries += 1;

                Err(anyhow!("Failed to find peer"))
            }
        }
    }

    /// Get the closest peers to a key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to find closest peers for
    ///
    /// # Returns
    ///
    /// A vector of peer IDs, sorted by XOR distance to the key
    pub fn get_closest_peers(&self, _key: &[u8]) -> Vec<PeerId> {
        // This is a simplified version - in production, we'd query the routing table
        // For now, return empty vector
        Vec::new()
    }

                        let _ = self
                            .event_tx
                            .send(DhtEvent::ProvidersFound { cid, providers });
                    }
                    QueryResult::StartProviding(Ok(_)) => {
                        // Provider record published
                        tracing::debug!("Content announced to DHT");
                    }
                    QueryResult::Bootstrap(Ok(_)) => {
                        let _ = self.event_tx.send(DhtEvent::BootstrapComplete);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Handle Kademlia-specific events
    async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryProgressed { id, result, .. } => {
                self.handle_query_result(id, result).await;
            }
            KademliaEvent::RoutingUpdated { peer, .. } => {
                tracing::debug!("Routing table updated with peer: {}", peer);

                // Count peers in routing table
                let num_peers = self.stats.lock().unwrap().num_peers;

                let _ = self.event_tx.send(DHTEvent::RoutingTableUpdated {
                    num_peers,
                });
            }
            KademliaEvent::InboundRequest { request } => {
                tracing::trace!("Inbound DHT request: {:?}", request);
            }
            _ => {}
        }
    }

    /// Handle query results
    async fn handle_query_result(&mut self, query_id: libp2p::kad::QueryId, result: QueryResult) {
        // Get query info
        let query_info = {
            let mut queries = self.active_queries.lock().unwrap();
            queries.remove(&query_id)
        };

        let Some(query_info) = query_info else {
            tracing::warn!("Received result for unknown query: {:?}", query_id);
            return;
        };

        match result {
            QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders {
                key,
                providers,
                ..
            })) => {
                self.handle_providers_found(query_info, key, providers)
                    .await;
            }
            QueryResult::GetProviders(Err(e)) => {
                tracing::error!("Get providers failed: {:?}", e);

                if let Some(tx) = query_info.response_tx {
                    let _ = tx.send(QueryResponse::Error(format!("{:?}", e)));
                }
            }
            QueryResult::StartProviding(Ok(_)) => {
                if let QueryType::StartProviding { cid } = query_info.query_type {
                    tracing::info!("Successfully announced content: {}", cid);

                    let _ = self
                        .event_tx
                        .send(DHTEvent::ProvideSuccess { cid: cid.clone() });

                    if let Some(tx) = query_info.response_tx {
                        let _ = tx.send(QueryResponse::ProvideComplete);
                    }
                }
            }
            QueryResult::StartProviding(Err(e)) => {
                tracing::error!("Start providing failed: {:?}", e);

                if let QueryType::StartProviding { cid } = query_info.query_type {
                    let _ = self.event_tx.send(DHTEvent::ProvideFailed {
                        cid,
                        error: format!("{:?}", e),
                    });
                }
            }
            QueryResult::Bootstrap(Ok(BootstrapOk { peer: _, .. })) => {
                tracing::info!("Bootstrap successful with {} peers", 1);

                let mut stats = self.stats.lock().unwrap();
                stats.is_bootstrapped = true;
                stats.last_bootstrap = Some(SystemTime::now());
                let num_peers = stats.num_peers;
                drop(stats);

                let _ = self
                    .event_tx
                    .send(DHTEvent::BootstrapComplete { num_peers });

                if let Some(tx) = query_info.response_tx {
                    let _ = tx.send(QueryResponse::BootstrapComplete { num_peers });
                }
            }
            QueryResult::Bootstrap(Err(e)) => {
                tracing::error!("Bootstrap failed: {:?}", e);

                let _ = self.event_tx.send(DHTEvent::BootstrapFailed {
                    error: format!("{:?}", e),
                });
            }
            QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { key: _, peers })) => {
                tracing::debug!("Found {} closest peers", peers.len());

                if let Some(tx) = query_info.response_tx {
                    let _ = tx.send(QueryResponse::ClosestPeers(peers));
                }
            }
            QueryResult::GetClosestPeers(Err(e)) => {
                tracing::error!("Get closest peers failed: {:?}", e);

                if let Some(tx) = query_info.response_tx {
                    let _ = tx.send(QueryResponse::Error(format!("{:?}", e)));
                }
            }
            _ => {
                tracing::trace!("Unhandled query result: {:?}", result);
            }
        }
    }

    /// Handle providers found event
    async fn handle_providers_found(
        &mut self,
        query_info: QueryInfo,
        key: RecordKey,
        providers: HashSet<PeerId>,
    ) {
        // Convert key back to CID
        let cid = match kad_key_to_cid(&key) {
            Ok(cid) => cid,
            Err(e) => {
                tracing::error!("Failed to convert key to CID: {}", e);
                return;
            }
        };

        tracing::info!("Found {} providers for {}", providers.len(), cid);

        // Convert to PeerInfo
        let peer_infos: Vec<PeerInfo> = providers
            .into_iter()
            .map(|peer_id| {
                // Note: libp2p 0.53 doesn't expose addresses_of_peer in the public API
                // In a production environment, we'd maintain our own address book
                // For now, we return peers without addresses
                PeerInfo::new(peer_id)
            })
            .collect();

        // Store provider records
        {
            let mut records = self.provider_records.lock().unwrap();
            let entry = records.entry(cid.clone()).or_default();

            for peer_info in &peer_infos {
                let mut record = ProviderRecord::new(cid.clone(), peer_info.clone());
                record.distance = Distance::between(
                    self.peer_id.to_bytes().as_slice(),
                    peer_info.peer_id.to_bytes().as_slice(),
                );
                entry.push(record);
            }
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.num_providers = self.provider_records.lock().unwrap().len();
        }

        // Send event
        let _ = self.event_tx.send(DHTEvent::ProvidersFound {
            cid: cid.clone(),
            providers: peer_infos.clone(),
        });

        // Send response if waiting
        if let Some(tx) = query_info.response_tx {
            let _ = tx.send(QueryResponse::Providers(peer_infos));
        }
    }

    /// Republish content we're providing
    async fn republish_content(&mut self) {
        let cids: Vec<ContentId> = {
            let providers = self.local_providers.lock().unwrap();
            providers.iter().cloned().collect()
        };

        if cids.is_empty() {
            return;
        }

        tracing::info!("Republishing {} content items", cids.len());

        for cid in cids {
            let key = cid_to_kad_key(&cid);
            if let Err(e) = self.swarm.behaviour_mut().start_providing(key) {
                tracing::error!("Failed to republish {}: {:?}", cid, e);
            }
        }

        // Update last republish time
        let mut last = self.last_republish.lock().unwrap();
        *last = SystemTime::now();
    }

    /// Clean up expired provider records
    fn cleanup_expired_records(&mut self) {
        let timeout = self.config.provider_timeout;
        let mut expired = Vec::new();

        {
            let mut records = self.provider_records.lock().unwrap();

            for (cid, providers) in records.iter_mut() {
                providers.retain(|record| {
                    if record.is_expired(timeout) {
                        expired.push((cid.clone(), record.provider.peer_id));
                        false
                    } else {
                        true
                    }
                });
            }

            // Remove empty entries
            records.retain(|_, providers| !providers.is_empty());
        }

        // Send expiration events
        for (cid, provider) in expired {
            tracing::debug!("Provider record expired: {} from {}", cid, provider);

            let _ = self
                .event_tx
                .send(DHTEvent::ProviderExpired { cid, provider });
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.num_providers = self.provider_records.lock().unwrap().len();
            stats.pending_queries = self.active_queries.lock().unwrap().len();
        }
    }
}

/// Convert ContentId to Kademlia RecordKey
fn cid_to_kad_key(cid: &ContentId) -> RecordKey {
    RecordKey::new(cid.hash())
}

/// Convert Kademlia RecordKey to ContentId
fn kad_key_to_cid(key: &RecordKey) -> Result<ContentId> {
    // The key is the raw hash bytes
    let key_bytes = key.as_ref();

    if key_bytes.len() != 32 {
        return Err(anyhow!(
            "Invalid key length: expected 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    // Reconstruct CID from hash
    let multibase = format!("Qm{}", bs58::encode(key_bytes).into_string());
    multibase.parse().map_err(|e| anyhow!("Failed to parse CID: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dht_creation() {
        let config = DHTConfig::default();
        let dht = DHTManager::new(config).await;
        assert!(dht.is_ok());
    }

    #[tokio::test]
    async fn test_cid_key_conversion() {
        let cid = ContentId::new(b"Test content");
        let key = cid_to_kad_key(&cid);
        let cid2 = kad_key_to_cid(&key).unwrap();
        assert_eq!(cid, cid2);
    }

    #[tokio::test]
    async fn test_peer_info_creation() {
        let peer_id = PeerId::random();
        let peer_info = PeerInfo::new(peer_id);
        assert_eq!(peer_info.peer_id, peer_id);
        assert!(peer_info.addresses.is_empty());
    }

    #[tokio::test]
    async fn test_distance_calculation() {
        let a = [0u8; 32];
        let b = [255u8; 32];
        let dist = Distance::between(&a, &b);
        assert_eq!(dist.as_bytes()[0], 255);
    }

    #[tokio::test]
    async fn test_provider_record_expiration() {
        let cid = ContentId::new(b"Test");
        let peer_info = PeerInfo::new(PeerId::random());
        let mut record = ProviderRecord::new(cid, peer_info);

        // Fresh record should not be expired
        assert!(!record.is_expired(Duration::from_secs(3600)));

        // Manually set old timestamp
        record.timestamp = SystemTime::now() - Duration::from_secs(7200);
        assert!(record.is_expired(Duration::from_secs(3600)));
    }

    #[tokio::test]
    async fn test_stats_success_rate() {
        let mut stats = DHTStats::default();
        assert_eq!(stats.success_rate(), 0.0);

        stats.total_queries = 10;
        stats.successful_queries = 8;
        assert_eq!(stats.success_rate(), 0.8);
    }
}
