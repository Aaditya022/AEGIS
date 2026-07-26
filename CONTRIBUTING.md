# Contributing to AEGIS

## Development Setup

1. **Prerequisites**: Rust 1.79+, Go 1.22+, Docker, protoc
2. **Clone**: `git clone https://github.com/aegis-ai/aegis.git && cd aegis`
3. **Bootstrap**: `make dev-env && make build && make test`

## Code Standards

- **Rust**: Follow Rust 2021 idioms. Use `cargo clippy` and `cargo fmt`.
- **Go**: Follow standard Go conventions. Use `gofmt` and `go vet`.
- **Protobuf**: Version all APIs with `/v1/` prefixes. Backward compatibility required.
- **Rego**: All policies under `policies/` must include package and test files.
- **Kubernetes**: All manifests must pass `helm lint` and `kubeconform`.

## Pull Request Process

1. Create an issue describing the change.
2. Fork the repo and create a feature branch.
3. All changes must include tests.
4. Run `make ci` locally before submitting.
5. PRs require at least one review from a maintainer.

## Commit Messages

```
component: brief description

- Detailed bullet points
- Include motivation for non-obvious changes

Fixes #123
```

Components: `sidecar`, `gateway`, `control-plane`, `policy`, `audit`, `cli`, `ebpf`, `k8s`, `docs`

## Testing

```bash
cargo test --workspace                    # All Rust tests
cd aegis-gateway && go test ./...        # Go tests
cargo bench --workspace                   # Benchmarks
```

## Security

Report vulnerabilities to security@aegis.ai. Do not file public issues.
