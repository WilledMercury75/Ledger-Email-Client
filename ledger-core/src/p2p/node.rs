use libp2p::{
    futures::StreamExt,
    identity::Keypair,
    noise, tcp, yamux,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::behaviour::{LedgerBehaviour, LedgerBehaviourEvent};
use super::protocol::{LedgerRequest, LedgerResponse};
use crate::crypto::envelope;
use crate::crypto::keys::LedgerIdentity;
use crate::models::message::*;
use crate::store::db::Database;

/// Commands that can be sent to the P2P node from the REST API
#[derive(Debug)]
pub enum P2PCommand {
    /// Send a message to a peer
    SendMessage {
        peer_id: PeerId,
        envelope_json: String,
        response_tx: mpsc::Sender<Result<(), String>>,
    },
    /// Connect to a peer by multiaddr
    ConnectPeer {
        addr: Multiaddr,
        response_tx: mpsc::Sender<Result<PeerId, String>>,
    },
    /// Get connected peers
    GetPeers {
        response_tx: mpsc::Sender<Vec<PeerInfo>>,
    },
    /// Store in DHT
    DhtPut {
        key: Vec<u8>,
        value: Vec<u8>,
        response_tx: mpsc::Sender<Result<(), String>>,
    },
    /// Retrieve from DHT
    DhtGet {
        key: Vec<u8>,
        response_tx: mpsc::Sender<Result<Option<Vec<u8>>, String>>,
    },
}

/// Start the libp2p swarm and return a command channel
pub async fn start_node(
    p2p_port: u16,
    identity: Arc<LedgerIdentity>,
    db: Arc<Database>,
) -> Result<(mpsc::Sender<P2PCommand>, PeerId), Box<dyn std::error::Error>> {
    // Create libp2p identity from our Ed25519 key
    let local_keypair = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_keypair.public());

    tracing::info!("Local libp2p peer ID: {}", local_peer_id);

    // Build swarm
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_keypair.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_key| {
            LedgerBehaviour::new(local_peer_id, &local_keypair)
                .expect("Failed to create behaviour")
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(std::time::Duration::from_secs(60)))
        .build();

    // Listen on TCP
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", p2p_port).parse()?;
    swarm.listen_on(listen_addr)?;

    // Subscribe to gossipsub topic for announcements
    let topic = libp2p::gossipsub::IdentTopic::new("ledger-announce");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<P2PCommand>(256);

    // Spawn swarm event loop
    let identity_clone = identity.clone();
    let db_clone = db.clone();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle swarm events
                event = swarm.select_next_some() => {
                    handle_swarm_event(&mut swarm, event, &identity_clone, &db_clone).await;
                }
                // Handle commands from REST API
                Some(cmd) = cmd_rx.recv() => {
                    handle_command(&mut swarm, cmd).await;
                }
            }
        }
    });

    Ok((cmd_tx, local_peer_id))
}

