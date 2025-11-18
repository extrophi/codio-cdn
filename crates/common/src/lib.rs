pub use codio_content_id::ContentId;

/// Peer identifier in the network
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub String);

impl PeerId {
    /// Create a new PeerId
    pub fn new(id: String) -> Self {
        PeerId(id)
    }

    /// Get the peer ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Network address for a peer
#[derive(Debug, Clone)]
pub struct PeerAddr {
    pub peer_id: PeerId,
    pub multiaddr: String, // /ip4/127.0.0.1/tcp/4001
}

impl PeerAddr {
    /// Create a new PeerAddr
    pub fn new(peer_id: PeerId, multiaddr: String) -> Self {
        PeerAddr { peer_id, multiaddr }
    }
}

/// Content metadata
#[derive(Debug, Clone)]
pub struct ContentMeta {
    pub cid: ContentId,
    pub size: u64,
    pub providers: Vec<PeerAddr>,
}

impl ContentMeta {
    /// Create new content metadata
    pub fn new(cid: ContentId, size: u64) -> Self {
        ContentMeta {
            cid,
            size,
            providers: Vec::new(),
        }
    }

    /// Add a provider to the content metadata
    pub fn add_provider(&mut self, provider: PeerAddr) {
        self.providers.push(provider);
    }

    /// Get the number of providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_creation() {
        let peer_id = PeerId::new("peer123".to_string());
        assert_eq!(peer_id.as_str(), "peer123");
    }

    #[test]
    fn test_peer_id_display() {
        let peer_id = PeerId::new("peer456".to_string());
        assert_eq!(format!("{}", peer_id), "peer456");
    }

    #[test]
    fn test_peer_id_equality() {
        let peer_id1 = PeerId::new("peer789".to_string());
        let peer_id2 = PeerId::new("peer789".to_string());
        let peer_id3 = PeerId::new("peer000".to_string());

        assert_eq!(peer_id1, peer_id2);
        assert_ne!(peer_id1, peer_id3);
    }

    #[test]
    fn test_peer_addr_creation() {
        let peer_id = PeerId::new("peer123".to_string());
        let addr = PeerAddr::new(peer_id.clone(), "/ip4/127.0.0.1/tcp/4001".to_string());

        assert_eq!(addr.peer_id, peer_id);
        assert_eq!(addr.multiaddr, "/ip4/127.0.0.1/tcp/4001");
    }

    #[test]
    fn test_content_meta_creation() {
        let content = b"Test content";
        let cid = ContentId::new(content);
        let meta = ContentMeta::new(cid.clone(), 1234);

        assert_eq!(meta.cid, cid);
        assert_eq!(meta.size, 1234);
        assert_eq!(meta.provider_count(), 0);
    }

    #[test]
    fn test_content_meta_add_provider() {
        let content = b"Test content";
        let cid = ContentId::new(content);
        let mut meta = ContentMeta::new(cid, 1234);

        let peer_id = PeerId::new("peer1".to_string());
        let addr = PeerAddr::new(peer_id, "/ip4/127.0.0.1/tcp/4001".to_string());

        meta.add_provider(addr);
        assert_eq!(meta.provider_count(), 1);
    }

    #[test]
    fn test_content_meta_multiple_providers() {
        let content = b"Test content";
        let cid = ContentId::new(content);
        let mut meta = ContentMeta::new(cid, 1234);

        for i in 0..5 {
            let peer_id = PeerId::new(format!("peer{}", i));
            let addr = PeerAddr::new(peer_id, format!("/ip4/127.0.0.1/tcp/{}", 4000 + i));
            meta.add_provider(addr);
        }

        assert_eq!(meta.provider_count(), 5);
    }
}
