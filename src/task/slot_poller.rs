use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::future::try_join_all;
use tokio::{select, time::MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::{
    metrics::Metrics,
    solana_rpc::{CommitmentConfig, SolanaRPCClient},
    BoxError,
};

#[derive(Debug)]
pub struct SlotError {
    id: String,
    source: BoxError,
}

impl SlotError {
    pub fn new<I: Into<String>, E: Into<BoxError>>(id: I, source: E) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
        }
    }
}

impl std::fmt::Display for SlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to get slot for '{}': {:?}", self.id, self.source)
    }
}

impl std::error::Error for SlotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

pub async fn slot_poller(
    cancel: CancellationToken,
    poll_interval: Duration,
    metrics: Arc<Metrics>,
    upstream_client: Option<SolanaRPCClient>,
    downstream_clients: HashMap<String, SolanaRPCClient>,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(), BoxError> {
    let mut interval = tokio::time::interval(poll_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                if let Err(err) = update_slot_metrics(
                    &metrics,
                    &upstream_client,
                    &downstream_clients,
                    commitment_config
                ).await {
                    error!(?err, "failed to update slot metrics");
                }
            }
        }
    }

    Ok(())
}

async fn get_node_slot(
    id: String,
    client: &SolanaRPCClient,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(String, Option<u64>), BoxError> {
    let slot = client.get_slot(commitment_config).await;

    match slot {
        Ok(slot) => Ok((id, Some(slot))),
        Err(err)
            if err.is_redirect()
                || err.is_status()
                || err.is_timeout()
                || err.is_request()
                || err.is_connect()
                || err.is_body()
                || err.is_decode() =>
        {
            debug!(?err, "failed to get slot for '{}', ignoring", id);

            Ok((id, None))
        }
        Err(e) => Err(SlotError::new(id, e).into()),
    }
}

async fn get_node_slots(
    upstream: &Option<SolanaRPCClient>,
    nodes: &HashMap<String, SolanaRPCClient>,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(Option<u64>, HashMap<String, Option<u64>>), BoxError> {
    let mut tasks = vec![];

    if let Some(upstream) = upstream {
        tasks.push(get_node_slot(
            "upstream".to_string(),
            upstream,
            commitment_config,
        ));
    }

    for (id, client) in nodes {
        tasks.push(get_node_slot(id.clone(), client, commitment_config))
    }

    let mut results: HashMap<String, Option<u64>> =
        try_join_all(tasks).await?.into_iter().collect();
    let upstream_slot = results.remove("upstream").unwrap_or(None);

    Ok((upstream_slot, results))
}

async fn update_slot_metrics(
    metrics: &Metrics,
    upstream_client: &Option<SolanaRPCClient>,
    downstream_clients: &HashMap<String, SolanaRPCClient>,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(), BoxError> {
    let (upstream_slot, downstream_slots) =
        get_node_slots(upstream_client, downstream_clients, commitment_config).await?;

    if metrics.upstream_slot.is_some() {
        debug!(?upstream_slot, ?downstream_slots, "slots");
    } else {
        debug!(?downstream_slots, "slots");
    }

    if let (Some(slot), Some(gauge)) = (upstream_slot, metrics.upstream_slot.as_ref()) {
        gauge.set(slot);
    }

    for (id, slot) in downstream_slots {
        if let Some(slot) = slot {
            metrics.downstream_slots.with_label_values(&[&id]).set(slot);
        } else {
            let _ = metrics.downstream_slots.remove_label_values(&[&id]);
        }
    }

    Ok(())
}
