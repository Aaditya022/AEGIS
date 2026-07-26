/// Agent-specific built-in functions for Rego policy evaluation.
/// These extend OPA with agent-specific primitives.
use chrono::{Datelike, Timelike};

/// Detect if an operation is a recursive call (same tool within N steps)
pub fn detect_recursion(trace: &[String], current_tool: &str, max_depth: usize) -> bool {
    let recent: Vec<&String> = trace.iter().rev().take(max_depth).collect();
    let count = recent.iter().filter(|t| ***t == *current_tool).count();
    count >= max_depth
}

/// Check if budget is exceeded
pub fn budget_exceeded(spent: f64, limit: f64) -> bool {
    spent > limit
}

/// Evaluate a reasoning pattern risk score (0.0 to 1.0)
pub fn reasoning_risk_score(text: &str) -> f64 {
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
        "ignore previous",
        "disregard",
    ];
    let mut score: f64 = 0.0;
    for pattern in &high_risk_patterns {
        if text.to_lowercase().contains(pattern) {
            score += 0.15;
        }
    }
    score.min(1.0)
}

/// Validate tool call parameters against a JSON schema
pub fn validate_tool_params(params: &str, schema: &str) -> Result<bool, String> {
    let params_val: serde_json::Value =
        serde_json::from_str(params).map_err(|e| format!("invalid params JSON: {e}"))?;
    let schema_val: serde_json::Value =
        serde_json::from_str(schema).map_err(|e| format!("invalid schema: {e}"))?;

    if let Some(required) = schema_val.get("required").and_then(|r| r.as_array()) {
        for field in required {
            let field_name = field.as_str().ok_or("invalid schema: required field")?;
            if params_val.get(field_name).is_none() {
                return Ok(false);
            }
        }
    }

    if let Some(properties) = schema_val.get("properties").and_then(|p| p.as_object()) {
        for (field_name, field_schema) in properties {
            if let Some(param_val) = params_val.get(field_name) {
                if let Some(expected_type) = field_schema.get("type").and_then(|t| t.as_str()) {
                    let matches = match expected_type {
                        "string" => param_val.is_string(),
                        "number" | "integer" => param_val.is_number(),
                        "boolean" => param_val.is_boolean(),
                        "array" => param_val.is_array(),
                        "object" => param_val.is_object(),
                        _ => true,
                    };
                    if !matches {
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Check if resource access is scoped to the correct environment
pub fn environment_scope(token_env: &str, resource_env: &str) -> bool {
    match token_env {
        "production" => resource_env == "production",
        "staging" => resource_env != "production",
        "development" => resource_env != "production" && resource_env != "staging",
        _ => true,
    }
}

/// Validate a delegation chain depth
pub fn delegation_chain_valid(chain_depth: usize, max_allowed: usize) -> bool {
    chain_depth <= max_allowed
}

/// Check if current time is within allowed window
pub fn in_business_hours() -> bool {
    let now = chrono::Utc::now();
    let hour = now.hour();
    let weekday = now.weekday();
    let is_weekday = weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun;
    is_weekday && (9..=17).contains(&hour)
}

/// Check if a model provider is in the allowed list
pub fn model_allowed(model: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|a| model.starts_with(a))
}

/// Check if a URL matches any allowed pattern
pub fn url_allowed(url: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|a| url.contains(a))
}

/// Check if a tool is in the allowed list (supports glob patterns)
pub fn tool_allowed(tool: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|a| {
        if a == tool {
            return true;
        }
        if a.ends_with('*') {
            let prefix = a.trim_end_matches('*');
            tool.starts_with(prefix)
        } else {
            false
        }
    })
}
