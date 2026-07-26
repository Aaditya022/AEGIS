use thiserror::Error;

#[derive(Error, Debug)]
pub enum AegisError {
    #[error("policy evaluation failed: {0}")]
    PolicyEvaluation(String),

    #[error("identity verification failed: {0}")]
    IdentityVerification(String),

    #[error("audit log error: {0}")]
    AuditLog(String),

    #[error("configuration error: {0}")]
    Configuration(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("budget exceeded: spent {spent:.2}, limit {limit:.2}")]
    BudgetExceeded { spent: f64, limit: f64 },

    #[error("recursion limit reached: depth {depth}, limit {limit}")]
    RecursionLimit { depth: u32, limit: u32 },

    #[error("unauthorized tool: {tool}")]
    UnauthorizedTool { tool: String },

    #[error("unauthorized url: {url}")]
    UnauthorizedUrl { url: String },

    #[error("kafka error: {0}")]
    Kafka(String),

    #[error("etcd error: {0}")]
    Etcd(String),

    #[error("eBPF error: {0}")]
    Ebpf(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AegisError>;
