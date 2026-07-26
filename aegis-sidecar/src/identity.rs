use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aegis_common::types::PolicyContext;
use hyper::HeaderMap;
use tokio::sync::RwLock;
use tracing::{debug, warn};

struct IdentityCacheEntry {
    valid: bool,
    expires_at: Instant,
}

pub struct IdentityVerifier {
    control_plane_addr: String,
    cache: Arc<RwLock<HashMap<String, IdentityCacheEntry>>>,
    client: reqwest::Client,
}

impl IdentityVerifier {
    pub fn new(control_plane_addr: String) -> Self {
        Self {
            control_plane_addr,
            cache: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_else(|e| {
                    warn!(error = %e, "Failed to build HTTP client, using default");
                    reqwest::Client::new()
                }),
        }
    }

    pub async fn verify_identity(&self, ctx: &PolicyContext, headers: &HeaderMap) -> bool {
        let agent_id = &ctx.agent_id;

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(agent_id) {
                if entry.expires_at > Instant::now() {
                    debug!(agent = %agent_id, cached = true, "Identity cache hit");
                    return entry.valid;
                }
            }
        }

        // Verify via control plane
        let token = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        match self.verify_remote(agent_id, token).await {
            Ok(valid) => {
                let mut cache = self.cache.write().await;
                cache.insert(
                    agent_id.clone(),
                    IdentityCacheEntry {
                        valid,
                        expires_at: Instant::now() + Duration::from_secs(300),
                    },
                );
                if !valid {
                    warn!(agent = %agent_id, "Identity rejected by control plane");
                }
                valid
            }
            Err(e) => {
                warn!(error = %e, "Identity verification service unreachable");
                // Fail-closed: deny on verification failure
                false
            }
        }
    }

    async fn verify_remote(&self, agent_id: &str, token: &str) -> Result<bool, String> {
        let url = format!("{}/v1/identity/verify", self.control_plane_addr);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "agent_id": agent_id,
                "token": token,
            }))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(body["valid"].as_bool().unwrap_or(false))
        } else {
            Err(format!("upstream returned {}", resp.status()))
        }
    }

    pub fn verify_local(
        &self,
        agent_id: &str,
        signature: &[u8],
        public_key: &[u8],
    ) -> bool {
        let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(
            public_key.try_into().unwrap_or_default(),
        ) {
            Ok(k) => k,
            Err(_) => return false,
        };
        let sig = match ed25519_dalek::Signature::from_slice(signature) {
            Ok(s) => s,
            Err(_) => return false,
        };
        aegis_common::crypto::verify(&verifying_key, agent_id.as_bytes(), &sig).is_ok()
    }
}
