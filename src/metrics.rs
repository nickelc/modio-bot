use prometheus::core::{AtomicU64, GenericGauge};
use prometheus::{IntCounter, IntCounterVec, Opts, Registry};

use crate::Result;

#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub guilds: GenericGauge<AtomicU64>,
    pub notifications: IntCounter,
    pub commands: Commands,
}

#[derive(Clone)]
pub struct Commands {
    pub total: IntCounter,
    pub counts: IntCounterVec,
    pub errored: IntCounter,
}

impl Commands {
    pub fn inc(&self, cmd: &str) {
        self.total.inc();
        self.counts.with_label_values(&[cmd]).inc();
    }
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let guilds = GenericGauge::<AtomicU64>::new("guilds", "Current guilds")?;
        let notifications = IntCounter::new("notifications", "Notifications")?;
        let commands = Commands {
            total: IntCounter::with_opts(
                Opts::new("total", "Total executed commands").namespace("commands"),
            )?,
            counts: IntCounterVec::new(
                Opts::new("counts", "Executed commands").namespace("commands"),
                &["name"],
            )?,
            errored: IntCounter::with_opts(
                Opts::new("errored", "Errored commands").namespace("commands"),
            )?,
        };

        let registry = Registry::new_custom(Some(String::from("modbot")), None)?;

        registry.register(Box::new(guilds.clone()))?;
        registry.register(Box::new(notifications.clone()))?;
        registry.register(Box::new(commands.total.clone()))?;
        registry.register(Box::new(commands.counts.clone()))?;
        registry.register(Box::new(commands.errored.clone()))?;

        Ok(Self {
            registry,
            guilds,
            notifications,
            commands,
        })
    }
}

pub use server::serve;

mod server {
    use std::convert::Infallible;

    use http_body_util::Full;
    use hyper::body::{Bytes, Incoming};
    use hyper::header::CONTENT_TYPE;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Method, StatusCode};
    use hyper_util::rt::TokioIo;
    use prometheus::{Encoder, TextEncoder};
    use tokio::net::TcpListener;

    use crate::config::MetricsConfig;
    use crate::Metrics;

    type Body = Full<Bytes>;
    type Request = hyper::Request<Incoming>;
    type Response = hyper::Response<Body>;

    fn request(req: &Request, metrics: &Metrics) -> Response {
        if let (&Method::GET, "/metrics") = (req.method(), req.uri().path()) {
            let internal_error = || {
                let mut resp = Response::default();
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                resp
            };

            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = metrics.registry.gather();

            if encoder.encode(&metric_families, &mut buffer).is_err() {
                return internal_error();
            }

            hyper::Response::builder()
                .header(CONTENT_TYPE, encoder.format_type())
                .body(Body::from(buffer))
                .unwrap_or_else(|_| internal_error())
        } else {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            not_found
        }
    }

    pub async fn serve(config: MetricsConfig, metrics: Metrics) {
        tracing::info!("Metrics server listening on http://{}/metrics", config.addr);

        let listener = match TcpListener::bind(config.addr).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!("Metrics server error: {err}");
                return;
            }
        };

        loop {
            let stream = match listener.accept().await {
                Ok((stream, _)) => stream,
                Err(err) => {
                    tracing::warn!("Metrics server error: {err}");
                    continue;
                }
            };

            let metrics = metrics.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                let service = service_fn(|req| {
                    let resp = request(&req, &metrics);
                    async { Ok::<_, Infallible>(resp) }
                });

                let conn = http1::Builder::new().serve_connection(io, service);

                if let Err(err) = conn.await {
                    tracing::warn!("Failed to serve connection: {err}");
                }
            });
        }
    }
}
