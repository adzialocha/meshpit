use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::task;
use tracing::{error, info};

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
    inner: Arc<RwLock<NodeInner>>,
}

#[derive(Debug)]
pub struct NodeInner {}

impl Node {
    pub fn new(config: Config) -> Self {
        let inner = NodeInner {};

        Self {
            config,
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub async fn spawn(&mut self) -> Result<()> {
        let udp_server = UdpSocket::bind(format!(
            "{}:{}",
            self.config.udp_server_addr, self.config.udp_server_port
        ))
        .await
        .context("bind udp server")?;

        info!("udp server: {}", udp_server.local_addr()?);
        info!(
            "udp client: {}:{}",
            self.config.udp_client_addr, self.config.udp_client_port
        );

        let (from_udp_tx, from_udp_rx) = mpsc::channel::<Vec<u8>>(128);
        let (to_udp_tx, mut to_udp_rx) = mpsc::channel::<Vec<u8>>(128);
        {
            let client_addr: SocketAddr = format!(
                "{}:{}",
                self.config.udp_client_addr, self.config.udp_client_port
            )
            .parse()
            .expect("valid udp client addr and port");

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
        }

        Ok(())
    }
}
