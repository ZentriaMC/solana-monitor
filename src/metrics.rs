use std::sync::Arc;

use http::{header::CONTENT_TYPE, StatusCode};
use http_body_util::Full;
use hyper::{body::Bytes, Request, Response};
use prometheus::{Encoder, Opts, Registry, TextEncoder};

use crate::{
    prom_u64::{GaugeU64, GaugeVecU64},
    BoxError,
};

fn create_upstream_slot_gauge(registry: &Registry, network_id: &str) -> GaugeU64 {
    let gauge = GaugeU64::with_opts(
        Opts::new(
            "solana_upstream_slot",
            "The latest slot from the upstream RPC",
        )
        .const_label("network", network_id),
    )
    .unwrap();
    registry.register(Box::new(gauge.clone())).unwrap();

    gauge
}

fn create_downstream_slot_gauge(registry: &Registry, network_id: &str) -> GaugeVecU64 {
    let gauge = GaugeVecU64::new(
        Opts::new(
            "solana_downstream_slots",
            "The latest slot from the upstream RPC",
        )
        .const_label("network", network_id),
        &["node_id"],
    )
    .unwrap();
    registry.register(Box::new(gauge.clone())).unwrap();

    gauge
}

pub struct Metrics {
    // pub network_id: String,
    pub registry: Registry,
    pub upstream_slot: Option<GaugeU64>,
    pub downstream_slots: GaugeVecU64,
}

impl Metrics {
    pub fn new(network_id: &str, track_upstream: bool) -> Self {
        let registry = Registry::new();

        let upstream_slot =
            track_upstream.then(|| create_upstream_slot_gauge(&registry, network_id));
        let downstream_slots = create_downstream_slot_gauge(&registry, network_id);

        Self {
            // network_id: network_id.to_string(),
            registry,
            upstream_slot,
            downstream_slots,
        }
    }

    pub fn to_response(&self) -> Response<Full<Bytes>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, encoder.format_type())
            .body(Full::new(Bytes::from(buffer)))
            .unwrap()
    }
}

pub async fn serve_metrics(
    _: Request<hyper::body::Incoming>,
    metrics: Arc<Metrics>,
) -> Result<Response<Full<Bytes>>, BoxError> {
    Ok(metrics.to_response())
}
