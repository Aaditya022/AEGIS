use crate::Policy;

/// Parse a Rego policy source string into a Policy struct.
/// Extracts metadata from Rego comments using directives:
///   # @id="policy-id"
///   # @name="Policy Name"
///   # @category="recursion"
///   # @severity="block"
///   # @max=10
///   # @allowed="tool1,tool2"
///   # @threshold=0.8
///   # @description="Description"
pub fn parse_rego(source: &str) -> Result<Policy, String> {
    let lines: Vec<&str> = source.lines().collect();

    let mut id = String::new();
    let mut name = String::new();
    let mut category = String::new();
    let mut severity = String::from("block");
    let mut description = None;
    let mut enabled = true;
    let mut version = String::from("1.0.0");

    // Extract package name as fallback ID
    let package = extract_rego_package(source).unwrap_or("unknown");

    for line in &lines {
        let line = line.trim();

        // Skip empty lines and code
        if line.is_empty() || (!line.starts_with('#')) {
            continue;
        }

        // Parse directive comments
        let directive = line.trim_start_matches('#').trim();
        if let Some(val) = directive.strip_prefix("@id=") {
            id = val.trim_matches('"').trim().to_string();
        } else if let Some(val) = directive.strip_prefix("@name=") {
            name = val.trim_matches('"').trim().to_string();
        } else if let Some(val) = directive.strip_prefix("@category=") {
            category = val.trim_matches('"').trim().to_string();
        } else if let Some(val) = directive.strip_prefix("@severity=") {
            severity = val.trim_matches('"').trim().to_string();
        } else if let Some(val) = directive.strip_prefix("@description=") {
            description = Some(val.trim_matches('"').trim().to_string());
        } else if let Some(val) = directive.strip_prefix("@version=") {
            version = val.trim_matches('"').trim().to_string();
        } else if directive == "@disabled" {
            enabled = false;
        }
    }

    if id.is_empty() {
        id = format!("{package}-policy");
    }
    if name.is_empty() {
        name = package.to_string();
    }
    if category.is_empty() {
        // Try to infer from package name
        if source.contains("recursion") {
            category = crate::POLICY_RECURSION.into();
        } else if source.contains("budget") {
            category = crate::POLICY_BUDGET.into();
        } else if source.contains("tool") {
            category = crate::POLICY_ALLOWED_TOOLS.into();
        } else if source.contains("model") {
            category = crate::POLICY_ALLOWED_MODELS.into();
        } else if source.contains("environment") {
            category = crate::POLICY_ENVIRONMENT.into();
        } else if source.contains("human") {
            category = crate::POLICY_HUMAN_APPROVAL.into();
        } else if source.contains("prompt") || source.contains("risk") {
            category = crate::POLICY_PROMPT_RISK.into();
        } else {
            category = "custom".into();
        }
    }

    Ok(Policy {
        id,
        name,
        category,
        severity,
        rego_source: source.to_string(),
        wasm_binary: None,
        enabled,
        version,
        description,
    })
}

/// Extract the package name from a Rego source
pub fn extract_rego_package(source: &str) -> Option<&str> {
    for line in source.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("package ") {
            return Some(name.trim());
        }
    }
    None
}

/// Extract rule names from a Rego source
pub fn extract_rules(source: &str) -> Vec<String> {
    let mut rules = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        // Match lines like: "allow { ... }" or "deny[reason] { ... }"
        if line.starts_with("allow") || line.starts_with("deny") || line.starts_with("escalate") {
            if let Some(name) = line.split([' ', '[', '{']).next() {
                rules.push(name.to_string());
            }
        }
    }
    rules
}

/// Merge multiple Rego sources into a single source
pub fn merge_rego(sources: &[&str]) -> String {
    let mut merged = String::new();
    for source in sources {
        merged.push_str(source);
        merged.push('\n');
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rego_with_directives() {
        let source = r#"# @id="recursion-limit-001"
# @name="Maximum Recursion Depth"
# @category="recursion"
# @severity="block"
# @max=10
# @description="Prevents infinite loops by limiting recursion depth"

package aegis.recursion

default allow = true

deny[reason] {
    input.recursion_depth > 10
    reason := sprintf("recursion limit exceeded: depth %d", [input.recursion_depth])
}
"#;

        let policy = parse_rego(source).unwrap();
        assert_eq!(policy.id, "recursion-limit-001");
        assert_eq!(policy.name, "Maximum Recursion Depth");
        assert_eq!(policy.category, "recursion");
        assert_eq!(policy.severity, "block");
        assert!(policy.enabled);
    }

    #[test]
    fn test_parse_rego_without_directives() {
        let source = r#"package aegis.budget

default allow = true

deny[reason] {
    input.budget_consumed > input.budget_limit
    reason := "budget exceeded"
}
"#;
        let policy = parse_rego(source).unwrap();
        assert_eq!(policy.category, "budget");
        assert!(policy.enabled);
    }

    #[test]
    fn test_extract_package() {
        let source = r#"package aegis.recursion"#;
        assert_eq!(extract_rego_package(source), Some("aegis.recursion"));
    }

    #[test]
    fn test_extract_rules() {
        let source = r#"default allow = false
allow { input.valid == true }
deny[reason] { not allow }
"#;
        let rules = extract_rules(source);
        assert!(rules.contains(&"allow".to_string()));
        assert!(rules.contains(&"deny".to_string()));
    }

    #[test]
    fn test_disabled_policy() {
        let source = r#"# @id="test"
# @category="budget"
# @disabled

package aegis.test
"#;
        let policy = parse_rego(source).unwrap();
        assert!(!policy.enabled);
    }
}
