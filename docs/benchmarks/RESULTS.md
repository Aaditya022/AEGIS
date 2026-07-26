# AEGIS Benchmark Results

> **NOTE**: These results are placeholders. Run `./scripts/bench.sh` or `cargo test --test benchmarks -- --nocapture` to generate real measurements.

## Methodology

All benchmarks follow the measurement framework described in the AEGIS paper (Section 9.1).

### Tools Used
- **Latency**: wrk2 (HTTP load generator)
- **Throughput**: k6 (load testing tool)
- **Policy Evaluation**: Criterion.rs (Rust microbenchmarking)
- **Memory**: ps/pmap (RSS measurement)
- **Cold Start**: time command + health check polling

## Results

| Metric | Result | Target | Status |
|--------|--------|--------|--------|
| Sidecar P50 Latency | — | <1ms | ⏳ |
| Sidecar P95 Latency | — | <3ms | ⏳ |
| Sidecar P99 Latency | — | <5ms | ⏳ |
| Gateway Throughput | — | 10K req/s | ⏳ |
| Policy Eval P50 | — | <0.5ms | ⏳ |
| Policy Eval P99 | — | <1ms | ⏳ |
| Sidecar Memory | — | <50MB | ⏳ |
| Cold Start | — | <2s | ⏳ |
| eBPF Overhead | — | <10% | ⏳ |

## Running Benchmarks

```bash
# Full benchmark suite
./scripts/bench.sh

# Rust microbenchmarks
cargo bench --workspace

# k6 load test
k6 run tests/benchmarks/k6-gateway.js

# wrk2 latency
wrk2 -t2 -c10 -d30s -R1000 -s tests/benchmarks/wrk2-sidecar.lua http://localhost:9000/

# eBPF overhead
sudo ./scripts/bench-ebpf.sh
```

## Comparison Framework

When AEGIS reaches production, compare against:

| System | Type | Expected P99 | Expected Throughput |
|--------|------|-------------|-------------------|
| AEGIS Sidecar | Governance proxy | <5ms | 10K req/s |
| Envoy | Service proxy | <1ms | 50K req/s |
| Istio Proxy | Service mesh | <3ms | 20K req/s |
| OPA | Policy engine | <0.5ms/policy | — |
| OpenAI SDK | Agent framework | — | — |
| AWS Bedrock AgentCore | Managed governance | — | — |
| Google Model Armor | AI safety | — | — |
