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
    use std::future::Future;

    use hyper::header::CONTENT_TYPE;
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Method, Server, StatusCode};
    use prometheus::{Encoder, TextEncoder};

    use crate::config::MetricsConfig;
    use crate::Metrics;

    type Request = hyper::Request<Body>;
    type Response = hyper::Response<Body>;

    fn request(req: &Request, metrics: &Metrics) -> Response {
        if let (&Method::GET, "/metrics") = (req.method(), req.uri().path()) {
            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = metrics.registry.gather();
            encoder.encode(&metric_families, &mut buffer).unwrap();

            hyper::Response::builder()
                .header(CONTENT_TYPE, encoder.format_type())
                .body(Body::from(buffer))
                .unwrap()
        } else {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            not_found
        }
    }

    pub fn serve(config: &MetricsConfig, metrics: Metrics) -> impl Future<Output = ()> {
        let service = make_service_fn(move |_| {
            let metrics = metrics.clone();
            let service = service_fn(move |req| {
                let resp = request(&req, &metrics);
                async { Ok::<_, Infallible>(resp) }
            });
            async { Ok::<_, Infallible>(service) }
        });
        tracing::info!("Metrics server listening on http://{}/metrics", config.addr);

        let server = Server::bind(&config.addr).serve(service);
        async move {
            if let Err(err) = server.await {
                tracing::warn!("Metrics server error: {err}");
            }
        }
    }
}
