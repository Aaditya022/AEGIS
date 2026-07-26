use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AegisEventType {
    Open,
    Connect,
    Execve,
    Unlink,
    Write,
    Rename,
    Send,
    Recv,
    CredAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AegisEvent {
    pub event_type: AegisEventType,
    pub pid: u32,
    pub uid: u32,
    pub comm: String,
    pub timestamp_ns: u64,
    pub details: EventDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventDetails {
    File {
        filename: String,
        flags: i32,
        is_sensitive: bool,
    },
    Network {
        saddr: String,
        daddr: String,
        sport: u16,
        dport: u16,
        bytes: u64,
    },
    Credential {
        filename: String,
        access_type: u8,
    },
}

impl AegisEvent {
    pub fn is_blocked(&self) -> bool {
        match &self.details {
            EventDetails::File { is_sensitive, .. } => *is_sensitive,
            EventDetails::Network { .. } => false,
            EventDetails::Credential { .. } => true,
        }
    }

    pub fn description(&self) -> String {
        match &self.details {
            EventDetails::File { filename, is_sensitive, .. } => {
                format!(
                    "{}{}",
                    if *is_sensitive { "SENSITIVE " } else { "" },
                    filename
                )
            }
            EventDetails::Network { daddr, dport, .. } => {
                format!("{}:{}", daddr, dport)
            }
            EventDetails::Credential { filename, .. } => {
                format!("CREDENTIAL: {}", filename)
            }
        }
    }
}

impl std::fmt::Display for AegisEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] pid={} comm={} {}",
            self.event_type, self.pid, self.comm, self.description()
        )
    }
}

impl std::fmt::Display for AegisEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AegisEventType::Open => write!(f, "OPEN"),
            AegisEventType::Connect => write!(f, "CONNECT"),
            AegisEventType::Execve => write!(f, "EXECVE"),
            AegisEventType::Unlink => write!(f, "UNLINK"),
            AegisEventType::Write => write!(f, "WRITE"),
            AegisEventType::Rename => write!(f, "RENAME"),
            AegisEventType::Send => write!(f, "SEND"),
            AegisEventType::Recv => write!(f, "RECV"),
            AegisEventType::CredAccess => write!(f, "CRED_ACCESS"),
        }
    }
}
