use std::collections::HashMap;

use chrono::{Datelike, Timelike};

mod builtins;
mod compiler;
mod engine;
pub mod parser;
pub mod test_framework;
mod wasm;

pub use builtins::*;
pub use compiler::RegoCompiler;
pub use parser::*;

pub const POLICY_RECURSION: &str = "recursion";
pub const POLICY_BUDGET: &str = "budget";
pub const POLICY_ALLOWED_MODELS: &str = "allowed_models";
pub const POLICY_ALLOWED_TOOLS: &str = "allowed_tools";
pub const POLICY_ALLOWED_URLS: &str = "allowed_urls";
pub const POLICY_ALLOWED_DATABASES: &str = "allowed_databases";
pub const POLICY_TIME_RESTRICTIONS: &str = "time_restrictions";
pub const POLICY_HUMAN_APPROVAL: &str = "human_approval";
pub const POLICY_PROMPT_RISK: &str = "prompt_risk";
pub const POLICY_DELEGATION: &str = "delegation";
pub const POLICY_ENVIRONMENT: &str = "environment";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub category: String,
    pub severity: String,
    pub rego_source: String,
    pub wasm_binary: Option<Vec<u8>>,
    pub enabled: bool,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Default)]
pub struct PolicyEngine {
    policies: Vec<Policy>,
    builtins: BuiltinRegistry,
    metrics: EngineMetrics,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct EngineMetrics {
    evaluations: u64,
    cache_hits: u64,
    avg_eval_time_ns: f64,
}

pub struct BuiltinRegistry {
    functions: HashMap<String, Box<dyn BuiltinFunction + Send + Sync>>,
}

pub trait BuiltinFunction: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String>;
}

impl std::fmt::Debug for BuiltinRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinRegistry")
            .field("functions", &self.functions.keys())
            .finish()
    }
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut functions: HashMap<String, Box<dyn BuiltinFunction + Send + Sync>> = HashMap::new();

        functions.insert("aegis.is_recursion_loop".into(), Box::new(IsRecursionLoop));
        functions.insert("aegis.budget_exceeded".into(), Box::new(BudgetExceeded));
        functions.insert("aegis.tool_allowed".into(), Box::new(ToolAllowed));
        functions.insert("aegis.url_allowed".into(), Box::new(UrlAllowed));
        functions.insert("aegis.model_allowed".into(), Box::new(ModelAllowed));
        functions.insert(
            "aegis.reasoning_risk_score".into(),
            Box::new(ReasoningRiskScore),
        );
        functions.insert("aegis.environment_match".into(), Box::new(EnvironmentMatch));
        functions.insert("aegis.delegation_depth".into(), Box::new(DelegationDepth));
        functions.insert("aegis.in_business_hours".into(), Box::new(InBusinessHours));
        functions.insert("aegis.hash_equals".into(), Box::new(HashEquals));

        Self { functions }
    }

    pub fn get(&self, name: &str) -> Option<&(dyn BuiltinFunction + Send + Sync)> {
        self.functions.get(name).map(|f| f.as_ref())
    }

    pub fn evaluate(
        &self,
        name: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, String> {
        match self.get(name) {
            Some(f) => f.evaluate(args),
            None => Err(format!("unknown builtin: {name}")),
        }
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct IsRecursionLoop;
impl BuiltinFunction for IsRecursionLoop {
    fn name(&self) -> &'static str {
        "aegis.is_recursion_loop"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let depth = args.first().and_then(|v| v.as_u64()).unwrap_or(0);
        let max_depth = args.get(1).and_then(|v| v.as_u64()).unwrap_or(5);
        Ok(serde_json::Value::Bool(depth > max_depth))
    }
}

#[derive(Debug)]
struct BudgetExceeded;
impl BuiltinFunction for BudgetExceeded {
    fn name(&self) -> &'static str {
        "aegis.budget_exceeded"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let spent = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let limit = args.get(1).and_then(|v| v.as_f64()).unwrap_or(100.0);
        Ok(serde_json::Value::Bool(spent > limit))
    }
}

