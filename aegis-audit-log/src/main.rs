use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aegis_common::crypto;
use chrono::{DateTime, Utc};
use clap::Parser;
use kafka::producer::{Producer, Record, RequiredAcks};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Parser)]
#[command(name = "aegis-audit-log", about = "AEGIS Immutable Audit Log Service")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:9100")]
    listen: String,

    #[arg(long, default_value = "")]
    kafka: String,

    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEvent {
    event_id: String,
    trace_id: String,
    agent_id: String,
    sidecar_id: String,
    operation: String,
    resource: String,
    decision: String,
    timestamp: String,
    signature: String,
    sequence_number: u64,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditSegment {
    id: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    event_count: u64,
    root_hash: String,
    prev_segment_hash: String,
    events: Vec<AuditEvent>,
}

struct AuditLogService {
    segments: RwLock<Vec<AuditSegment>>,
    prev_hash: RwLock<String>,
    kafka_producer: Option<Producer>,
    kafka_topic: String,
}

impl AuditLogService {
    fn new(kafka_brokers: &str) -> Self {
        let kafka_producer = if !kafka_brokers.is_empty() {
            match Producer::from_hosts(vec![kafka_brokers.to_string()])
                .with_ack_timeout(Duration::from_secs(2))
                .with_required_acks(RequiredAcks::One)
                .create()
            {
                Ok(producer) => {
                    info!(brokers = %kafka_brokers, "Connected to Kafka");
                    Some(producer)
                }
                Err(e) => {
                    warn!(error = %e, "Failed to connect to Kafka, events stored in-memory only");
                    None
                }
            }
        } else {
            warn!("No Kafka brokers configured, events stored in-memory only");
            None
        };

        Self {
            segments: RwLock::new(Vec::new()),
            prev_hash: RwLock::new(
                "0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
            kafka_producer,
            kafka_topic: "aegis-audit-events".into(),
        }
    }

    async fn ingest_event(&self, event: AuditEvent) -> anyhow::Result<()> {
        debug!(
            event_id = %event.event_id,
            agent = %event.agent_id,
            decision = %event.decision,
            "Ingesting audit event"
        );

        let mut segments = self.segments.write().await;
        let mut prev_hash = self.prev_hash.write().await;

        let event_bytes = serde_json::to_vec(&event)?;
        let event_hash = crypto::hash_chain(prev_hash.as_bytes(), &event_bytes);
        let event_hash_hex = hex::encode(&event_hash);

        let segment = AuditSegment {
            id: uuid::Uuid::new_v4().to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            event_count: 1,
            root_hash: event_hash_hex.clone(),
            prev_segment_hash: prev_hash.clone(),
            events: vec![event],
        };

        segments.push(segment);
        *prev_hash = event_hash_hex;

        Ok(())
    }

    async fn publish_to_kafka(&self, event: &AuditEvent) {
        if let Some(ref producer) = self.kafka_producer {
            let payload = serde_json::to_vec(event).unwrap_or_default();
            let record =
                Record::from_key_value(&self.kafka_topic, event.event_id.as_bytes(), &payload);
            match producer.send(&record) {
                Ok(_) => debug!(event_id = %event.event_id, "Published to Kafka"),
                Err(e) => warn!(error = %e, "Failed to publish to Kafka"),
            }
        }
    }

    async fn verify_integrity(&self) -> anyhow::Result<bool> {
        let segments = self.segments.read().await;
        let mut expected_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        for segment in segments.iter() {
            if segment.prev_segment_hash != expected_hash {
                return Ok(false);
            }
            for event in &segment.events {
                let event_bytes = serde_json::to_vec(event)?;
                let computed = crypto::hash_chain(expected_hash.as_bytes(), &event_bytes);
                expected_hash = hex::encode(computed);
            }
        }

        Ok(true)
    }

    async fn query(
        &self,
        agent_id: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Vec<AuditEvent> {
        let segments = self.segments.read().await;
        let mut results = Vec::new();

        for segment in segments.iter().rev() {
            for event in &segment.events {
                let ts = DateTime::parse_from_rfc3339(&event.timestamp)
                    .map(|t| t.with_timezone(&Utc))
                    .unwrap_or(Utc::now());

                if let Some(aid) = agent_id {
                    if event.agent_id != *aid {
                        continue;
                    }
                }
                if let Some(from_ts) = from {
                    if ts < from_ts {
                        continue;
                    }
                }
                if let Some(to_ts) = to {
                    if ts > to_ts {
                        continue;
                    }
                }

                results.push(event.clone());
                if results.len() >= limit {
                    return results;
                }
            }
        }

        results
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .json()
        .init();

    info!(
        listen = %args.listen,
        kafka = if args.kafka.is_empty() { "none" } else { &args.kafka },
        "AEGIS Audit Log Service starting"
    );

    let service = Arc::new(AuditLogService::new(&args.kafka));

    // HTTP ingestion API
    let svc = service.clone();
    let app = axum::Router::new()
        .route(
            "/api/v1/audit/events",
            axum::routing::post(move |body: String| {
                let svc = svc.clone();
                async move {
                    match serde_json::from_str::<AuditEvent>(&body) {
                        Ok(event) => {
                            let _ = svc.publish_to_kafka(&event).await;
                            match svc.ingest_event(event).await {
                                Ok(_) => axum::Json(serde_json::json!({"status": "ok"})),
                                Err(e) => {
                                    error!(error = %e, "Failed to ingest event");
                                    axum::Json(serde_json::json!({"status": "error", "error": e.to_string()}))
                                }
                            }
                        }
                        Err(e) => axum::Json(serde_json::json!({"status": "error", "error": e.to_string()})),
                    }
                }
            }),
        )
        .route(
            "/api/v1/audit/verify",
            axum::routing::get(move || {
                let svc = service.clone();
                async move {
                    let valid = svc.verify_integrity().await.unwrap_or(false);
                    axum::Json(serde_json::json!({"valid": valid}))
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!("Listening on {}", args.listen);
    axum::serve(listener, app).await?;

    Ok(())
}