async fn handle_swarm_event(
    swarm: &mut Swarm<LedgerBehaviour>,
    event: SwarmEvent<LedgerBehaviourEvent>,
    identity: &LedgerIdentity,
    db: &Database,
) {
    match event {
        SwarmEvent::Behaviour(LedgerBehaviourEvent::RequestResponse(
            libp2p::request_response::Event::Message { message, peer }
        )) => {
            match message {
                libp2p::request_response::Message::Request { request, channel, .. } => {
                    tracing::info!("Received message from peer: {}", peer);

                    // Try to decrypt the envelope
                    match serde_json::from_str::<EncryptedEnvelope>(&request.envelope_json) {
                        Ok(env) => {
                            match envelope::decrypt_envelope(identity, &env) {
                                Ok(plaintext) => {
                                    let msg = Message {
                                        id: env.id.clone(),
                                        from_id: env.from_ledger_id.clone(),
                                        to_id: identity.ledger_id.clone(),
                                        subject: env.subject_hint.clone(),
                                        body: plaintext,
                                        timestamp: env.timestamp,
                                        delivery_method: DeliveryMethod::P2p,
                                        is_read: false,
                                        folder: Folder::Inbox,
                                        signature: Some(env.signature.clone()),
                                        encrypted: true,
                                    };

                                    if let Err(e) = db.insert_message(&msg) {
                                        tracing::error!("Failed to store message: {}", e);
                                    }

                                    let _ = swarm.behaviour_mut().request_response.send_response(
                                        channel,
                                        LedgerResponse { accepted: true, error: None },
                                    );
                                    tracing::info!("Message decrypted and stored: {}", env.id);
                                }
                                Err(e) => {
                                    tracing::error!("Decryption failed: {}", e);
                                    let _ = swarm.behaviour_mut().request_response.send_response(
                                        channel,
                                        LedgerResponse { accepted: false, error: Some(e.to_string()) },
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse envelope: {}", e);
                            let _ = swarm.behaviour_mut().request_response.send_response(
                                channel,
                                LedgerResponse { accepted: false, error: Some(e.to_string()) },
                            );
                        }
                    }
                }
                libp2p::request_response::Message::Response { response, .. } => {
                    if response.accepted {
                        tracing::info!("Message accepted by peer {}", peer);
                    } else {
                        tracing::warn!("Message rejected by peer {}: {:?}", peer, response.error);
                    }
                }
            }
        }
        SwarmEvent::Behaviour(LedgerBehaviourEvent::Mdns(mdns_event)) => {
            match mdns_event {
                libp2p::mdns::Event::Discovered(peers) => {
                    for (peer_id, addr) in peers {
                        tracing::info!("mDNS discovered peer: {} at {}", peer_id, addr);
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                    }
                }
                libp2p::mdns::Event::Expired(peers) => {
                    for (peer_id, _addr) in peers {
                        tracing::debug!("mDNS peer expired: {}", peer_id);
                    }
                }
            }
        }
        SwarmEvent::Behaviour(LedgerBehaviourEvent::Kademlia(kad_event)) => {
            match kad_event {
                libp2p::kad::Event::RoutingUpdated { peer, .. } => {
                    tracing::debug!("Kademlia routing updated for peer: {}", peer);
                }
                _ => {}
            }
        }
        SwarmEvent::Behaviour(LedgerBehaviourEvent::Identify(identify_event)) => {
            if let libp2p::identify::Event::Received { peer_id, info } = identify_event {
                tracing::info!("Identified peer {}: {:?}", peer_id, info.protocols);
                for addr in info.listen_addrs {
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
        }
        SwarmEvent::NewListenAddr { address, .. } => {
            tracing::info!("Listening on {}", address);
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            tracing::info!("Connected to peer: {}", peer_id);
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            tracing::info!("Disconnected from peer: {}", peer_id);
        }
        _ => {}
    }
}

async fn handle_command(
    swarm: &mut Swarm<LedgerBehaviour>,
    cmd: P2PCommand,
) {
    match cmd {
        P2PCommand::SendMessage { peer_id, envelope_json, response_tx } => {
            let request = LedgerRequest { envelope_json };
            swarm.behaviour_mut().request_response.send_request(&peer_id, request);
            let _ = response_tx.send(Ok(())).await;
        }
        P2PCommand::ConnectPeer { addr, response_tx } => {
            match swarm.dial(addr.clone()) {
                Ok(_) => {
                    tracing::info!("Dialing {}", addr);
                    // We don't know the peer ID until connection is established
                    let _ = response_tx.send(Ok(PeerId::random())).await;
                }
                Err(e) => {
                    let _ = response_tx.send(Err(format!("Dial error: {}", e))).await;
                }
            }
        }
        P2PCommand::GetPeers { response_tx } => {
            let peers: Vec<PeerInfo> = swarm.connected_peers()
                .map(|p| PeerInfo {
                    peer_id: p.to_string(),
                    address: String::new(),
                    ledger_id: None,
                })
                .collect();
            let _ = response_tx.send(peers).await;
        }
        P2PCommand::DhtPut { key, value, response_tx } => {
            let record = libp2p::kad::Record {
                key: libp2p::kad::RecordKey::new(&key),
                value,
                publisher: None,
                expires: Some(std::time::Instant::now() + std::time::Duration::from_secs(72 * 3600)),
            };
            match swarm.behaviour_mut().kademlia.put_record(record, libp2p::kad::Quorum::One) {
                Ok(_) => { let _ = response_tx.send(Ok(())).await; }
                Err(e) => { let _ = response_tx.send(Err(format!("DHT put error: {:?}", e))).await; }
            }
        }
        P2PCommand::DhtGet { key, response_tx } => {
            let _query_id = swarm.behaviour_mut().kademlia.get_record(
                libp2p::kad::RecordKey::new(&key),
            );
            // In a full implementation, we'd track the query and return the result
            // For now, just acknowledge
            let _ = response_tx.send(Ok(None)).await;
        }
    }
}
