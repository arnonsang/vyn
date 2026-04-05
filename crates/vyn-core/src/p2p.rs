use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use libp2p::futures::StreamExt;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, gossipsub, identify, kad, mdns, noise, ping, tcp, yamux,
};

pub const SYNC_TOPIC: &str = "vyn-sync";

#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("failed to build gossipsub config")]
    GossipsubConfig,
    #[error("failed to initialize gossipsub behaviour")]
    GossipsubInit,
    #[error("failed to initialize mDNS behaviour")]
    MdnsInit,
    #[error("failed to build libp2p swarm: {0}")]
    SwarmBuild(String),
    #[error("failed to listen on address: {0}")]
    Listen(String),
    #[error("failed to subscribe to sync topic")]
    Subscribe,
    #[error("timed out waiting for peer connection")]
    Timeout,
}

#[derive(NetworkBehaviour)]
pub struct VynP2PBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub ping: ping::Behaviour,
}

pub type VynSwarm = Swarm<VynP2PBehaviour>;

pub fn build_swarm() -> Result<(PeerId, VynSwarm), P2PError> {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = local_key.public().to_peer_id();

    let swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|err| P2PError::SwarmBuild(err.to_string()))?
        .with_behaviour(|key| {
            let peer_id = key.public().to_peer_id();
            let mut hasher = DefaultHasher::new();
            SYNC_TOPIC.hash(&mut hasher);
            let message_id_fn = move |message: &gossipsub::Message| {
                let mut h = DefaultHasher::new();
                message.data.hash(&mut h);
                gossipsub::MessageId::from(h.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .map_err(|_| P2PError::GossipsubConfig)?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .map_err(|_| P2PError::GossipsubInit)?;

            let mut kademlia = kad::Behaviour::new(peer_id, MemoryStore::new(peer_id));
            kademlia.set_mode(Some(kad::Mode::Server));

            let identify = identify::Behaviour::new(identify::Config::new(
                "/vyn/1.0.0".to_string(),
                key.public(),
            ));

            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
                .map_err(|_| P2PError::MdnsInit)?;

            let ping = ping::Behaviour::new(ping::Config::new());

            Ok(VynP2PBehaviour {
                gossipsub,
                kademlia,
                identify,
                mdns,
                ping,
            })
        })
        .map_err(|err| P2PError::SwarmBuild(err.to_string()))?
        .build();

    Ok((local_peer_id, swarm))
}

pub fn subscribe_sync_topic(swarm: &mut VynSwarm) -> Result<gossipsub::IdentTopic, P2PError> {
    let topic = gossipsub::IdentTopic::new(SYNC_TOPIC);
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&topic)
        .map_err(|_| P2PError::Subscribe)?;
    Ok(topic)
}

pub fn listen_localhost(swarm: &mut VynSwarm) -> Result<Multiaddr, P2PError> {
    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0"
        .parse()
        .map_err(|err: libp2p::multiaddr::Error| P2PError::Listen(err.to_string()))?;
    swarm
        .listen_on(addr.clone())
        .map_err(|err| P2PError::Listen(err.to_string()))?;
    Ok(addr)
}

pub fn handle_discovery_event(swarm: &mut VynSwarm, event: &VynP2PBehaviourEvent) {
    if let VynP2PBehaviourEvent::Mdns(mdns::Event::Discovered(list)) = event {
        for (peer_id, addr) in list {
            swarm.behaviour_mut().gossipsub.add_explicit_peer(peer_id);
            swarm
                .behaviour_mut()
                .kademlia
                .add_address(peer_id, addr.clone());
        }
    }
}

pub async fn wait_for_connection(
    swarm: &mut VynSwarm,
    timeout: Duration,
) -> Result<PeerId, P2PError> {
    let fut = async {
        loop {
            if let SwarmEvent::ConnectionEstablished { peer_id, .. } =
                swarm.select_next_some().await
            {
                return Ok(peer_id);
            }
        }
    };

    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| P2PError::Timeout)?
}

#[cfg(test)]
mod tests {
    use super::{build_swarm, listen_localhost, subscribe_sync_topic, wait_for_connection};
    use libp2p::futures::StreamExt;
    use libp2p::swarm::SwarmEvent;
    use std::time::Duration;

    #[tokio::test]
    async fn libp2p_local_discovery() {
        let (_peer_a, mut swarm_a) = build_swarm().expect("swarm A should build");
        let (_peer_b, mut swarm_b) = build_swarm().expect("swarm B should build");

        subscribe_sync_topic(&mut swarm_a).expect("swarm A topic subscription should work");
        subscribe_sync_topic(&mut swarm_b).expect("swarm B topic subscription should work");

        listen_localhost(&mut swarm_a).expect("swarm A should listen");

        let addr = loop {
            if let SwarmEvent::NewListenAddr { address, .. } = swarm_a.select_next_some().await {
                break address;
            }
        };

        swarm_b
            .dial(addr)
            .expect("swarm B should dial swarm A listen address");

        let connected_a = wait_for_connection(&mut swarm_a, Duration::from_secs(10));
        let connected_b = wait_for_connection(&mut swarm_b, Duration::from_secs(10));
        let (a_peer, b_peer) = tokio::join!(connected_a, connected_b);

        assert!(a_peer.is_ok());
        assert!(b_peer.is_ok());
    }
}
