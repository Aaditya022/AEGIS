use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarConfig {
    pub sidecar_id: String,
    pub agent_id: String,
    pub listen_addr: String,
    pub gateway_addr: String,
    pub control_plane_addr: String,
    pub kafka_brokers: String,
    pub policy_dir: String,
    pub max_recursion_depth: u32,
    pub budget_limit_usd: f64,
    pub allowed_domains: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub otlp_endpoint: String,
    pub enable_ebpf: bool,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub gateway_id: String,
    pub listen_addr: String,
    pub control_plane_addr: String,
    pub redis_addr: String,
    pub otlp_endpoint: String,
    pub log_level: String,
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key_env: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub weight: u32,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPlaneConfig {
    pub listen_addr: String,
    pub etcd_endpoints: Vec<String>,
    pub postgres_dsn: String,
    pub kafka_brokers: String,
    pub redis_addr: String,
    pub otlp_endpoint: String,
    pub log_level: String,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            sidecar_id: uuid::Uuid::new_v4().to_string(),
            agent_id: String::new(),
            listen_addr: "0.0.0.0:9000".into(),
            gateway_addr: "http://localhost:8000".into(),
            control_plane_addr: "http://localhost:8500".into(),
            kafka_brokers: "localhost:9092".into(),
            policy_dir: "/etc/aegis/policies".into(),
            max_recursion_depth: 5,
            budget_limit_usd: 100.0,
            allowed_domains: vec![],
            allowed_tools: vec![],
            otlp_endpoint: "http://localhost:4317".into(),
            enable_ebpf: false,
            log_level: "info".into(),
        }
    }
}
