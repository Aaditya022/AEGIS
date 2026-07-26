use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use aegis_common::types::{Decision, PolicyContext, PolicyResult};
use aegis_policy_engine::PolicyEngine;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::metrics::MetricsRegistry;

pub struct PolicyEvaluator {
    engine: Arc<RwLock<PolicyEngine>>,
    control_plane_addr: String,
    policy_dir: String,
    metrics: MetricsRegistry,
    client: reqwest::Client,
}

impl PolicyEvaluator {
    pub fn new(control_plane_addr: String, policy_dir: String, metrics: &MetricsRegistry) -> Self {
        let engine = match PolicyEngine::new(&policy_dir) {
            Ok(e) => {
                info!(dir = %policy_dir, count = e.policy_count(), "Policies loaded");
                e
            }
            Err(e) => {
                warn!(error = %e, "Failed to load policies from disk, using empty engine");
                PolicyEngine::default()
            }
        };

        Self {
            engine: Arc::new(RwLock::new(engine)),
            control_plane_addr,
            policy_dir,
            metrics: metrics.clone_handle(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_else(|e| {
                    warn!(error = %e, "Failed to build policy HTTP client");
                    reqwest::Client::new()
                }),
        }
    }

    pub async fn evaluate(&self, ctx: &PolicyContext) -> anyhow::Result<PolicyResult> {
        let engine = self.engine.read().await;
        let start = std::time::Instant::now();

        let result = match engine.evaluate(ctx) {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, "Policy evaluation error");
                PolicyResult {
                    decision: Decision::Deny,
                    reason: format!("evaluation error: {e}"),
                    violated_policies: vec!["default-deny".into()],
                    evaluation_time_ns: 0,
                }
            }
        };

        let elapsed = start.elapsed().as_nanos() as i64;
        debug!(
            decision = %result.decision,
            reason = %result.reason,
            time_ns = elapsed,
            "Policy evaluated"
        );

        Ok(PolicyResult {
            evaluation_time_ns: elapsed,
            ..result
        })
    }

    pub async fn reload_from_disk(&self) -> anyhow::Result<()> {
        let path = Path::new(&self.policy_dir);
        if !path.exists() {
            warn!(dir = %self.policy_dir, "Policy directory does not exist");
            return Ok(());
        }

        let new_engine = PolicyEngine::new(&self.policy_dir)?;
        let count = new_engine.policy_count();
        *self.engine.write().await = new_engine;
        info!(count, "Policies reloaded from disk");
        Ok(())
    }

    pub async fn reload_from_control_plane(&self) -> anyhow::Result<()> {
        let url = format!("{}/v1/policies", self.control_plane_addr);
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let policies: Vec<String> = resp.json().await?;
                let new_engine = PolicyEngine::with_policies(&policies);
                let count = new_engine.policy_count();
                *self.engine.write().await = new_engine;
                info!(count, "Policies updated from control plane");
            }
            Ok(resp) => {
                warn!(status = %resp.status(), "Control plane returned error");
            }
            Err(e) => {
                warn!(error = %e, "Cannot reach control plane for policy update");
            }
        }
        Ok(())
    }
}
