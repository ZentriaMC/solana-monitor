use std::{collections::HashMap, net::SocketAddr, str::FromStr, time::Duration};

use clap::Parser;
use http::Uri;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use tokio::{signal::ctrl_c, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

mod metrics;
mod solana_rpc;
mod task;

use crate::solana_rpc::CommitmentConfig;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(
        long,
        env = "SOLANA_MONITOR_LISTEN_ADDRESS",
        default_value = "127.0.0.1:2112"
    )]
    pub listen_addr: SocketAddr,

    #[clap(
        long,
        env = "SOLANA_MONITOR_UPSTREAM_RPC",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    pub upstream_rpc: Uri,

    #[clap(
        long,
        env = "SOLANA_MONITOR_DOWNSTREAM_RPC",
        default_value = "localhost=http://127.0.0.1:8899"
    )]
    pub downstream_rpc: Vec<IdUrlPair>,

    #[clap(long, env = "SOLANA_MONITOR_POLL_INTERVAL", value_parser = parse_duration::parse, default_value = "2500ms")]
    pub poll_interval: Duration,
}

#[derive(Clone)]
pub struct IdUrlPair(pub (String, Uri));

impl FromStr for IdUrlPair {
    type Err = BoxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, '=');

        let id = split.next().ok_or("missing id")?.to_string();
        let uri: Uri = split.next().ok_or("missing uri")?.parse()?;

        Ok(Self((id, uri)))
    }
}

impl std::fmt::Debug for IdUrlPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.0 .0, self.0 .1)
    }
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args = Cli::parse();

    // Prepare clients
    let upstream_client = HttpClientBuilder::new().build(args.upstream_rpc.to_string())?;
    let mut downstream_clients: HashMap<String, HttpClient> =
        HashMap::with_capacity(args.downstream_rpc.len());
    for IdUrlPair((id, uri)) in args.downstream_rpc {
        let client = HttpClientBuilder::new().build(&uri.to_string())?;
        downstream_clients.insert(id, client);
    }

    let cancel = CancellationToken::new();
    let mut rs: JoinSet<_> = JoinSet::new();

    // Spawn slot poller
    let commitment_config = CommitmentConfig::finalized();

    rs.spawn(crate::task::slot_poller(
        cancel.clone(),
        args.poll_interval,
        upstream_client,
        downstream_clients,
        commitment_config,
    ));

    rs.spawn(crate::task::metrics_server(
        cancel.clone(),
        args.listen_addr,
    ));

    ctrl_c().await?;
    cancel.cancel();
    info!("got signal, exiting");

    while let Some(res) = rs.join_next().await {
        if let Err(err) = res {
            error!(?err, "task failed");
        }
    }

    Ok(())
}
