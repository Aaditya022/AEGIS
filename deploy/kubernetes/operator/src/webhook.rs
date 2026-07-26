use serde::{Deserialize, Serialize};
use warp::Filter;

/// MutatingAdmissionWebhook handler for sidecar injection
/// Listens on /v1/inject and returns JSON patches

#[derive(Serialize, Deserialize)]
struct AdmissionReview {
    kind: String,
    api_version: String,
    request: Option<AdmissionRequest>,
    response: Option<AdmissionResponse>,
}

#[derive(Serialize, Deserialize)]
struct AdmissionRequest {
    uid: String,
    kind: ObjectMeta,
    resource: ObjectMeta,
    namespace: String,
    operation: String,
    object: serde_json::Value,
    old_object: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ObjectMeta {
    group: Option<String>,
    version: String,
    resource: String,
}

#[derive(Serialize, Deserialize)]
struct AdmissionResponse {
    uid: String,
    allowed: bool,
    patch_type: Option<String>,
    patch: Option<String>,
    status: Option<Status>,
}

#[derive(Serialize, Deserialize)]
struct Status {
    code: i32,
    message: String,
}

pub async fn start_webhook(port: u16) {
    let inject = warp::path("v1")
        .and(warp::path("inject"))
        .and(warp::post())
        .and(warp::body::json())
        .map(|body: AdmissionReview| {
            let request = match body.request {
                Some(r) => r,
                None => {
                    return warp::reply::json(&AdmissionReview {
                        kind: "AdmissionReview".into(),
                        api_version: "admission.k8s.io/v1".into(),
                        request: None,
                        response: Some(AdmissionResponse {
                            uid: "".into(),
                            allowed: true,
                            patch_type: None,
                            patch: None,
                            status: Some(Status {
                                code: 400,
                                message: "no request in admission review".into(),
                            }),
                        }),
                    });
                }
            };

            // Build JSON patch for sidecar injection
            let patch = build_injection_patch(&request.object);
            let patch_b64 = base64_encode(&patch);

            warp::reply::json(&AdmissionReview {
                kind: "AdmissionReview".into(),
                api_version: "admission.k8s.io/v1".into(),
                request: None,
                response: Some(AdmissionResponse {
                    uid: request.uid,
                    allowed: true,
                    patch_type: Some("JSONPatch".into()),
                    patch: Some(patch_b64),
                    status: None,
                }),
            })
        });

    warp::serve(inject).run(([0, 0, 0, 0], port)).await;
}

fn build_injection_patch(object: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut patches = Vec::new();

    // Add init container
    patches.push(serde_json::json!({
        "op": "add",
        "path": "/spec/initContainers/-",
        "value": {
            "name": "aegis-init-iptables",
            "image": "ghcr.io/aegis-ai/sidecar:latest",
            "command": ["sh", "-c", "iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-port 9000; iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-port 9000"],
            "securityContext": { "capabilities": { "add": ["NET_ADMIN"] } },
        }
    }));

    // Add sidecar container
    patches.push(serde_json::json!({
        "op": "add",
        "path": "/spec/containers/-",
        "value": {
            "name": "aegis-sidecar",
            "image": "ghcr.io/aegis-ai/sidecar:latest",
            "args": ["--config", "/etc/aegis/sidecar.yaml"],
            "env": [
                { "name": "POD_NAME", "valueFrom": { "fieldRef": { "fieldPath": "metadata.name" } } },
                { "name": "AEGIS_AGENT_ID", "valueFrom": { "fieldRef": { "fieldPath": "metadata.labels['aegis.ai/agent-id']" } } },
                { "name": "AEGIS_ENV", "valueFrom": { "fieldRef": { "fieldPath": "metadata.namespace" } } }
            ],
            "ports": [
                { "containerPort": 9000, "name": "proxy" },
                { "containerPort": 9090, "name": "metrics" }
            ],
            "volumeMounts": [
                { "name": "aegis-config", "mountPath": "/etc/aegis" },
                { "name": "aegis-policies", "mountPath": "/etc/aegis/policies" }
            ],
            "resources": {
                "requests": { "cpu": "100m", "memory": "64Mi" },
                "limits": { "cpu": "500m", "memory": "256Mi" }
            }
        }
    }));

    // Add volumes
    patches.push(serde_json::json!({
        "op": "add",
        "path": "/spec/volumes/-",
        "value": { "name": "aegis-config", "configMap": { "name": "aegis-sidecar-config" } }
    }));
    patches.push(serde_json::json!({
        "op": "add",
        "path": "/spec/volumes/-",
        "value": { "name": "aegis-policies", "configMap": { "name": "aegis-policies" } }
    }));

    // Annotate
    patches.push(serde_json::json!({
        "op": "add",
        "path": "/metadata/annotations/aegis.ai/injected",
        "value": "true"
    }));

    patches
}

fn base64_encode(data: &[serde_json::Value]) -> String {
    let json = serde_json::to_string(data).unwrap_or_default();
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    engine.encode(json.as_bytes())
}
