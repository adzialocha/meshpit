use anyhow::Result;
use clap::Parser;
use meshpit::{setup_tracing, Config, Node};
use p2panda_core::PrivateKey;
use serde::Serialize;
use tracing::info;

#[derive(Debug, Serialize, Parser)]
#[command(
    name = "meshpit",
    long_about = None,
    version
)]
struct Args {
    /// Set log verbosity. Use this for learning more about how your node behaves or for debugging.
    ///
    /// Possible log levels are: ERROR, WARN, INFO, DEBUG, TRACE. They are scoped to "meshpit" by
    /// default.
    ///
    /// If you want to adjust the scope for deeper inspection use a filter value, for example
    /// "=TRACE" for logging _everything_ or "meshpit=INFO,p2panda_net=DEBUG" etc.
    #[arg(short = 'l', long, value_name = "LEVEL")]
    #[serde(skip_serializing_if = "Option::is_none")]
    log_level: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    setup_tracing(&args.log_level.unwrap_or_default());

    let config = Config::default();
    let private_key = PrivateKey::new();
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
