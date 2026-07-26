use hyper::{header, HeaderMap};
use tracing::{debug, warn};

use aegis_common::types::PolicyContext;

use crate::AppState;

#[derive(Debug)]
pub enum GovernanceOutcome {
    Allow,
    Deny { reason: String, category: String },
    Escalate { reason: String, category: String },
}

pub fn extract_agent_id(headers: &HeaderMap) -> Option<String> {
    // Check multiple header sources for agent identity
    if let Some(id) = headers
        .get("x-aegis-agent-id")
        .and_then(|v| v.to_str().ok())
    {
        return Some(id.to_string());
    }
    if let Some(id) = headers.get("x-agent-id").and_then(|v| v.to_str().ok()) {
        return Some(id.to_string());
    }
    // SPIFFE SVID from mTLS
    if let Some(svid) = headers
        .get("x-forwarded-client-cert")
        .and_then(|v| v.to_str().ok())
    {
        // Extract SPIFFE ID from the cert header
        if let Some(spiffe_id) = svid.split(',').find_map(|part| {
            let p = part.trim();
            p.strip_prefix("URI=")
                .or_else(|| p.strip_prefix("Subject="))
        }) {
            return Some(spiffe_id.to_string());
        }
    }
    None
}

pub async fn run_governance(
    state: &AppState,
    ctx: &PolicyContext,
    headers: &HeaderMap,
) -> GovernanceOutcome {
    // 1. Identity verification
    let identity_valid = state.identity.verify_identity(ctx, headers).await;
    if !identity_valid {
        warn!(agent = %ctx.agent_id, "Identity verification failed");
        return GovernanceOutcome::Deny {
            reason: "identity verification failed".into(),
            category: "identity".into(),
        };
    }

    // 2. Policy evaluation
    match state.policy.evaluate(ctx).await {
        Ok(result) => {
            use aegis_common::types::Decision;
            match result.decision {
                Decision::Deny => {
                    warn!(
                        agent = %ctx.agent_id,
                        operation = %ctx.operation,
                        reason = %result.reason,
                        "Policy denied"
                    );
                    return GovernanceOutcome::Deny {
                        reason: result.reason,
                        category: "policy".into(),
                    };
                }
                Decision::Escalate => {
                    warn!(
                        agent = %ctx.agent_id,
                        reason = %result.reason,
                        "Policy escalation"
                    );
                    return GovernanceOutcome::Escalate {
                        reason: result.reason,
                        category: result
                            .violated_policies
                            .first()
                            .cloned()
                            .unwrap_or_default(),
                    };
                }
                Decision::Allow => {
                    debug!(agent = %ctx.agent_id, "Policy allowed");
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Policy evaluation error, denying by default");
            return GovernanceOutcome::Deny {
                reason: "policy evaluation error".into(),
                category: "policy-error".into(),
            };
        }
    }

    // 3. Tool gate check
    if let Some(tool) = extract_tool_name(&ctx.resource) {
        if !state.tool_gate.is_allowed(&tool).await {
            warn!(agent = %ctx.agent_id, tool = %tool, "Unauthorized tool");
            return GovernanceOutcome::Deny {
                reason: format!("unauthorized tool: {tool}"),
                category: "tool-gate".into(),
            };
        }
    }

    // 4. Recursion check
    if let Some(parent_trace) = headers
        .get("x-aegis-parent-trace")
        .and_then(|v| v.to_str().ok())
    {
        if state.recursion.check(parent_trace).await {
            warn!(agent = %ctx.agent_id, trace = %parent_trace, "Recursion limit exceeded");
            return GovernanceOutcome::Deny {
                reason: "recursion limit exceeded".into(),
                category: "recursion".into(),
            };
        }
    }

    // 5. Cost circuit check
    if let Err(msg) = state.cost.check() {
        warn!(agent = %ctx.agent_id, error = %msg, "Budget exceeded");
        return GovernanceOutcome::Deny {
            reason: msg,
            category: "budget".into(),
        };
    }

    // 6. Environment scoping
    let env = std::env::var("AEGIS_ENV").unwrap_or_default();
    if !env.is_empty() {
        let resource = &ctx.resource;
        if env == "staging" && resource.contains("production") {
            return GovernanceOutcome::Deny {
                reason: "staging credentials cannot access production resources".into(),
                category: "environment".into(),
            };
        }
    }

    GovernanceOutcome::Allow
}

fn extract_tool_name(resource: &str) -> Option<String> {
    if let Some(tool) = resource.strip_prefix("/tools/") {
        return Some(
            tool.split(&['/', '?', '&'])
                .next()
                .unwrap_or(tool)
                .to_string(),
        );
    }
    if let Some(tool) = resource.strip_prefix("/v1/tools/") {
        return Some(
            tool.split(&['/', '?', '&'])
                .next()
                .unwrap_or(tool)
                .to_string(),
        );
    }
    let uri: hyper::Uri = resource.parse().ok()?;
    uri.query().and_then(|q| {
        q.split('&')
            .find_map(|p| p.strip_prefix("tool=").map(|s| s.to_string()))
    })
}
