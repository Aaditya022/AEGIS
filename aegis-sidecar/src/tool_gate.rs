use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::debug;

pub struct ToolGate {
    allowed: Arc<RwLock<HashSet<String>>>,
    schemas: Arc<RwLock<std::collections::HashMap<String, serde_json::Value>>>,
}

impl ToolGate {
    pub fn new(allowed: Vec<String>) -> Self {
        Self {
            allowed: Arc::new(RwLock::new(allowed.into_iter().collect())),
            schemas: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn is_allowed(&self, tool: &str) -> bool {
        let allowed = self.allowed.read().await;
        if allowed.is_empty() {
            return true;
        }

        // Support glob-like patterns: "search_*", "database_*"
        for pattern in allowed.iter() {
            if pattern == tool {
                return true;
            }
            if pattern.ends_with('*') {
                let prefix = pattern.trim_end_matches('*');
                if tool.starts_with(prefix) {
                    return true;
                }
            }
        }
        false
    }

    pub async fn validate_tool_params(&self, tool: &str, params: &str) -> Result<bool, String> {
        let schemas = self.schemas.read().await;
        let schema = match schemas.get(tool) {
            Some(s) => s,
            None => return Ok(true), // No schema = pass
        };

        let params_val: serde_json::Value =
            serde_json::from_str(params).map_err(|e| format!("invalid params JSON: {e}"))?;

        // Validate required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for field in required {
                let field_name = field
                    .as_str()
                    .ok_or("invalid schema: required field not string")?;
                if params_val.get(field_name).is_none() {
                    return Ok(false);
                }
            }
        }

        // Validate property types
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
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

    pub async fn update_allowed(&self, tools: Vec<String>) {
        let mut allowed = self.allowed.write().await;
        *allowed = tools.into_iter().collect();
        debug!(count = allowed.len(), "Allowed tools updated");
    }

    pub async fn register_schema(&self, tool: &str, schema: serde_json::Value) {
        let mut schemas = self.schemas.write().await;
        schemas.insert(tool.to_string(), schema);
    }
}
