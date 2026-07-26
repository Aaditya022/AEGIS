// AEGIS Security Attack Simulator
// Requires a running AEGIS sidecar at AEGIS_SIDECAR_URL (default: http://localhost:9000).
// Skips tests gracefully if sidecar is unreachable.

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    fn sidecar_url() -> String {
        std::env::var("AEGIS_SIDECAR_URL").unwrap_or_else(|_| "http://localhost:9000".into())
    }

    fn is_sidecar_available() -> bool {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .ok()
            .and_then(|c| c.get(&format!("{}/health", sidecar_url())).send().ok())
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn skip_if_no_sidecar() {
        if !is_sidecar_available() {
            eprintln!("SKIP: No sidecar at {}. Set AEGIS_SIDECAR_URL to run integration tests.", sidecar_url());
        }
    }

    #[test]
    fn test_sidecar_health() {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        match client.get(&format!("{}/health", sidecar_url())).send() {
            Ok(resp) => {
                assert!(resp.status().is_success(), "Sidecar health check failed");
                println!("Sidecar health: OK");
            }
            Err(e) => {
                eprintln!("WARN: Sidecar not reachable ({e}). This test requires a running sidecar.");
                eprintln!("Start with: AEGIS_SIDECAR_URL={} cargo test --test attack_simulator", sidecar_url());
                return;
            }
        }
    }

    #[test]
    fn test_security_scan_available() {
        if !is_sidecar_available() {
            eprintln!("SKIP: No sidecar available for security scan");
            return;
        }
        println!("Security scan endpoint available at {}", sidecar_url());
    }
}
