use libp2p::{
    gossipsub, identify, kad, mdns,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
};

use super::protocol::{LedgerRequest, LedgerResponse, PROTOCOL_NAME};

/// Ledger's composite network behaviour
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "LedgerBehaviourEvent")]
pub struct LedgerBehaviour {
    /// Direct message delivery
    pub request_response: request_response::cbor::Behaviour<LedgerRequest, LedgerResponse>,
    /// Pub/sub for announcements
    pub gossipsub: gossipsub::Behaviour,
    /// DHT for offline message storage & peer discovery
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    /// Local peer discovery
    pub mdns: mdns::tokio::Behaviour,
    /// Peer identification
    pub identify: identify::Behaviour,
}

/// Combined events from all sub-behaviours
#[derive(Debug)]
pub enum LedgerBehaviourEvent {
    RequestResponse(request_response::Event<LedgerRequest, LedgerResponse>),
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    Mdns(mdns::Event),
    Identify(identify::Event),
}

impl From<request_response::Event<LedgerRequest, LedgerResponse>> for LedgerBehaviourEvent {
    fn from(e: request_response::Event<LedgerRequest, LedgerResponse>) -> Self {
        LedgerBehaviourEvent::RequestResponse(e)
    }
}

impl From<gossipsub::Event> for LedgerBehaviourEvent {
    fn from(e: gossipsub::Event) -> Self {
        LedgerBehaviourEvent::Gossipsub(e)
    }
}

impl From<kad::Event> for LedgerBehaviourEvent {
    fn from(e: kad::Event) -> Self {
        LedgerBehaviourEvent::Kademlia(e)
    }
}

impl From<mdns::Event> for LedgerBehaviourEvent {
    fn from(e: mdns::Event) -> Self {
        LedgerBehaviourEvent::Mdns(e)
    }
}

impl From<identify::Event> for LedgerBehaviourEvent {
    fn from(e: identify::Event) -> Self {
        LedgerBehaviourEvent::Identify(e)
    }
}

impl LedgerBehaviour {
    pub fn new(
        local_peer_id: libp2p::PeerId,
        keypair: &libp2p::identity::Keypair,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Request-Response for direct messaging
        let request_response = request_response::cbor::Behaviour::new(
            [(PROTOCOL_NAME, ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        // Gossipsub for announcements
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| format!("Gossipsub config error: {}", e))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        ).map_err(|e| format!("Gossipsub error: {}", e))?;

        // Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kademlia = kad::Behaviour::new(local_peer_id, store);
        kademlia.set_mode(Some(kad::Mode::Server));

        // mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            local_peer_id,
        )?;

        // Identify protocol
        let identify = identify::Behaviour::new(identify::Config::new(
            "/ledger/id/1.0.0".to_string(),
            keypair.public(),
        ));

        Ok(Self {
            request_response,
            gossipsub,
            kademlia,
            mdns,
            identify,
        })
    }
}
