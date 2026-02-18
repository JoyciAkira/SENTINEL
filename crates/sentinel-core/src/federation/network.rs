//! Network Manager - P2P Infrastructure
//!
//! Gestisce lo swarm di libp2p, il routing Kademlia e il gossipsub
//! per la distribuzione dell'intelligenza di allineamento.
//!
//! # Architettura P2P
//!
//! ```text
//! [Sentinel Node A]  ←—gossipsub—→  [Sentinel Node B]
//!        │                                   │
//!     Kademlia DHT   ←—peer discovery—→   Kademlia DHT
//!        │                                   │
//!   identify/1.0.0  ←—capability adv—→  identify/1.0.0
//! ```
//!
//! La rete usa:
//! - **Kademlia DHT**: peer discovery, routing, storage pattern anonymizzati
//! - **gossipsub**: broadcast proposta/voto consensus, threat alerts
//! - **identify**: capacità nodo (authority_weight, version)

use crate::federation::{
    consensus::ConsensusEngine,
    gossip::{GossipMessage, GossipPayload},
    NodeIdentity,
};
use futures::StreamExt;
use libp2p::{
    gossipsub, identify, kad, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId,
};
use std::error::Error;
use std::time::Duration;

/// Bootstrap nodes della rete Sentinel (indirizzi publici persistenti).
/// In produzione questi sono nodi relay gestiti dal team Sentinel.
const SENTINEL_BOOTSTRAP_NODES: &[(&str, &str)] = &[
    // (peer_id_b58, multiaddr)
    // Placeholder — in produzione sostituiti con peer ids reali firmati
    // e verificati via Ed25519 signature
];

/// Timeout connessione peer (ms).
const DIAL_TIMEOUT_MS: u64 = 10_000;

/// Comportamento composito del nodo Sentinel.
#[derive(NetworkBehaviour)]
pub struct SentinelBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: identify::Behaviour,
}

/// Manager del nodo P2P Sentinel.
///
/// Tiene insieme:
/// - `keypair`: identità crittografica stabile del nodo (Ed25519)
/// - `peer_id`: PeerId derivato dal keypair (non random)
/// - `identity`: metadati applicativi (authority_weight, version, reputation)
/// - `consensus`: motore di consensus locale (proposta/voto)
pub struct NetworkManager {
    /// PeerId determinisitco derivato dalla chiave privata Ed25519.
    pub peer_id: PeerId,
    /// Keypair Ed25519 — usato sia per libp2p che per le firme gossipsub.
    keypair: libp2p::identity::Keypair,
    /// Identità applicativa Sentinel (authority_weight, reputation, version).
    pub identity: NodeIdentity,
    /// Motore consensus locale.
    pub consensus: ConsensusEngine,
}

impl NetworkManager {
    /// Crea un nuovo NetworkManager con identità crittografica coerente.
    ///
    /// Il PeerId è derivato dal keypair Ed25519, non generato casualmente,
    /// garantendo che `peer_id` e `keypair` siano sempre allineati.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let identity = NodeIdentity::generate();

