# AEGIS Architecture

## Design Principles

### P1: External Governance
Policy enforcement sits outside the agent process. System prompts and internal guardrails are insufficient — agents can bypass their own instructions.

### P2: Framework Agnosticism
Must work with LangGraph, CrewAI, AutoGen, OpenAI SDK, MCP, Google ADK, and future frameworks without code changes.

### P3: Provider Agnosticism
Must work with OpenAI, Anthropic, Google Gemini, Mistral, Cohere, Ollama, OpenRouter, and local LLMs.

### P4: Cloud Agnosticism
Must deploy on AWS, GCP, Azure, on-premises, or edge environments.

### P5: Lightweight
Sidecar overhead <5ms p99 latency and <50MB memory.

### P6: Open Standard
Protocols and APIs must be open, documented, and governed by a neutral body.

## Four-Layer Architecture

1. **Agent Layer**: LangGraph, CrewAI, AutoGen, etc.
2. **Governance Layer** (AEGIS Sidecar): Identity, policy, cost, audit
3. **Gateway Layer**: Multi-provider routing, rate limiting, protocol normalization
4. **Resource Layer**: LLM APIs, tools, databases, internal services

Cross-cutting: Control Plane, eBPF Infra-Plane, Observability

## Two-Plane Verification

```
Operation o
    ├── πA (App-Plane) ── allow ──▶ πI (Infra-Plane eBPF) ── allow ──▶ Permitted
    │                                                          └── deny ──▶ Blocked + Alert
    ├── πA (App-Plane) ── deny ──▶ Blocked
    └── πA compromised ───────────▶ πI still enforces independently
```

## Deployment Models

- **Kubernetes**: DaemonSet sidecar injection via mutating admission webhook
- **VM/Bare Metal**: systemd service with iptables/nftables redirection
- **Serverless**: Lambda layer or Cloudflare Worker binding
- **Edge**: Lightweight binary (<20MB)
- **Container (non-K8s)**: Docker Compose sidecar or Podman quadlet
