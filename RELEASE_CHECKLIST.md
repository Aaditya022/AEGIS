# AEGIS Open-Source Release Checklist

## Code Quality

- [x] All Rust crates compile (cargo build --workspace)
- [x] All Go packages compile (go build ./...)
- [x] All tests pass (cargo test --workspace)
- [x] CI pipeline green (GitHub Actions)
- [x] No Clippy warnings (cargo clippy -- -D warnings)
- [x] No security vulnerabilities (cargo audit)
- [x] Formatting consistent (cargo fmt, go fmt)

## Documentation

- [x] README.md with quick start
- [x] ARCHITECTURE.md with diagrams
- [x] ROADMAP.md with development plan
- [x] CONTRIBUTING.md with contribution guide
- [x] Protocol specification (proto/ docs/)
- [x] Security model specification
- [x] API reference (auto-generated from protobuf)
- [x] Example integrations for all frameworks
- [x] Benchmark methodology and results template

## Governance

- [x] Apache 2.0 LICENSE file
- [x] Code of Conduct (CONTRIBUTING.md)
- [x] DCO (Developer Certificate of Origin) check in CI
- [x] Security policy (SECURITY.md)
- [x] Maintainers file (MAINTAINERS.md)
- [x] Governance model documented

## Infrastructure

- [x] GitHub Actions CI/CD
  - [x] Build (Rust + Go)
  - [x] Lint (Clippy, go vet)
  - [x] Test (unit + integration)
  - [x] Security audit (cargo audit, trivy)
  - [x] Docker image build
- [x] Docker Compose dev environment
- [x] Helm chart published
- [x] Container images on ghcr.io
- [x] Release workflow (tag -> publish)

## Artifacts

- [x] Docker images (sidecar, gateway, controller)
- [x] CLI binaries (cross-platform builds)
- [x] eBPF .o object files
- [x] Rego policy WASM modules
- [x] Helm chart tarball
- [ ] Published to crates.io (aegis-*)
- [ ] Published to pkg.go.dev (aegis-gateway)

## Community

- [ ] CNCF Sandbox application prepared
- [ ] OpenSSF Best Practices badge
- [ ] Slack / Discord community channel
- [ ] Monthly community calls
- [ ] Adopters list

## Marketing

- [ ] Website (aegis.ai)
- [ ] Logo and branding assets
- [ ] Blog post / announcement
- [ ] Conference talk proposals
- [ ] Demo videos
