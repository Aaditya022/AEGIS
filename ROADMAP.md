# AEGIS Roadmap

## Milestone 1 — Project Architecture ✅
Scaffolding, build system, protobufs, CI/CD, dev environment
- [x] Cargo workspace, Go modules
- [x] Protobuf definitions (sidecar, gateway, control-plane)
- [x] Docker compose development environment
- [x] CI pipeline (lint, test, build, security audit)
- [x] Makefile + justfile for dev commands

## Milestone 2 — Rust Sidecar
Transparent proxy, identity verification, policy evaluation, tool gate
- [ ] TLS interception (HTTP/HTTPS/gRPC)
- [ ] MCP/A2A protocol detection
- [ ] Identity verification via SPIFFE
- [ ] OPA policy evaluation engine integration
- [ ] Tool gate with schema validation
- [ ] Cost circuit breaker
- [ ] Recursion detection
- [ ] Audit event producer

## Milestone 3 — OPA Policy Engine
Custom Rego policies with agent-specific builtins
- [ ] WASM compilation pipeline
- [ ] Custom builtins: recursion detection, budget, reasoning patterns
- [ ] Policy hot-reload from etcd
- [ ] Policy test framework

## Milestone 4 — Agent Gateway
Multi-provider routing, rate limiting, protocol normalization
- [ ] Provider router (OpenAI, Anthropic, Gemini, Mistral, Ollama)
- [ ] Distributed rate limiter (Redis)
- [ ] MCP/A2A/ACP protocol adapter
- [ ] Circuit breaker per provider
- [ ] Cost tracking per route

## Milestone 5 — eBPF Runtime Monitoring
Kernel-level infrastructure-plane verification
- [ ] eBPF syscall monitor (read/write/connect/openat)
- [ ] eBPF TCP monitor
- [ ] eBPF file access monitor
- [ ] Rust userspace loader (aya)
- [ ] Two-Plane divergence detection
- [ ] Alerting on policy bypass attempts

## Milestone 6 — Kubernetes Integration
Operator, admission controller, CRDs, Helm, HA
- [ ] Mutating admission webhook for auto-injection
- [ ] AgentPolicy CRD
- [ ] AgentRegistration CRD
- [ ] Kubernetes operator (kube-rs)
- [ ] Helm chart with production defaults
- [ ] HA deployment (3 control-plane replicas)

## Milestone 7 — Benchmark Suite
Quantitative performance measurement framework
- [ ] Sidecar latency (P50, P95, P99) via wrk2
- [ ] Gateway throughput (req/s) via k6
- [ ] Policy evaluation latency via Criterion
- [ ] Memory profiling
- [ ] Cold start measurement
- [ ] Comparison against Envoy, Istio, OPA

## Milestone 8 — Security Evaluation
Red-team attack simulation and detection metrics
- [ ] Prompt injection (direct + indirect)
- [ ] Tool hijacking
- [ ] Credential theft
- [ ] Privilege escalation
- [ ] Policy bypass
- [ ] Recursive agent loops
- [ ] Detection rate, FP/FN rate, MTTD, MTTR

## Milestone 9 — Paper Rewrite
Replace theoretical claims with empirical evidence
- [ ] Architecture diagrams (PlantUML)
- [ ] Sequence diagrams
- [ ] Threat model
- [ ] Evaluation section with real numbers
- [ ] Experimental methodology
- [ ] Reproducibility appendix
- [ ] Limitations and future work

## Milestone 10 — Springer Submission Package
Final artifacts for Springer/IEEE submission
- [ ] Complete paper with empirical sections
- [ ] Artifact appendix (Docker, K8s, scripts)
- [ ] Supplementary material (raw data, configs)
- [ ] Open-source release (CNCF sandbox ready)
- [ ] Website with documentation
- [ ] SDK examples for all frameworks
