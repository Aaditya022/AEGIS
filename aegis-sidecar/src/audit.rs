use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use aegis_common::crypto;
use chrono::Utc;
use tokio::sync::Mutex;
use tokio::time;
use tracing::{debug, error, info, warn};

const MAX_BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL_SECS: u64 = 1;

pub struct AuditLogger {
    sidecar_id: String,
    sequence: std::sync::atomic::AtomicU64,
    buffer: Arc<Mutex<VecDeque<serde_json::Value>>>,
    client: reqwest::Client,
    audit_service_url: String,
}

impl AuditLogger {
    pub fn new(_kafka_brokers: String, sidecar_id: String) -> Self {
        let logger = Self {
            sidecar_id,
            sequence: std::sync::atomic::AtomicU64::new(0),
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BATCH_SIZE * 2))),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_else(|e| {
                    warn!(error = %e, "Failed to build audit HTTP client");
                    reqwest::Client::new()
                }),
            audit_service_url: "http://localhost:9100/api/v1/audit/events".into(),
        };

        let buffer = logger.buffer.clone();
        let client = logger.client.clone();
        let url = logger.audit_service_url.clone();
        let sidecar_id = logger.sidecar_id.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(FLUSH_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let mut buf = buffer.lock().await;
                if buf.is_empty() {
                    continue;
                }

                let batch: Vec<serde_json::Value> = buf.drain(..).collect();
                let payload = serde_json::json!({
                    "events": batch,
                    "sidecar_id": sidecar_id,
                });

                match client.post(&url).json(&payload).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        debug!(count = batch.len(), "Audit batch flushed");
                    }
                    Ok(resp) => {
                        warn!(
                            status = %resp.status(),
                            count = batch.len(),
                            "Audit service returned error, re-queuing"
                        );
                        // Re-queue on failure
                        for event in batch {
                            if buf.len() < MAX_BATCH_SIZE * 2 {
                                buf.push_back(event);
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to send audit batch");
                        for event in batch {
                            if buf.len() < MAX_BATCH_SIZE * 2 {
                                buf.push_back(event);
                            }
                        }
                    }
                }
            }
        });

        info!("Audit logger initialized");
        logger
    }

    pub async fn log_event(
        &self,
        agent_id: &str,
        operation: &str,
        resource: &str,
        decision: &str,
        trace_id: &str,
    ) {
        let seq = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let timestamp = Utc::now();

        let event_data = format!(
            "{agent_id}|{operation}|{resource}|{decision}|{timestamp}|{seq}|{}",
            self.sidecar_id
        );
        let event_hash = crypto::hash_str(&event_data);

        let entry = serde_json::json!({
            "event_id": uuid::Uuid::new_v4().to_string(),
            "trace_id": trace_id,
            "agent_id": agent_id,
            "sidecar_id": self.sidecar_id,
            "operation": operation,
            "resource": resource,
            "decision": decision,
            "timestamp": timestamp.to_rfc3339(),
            "signature": event_hash,
            "sequence_number": seq,
            "metadata": {
                "hash": event_hash,
            }
        });

        let mut buf = self.buffer.lock().await;
        if buf.len() < MAX_BATCH_SIZE * 2 {
            buf.push_back(entry);
        } else {
            warn!(agent = %agent_id, "Audit buffer full, dropping event");
        }
    }
}
