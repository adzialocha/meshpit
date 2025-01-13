use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use iroh_gossip::proto::Config as GossipConfig;
use p2panda_core::{Extension, Hash, PrivateKey, PublicKey};
use p2panda_discovery::mdns::LocalDiscovery;
use p2panda_net::{FromNetwork, Network, NetworkBuilder, SyncConfiguration, ToNetwork, TopicId};
use p2panda_store::MemoryStore;
use p2panda_stream::operation::{ingest_operation, IngestResult};
use p2panda_stream::{DecodeExt, IngestExt};
use p2panda_sync::log_sync::LogSyncProtocol;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tracing::{debug, error, warn};

use crate::operation::{
    create_operation, decode_gossip_message, encode_gossip_message, Extensions,
};
use crate::topic::{AuthorStore, LogId, Topic};

const RELAY_ENDPOINT: &str = "https://staging-euw1-1.relay.iroh.network";

const NETWORK_ID: &str = "meshpit";

const DEFAULT_TOPIC: &str = "peers-for-peers";

const MAX_MESSAGE_SIZE: usize = 1000 * 10; // 10kb max. UDP payload size

#[derive(Clone, Debug)]
pub struct Config {
    pub topic: Topic,
    pub udp_server_addr: SocketAddr,
    pub udp_client_addr: SocketAddr,
    pub bootstrap: Option<PublicKey>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            topic: Topic::from_str(DEFAULT_TOPIC).unwrap(),
            udp_server_addr: (Ipv4Addr::LOCALHOST, 0).into(),
            udp_client_addr: (Ipv4Addr::LOCALHOST, 49494).into(),
            bootstrap: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    network: Network<Topic>,
    udp_server: Arc<UdpSocket>,
    config: Config,
}

