# AEGIS — Universal Governance Primitive for Autonomous AI Systems

[![CI](https://github.com/aegis-ai/aegis/actions/workflows/ci.yaml/badge.svg)](https://github.com/aegis-ai/aegis/actions/workflows/ci.yaml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.79+-orange)](https://www.rust-lang.org/)
[![Go](https://img.shields.io/badge/Go-1.22+-blue)](https://go.dev/)

AEGIS is a lightweight, embeddable, framework-agnostic governance layer that sits between **any AI agent** and **any resource**. Think of it as Envoy/Istio for AI Agents.

## Why AEGIS?

> _"In April 2026, an AI coding agent deleted the entire production database of a SaaS platform in approximately nine seconds."_

The rapid proliferation of autonomous AI agents has created a critical **governance gap**. Current deployments lack identity verification, policy enforcement, audit trails, and human oversight — exactly the problems service meshes solved for microservices a decade ago.

## Architecture

```
Agent (LangGraph/CrewAI/AutoGen)
        │
        ▼
┌───────────────────┐
│  Governance        │
│  Sidecar (Rust)    │  ◄── Identity, Policy, Cost, Audit
│         │          │
│   ┌─────▼─────┐   │
│   │  eBPF     │   │  ◄── Non-bypassable infra-plane checks
│   │  Probes   │   │
│   └───────────┘   │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│  Agent Gateway    │  ◄── Multi-provider routing, rate limiting
│  (Go)             │
└────────┬──────────┘
         │
    ┌────┴────┐
    ▼         ▼
 LLM APIs    Tools/Services
 (OpenAI,    (Databases, APIs,
  Anthropic)  Internal Services)
```

## Quick Start

```bash
# Prerequisites: Rust, Go, Docker, protoc
make dev-env        # Start dependencies (etcd, Kafka, Postgres, Redis)
make build          # Build all Rust crates
make test           # Run tests

# Deploy to Kubernetes
kind create cluster --name aegis
helm install aegis deploy/kubernetes/charts/aegis
```

## Supported Frameworks

| Framework        | Integration Status |
|-----------------|-------------------|
| LangGraph       | ✅ Supported      |
| CrewAI          | ✅ Supported      |
| OpenAI Agents SDK | ✅ Supported    |
| AutoGen         | ✅ Supported      |
| MCP             | ✅ Supported      |
| Google ADK      | 🔄 In Progress    |

## Supported Providers

| Provider        | Status            |
|----------------|-------------------|
| OpenAI         | ✅ Supported      |
| Anthropic      | ✅ Supported      |
| Google Gemini  | ✅ Supported      |
| Mistral        | ✅ Supported      |
| Ollama         | ✅ Supported      |
| OpenRouter     | ✅ Supported      |

## Core Capabilities

| Capability               | Implementation                    |
|-------------------------|-----------------------------------|
| Identity Verification   | SPIFFE-compatible X.509 SVIDs     |
| Policy Enforcement      | OPA/Rego via WASM sandbox         |
| Audit Trail             | Append-only, tamper-evident log   |
| Tool Access Control     | Schema-validated tool gates       |
| Budget Circuit Breaker  | Micro-cent precision tracking     |
| Recursion Protection    | Operation fingerprinting          |
| Multi-Provider Routing  | Weighted round-robin + failover   |
| Environment Scoping     | Credential-resource matching      |
| Two-Plane Verification  | App-plane + eBPF infra-plane      |
| Human Escalation        | Graduated intervention            |

## Compliance

AEGIS maps regulatory requirements to infrastructure capabilities:

| Framework       | Coverage |
|----------------|----------|
| **EU AI Act**  | Articles 9, 10, 11, 12, 14, 15, 26, 72 |
| **NIST**       | OAuth 2.1, OIDC, SPIFFE, NGAC, SCIM |
| **Singapore IMDA** | 5-tier autonomy classification |

## Project Structure

```
aegis/
├── aegis-sidecar/         # Rust sidecar proxy (core)
├── aegis-policy-engine/   # OPA/WASM policy engine
├── aegis-audit-log/       # Immutable audit log service
├── aegis-gateway/         # Go-based gateway
├── aegis-control-plane/   # Go-based control plane
├── aegis-cli/             # CLI tool
├── aegis-common/          # Shared types, crypto, protobuf
├── aegis-ebpf/            # eBPF probes (kernel + userspace)
├── deploy/                # K8s, Docker, Helm
├── policies/              # OPA/Rego policy library
├── integrations/          # Framework-specific adapters
└── tests/                 # E2E, security, compliance tests
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the complete development plan across 10 milestones.

## License

Apache 2.0 — see [LICENSE](LICENSE)

## Citation

```bibtex
@article{aggarwal2026aegis,
  title={AEGIS: A Universal Governance Primitive for Autonomous AI Systems},
  author={Aggarwal, Aaditya},
  year={2026}
}
```
