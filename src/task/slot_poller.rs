use std::{collections::HashMap, time::Duration};

use anyhow::anyhow;
use futures::future::try_join_all;
use jsonrpsee::{core::ClientError, http_client::HttpClient};
use tokio::{select, time::MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::{
    solana_rpc::{CommitmentConfig, SolanaRPCClient},
    BoxError,
};

pub async fn slot_poller(
    cancel: CancellationToken,
    poll_interval: Duration,
    upstream_client: HttpClient,
    downstream_clients: HashMap<String, HttpClient>,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(), BoxError> {
    let mut interval = tokio::time::interval(poll_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                if let Err(err) = update_slot_metrics(
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
    client: &HttpClient,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(String, Option<u64>), BoxError> {
    let slot = client.get_slot(commitment_config).await;

    match slot {
        Ok(slot) => Ok((id, Some(slot))),

        err @ Err(ClientError::Transport(_))
        | err @ Err(ClientError::RequestTimeout)
        | err @ Err(ClientError::ParseError(_))
        | err @ Err(ClientError::InvalidRequestId(_)) => {
            debug!(?err, "failed to get slot for '{}', ignoring", id);

            Ok((id, None))
        }

        // TODO: anyhow is overkill for this
        Err(e) => Err(anyhow!(e)
            .context(format!("failed to get slot for '{}'", id))
            .into()),
    }
}

async fn get_node_slots(
    upstream: &HttpClient,
    nodes: &HashMap<String, HttpClient>,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(Option<u64>, HashMap<String, Option<u64>>), BoxError> {
    let mut tasks = vec![];

    tasks.push(get_node_slot(
        "upstream".to_string(),
        upstream,
        commitment_config,
    ));

    for (id, client) in nodes {
        tasks.push(get_node_slot(id.clone(), client, commitment_config))
    }

    let mut results: HashMap<String, Option<u64>> =
        try_join_all(tasks).await?.into_iter().collect();
    let upstream_slot = results.remove("upstream").unwrap();

    Ok((upstream_slot, results))
}

async fn update_slot_metrics(
    upstream_client: &HttpClient,
    downstream_clients: &HashMap<String, HttpClient>,
    commitment_config: Option<CommitmentConfig>,
) -> Result<(), BoxError> {
    let (upstream_slot, downstream_slots) =
        get_node_slots(upstream_client, downstream_clients, commitment_config).await?;

    debug!(?upstream_slot, ?downstream_slots, "slots");

    crate::metrics::UPSTREAM_SLOT.set(upstream_slot.unwrap_or_default());
    for (id, slot) in downstream_slots {
        if let Some(slot) = slot {
            crate::metrics::DOWNSTREAM_SLOTS
                .with_label_values(&[&id])
                .set(slot);
        } else {
            let _ = crate::metrics::DOWNSTREAM_SLOTS.remove_label_values(&[&id]);
        }
    }

    Ok(())
}
