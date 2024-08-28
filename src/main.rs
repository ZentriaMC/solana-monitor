use std::{collections::HashMap, net::SocketAddr, ops::Not, sync::Arc};

use clap::Parser;
use duration_string::DurationString;
use http::Uri;
use tokio::{signal::ctrl_c, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, level_filters::LevelFilter, trace};
use tracing_subscriber::EnvFilter;

mod id_url;
mod metrics;
mod prom_u64;
mod solana_rpc;
mod task;

use crate::id_url::IdUrlPair;
use crate::metrics::Metrics;
use crate::solana_rpc::{CommitmentConfig, SolanaRPCClient};

pub static HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Exporter listen address
    #[clap(
        long,
        env = "SOLANA_MONITOR_LISTEN_ADDRESS",
        default_value = "127.0.0.1:2112"
    )]
    pub listen_addr: SocketAddr,

    /// Solana network ID. Exposed in Prometheus metrics as a label
    #[clap(
        long,
        env = "SOLANA_MONITOR_NETWORK_ID",
        default_value = "mainnet-beta"
    )]
    pub network_id: String,

    /// Upstream RPC URL to monitor
    #[clap(
        long,
        env = "SOLANA_MONITOR_UPSTREAM_RPC",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    pub upstream_rpc: Uri,

    /// Whether to poll upstream slot
    #[clap(long, env = "SOLANA_MONITOR_DISABLE_UPSTREAM", default_value = "false")]
    pub disable_upstream: bool,

    /// Downstream RPCs to monitor, format: `localhost=http://127.0.0.1:8899[,...]`
    #[clap(long, env = "SOLANA_MONITOR_DOWNSTREAM_RPC", value_delimiter = ',')]
    pub downstream_rpc: Vec<IdUrlPair>,

    /// Poll interval
    #[clap(long, env = "SOLANA_MONITOR_POLL_INTERVAL", default_value = "2500ms")]
    pub poll_interval: DurationString,
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
    trace!(?args, "arguments");

    // Basic validation
    if args.disable_upstream && args.downstream_rpc.is_empty() {
        error!("`disable-upstream` is set to true, but no downstream RPCs are provided");
        std::process::exit(1);
    }

    let metrics = Arc::new(Metrics::new(&args.network_id, !args.disable_upstream));

    // Prepare clients
    // Request timeout is half of the poll interval, I think it's a good starting point
    let request_timeout = *args.poll_interval / 2;

    let upstream_client = args
        .disable_upstream
        .not()
        .then(|| SolanaRPCClient::new(args.upstream_rpc.to_string(), request_timeout));

    let mut downstream_clients: HashMap<String, SolanaRPCClient> =
        HashMap::with_capacity(args.downstream_rpc.len());
    for IdUrlPair((id, uri)) in args.downstream_rpc.iter() {
        let client = SolanaRPCClient::new(uri.to_string(), request_timeout);
        downstream_clients.insert(id.clone(), client);
    }

    let cancel = CancellationToken::new();
    let mut rs: JoinSet<_> = JoinSet::new();

    // Spawn slot poller
    let commitment_config = CommitmentConfig::finalized();

    rs.spawn(crate::task::slot_poller(
        cancel.clone(),
        *args.poll_interval,
        Arc::clone(&metrics),
        upstream_client,
        downstream_clients,
        commitment_config,
    ));

    rs.spawn(crate::task::metrics_server(
        cancel.clone(),
        metrics,
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
