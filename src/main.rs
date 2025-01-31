use std::{net::SocketAddr, str::FromStr};

use anyhow::Result;
use clap::Parser;
use meshpit::{setup_tracing, Config, Node, Topic};
use p2panda_core::{PrivateKey, PublicKey};
use tracing::info;

#[derive(Debug, Parser)]
#[command(
    name = "meshpit",
    long_about = None,
    version
)]
struct Args {
    /// Define a short text-string which will be automatically hashed and used as a "topic".
    ///
    /// If peers are configured to the same topic, they will find each other automatically, connect
    /// and sync data.
    #[arg(short = 't', long, value_name = "STRING")]
    topic: Option<String>,

    /// Mention the public key of another peer to use it as a "bootstrap node" for discovery over
    /// the internet.
    ///
    /// If no value is given here, meshpit can only find other peers in your local area network.
    #[arg(short = 'b', long, value_name = "PUBLIC_KEY")]
    bootstrap: Option<PublicKey>,

    /// UDP server address and port. Send your data to this address, it will automatically be
    /// forwarded to all peers in the network who are subscribed to the same topic.
    ///
    /// meshpit will use localhost and a random port by default. Set it to 0.0.0.0 (bind to all
    /// network interfaces) if you want the UDP server to be accessible for other devices on the
    /// network.
    #[arg(short = 's', long, value_name = "ADDR:PORT")]
    udp_server: Option<SocketAddr>,

    /// UDP client address and port (default is 49494). meshpit will automatically forward all
    /// received data from other peers to this address.
    #[arg(short = 'c', long, value_name = "ADDR:PORT")]
    udp_client: Option<SocketAddr>,

    /// Disable sync for this node.
    ///
    /// Nodes without sync will not "catch up" on past data and only receive new messages via the
    /// broadcast gossip overlay.
    #[arg(short = 'n', long)]
    no_sync: bool,

    /// Set log verbosity. Use this for learning more about how your node behaves or for debugging.
    ///
    /// Possible log levels are: ERROR, WARN, INFO, DEBUG, TRACE. They are scoped to "meshpit" by
    /// default.
    ///
    /// If you want to adjust the scope for deeper inspection use a filter value, for example
    /// "=TRACE" for logging _everything_ or "meshpit=INFO,p2panda_net=DEBUG" etc.
    #[arg(short = 'l', long, value_name = "LEVEL")]
    log_level: Option<String>,
}

impl TryFrom<Args> for Config {
    type Error = anyhow::Error;

    fn try_from(args: Args) -> std::result::Result<Self, Self::Error> {
        let mut config = Config::default();

        config.bootstrap = args.bootstrap;
        config.no_sync = args.no_sync;

        if let Some(topic) = &args.topic {
            config.topic = Topic::from_str(topic)?;
        }

        if let Some(addr) = &args.udp_server {
            config.udp_server_addr = *addr;
        }

        if let Some(addr) = &args.udp_client {
            config.udp_client_addr = *addr;
        }

        Ok(config)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    setup_tracing(args.log_level.clone().unwrap_or_default());

    let config: Config = args.try_into()?;
    let private_key = PrivateKey::new();

    info!(" █▄ ▄█ ██▀ ▄▀▀ █▄█ █▀▄ █ ▀█▀");
    info!(" █ ▀ █ █▄▄ ▄██ █ █ █▀  █  █ ");
    info!("");

    info!("topic id: {}", config.topic);
    info!("public key: {}", private_key.public_key());

    let node = Node::new(private_key, config).await?;

    info!("p2p node:");
    for addr in node.addrs().await? {
        info!("- {}", addr);
    }
    info!("udp server: {}", node.udp_server_addr().await?);
    info!("udp client: {}", node.udp_client_addr());

    tokio::signal::ctrl_c().await?;

    node.shutdown().await?;

    Ok(())
}
