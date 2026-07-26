use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aegis", about = "AEGIS CLI for AI Agent Governance", version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long, default_value = "http://localhost:8500")]
    control_plane: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Agent management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Policy management
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    /// Deploy AEGIS components
    Deploy {
        #[command(subcommand)]
        target: DeployTarget,
    },
    /// Inspect system state
    Inspect,
    /// Check system health
    Status,
}

#[derive(Subcommand)]
enum AgentAction {
    /// Register a new agent
    Register { id: String, framework: String },
    /// List all agents
    List,
    /// Get agent details
    Get { id: String },
    /// Update agent status
    Status { id: String, status: String },
}

#[derive(Subcommand)]
enum PolicyAction {
    /// Push a new policy
    Push { file: String },
    /// List all policies
    List,
    /// Get policy details
    Get { id: String },
    /// Validate a policy file
    Validate { file: String },
}

#[derive(Subcommand)]
enum DeployTarget {
    /// Deploy sidecar
    Sidecar,
    /// Deploy gateway
    Gateway,
    /// Deploy control plane
    ControlPlane,
    /// Deploy full stack
    All,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Agent { action } => handle_agent(action, &cli.control_plane).await?,
        Commands::Policy { action } => handle_policy(action, &cli.control_plane).await?,
        Commands::Deploy { target } => handle_deploy(target).await?,
        Commands::Inspect => handle_inspect(&cli.control_plane).await?,
        Commands::Status => handle_status(&cli.control_plane).await?,
    }
    Ok(())
}

async fn handle_agent(action: AgentAction, cp: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    match action {
        AgentAction::Register { id, framework } => {
            let resp = client
                .post(format!("{cp}/v1/agents"))
                .json(&serde_json::json!({"agent_id": id, "framework": framework}))
                .send()
                .await?;
            println!("Agent registered: {}", resp.status());
        }
        AgentAction::List => {
            let resp = client.get(format!("{cp}/v1/agents")).send().await?;
            let agents: serde_json::Value = resp.json().await?;
            println!("{}", serde_json::to_string_pretty(&agents)?);
        }
        AgentAction::Get { id } => {
            let resp = client.get(format!("{cp}/v1/agents/{id}")).send().await?;
            let agent: serde_json::Value = resp.json().await?;
            println!("{}", serde_json::to_string_pretty(&agent)?);
        }
        AgentAction::Status { id, status } => {
            let resp = client
                .patch(format!("{cp}/v1/agents/{id}/status"))
                .json(&serde_json::json!({"status": status}))
                .send()
                .await?;
            println!("Status updated: {}", resp.status());
        }
    }
    Ok(())
}

async fn handle_policy(action: PolicyAction, cp: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    match action {
        PolicyAction::Push { file } => {
            let content = tokio::fs::read_to_string(&file).await?;
            let resp = client
                .post(format!("{cp}/v1/policies"))
                .body(content)
                .header("content-type", "application/x-yaml")
                .send()
                .await?;
            println!("Policy pushed: {}", resp.status());
        }
        PolicyAction::List => {
            let resp = client.get(format!("{cp}/v1/policies")).send().await?;
            let policies: serde_json::Value = resp.json().await?;
            println!("{}", serde_json::to_string_pretty(&policies)?);
        }
        PolicyAction::Get { id } => {
            let resp = client.get(format!("{cp}/v1/policies/{id}")).send().await?;
            let policy: serde_json::Value = resp.json().await?;
            println!("{}", serde_json::to_string_pretty(&policy)?);
        }
        PolicyAction::Validate { file } => {
            let content = tokio::fs::read_to_string(&file).await?;
            let policy = aegis_policy_engine::parser::parse_rego(&content);
            match policy {
                Ok(p) => println!("Policy valid: {} ({})", p.name, p.id),
                Err(e) => println!("Policy invalid: {e}"),
            }
        }
    }
    Ok(())
}

async fn handle_deploy(_target: DeployTarget) -> anyhow::Result<()> {
    println!("Deploy command — requires Kubernetes context");
    println!("Use: helm install aegis ./deploy/kubernetes/charts/aegis");
    Ok(())
}

async fn handle_inspect(cp: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{cp}/v1/inspect")).send().await;
    match resp {
        Ok(r) => {
            let state: serde_json::Value = r.json().await?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Err(e) => println!("Cannot reach control plane: {e}"),
    }
    Ok(())
}

async fn handle_status(cp: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{cp}/v1/health")).send().await;
    match resp {
        Ok(r) => {
            let health: serde_json::Value = r.json().await?;
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
        Err(e) => println!("Control plane unhealthy: {e}"),
    }
    Ok(())
}