impl Node {
    pub async fn new(private_key: PrivateKey, config: Config) -> Result<Self> {
        let (to_udp_tx, mut to_udp_rx) = mpsc::channel::<Vec<u8>>(128);

        // Launch an p2p network.
        let network_id = Hash::new(NETWORK_ID.as_bytes());

        let mdns = LocalDiscovery::new().context("bind socket for mDNS discovery")?;

        let operation_store = MemoryStore::<LogId, Extensions>::new();
        let author_store = AuthorStore::new();

        let sync_protocol = LogSyncProtocol::new(author_store.clone(), operation_store.clone());
        let sync_config = SyncConfiguration::new(sync_protocol);

        let relay_url = RELAY_ENDPOINT.parse()?;

        let mut network_builder = NetworkBuilder::new(network_id.into())
            .discovery(mdns)
            .sync(sync_config)
            .gossip(GossipConfig {
                max_message_size: MAX_MESSAGE_SIZE,
                ..Default::default()
            })
            .relay(relay_url, false, 0);

        if let Some(bootstrap) = config.bootstrap {
            network_builder = network_builder.direct_address(bootstrap, vec![], None);
        }

        let network = network_builder.build().await.context("spawn p2p network")?;

        let topic = config.topic.clone();
        let (network_tx, network_rx, gossip_ready) = network.subscribe(topic).await?;

        task::spawn(async move {
            if gossip_ready.await.is_ok() {
                debug!("joined gossip overlay");
            }
        });

        let stream = ReceiverStream::new(network_rx);
        let stream = stream.filter_map(|event| match event {
            FromNetwork::GossipMessage { bytes, .. } => match decode_gossip_message(&bytes) {
                Ok(result) => Some(result),
                Err(err) => {
                    warn!("could not decode gossip message: {err}");
                    None
                }
            },
            FromNetwork::SyncMessage {
                header, payload, ..
            } => Some((header, payload)),
        });

        // Decode and ingest the p2panda operations.
        let mut stream = stream
            .decode()
            .filter_map(|result| match result {
                Ok(operation) => Some(operation),
                Err(err) => {
                    warn!("decode operation error: {err}");
                    None
                }
            })
            .ingest(operation_store.clone(), 128)
            .filter_map(|result| match result {
                Ok(operation) => Some(operation),
                Err(err) => {
                    warn!("ingest operation error: {err}");
                    None
                }
            });

        {
            let mut author_store = author_store.clone();

            task::spawn(async move {
                while let Some(operation) = stream.next().await {
                    let log_id: Option<LogId> = operation.header.extract();
                    let topic = Topic::new(log_id.expect("log id exists in header extensions"));
                    author_store
                        .add_author(topic, operation.header.public_key)
                        .await;

                    let body_len = operation.body.as_ref().map_or(0, |body| body.size());
                    debug!(
                        seq_num = operation.header.seq_num,
                        len = body_len,
                        hash = %operation.hash,
                        "received operation"
                    );

                    match operation.body {
                        Some(body) => {
                            if to_udp_tx.send(body.to_bytes()).await.is_err() {
                                break;
                            }
                        }
                        None => {
                            continue;
                        }
                    }
                }
            });
        }

        // Launch an UDP server which listens for incoming UDP packets of any data.
        let udp_server = UdpSocket::bind(config.udp_server_addr)
            .await
            .context("bind udp server")?;
        let udp_server = Arc::new(udp_server);

        {
            let mut operation_store = operation_store.clone();
            let mut author_store = author_store;
            let udp_server = udp_server.clone();
            let log_id = config.topic.id();

            task::spawn(async move {
                let mut buf = [0; MAX_MESSAGE_SIZE];

                loop {
                    tokio::select! {
                        message = udp_server.recv(&mut buf) => {
                            match message {
                                Ok(len) => {
                                    let prune = false;

                                    let (header, body) = create_operation(
                                        &mut operation_store,
                                        log_id,
                                        &private_key,
                                        Some(&buf[..len]),
                                        prune,
                                    )
                                    .await;

                                    let Ok(gossip_message_bytes) = encode_gossip_message(&header, body.as_ref()) else {
                                        error!("could not encode gossip message");
                                        break;
                                    };
                                    let header_bytes = header.to_bytes();

                                    let Ok(result) = ingest_operation(
                                        &mut operation_store,
                                        header,
                                        body,
                                        header_bytes,
                                        &log_id,
                                        prune,
                                    )
                                    .await else {
                                        error!("could not ingest p2panda operation");
                                        break;
                                    };

                                    match result {
                                        IngestResult::Complete(operation) => {
                                            author_store
                                                .add_author(
                                                    Topic::new(log_id),
                                                    operation.header.public_key
                                                )
                                                .await;

                                            debug!(
                                                seq_num = operation.header.seq_num,
                                                len = len,
                                                hash = %operation.hash,
                                                "publish operation"
                                            );
                                        },
                                        _ => unreachable!(),
                                    }

                                    if network_tx.send(ToNetwork::Message {
                                        bytes: gossip_message_bytes,
                                    }).await.is_err() {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    error!("udp server error on recv: {err}");
                                }
                            }
                        }
                        Some(message) = to_udp_rx.recv() => {
                            if let Err(err) = udp_server.send_to(&message, config.udp_client_addr).await {
                                error!("udp error on send to client: {err}");
                            }
                        }
                    }
                }
            });
        }

        Ok(Self {
            network,
            udp_server,
            config,
        })
    }

    pub async fn addrs(&self) -> Result<Vec<SocketAddr>> {
        let node_addrs = self
            .network
            .direct_addresses()
            .await
            .ok_or(anyhow!("could not determine local node addresses"))?;
        Ok(node_addrs)
    }

    pub async fn udp_server_addr(&self) -> Result<SocketAddr> {
        let server_addr = self.udp_server.local_addr()?;
        Ok(server_addr)
    }

    pub fn udp_client_addr(&self) -> SocketAddr {
        self.config.udp_client_addr
    }

    pub async fn shutdown(self) -> Result<()> {
        self.network.shutdown().await?;
        Ok(())
    }
}
