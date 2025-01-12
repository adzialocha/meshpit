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
    #[arg(short = 't', long, value_name = "STRING")]
    topic: Option<String>,

    #[arg(short = 'b', long, value_name = "PUBLIC_KEY")]
    bootstrap: Option<PublicKey>,

    #[arg(short = 's', long, value_name = "ADDR:PORT")]
    udp_server: Option<SocketAddr>,

    #[arg(short = 'c', long, value_name = "ADDR:PORT")]
    udp_client: Option<SocketAddr>,

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

        if let Some(bootstrap) = &args.bootstrap {
            config.bootstrap = Some(bootstrap.to_owned());
        }

        if let Some(topic) = &args.topic {
            config.topic = Topic::from_str(&topic)?;
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
