//! Network Manager - P2P Infrastructure
//!
//! Gestisce lo swarm di libp2p, il routing Kademlia e il gossipsub
//! per la distribuzione dell'intelligenza.

use crate::federation::{NodeIdentity, gossip::{GossipMessage, GossipPayload}, consensus::{ConsensusEngine, Proposal, Vote}};
use libp2p::{
    gossipsub, kad, identify, noise, tcp, yamux,
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId,
};
use std::error::Error;
use std::time::Duration;
use futures::StreamExt;

#[derive(NetworkBehaviour)]
pub struct SentinelBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: identify::Behaviour,
}

pub struct NetworkManager {
    pub peer_id: PeerId,
    pub identity: NodeIdentity,
    pub consensus: ConsensusEngine,
}

impl NetworkManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let identity = NodeIdentity::generate();
        let local_peer_id = PeerId::random();
        
        Ok(Self {
            peer_id: local_peer_id,
            identity,
            consensus: ConsensusEngine::new(),
        })
    }

    /// Avvia l'ascolto e gestisce gli eventi di rete (Event Loop)
    pub async fn run_node(&mut self) -> Result<(), Box<dyn Error>> {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let gossipsub_config = gossipsub::Config::default();
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                ).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

                let store = kad::store::MemoryStore::new(key.public().to_peer_id());
                let kademlia = kad::Behaviour::new(key.public().to_peer_id(), store);

                let identify = identify::Behaviour::new(identify::Config::new(
                    "sentinel/1.0.0".into(),
                    key.public(),
                ));

                Ok(SentinelBehaviour { gossipsub, kademlia, identify })
            })?
            .build();

        // Subscribe to consensus topic
        let topic = gossipsub::IdentTopic::new("sentinel-consensus");
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        while let Some(event) = swarm.next().await {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("📍 Sentinel Node listening on: {}", address);
                }
                SwarmEvent::Behaviour(SentinelBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) => {
                    if let Ok(gossip_msg) = serde_json::from_slice::<GossipMessage>(&message.data) {
                        self.handle_incoming_gossip(gossip_msg);
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!("🔗 Connection ESTABLISHED with: {}", peer_id);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_incoming_gossip(&mut self, msg: GossipMessage) {
        // Verifica crittografica Zero-Trust
        let payload_json = serde_json::to_string(&msg.payload).unwrap_or_default();
        if !NodeIdentity::verify_signature(&msg.sender_public_key, payload_json.as_bytes(), &msg.signature) {
            println!("⚠️ Rejected INVALID signature from {}", msg.sender_id);
            return;
        }

        match msg.payload {
            GossipPayload::ConsensusProposal(proposal) => {
                println!("📝 New PROPOSAL received: {} from Agent {}", proposal.id, proposal.agent_id);
                self.consensus.submit_proposal(proposal);
            }
            GossipPayload::ConsensusVote(vote) => {
                println!("🗳️ New VOTE received for Proposal {}: {}", vote.proposal_id, if vote.approve { "APPROVE" } else { "REJECT" });
                self.consensus.cast_vote(vote);
            }
            _ => {}
        }
    }
}
