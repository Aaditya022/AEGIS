use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// AgentPolicy defines a set of governance policies for AI agents
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    kind = "AgentPolicy",
    group = "aegis.ai",
    version = "v1",
    namespaced,
    shortname = "apol"
)]
pub struct AgentPolicySpec {
    /// Selector for target agents
    pub agent_selector: Option<AgentSelector>,

    /// List of policy rules to enforce
    pub policies: Vec<PolicyRule>,

    /// Compliance framework to enforce (eu_ai_act, nist, imda)
    pub compliance_framework: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AgentSelector {
    pub match_labels: Option<std::collections::BTreeMap<String, String>>,
    pub match_frameworks: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PolicyRule {
    pub category: String,
    pub severity: String,
    pub condition: String,
    pub enabled: bool,
}

/// AgentRegistration registers an AI agent with AEGIS
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    kind = "AgentRegistration",
    group = "aegis.ai",
    version = "v1",
    namespaced,
    shortname = "areg"
)]
pub struct AgentRegistrationSpec {
    pub agent_id: String,
    pub framework: String,
    pub identity_type: String,
    pub allowed_tools: Vec<String>,
    pub allowed_models: Vec<String>,
    pub max_recursion_depth: u32,
    pub budget_limit_usd: f64,
    pub human_approval_required: bool,
    pub autonomy_level: u32,
}
