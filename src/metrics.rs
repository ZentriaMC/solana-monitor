use http::{header::CONTENT_TYPE, StatusCode};
use http_body_util::Full;
use hyper::{body::Bytes, Request, Response};
use lazy_static::lazy_static;
use prometheus::{
    labels, opts, register_gauge, register_gauge_vec, Encoder, Gauge, GaugeVec, TextEncoder,
};

use crate::BoxError;

lazy_static! {
    pub static ref UPSTREAM_SLOT: Gauge = register_gauge!(opts!(
        "solana_upstream_slot",
        "The latest slot from the upstream RPC",
        labels! {"network" => "mainnet-beta"}
    ))
    .unwrap();
    pub static ref DOWNSTREAM_SLOTS: GaugeVec = register_gauge_vec!(
        opts!(
            "solana_downstream_slots",
            "The latest slot from the downstream RPC",
            labels! {"network" => "mainnet-beta"},
        ),
        &["node_id"]
    )
    .unwrap();
}

pub async fn serve_metrics(
    _: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, BoxError> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, encoder.format_type())
        .body(Full::new(Bytes::from(buffer)))?;

    Ok(response)
}
