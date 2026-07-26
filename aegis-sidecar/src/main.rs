use std::path::PathBuf;
use std::sync::Arc;

use aegis_common::config::SidecarConfig;
use bytes::Bytes;
use clap::Parser;
use http_body_util::Full;
use tokio::signal;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

mod audit;
mod cost_circuit;
mod ebpf;
mod identity;
mod metrics;
mod middleware;
mod policy;
mod protocol;
mod proxy;
mod recursion;
mod telemetry;
mod tool_gate;

#[derive(Parser)]
#[command(
    name = "aegis-sidecar",
    about = "AEGIS Governance Sidecar — policy enforcement proxy for AI agents",
    version
)]
struct Args {
    #[arg(short, long, default_value = "/etc/aegis/sidecar.yaml")]
    config: PathBuf,

    #[arg(short, long)]
    agent_id: Option<String>,

    #[arg(long)]
    enable_ebpf: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut config: SidecarConfig = {
        let data = tokio::fs::read_to_string(&args.config)
            .await
            .unwrap_or_else(|_| {
                warn!(config = %args.config.display(), "Config file not found, using defaults");
                "".into()
            });
        if data.is_empty() {
            SidecarConfig::default()
        } else {
            serde_yaml::from_str(&data)?
        }
    };

    if let Some(agent_id) = args.agent_id {
        config.agent_id = agent_id;
    }
    if args.enable_ebpf {
        config.enable_ebpf = true;
    }

    let (tracer_provider, telemetry) = telemetry::init_telemetry(&config).await?;
    let metrics_registry = metrics::MetricsRegistry::new();

    let state = Arc::new(AppState {
        config: RwLock::new(config.clone()),
        identity: identity::IdentityVerifier::new(config.control_plane_addr.clone()),
        policy: policy::PolicyEvaluator::new(
            config.control_plane_addr.clone(),
            config.policy_dir.clone(),
            &metrics_registry,
        ),
        tool_gate: tool_gate::ToolGate::new(config.allowed_tools.clone()),
        cost: cost_circuit::CostCircuit::new(config.budget_limit_usd, config.sidecar_id.clone()),
        recursion: recursion::RecursionDetector::new(config.max_recursion_depth),
        audit: audit::AuditLogger::new(config.kafka_brokers.clone(), config.sidecar_id.clone()),
        ebpf: if config.enable_ebpf {
            Some(ebpf::EbpfManager::new().await?)
        } else {
            None
        },
        metrics: metrics_registry,
        _telemetry: telemetry,
    });

    info!(
        sidecar_id = %config.sidecar_id,
        agent_id = %config.agent_id,
        listen = %config.listen_addr,
        ebpf = config.enable_ebpf,
        "AEGIS sidecar starting"
    );

    let proxy_listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    let health_listener =
        match tokio::net::TcpListener::bind(config.listen_addr.replace(":9000", ":9090")).await {
            Ok(l) => l,
            Err(_) => tokio::net::TcpListener::bind("0.0.0.0:9090").await.unwrap(),
        };

    let state_clone = state.clone();
    let proxy_handle = tokio::spawn(async move {
        proxy::Proxy::run(proxy_listener, state_clone).await;
    });

    let health_handle = tokio::spawn(async move {
        serve_health(health_listener, state).await;
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
        _ = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap().recv() => {
            info!("SIGTERM received");
        }
    }

    info!("Shutting down sidecar");
    proxy_handle.abort();
    health_handle.abort();
    tracer_provider.shutdown()?;

    Ok(())
}

async fn handle_health(
    req: hyper::Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<hyper::Response<Full<Bytes>>, hyper::Error> {
    match req.uri().path() {
        "/health" | "/ready" => Ok(hyper::Response::builder()
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(
                serde_json::json!({
                    "status": "ok",
                    "sidecar_id": state.config.read().await.sidecar_id,
                    "agent_id": state.config.read().await.agent_id,
                    "uptime_seconds": 0,
                    "connections_active": 0,
                })
                .to_string(),
            )))
            .unwrap()),
        "/metrics" => {
            let metrics = state.metrics.snapshot().await;
            Ok(hyper::Response::builder()
                .header("content-type", "text/plain")
                .body(Full::new(Bytes::from(metrics)))
                .unwrap())
        }
        _ => Ok(hyper::Response::builder()
            .status(404)
            .body(Full::new(Bytes::from("not found")))
            .unwrap()),
    }
}

async fn serve_health(listener: tokio::net::TcpListener, state: Arc<AppState>) {
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, "Health accept error");
                continue;
            }
        };
        let state = state.clone();
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |req| handle_health(req, state.clone()));
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await
            {
                error!(error = %e, "Health connection error");
            }
        });
    }
}

pub struct AppState {
    pub config: RwLock<SidecarConfig>,
    pub identity: identity::IdentityVerifier,
    pub policy: policy::PolicyEvaluator,
    pub tool_gate: tool_gate::ToolGate,
    pub cost: cost_circuit::CostCircuit,
    pub recursion: recursion::RecursionDetector,
    pub audit: audit::AuditLogger,
    pub ebpf: Option<ebpf::EbpfManager>,
    pub metrics: metrics::MetricsRegistry,
    pub _telemetry: telemetry::TelemetryHandle,
}
