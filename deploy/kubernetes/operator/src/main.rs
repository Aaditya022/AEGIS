use std::sync::Arc;

use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Patch, PatchParams};
use kube::runtime::controller::{Action, Controller};
use kube::runtime::finalizer::{finalizer, Event};
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, Client, ResourceExt};
use serde::{Deserialize, Serialize};
use tracing::*;

mod webhook;
mod crds;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AgentPolicySpec {
    pub agent_selector: Option<AgentSelector>,
    pub policies: Vec<PolicyRule>,
    pub compliance_framework: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AgentSelector {
    pub match_labels: Option<std::collections::BTreeMap<String, String>>,
    pub match_frameworks: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PolicyRule {
    pub category: String,
    pub severity: String,
    pub condition: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AgentRegistrationSpec {
    pub agent_id: String,
    pub framework: String,
    pub identity_type: String,
    pub allowed_tools: Vec<String>,
    pub allowed_models: Vec<String>,
    pub max_recursion_depth: u32,
    pub budget_limit_usd: f64,
    pub human_approval_required: bool,
    pub autonomy_level: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum AgentPhase {
    Pending,
    Active,
    Paused,
    Terminated,
    Compromised,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aegis_operator=info,kube=info".into()),
        )
        .json()
        .init();

    let client = Client::try_default().await?;
    info!("AEGIS Operator connected to Kubernetes");

    // Start admission webhook server
    let webhook_port: u16 = std::env::var("AEGIS_WEBHOOK_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8443);
    tokio::spawn(async move {
        webhook::start_webhook(webhook_port).await;
    });

    let pods = Api::<Pod>::all(client.clone());

    // Watch pods with aegis annotation
    let controller = Controller::new(pods, watcher::Config::default().any_semantic())
        .owns(
            Api::<AgentPolicyCrd>::all(client.clone()),
            watcher::Config::default(),
        )
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(client))
        .for_each(|result| async move {
            match result {
                Ok(o) => info!("Reconciled: {:?}", o),
                Err(e) => error!("Reconcile error: {:?}", e),
            }
        })
        .await;

    Ok(())
}

async fn reconcile(pod: Arc<Pod>, ctx: Arc<Client>) -> Result<Action, anyhow::Error> {
    let name = pod.name_any();
    let ns = pod.namespace().unwrap_or_default();

    // Check if this pod has aegis injection annotation
    let should_inject = pod
        .metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get("aegis.ai/inject"))
        .map(|v| v == "enabled" || v == "true")
        .unwrap_or(false);

    if !should_inject {
        return Ok(Action::await_change());
    }

    // Check if already injected
    let already_injected = pod
        .metadata
        .annotations
        .as_ref()
        .and_then(|a| a.get("aegis.ai/injected"))
        .map(|v| v == "true")
        .unwrap_or(false);

    if already_injected {
        return Ok(Action::await_change());
    }

    info!(pod = %name, ns = %ns, "Injecting AEGIS sidecar");

    // Patch pod with sidecar container + init container
    let patch = serde_json::json!({
        "metadata": {
            "annotations": {
                "aegis.ai/injected": "true",
                "aegis.ai/injected-at": chrono::Utc::now().to_rfc3339(),
            }
        },
        "spec": {
            "initContainers": [{
                "name": "aegis-init-iptables",
                "image": "ghcr.io/aegis-ai/sidecar:latest",
                "command": ["sh", "-c", concat!(
                    "iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-port 9000; ",
                    "iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-port 9000"
                )],
                "securityContext": {
                    "capabilities": { "add": ["NET_ADMIN"] },
                    "runAsNonRoot": false,
                    "runAsUser": 0,
                },
                "imagePullPolicy": "IfNotPresent",
            }],
            "containers": [{
                "name": "aegis-sidecar",
                "image": "ghcr.io/aegis-ai/sidecar:latest",
                "args": ["--config", "/etc/aegis/sidecar.yaml"],
                "env": [
                    {
                        "name": "POD_NAME",
                        "valueFrom": { "fieldRef": { "fieldPath": "metadata.name" } }
                    },
                    {
                        "name": "AEGIS_AGENT_ID",
                        "valueFrom": { "fieldRef": { "fieldPath": "metadata.labels['aegis.ai/agent-id']" } }
                    },
                    {
                        "name": "AEGIS_ENV",
                        "valueFrom": { "fieldRef": { "fieldPath": "metadata.namespace" } }
                    }
                ],
                "ports": [
                    { "containerPort": 9000, "name": "proxy" },
                    { "containerPort": 9001, "name": "grpc" },
                    { "containerPort": 9090, "name": "metrics" }
                ],
                "volumeMounts": [
                    { "name": "aegis-config", "mountPath": "/etc/aegis" },
                    { "name": "aegis-policies", "mountPath": "/etc/aegis/policies" }
                ],
                "resources": {
                    "requests": { "cpu": "100m", "memory": "64Mi" },
                    "limits": { "cpu": "500m", "memory": "256Mi" }
                },
                "livenessProbe": {
                    "httpGet": { "path": "/health", "port": 9090 },
                    "initialDelaySeconds": 5,
                    "periodSeconds": 10
                },
                "readinessProbe": {
                    "httpGet": { "path": "/ready", "port": 9090 },
                    "initialDelaySeconds": 3,
                    "periodSeconds": 5
                }
            }],
            "volumes": [
                {
                    "name": "aegis-config",
                    "configMap": { "name": "aegis-sidecar-config" }
                },
                {
                    "name": "aegis-policies",
                    "configMap": { "name": "aegis-policies" }
                }
            ]
        }
    });

    let api: Api<Pod> = Api::namespaced(ctx.as_ref(), &ns);
    let params = PatchParams::apply("aegis-operator").force();
    api.patch(&name, &params, &Patch::Apply(patch)).await?;

    info!(pod = %name, "Sidecar injected");
    Ok(Action::await_change())
}

fn error_policy(error: &anyhow::Error, _ctx: Arc<Client>) -> Action {
    error!(%error, "Reconcile error, retrying");
    Action::requeue(std::time::Duration::from_secs(10))
}

// CRD wrapper types
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AgentPolicyCrd {
    pub spec: AgentPolicySpec,
}
