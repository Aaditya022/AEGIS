use std::time::Instant;

use aegis_common::types::{Decision, PolicyContext, PolicyResult};
use tracing::{debug, info, warn};

use crate::{BuiltinRegistry, EngineMetrics, Policy, PolicyEngine};

impl PolicyEngine {
    pub fn new(policy_dir: &str) -> anyhow::Result<Self> {
        let path = std::path::Path::new(policy_dir);
        if !path.exists() {
            warn!(dir = %policy_dir, "Policy directory does not exist");
            return Ok(Self::default());
        }

        let mut policies = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let content = std::fs::read_to_string(entry.path())?;

            if entry.path().extension().is_some_and(|e| e == "rego") {
                match crate::parse_rego(&content) {
                    Ok(policy) => {
                        debug!(name = %policy.name, file = %entry.file_name().to_string_lossy(), "Loaded Rego policy");
                        policies.push(policy);
                    }
                    Err(e) => {
                        warn!(file = %entry.file_name().to_string_lossy(), error = %e, "Failed to parse Rego policy");
                    }
                }
            }
        }

        info!(count = policies.len(), "Policies loaded from disk");
        Ok(Self {
            policies,
            builtins: BuiltinRegistry::new(),
            metrics: Default::default(),
        })
    }

    pub fn with_policies(sources: &[String]) -> Self {
        let policies: Vec<Policy> = sources
            .iter()
            .filter_map(|s| {
                crate::parse_rego(s)
                    .map_err(|e| warn!(error = %e, "Failed to parse policy"))
                    .ok()
            })
            .collect();

        info!(count = policies.len(), "Policies loaded from sources");
        Self {
            policies,
            builtins: BuiltinRegistry::new(),
            metrics: Default::default(),
        }
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn policies(&self) -> &[Policy] {
        &self.policies
    }

    pub fn evaluate(&mut self, ctx: &PolicyContext) -> anyhow::Result<PolicyResult> {
        let start = Instant::now();

        // Builtin: reasoning risk score
        let prompt_risk = self
            .builtins
            .evaluate(
                "aegis.reasoning_risk_score",
                &[serde_json::Value::String(ctx.operation.clone())],
            )
            .unwrap_or(serde_json::Value::Number(
                serde_json::Number::from_f64(0.0).unwrap(),
            ));

        let prompt_risk = prompt_risk.as_f64().unwrap_or(0.0);

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }

            let violated = match policy.category.as_str() {
                crate::POLICY_RECURSION => {
                    let depth = ctx.recursion_depth as u64;
                    let max = parse_condition_int(&policy.rego_source, 5);
                    depth > max
                }
                crate::POLICY_BUDGET => {
                    let spent = ctx.budget_consumed_usd;
                    let limit = parse_condition_float(&policy.rego_source, 100.0);
                    spent > limit
                }
                crate::POLICY_ALLOWED_MODELS => {
                    let resource = &ctx.resource;
                    let allowed = parse_condition_list(&policy.rego_source);
                    !allowed.is_empty() && !allowed.iter().any(|a| resource.contains(a))
                }
                crate::POLICY_ALLOWED_TOOLS => {
                    let resource = &ctx.resource;
                    let allowed = parse_condition_list(&policy.rego_source);
                    !allowed.is_empty() && !allowed.iter().any(|a| resource.contains(a))
                }
                crate::POLICY_ALLOWED_URLS => {
                    let url = &ctx.resource;
                    let allowed = parse_condition_list(&policy.rego_source);
                    !allowed.is_empty() && !allowed.iter().any(|a| url.contains(a))
                }
                crate::POLICY_TIME_RESTRICTIONS => {
                    match self.builtins.evaluate("aegis.in_business_hours", &[]) {
                        Ok(val) => !val.as_bool().unwrap_or(true),
                        Err(_) => false,
                    }
                }
                crate::POLICY_HUMAN_APPROVAL => {
                    ctx.extra.get("human_approved").map(|s| s.as_str()) != Some("true")
                }
                crate::POLICY_PROMPT_RISK => {
                    let threshold = parse_condition_float(&policy.rego_source, 0.8);
                    prompt_risk > threshold
                }
                crate::POLICY_DELEGATION => {
                    let chain_depth = ctx
                        .extra
                        .get("delegation_depth")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    let max = parse_condition_int(&policy.rego_source, 3);
                    chain_depth > max
                }
                crate::POLICY_ENVIRONMENT => {
                    let env = &ctx.environment;
                    let resource = &ctx.resource;
                    env == "staging" && resource.contains("production")
                }
                _ => false,
            };

            if violated {
                let decision = match policy.severity.as_str() {
                    "block" => Decision::Deny,
                    "warn" | "escalate" => Decision::Escalate,
                    _ => Decision::Deny,
                };

                let elapsed = start.elapsed().as_nanos() as i64;
                self.metrics.evaluations += 1;
                self.metrics.avg_eval_time_ns = (self.metrics.avg_eval_time_ns
                    * (self.metrics.evaluations - 1) as f64
                    + elapsed as f64)
                    / self.metrics.evaluations as f64;

                debug!(
                    policy = %policy.name,
                    category = %policy.category,
                    decision = %decision,
                    reason = %policy.description.as_deref().unwrap_or("policy violated"),
                    "Policy violation"
                );

                return Ok(PolicyResult {
                    decision,
                    reason: policy
                        .description
                        .clone()
                        .unwrap_or_else(|| format!("policy '{}' violated", policy.name)),
                    violated_policies: vec![policy.id.clone()],
                    evaluation_time_ns: elapsed,
                });
            }
        }

        let elapsed = start.elapsed().as_nanos() as i64;
        self.metrics.evaluations += 1;

        Ok(PolicyResult {
            decision: Decision::Allow,
            reason: "all policies passed".into(),
            violated_policies: vec![],
            evaluation_time_ns: elapsed,
        })
    }

    pub fn get_metrics(&self) -> &EngineMetrics {
        &self.metrics
    }
}

fn parse_condition_int(source: &str, default: u64) -> u64 {
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("# @max=") {
            if let Ok(v) = line.trim_start_matches("# @max=").trim().parse() {
                return v;
            }
        }
        if line.starts_with("# @limit=") {
            if let Ok(v) = line.trim_start_matches("# @limit=").trim().parse() {
                return v;
            }
        }
    }
    default
}

fn parse_condition_float(source: &str, default: f64) -> f64 {
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("# @threshold=") {
            if let Ok(v) = line.trim_start_matches("# @threshold=").trim().parse() {
                return v;
            }
        }
        if line.starts_with("# @limit=") {
            if let Ok(v) = line.trim_start_matches("# @limit=").trim().parse() {
                return v;
            }
        }
    }
    default
}

fn parse_condition_list(source: &str) -> Vec<String> {
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("# @allowed=") {
            return line
                .trim_start_matches("# @allowed=")
                .trim()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    vec![]
}
