pub use codio_content_id::ContentId;

/// Peer identifier in the network
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub String);

/// Network address for a peer
#[derive(Debug, Clone)]
pub struct PeerAddr {
    pub peer_id: PeerId,
    pub multiaddr: String, // /ip4/127.0.0.1/tcp/4001
}

/// Content metadata
#[derive(Debug, Clone)]
pub struct ContentMeta {
    pub cid: ContentId,
    pub size: u64,
    pub providers: Vec<PeerAddr>,
}
