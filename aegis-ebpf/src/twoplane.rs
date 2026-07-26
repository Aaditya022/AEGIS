use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::warn;

/// Two-Plane Verification Engine
/// Compares application-plane (πA) decisions with infrastructure-plane (πI) observations
/// Implements Definition 5.4 from the AEGIS paper
pub struct TwoPlaneVerifier {
    violations: AtomicU64,
    total_events: AtomicU64,
    app_plane_decisions: Arc<RwLock<HashMap<String, AppPlaneDecision>>>,
    infra_plane_observations: Arc<RwLock<Vec<InfraPlaneObservation>>>,
    divergent_events: Arc<RwLock<Vec<DivergentEvent>>>,
}

#[derive(Debug, Clone)]
pub struct AppPlaneDecision {
    pub operation: String,
    pub resource: String,
    pub decision: String,
    pub agent_id: String,
    pub timestamp_ns: u64,
    pub trace_id: String,
}

#[derive(Debug, Clone)]
pub struct InfraPlaneObservation {
    pub pid: u32,
    pub operation: String,
    pub resource: String,
    pub actual_decision: String,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone)]
pub struct DivergentEvent {
    pub trace_id: String,
    pub app_plane_decision: String,
    pub infra_plane_decision: String,
    pub operation: String,
    pub resource: String,
    pub agent_id: String,
    pub timestamp_ns: u64,
}

impl Clone for TwoPlaneVerifier {
    fn clone(&self) -> Self {
        Self {
            violations: AtomicU64::new(self.violations.load(Ordering::SeqCst)),
            total_events: AtomicU64::new(self.total_events.load(Ordering::SeqCst)),
            app_plane_decisions: self.app_plane_decisions.clone(),
            infra_plane_observations: self.infra_plane_observations.clone(),
            divergent_events: self.divergent_events.clone(),
        }
    }
}

impl TwoPlaneVerifier {
    pub fn new() -> Self {
        Self {
            violations: AtomicU64::new(0),
            total_events: AtomicU64::new(0),
            app_plane_decisions: Arc::new(RwLock::new(HashMap::new())),
            infra_plane_observations: Arc::new(RwLock::new(Vec::new())),
            divergent_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record an application-plane decision (πA)
    /// Called by the sidecar when it makes a policy decision
    pub async fn record_app_plane(&self, decision: AppPlaneDecision) {
        self.total_events.fetch_add(1, Ordering::SeqCst);
        let mut decisions = self.app_plane_decisions.write().await;
        decisions.insert(decision.trace_id.clone(), decision);
    }

    /// Record an infrastructure-plane observation (πI)
    /// Called by the eBPF event processor
    pub async fn record_infra_plane(&self, observation: InfraPlaneObservation) {
        let mut observations = self.infra_plane_observations.write().await;
        observations.push(observation);

        // Prune if too large
        if observations.len() > 100_000 {
            observations.drain(0..50_000);
        }
    }

    /// Verify an operation against both planes
    /// Returns (allowed, divergence_detected)
    pub async fn verify_operation(
        &self,
        trace_id: &str,
        operation: &str,
        resource: &str,
        agent_id: &str,
    ) -> (bool, bool) {
        let decisions = self.app_plane_decisions.read().await;
        let app_decision = decisions.get(trace_id);

        let app_plane_allows = match app_decision {
            Some(d) => d.decision == "ALLOW",
            None => true, // No app-plane decision = allow (not yet processed)
        };

        // Check eBPF infra-plane observations for this operation
        let observations = self.infra_plane_observations.read().await;
        let infra_plane_allows = !observations.iter().any(|o| {
            o.operation == operation && o.resource == resource && o.actual_decision == "DENY"
        });

        // Two-Plane Verification: if planes disagree, block and alert
        if app_plane_allows != infra_plane_allows {
            self.violations.fetch_add(1, Ordering::SeqCst);
            let divergent = DivergentEvent {
                trace_id: trace_id.to_string(),
                app_plane_decision: if app_plane_allows {
                    "ALLOW".into()
                } else {
                    "DENY".into()
                },
                infra_plane_decision: if infra_plane_allows {
                    "ALLOW".into()
                } else {
                    "DENY".into()
                },
                operation: operation.to_string(),
                resource: resource.to_string(),
                agent_id: agent_id.to_string(),
                timestamp_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
            };

            warn!(
                trace_id = %divergent.trace_id,
                app = %divergent.app_plane_decision,
                infra = %divergent.infra_plane_decision,
                op = %divergent.operation,
                "Two-Plane divergence detected"
            );

            let mut divergent_events = self.divergent_events.write().await;
            divergent_events.push(divergent);

            return (false, true); // Blocked due to divergence
        }

        (app_plane_allows && infra_plane_allows, false)
    }

    /// Get all divergent events
    pub async fn get_divergent_events(&self) -> Vec<DivergentEvent> {
        self.divergent_events.read().await.clone()
    }

    /// Total events processed
    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::SeqCst)
    }

    /// Total violations detected
    pub fn total_violations(&self) -> u64 {
        self.violations.load(Ordering::SeqCst)
    }

    /// Check if eBPF infra-plane detected a policy violation for a file access
    pub async fn check_file_access(&self, pid: u32, path: &str) -> bool {
        let observations = self.infra_plane_observations.read().await;
        !observations.iter().any(|o| {
            o.pid == pid
                && o.operation == "file.open"
                && o.resource == path
                && o.actual_decision == "DENY"
        })
    }

    /// Check if eBPF infra-plane detected a policy violation for a network connection
    pub async fn check_network(&self, pid: u32, addr: &str) -> bool {
        let observations = self.infra_plane_observations.read().await;
        !observations.iter().any(|o| {
            o.pid == pid
                && o.operation == "net.connect"
                && o.resource == addr
                && o.actual_decision == "DENY"
        })
    }

    /// Periodic tick for maintenance
    pub fn tick(&self) {}

    /// Get a summary of the Two-Plane Verification state
    pub async fn summary(&self) -> serde_json::Value {
        let divergent = self.divergent_events.read().await.len();

        serde_json::json!({
            "total_events": self.total_events(),
            "total_violations": self.total_violations(),
            "divergent_events": divergent,
            "status": if divergent > 0 { "ALERT" } else { "NOMINAL" },
        })
    }
}
