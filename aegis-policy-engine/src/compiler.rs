use std::path::{Path, PathBuf};
use std::process::Command;

use tracing::{debug, info, warn};

use crate::Policy;

pub struct RegoCompiler {
    opa_binary: PathBuf,
    #[allow(dead_code)]
    output_dir: PathBuf,
}

impl RegoCompiler {
    pub fn new(opa_binary: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            opa_binary,
            output_dir,
        }
    }

    /// Check if OPA binary is available
    pub fn is_available() -> bool {
        which::which("opa").is_ok()
    }

    /// Get the OPA binary path
    pub fn opa_path() -> Option<PathBuf> {
        which::which("opa").ok()
    }

    /// Compile a Rego source file to WASM
    pub fn compile_to_wasm(&self, rego_source: &str, package: &str) -> anyhow::Result<Vec<u8>> {
        if !self.opa_binary.exists() {
            warn!(
                "OPA binary not found at {:?}, using native evaluation",
                self.opa_binary
            );
            return Err(anyhow::anyhow!("OPA binary not found"));
        }

        let tmp_dir = tempfile::TempDir::new()?;
        let rego_file = tmp_dir.path().join("policy.rego");
        std::fs::write(&rego_file, rego_source)?;

        let output_file = tmp_dir.path().join("policy.wasm");

        let status = Command::new(&self.opa_binary)
            .arg("build")
            .arg("--target=wasm")
            .arg("--entrypoint")
            .arg(format!("{package}/allow"))
            .arg("--output")
            .arg(&output_file)
            .arg(rego_file)
            .output()?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            warn!(stderr = %stderr, "OPA compilation failed");
            return Err(anyhow::anyhow!("OPA compilation failed: {stderr}"));
        }

        let wasm_bytes = std::fs::read(&output_file)?;
        let size = wasm_bytes.len();
        debug!(package, size, "Compiled Rego to WASM");

        Ok(wasm_bytes)
    }

    /// Compile all policies in a directory to WASM
    pub fn compile_directory(&self, rego_dir: &Path) -> anyhow::Result<Vec<Policy>> {
        let mut compiled = Vec::new();

        for entry in std::fs::read_dir(rego_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "rego") {
                let source = std::fs::read_to_string(&path)?;
                let package = extract_package_name(&source).unwrap_or("aegis");

                match self.compile_to_wasm(&source, package) {
                    Ok(wasm) => {
                        let policy = crate::parse_rego(&source)
                            .map_err(|e| anyhow::anyhow!("parse error: {e}"))?;
                        info!(
                            name = %policy.name,
                            file = %path.file_name().unwrap().to_string_lossy(),
                            "Compiled policy to WASM"
                        );
                        compiled.push(Policy {
                            wasm_binary: Some(wasm),
                            ..policy
                        });
                    }
                    Err(e) => {
                        warn!(
                            file = %path.file_name().unwrap().to_string_lossy(),
                            error = %e,
                            "Falling back to native evaluation"
                        );
                        if let Ok(policy) = crate::parse_rego(&source) {
                            compiled.push(policy);
                        }
                    }
                }
            }
        }

        Ok(compiled)
    }

    /// Check Rego syntax without compiling
    pub fn validate_rego(source: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check package declaration
        if !source.contains("package ") {
            errors.push("missing package declaration".into());
        }

        // For each line, check basic Rego syntax
        for (i, line) in source.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for common Rego syntax errors
            if line.contains("=")
                && !line.contains(":=")
                && !line.contains("==")
                && !line.starts_with("default ")
            {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let _left = parts[0].trim();
                    let right = parts[1].trim();
                    if !right.starts_with('{') && !right.starts_with('[') && !right.starts_with('"')
                    {
                        errors.push(format!(
                            "line {}: use := for assignment, = for comparison",
                            i + 1
                        ));
                    }
                }
            }

            // Check unclosed brackets
            let opens = line.matches('{').count();
            let closes = line.matches('}').count();
            if opens != closes && opens > 0 {
                errors.push(format!("line {}: unbalanced braces", i + 1));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn extract_package_name(source: &str) -> Option<&str> {
    for line in source.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("package ") {
            return Some(name.trim());
        }
    }
    None
}
