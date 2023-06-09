use crate::prelude::*;
use libp2p::{swarm::{Swarm, SwarmBuilder, SwarmEvent}, identity::Keypair, PeerId, tcp, Transport, core::{transport::OrTransport, upgrade}, mplex::MplexConfig, noise::{NoiseConfig, self}};
use tokio::sync::{mpsc::*, oneshot::{Sender as OneshotSender, channel as oneshot_channel}};
use futures::{StreamExt, future};

const FILTER_SIZE: usize = 125000;

pub struct KamilataNode {
    swarm: Swarm<KamilataBehavior<FILTER_SIZE, DocumentIndex<FILTER_SIZE>>>,
}

impl KamilataNode {
    pub async fn init(index: DocumentIndex<FILTER_SIZE>) -> KamilataNode {
        let local_key = Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());

        let behaviour = KamilataBehavior::new_with_store(peer_id, index);
        
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::new());

        let transport = tcp_transport
            .upgrade(upgrade::Version::V1Lazy)
            .authenticate(
                noise::Config::new(&local_key).expect("Signing libp2p-noise static DH keypair failed."),
            )
            .multiplex(MplexConfig::default())
            .boxed();
        
        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build();
        swarm.listen_on("/ip4/127.0.0.1/tcp/4002".parse().unwrap()).unwrap();

        KamilataNode {
            swarm,
        }
    }

    pub fn run(mut self) -> KamilataController {
        let (sender, mut receiver) = channel(1);
        tokio::spawn(async move {
            loop {
                let recv = Box::pin(receiver.recv());
                let value = futures::future::select(recv, self.swarm.select_next_some()).await;
                match value {
                    future::Either::Left((Some(command), _)) => match command {
                        ClientCommand::Search { queries, config, sender } => {
                            let controller = self.swarm.behaviour_mut().search_with_config(queries, config).await;
                            let _ = sender.send(controller);
                        }
                    },
                    future::Either::Left((None, _)) => break,
                    future::Either::Right((event, _)) => match event {
                        SwarmEvent::Behaviour(e) => println!("Produced behavior event {e:?}"),
                        SwarmEvent::NewListenAddr { listener_id, address } => println!("Listening on {address:?} (listener id: {listener_id:?})"),
                        _ => ()
                    },
                }
            }
        });
        KamilataController {
            sender,
        }
    }
}

enum ClientCommand {
    Search {
        queries: SearchQueries,
        config: SearchConfig,
        sender: OneshotSender<OngoingSearchController<DocumentResult>>,
    }
}

pub struct KamilataController {
    sender: Sender<ClientCommand>,
}

impl KamilataController {
    pub async fn search(&self, queries: SearchQueries) -> OngoingSearchController<DocumentResult> {
        let (sender, receiver) = oneshot_channel();
        let _ = self.sender.send(ClientCommand::Search {
            queries,
            config: SearchConfig::default(),
            sender,
        }).await;
        receiver.await.unwrap()
    }
}
