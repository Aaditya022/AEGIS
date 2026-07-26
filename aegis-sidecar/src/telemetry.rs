use aegis_common::config::SidecarConfig;
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler};
use opentelemetry_sdk::Resource;
use tracing::{info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

pub struct TelemetryHandle {
    _shutdown_tracer: bool,
}

pub async fn init_telemetry(
    config: &SidecarConfig,
) -> anyhow::Result<(opentelemetry_sdk::trace::TracerProvider, TelemetryHandle)> {
    let tracer_provider = trace::TracerProvider::builder()
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "aegis-sidecar"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("service.instance.id", config.sidecar_id.clone()),
            KeyValue::new("aegis.agent.id", config.agent_id.clone()),
        ]))
        .build();

    let tracer = tracer_provider.tracer("aegis-sidecar");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(
            tracing_subscriber::EnvFilter::try_new(&config.log_level)
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry_layer)
        .try_init()
        .unwrap_or_else(|_| warn!("Telemetry already initialized"));

    info!(
        endpoint = %config.otlp_endpoint,
        "OpenTelemetry initialized (batch exporter disabled for simplicity)"
    );

    Ok((
        tracer_provider,
        TelemetryHandle {
            _shutdown_tracer: false,
        },
    ))
}