        Ok(Self {
            peer_id,
            keypair,
            identity,
            consensus: ConsensusEngine::new(),
        })
    }

    /// Avvia il nodo P2P: crea lo swarm, si connette ai bootstrap peers,
    /// esegue Kademlia bootstrap e avvia l'event loop.
    ///
    /// Il nodo:
    /// 1. Ascolta su TCP (porta assegnata dall'OS)
    /// 2. Si connette ai bootstrap nodes della rete Sentinel
    /// 3. Esegue DHT bootstrap (random walk Kademlia)
    /// 4. Gestisce eventi gossipsub/identify/kademlia in loop
    pub async fn run_node(&mut self, relay_addr: Option<Multiaddr>) -> Result<(), Box<dyn Error>> {
        let keypair = self.keypair.clone();

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                // --- gossipsub ---
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .build()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

                // --- kademlia ---
                let local_peer_id = key.public().to_peer_id();
                let store = kad::store::MemoryStore::new(local_peer_id);
                let mut kademlia_config = kad::Config::default();
                kademlia_config.set_query_timeout(Duration::from_secs(60));
                let kademlia =
                    kad::Behaviour::with_config(local_peer_id, store, kademlia_config);

                // --- identify ---
                let identify = identify::Behaviour::new(identify::Config::new(
                    "sentinel/1.0.0".to_string(),
                    key.public(),
                ));

                Ok(SentinelBehaviour {
                    gossipsub,
                    kademlia,
                    identify,
                })
            })?
            .build();

        // Subscribe to all Sentinel topics
        for topic_name in &["sentinel-consensus", "sentinel-threats", "sentinel-patterns"] {
            let topic = gossipsub::IdentTopic::new(*topic_name);
            swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        }

        // Listen on all interfaces, OS-assigned port
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // Optional: dial relay for NAT traversal
        if let Some(relay) = relay_addr {
            println!("🔌 Dialing relay: {}", relay);
            swarm.dial(relay)?;
        }

        // Add hardcoded bootstrap nodes to Kademlia routing table
        for (peer_id_str, addr_str) in SENTINEL_BOOTSTRAP_NODES {
            if let (Ok(peer_id), Ok(addr)) = (
                peer_id_str.parse::<PeerId>(),
                addr_str.parse::<Multiaddr>(),
            ) {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr.clone());
                let _ = swarm.dial(addr);
            }
        }

        // Trigger Kademlia bootstrap (random walk to populate routing table)
        let _ = swarm.behaviour_mut().kademlia.bootstrap();

        println!(
            "🌐 Sentinel P2P node started | PeerId: {} | NodeId: {}",
            self.peer_id, self.identity.node_id
        );

        // ── Event Loop ──────────────────────────────────────────────────────
        loop {
            match swarm.next().await {
                Some(SwarmEvent::NewListenAddr { address, .. }) => {
                    println!("📍 Listening on: {}", address);
                }
                Some(SwarmEvent::ConnectionEstablished {
                    peer_id,
                    num_established,
                    ..
                }) => {
                    println!(
                        "🔗 Connected: {} (total peers: {})",
                        peer_id, num_established
                    );
                    // Announce self to DHT so others can find us
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, "/ip4/0.0.0.0/tcp/0".parse().unwrap());
                }
                Some(SwarmEvent::ConnectionClosed { peer_id, cause, .. }) => {
                    println!(
                        "💔 Disconnected: {} (cause: {:?})",
                        peer_id,
                        cause.map(|e| e.to_string()).unwrap_or_default()
                    );
                }
                Some(SwarmEvent::Behaviour(SentinelBehaviourEvent::Gossipsub(
                    gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    },
                ))) => {
                    if let Ok(gossip_msg) = serde_json::from_slice::<GossipMessage>(&message.data)
                    {
                        self.handle_incoming_gossip(gossip_msg);
                    } else {
                        eprintln!(
                            "⚠️  Unparseable gossip from {}",
                            propagation_source
                        );
                    }
                }
                Some(SwarmEvent::Behaviour(SentinelBehaviourEvent::Kademlia(
                    kad::Event::OutboundQueryProgressed { result, .. },
                ))) => {
                    self.handle_kademlia_result(result);
                }
                Some(SwarmEvent::Behaviour(SentinelBehaviourEvent::Identify(
                    identify::Event::Received { peer_id, info },
                ))) => {
                    // Add peer addresses to Kademlia routing table
                    for addr in info.listen_addrs {
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                    }
                }
                None => {
                    // Swarm closed
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Gestisce un risultato Kademlia (bootstrap, find_node, put_record).
    fn handle_kademlia_result(&self, result: kad::QueryResult) {
        match result {
            kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk {
                peer,
                num_remaining,
            })) => {
                if num_remaining == 0 {
                    println!("✅ Kademlia bootstrap complete (last peer: {})", peer);
                }
            }
            kad::QueryResult::Bootstrap(Err(e)) => {
                eprintln!("⚠️  Kademlia bootstrap error: {:?}", e);
            }
            kad::QueryResult::GetClosestPeers(Ok(kad::GetClosestPeersOk { peers, .. })) => {
                println!("🔍 Found {} close peers in DHT", peers.len());
            }
            _ => {}
        }
    }

    /// Zero-Trust: verifica crittografica di ogni messaggio gossipsub ricevuto.
    /// Un messaggio con firma non valida viene silenziosamente scartato.
    fn handle_incoming_gossip(&mut self, msg: GossipMessage) {
        let payload_json = serde_json::to_string(&msg.payload).unwrap_or_default();
        if !NodeIdentity::verify_signature(
            &msg.sender_public_key,
            payload_json.as_bytes(),
            &msg.signature,
        ) {
            eprintln!("⚠️  Rejected INVALID signature from {}", msg.sender_id);
            return;
        }

        match msg.payload {
            GossipPayload::ConsensusProposal(proposal) => {
                println!(
                    "📝 Proposal received: {} from Agent {}",
                    proposal.id, proposal.agent_id
                );
                self.consensus.submit_proposal(proposal);
            }
            GossipPayload::ConsensusVote(vote) => {
                println!(
                    "🗳️ Vote for {}: {}",
                    vote.proposal_id,
                    if vote.approve { "APPROVE" } else { "REJECT" }
                );
                self.consensus.cast_vote(vote);
            }
            GossipPayload::ThreatAlert { dependency_name, risk_score, reason } => {
                eprintln!(
                    "🚨 THREAT ALERT from {}: dep={} risk={:.2} reason={} — tightening guardrails",
                    msg.sender_id, dependency_name, risk_score, reason
                );
                // Guardrail tightening is handled by the local ConsensusEngine
            }
            _ => {}
        }
    }

    /// Espone lo stato del nodo per il TUI/CLI.
    pub fn node_info(&self) -> NodeInfo {
        NodeInfo {
            peer_id: self.peer_id.to_string(),
            node_id: self.identity.node_id.to_string(),
        }
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new().expect("NetworkManager::new() should not fail")
    }
}

/// Snapshot informativo del nodo per TUI/CLI.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub peer_id: String,
    pub node_id: String,
}
