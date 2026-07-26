# AEGIS development commands (requires `just` — https://github.com/casey/just)
alias b := build
alias t := test
alias l := lint
alias c := check
alias r := run

default: check

# List all available commands
list:
    @just --list

# Build all Rust crates
build:
    cargo build --release --workspace

# Build debug
build-debug:
    cargo build --workspace

# Check compilation without producing artifacts
check:
    cargo check --workspace

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Lint with clippy
lint:
    cargo clippy --workspace -- -D warnings
    cargo fmt --all --check

# Auto-fix formatting
fmt:
    cargo fmt --all

# Run benchmarks
bench:
    cargo bench --workspace

# Generate docs
docs:
    cargo doc --workspace --no-deps

# Start dev environment
dev-up:
    docker compose -f deploy/docker/docker-compose.yaml up -d

# Stop dev environment
dev-down:
    docker compose -f deploy/docker/docker-compose.yaml down

# Watch and test on changes
watch:
    cargo watch -x test

# Full CI pipeline
ci: proto check lint test

# Compile protobufs
proto:
    just _proto_rust
    just _proto_go

_proto_rust:
    protoc --proto_path=proto/aegis/v1 \
        --rust_out=aegis-common/src \
        --tonic_out=aegis-common/src \
        proto/aegis/v1/*.proto

_proto_go:
    protoc --proto_path=proto/aegis/v1 \
        --go_out=. \
        --go-grpc_out=. \
        proto/aegis/v1/*.proto

# Security audit
audit:
    cargo audit

# Build Go gateway
build-gateway:
    cd aegis-gateway && go build -o bin/gateway ./cmd/gateway/

# Build Go control plane
build-control-plane:
    cd aegis-control-plane && go build -o bin/controller ./cmd/controller/