#[derive(Debug)]
struct ToolAllowed;
impl BuiltinFunction for ToolAllowed {
    fn name(&self) -> &'static str {
        "aegis.tool_allowed"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let tool = args.first().and_then(|v| v.as_str()).unwrap_or("");
        let allowed_list = args
            .get(1)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        let allowed = allowed_list.is_empty() || allowed_list.contains(&tool);
        Ok(serde_json::Value::Bool(allowed))
    }
}

#[derive(Debug)]
struct UrlAllowed;
impl BuiltinFunction for UrlAllowed {
    fn name(&self) -> &'static str {
        "aegis.url_allowed"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let url = args.first().and_then(|v| v.as_str()).unwrap_or("");
        let allowed = args
            .get(1)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .any(|pattern| url.contains(pattern))
            })
            .unwrap_or(true);
        Ok(serde_json::Value::Bool(allowed))
    }
}

#[derive(Debug)]
struct ModelAllowed;
impl BuiltinFunction for ModelAllowed {
    fn name(&self) -> &'static str {
        "aegis.model_allowed"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let model = args.first().and_then(|v| v.as_str()).unwrap_or("");
        let allowed = args
            .get(1)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .any(|m| model.starts_with(m))
            })
            .unwrap_or(true);
        Ok(serde_json::Value::Bool(allowed))
    }
}

#[derive(Debug)]
struct ReasoningRiskScore;
impl BuiltinFunction for ReasoningRiskScore {
    fn name(&self) -> &'static str {
        "aegis.reasoning_risk_score"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let text = args.first().and_then(|v| v.as_str()).unwrap_or("");
        let high_risk_patterns = [
            "ignore",
            "bypass",
            "override",
            "delete",
            "drop ",
            "rm -rf",
            "sudo",
            "chmod 777",
            "admin",
            "password",
            "secret",
            "token",
            "credit card",
            "social security",
        ];
        let mut score: f64 = 0.0;
        for pattern in &high_risk_patterns {
            if text.to_lowercase().contains(pattern) {
                score += 0.15;
            }
        }
        Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(score.min(1.0))
                .unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
        ))
    }
}

#[derive(Debug)]
struct EnvironmentMatch;
impl BuiltinFunction for EnvironmentMatch {
    fn name(&self) -> &'static str {
        "aegis.environment_match"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let resource_env = args.first().and_then(|v| v.as_str()).unwrap_or("");
        let current_env = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
        let match_ = resource_env == current_env
            || (current_env == "staging" && !resource_env.contains("production"))
            || (current_env == "production" && resource_env.contains("production"));
        Ok(serde_json::Value::Bool(match_))
    }
}

#[derive(Debug)]
struct DelegationDepth;
impl BuiltinFunction for DelegationDepth {
    fn name(&self) -> &'static str {
        "aegis.delegation_depth"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let depth = args.first().and_then(|v| v.as_u64()).unwrap_or(0);
        let max = args.get(1).and_then(|v| v.as_u64()).unwrap_or(3);
        Ok(serde_json::Value::Bool(depth <= max))
    }
}

#[derive(Debug)]
struct InBusinessHours;
impl BuiltinFunction for InBusinessHours {
    fn name(&self) -> &'static str {
        "aegis.in_business_hours"
    }
    fn evaluate(&self, _args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let now = chrono::Utc::now();
        let hour = now.hour();
        let weekday = now.weekday();
        let is_weekday = weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun;
        let is_business_hours = is_weekday && (9..=17).contains(&hour);
        Ok(serde_json::Value::Bool(is_business_hours))
    }
}

#[derive(Debug)]
struct HashEquals;
impl BuiltinFunction for HashEquals {
    fn name(&self) -> &'static str {
        "aegis.hash_equals"
    }
    fn evaluate(&self, args: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let data = args.first().and_then(|v| v.as_str()).unwrap_or("");
        let expected = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
        use sha2::{Digest, Sha256};
        let hash = hex::encode(Sha256::digest(data.as_bytes()));
        Ok(serde_json::Value::Bool(hash == expected))
    }
}
