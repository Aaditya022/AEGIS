# AEGIS — Springer LNCS Submission Package

## Contents

| File | Description |
|------|-------------|
| `aegis-paper.pdf` | Compiled manuscript |
| `aegis-paper.tex` | LaTeX source (Springer LNCS format) |
| `llncs.cls` | Springer LNCS document class |
| `splncs04.bst` | Springer BibTeX style |
| `figs/` | Figure files (EPS/PDF) |
| `LICENSE` | Apache 2.0 license |

## Build Instructions

### Prerequisites
- LaTeX distribution (TeX Live 2021+ or MiKTeX)
- `latexmk` (included in most distributions)

### Build

```bash
make paper        # Build PDF
make zip          # Build PDF + create submission ZIP
make clean        # Remove build artifacts
```

### Manual Build

```bash
pdflatex aegis-paper
bibtex aegis-paper
pdflatex aegis-paper
pdflatex aegis-paper
```

## Artifact Appendix

The full AEGIS implementation and evaluation infrastructure is available at:

**GitHub**: https://github.com/aegis-ai/aegis

### Repository Structure

```
aegis/
  aegis-sidecar/         # Rust sidecar proxy (core)
  aegis-policy-engine/   # OPA/WASM policy engine
  aegis-gateway/         # Go-based multi-provider gateway
  aegis-control-plane/   # Centralized configuration (Go)
  aegis-ebpf/            # eBPF programs + Rust loader
  aegis-audit-log/       # Immutable audit log
  aegis-cli/             # CLI tool
  tests/                 # E2E, security, chaos, benchmarks
  deploy/                # K8s, Docker, Helm
  policies/              # OPA/Rego policy library
  proto/                 # Protobuf definitions
  scripts/               # Build, benchmark, security scripts
```

### Reproducing Results

```bash
# Security evaluation
./scripts/security-audit.sh

# Performance benchmarks
./scripts/bench.sh

# Kubernetes deployment
helm install aegis deploy/kubernetes/charts/aegis
```

### Docker Images

Images are available at: `ghcr.io/aegis-ai/aegis-*`

### Requirements

- Rust 1.79+
- Go 1.22+
- Docker + Docker Compose
- Linux kernel 5.19+ (for eBPF features)
- kind or minikube (for K8s deployment)

## License

Apache 2.0 — see LICENSE file.
