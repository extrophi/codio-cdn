// crates/dht/src/lib.rs

use libp2p::{
    kad::{
        store::MemoryStore, Config as KademliaConfig, Event as KademliaEvent,
        QueryResult, Behaviour as Kademlia,
    },
    swarm::{Swarm, SwarmEvent},
    PeerId, Multiaddr, SwarmBuilder,
};
use codio_content_id::ContentId;
use std::time::Duration;
use tokio::sync::mpsc;

/// DHT configuration
#[derive(Debug, Clone)]
pub struct DhtConfig {
    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    /// Query timeout
    pub query_timeout: Duration,
    /// Provider record TTL
    pub provider_ttl: Duration,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: vec![],
            query_timeout: Duration::from_secs(30),
            provider_ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// DHT node for content discovery
pub struct DhtNode {
    swarm: Swarm<Kademlia<MemoryStore>>,
    event_tx: mpsc::UnboundedSender<DhtEvent>,
}

/// Events from the DHT
#[derive(Debug)]
pub enum DhtEvent {
    /// Providers found for content
    ProvidersFound {
        cid: ContentId,
        providers: Vec<PeerId>,
    },
    /// Content announcement successful
    ProvideSuccess {
        cid: ContentId,
    },
    /// Bootstrap completed
    BootstrapComplete,
}

impl DhtNode {
    /// Create new DHT node
    pub async fn new(config: DhtConfig) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<DhtEvent>)> {
        // Generate keypair
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("Local peer ID: {}", local_peer_id);

        // Create Kademlia store
        let store = MemoryStore::new(local_peer_id);

        // Configure Kademlia
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(config.query_timeout);
        kad_config.set_provider_record_ttl(Some(config.provider_ttl));

        // Create Kademlia behaviour
        let mut kademlia = Kademlia::with_config(local_peer_id, store, kad_config);

        // Add bootstrap peers
        for (peer_id, addr) in config.bootstrap_peers {
            kademlia.add_address(&peer_id, addr);
        }

        // Create swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| kademlia)?
            .build();

        // Event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok((
            DhtNode { swarm, event_tx },
            event_rx
        ))
    }

    /// Start listening on address
    pub async fn listen(&mut self, addr: Multiaddr) -> anyhow::Result<()> {
        self.swarm.listen_on(addr)?;
        Ok(())
    }

    /// Bootstrap DHT (connect to network)
    pub async fn bootstrap(&mut self) -> anyhow::Result<()> {
        self.swarm.behaviour_mut().bootstrap()?;
        Ok(())
    }

    /// Announce content availability
    pub async fn provide(&mut self, cid: ContentId) -> anyhow::Result<()> {
        let key = cid_to_kad_key(&cid);
        self.swarm.behaviour_mut().start_providing(key)?;
        Ok(())
    }

    /// Find providers for content
    pub async fn find_providers(&mut self, cid: ContentId) -> anyhow::Result<()> {
        let key = cid_to_kad_key(&cid);
        self.swarm.behaviour_mut().get_providers(key);
        Ok(())
    }

    /// Event loop (call continuously)
    pub async fn poll(&mut self) -> Option<SwarmEvent<KademliaEvent>> {
        use futures::StreamExt;
        self.swarm.next().await
    }

    /// Handle Kademlia event
    pub fn handle_event(&self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryProgressed { result, .. } => {
                match result {
                    QueryResult::GetProviders(Ok(_result)) => {
                        // Providers found successfully
                        // Note: In libp2p 0.53, GetProvidersOk structure changed
                        // We would need to track queries separately to match CIDs
                        tracing::debug!("Get providers query completed");
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
}

/// Convert CID to Kademlia key
fn cid_to_kad_key(cid: &ContentId) -> libp2p::kad::RecordKey {
    libp2p::kad::RecordKey::new(cid.hash())
}

/// Convert Kademlia key to CID
fn kad_key_to_cid(key: &libp2p::kad::RecordKey) -> ContentId {
    // Reconstruct CID from hash bytes
    // Note: This is simplified - production needs proper CID reconstruction
    ContentId::from_str(&format!("Qm{}", bs58::encode(key.as_ref()).into_string()))
        .expect("Invalid CID from DHT")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dht_creation() {
        let config = DhtConfig::default();
        let result = DhtNode::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cid_kad_key_roundtrip() {
        let cid = ContentId::new(b"Test content");
        let key = cid_to_kad_key(&cid);
        let cid2 = kad_key_to_cid(&key);
        assert_eq!(cid, cid2);
    }

    #[tokio::test]
    async fn test_dht_listen() {
        let config = DhtConfig::default();
        let (mut node, _rx) = DhtNode::new(config).await.unwrap();

        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        let result = node.listen(addr).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dht_provide() {
        let config = DhtConfig::default();
        let (mut node, _rx) = DhtNode::new(config).await.unwrap();

        // Listen first
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        node.listen(addr).await.unwrap();

        // Provide content
        let cid = ContentId::new(b"Test content");
        let result = node.provide(cid).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dht_find_providers() {
        let config = DhtConfig::default();
        let (mut node, _rx) = DhtNode::new(config).await.unwrap();

        // Listen first
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        node.listen(addr).await.unwrap();

        // Find providers
        let cid = ContentId::new(b"Test content");
        let result = node.find_providers(cid).await;
        assert!(result.is_ok());
    }
}
