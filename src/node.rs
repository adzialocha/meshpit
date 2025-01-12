use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{Context, Result};
use p2panda_core::{Hash, PrivateKey};
use p2panda_discovery::mdns::LocalDiscovery;
use p2panda_net::{NetworkBuilder, SyncConfiguration};
use p2panda_store::MemoryStore;
use p2panda_sync::log_sync::LogSyncProtocol;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task;
use tracing::{error, info};

use crate::topic::{AuthorStore, LogId};

const RELAY_ENDPOINT: &str = "https://staging-euw1-1.relay.iroh.network";

const NETWORK_ID: &str = "meshpit";

#[derive(Clone, Debug)]
pub struct Config {
    udp_server_addr: Ipv4Addr,
    udp_server_port: u16,
    udp_client_addr: Ipv4Addr,
    udp_client_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            udp_server_addr: Ipv4Addr::LOCALHOST,
            udp_server_port: 0,
            udp_client_addr: Ipv4Addr::LOCALHOST,
            udp_client_port: 49494,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    config: Config,
    private_key: PrivateKey,
}

impl Node {
    pub fn new(private_key: PrivateKey, config: Config) -> Self {
        Self {
            config,
            private_key,
        }
    }

    pub async fn spawn(&mut self) -> Result<()> {
        let udp_server = UdpSocket::bind(format!(
            "{}:{}",
            self.config.udp_server_addr, self.config.udp_server_port
        ))
        .await
        .context("bind udp server")?;

        let server_addr = udp_server.local_addr()?;

        let client_addr: SocketAddr = format!(
            "{}:{}",
            self.config.udp_client_addr, self.config.udp_client_port
        )
        .parse()
        .context("parsing client address and port")?;

        info!("udp server: {}", server_addr);
        info!("udp client: {}", client_addr);

        let (from_udp_tx, from_udp_rx) = mpsc::channel::<Vec<u8>>(128);
        let (to_udp_tx, mut to_udp_rx) = mpsc::channel::<Vec<u8>>(128);

        task::spawn(async move {
            let mut buf = [0; 1000 * 10]; // 10kb max. UDP payload size

            loop {
                tokio::select! {
                    message = udp_server.recv(&mut buf) => {
                        match message {
                            Ok(len) => {
                                if let Err(_) = from_udp_tx.send(Vec::from(&buf[..len])).await {
                                    break;
                                }
                            }
                            Err(err) => {
                                error!("udp server error on recv: {err}");
                            }
                        }
                    }
                    Some(message) = to_udp_rx.recv() => {
                        if let Err(err) = udp_server.send_to(&message, client_addr).await {
                            error!("udp error on send to client: {err}");
                        }
                    }
                }
            }
        });

        let network_id = Hash::new(NETWORK_ID.as_bytes());

        let mdns = LocalDiscovery::new().context("bind socket for mDNS discovery")?;

        let operation_store = MemoryStore::<LogId, ()>::new();
        let author_store = AuthorStore::new();

        let sync_protocol = LogSyncProtocol::new(author_store, operation_store);
        let sync_config = SyncConfiguration::new(sync_protocol);

        let relay_url = RELAY_ENDPOINT.parse()?;

        let network = NetworkBuilder::new(network_id.into())
            .discovery(mdns)
            .sync(sync_config)
            .relay(relay_url, false, 0)
            .build();

        task::spawn(async move {});

        Ok(())
    }
}
