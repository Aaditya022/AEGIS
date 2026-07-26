use serde::{Deserialize, Serialize};

pub type AgentId = String;
pub type TraceId = String;
pub type SidecarId = String;
pub type PolicyId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Decision {
    Allow,
    Deny,
    Escalate,
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Allow => write!(f, "ALLOW"),
            Decision::Deny => write!(f, "DENY"),
            Decision::Escalate => write!(f, "ESCALATE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub agent_id: AgentId,
    pub operation: String,
    pub resource: String,
    pub environment: String,
    pub recursion_depth: u32,
    pub budget_consumed_usd: f64,
    pub trace_id: TraceId,
    pub extra: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub decision: Decision,
    pub reason: String,
    pub violated_policies: Vec<String>,
    pub evaluation_time_ns: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub event_id: String,
    pub trace_id: TraceId,
    pub agent_id: AgentId,
    pub sidecar_id: SidecarId,
    pub operation: String,
    pub resource: String,
    pub decision: Decision,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signature: String,
    pub sequence_number: u64,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRoute {
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub api_key_ref: String,
    pub weight: u32,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub agent_id: AgentId,
    pub framework: String,
    pub allowed_tools: Vec<String>,
    pub allowed_models: Vec<String>,
    pub allowed_urls: Vec<String>,
    pub max_recursion: u32,
    pub budget_limit_usd: f64,
    pub human_approval_required: bool,
}
