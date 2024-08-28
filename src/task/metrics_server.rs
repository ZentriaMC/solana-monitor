use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::pin;
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::metrics::Metrics;
use crate::BoxError;

pub async fn metrics_server(
    cancel: CancellationToken,
    metrics: Arc<Metrics>,
    listen_addr: SocketAddr,
) -> Result<(), BoxError> {
    let listener = TcpListener::bind(&listen_addr).await?;
    info!("serving metrics on http://{}", listen_addr);

    loop {
        select! {
            _ = cancel.cancelled() => {
                break;
            }
            Ok((tcp, _)) = listener.accept() => {
                let io = TokioIo::new(tcp);
                let cancel = cancel.clone();
                let metrics = metrics.clone();

                tokio::task::spawn(async move {
                    pin! {
                        let conn = http1::Builder::new()
                            .timer(TokioTimer::new())
                            .serve_connection(io, service_fn(move |req| {
                                let metrics = metrics.clone();
                                crate::metrics::serve_metrics(req, metrics)
                            }));
                    }

                    select! {
                        _ = cancel.cancelled() => {
                            conn.as_mut().graceful_shutdown();
                        },
                        res = conn.as_mut() => {
                            if let Err(err) = res {
                                error!(?err, "failed to serve connection");
                            }
                        }
                    }
                });
            }
        }
    }

    Ok(())
}
