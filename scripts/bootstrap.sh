#!/usr/bin/env bash
set -euo pipefail

echo "=== AEGIS Bootstrap ==="

# Check prerequisites
command -v rustc >/dev/null 2>&1 || { echo "Error: Rust not installed"; exit 1; }
command -v go >/dev/null 2>&1 || { echo "Error: Go not installed"; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "Error: Docker not installed"; exit 1; }
command -v protoc >/dev/null 2>&1 || { echo "Error: protoc not installed"; exit 1; }

echo "✓ Prerequisites satisfied"

# Install Rust toolchain
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# Install Go tools
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest

# Install cargo tools
cargo install cargo-audit cargo-watch just

echo "✓ Tools installed"

# Compile protobufs
make proto
echo "✓ Protobufs compiled"

# Build workspace
cargo build --workspace
echo "✓ Rust workspace built"

# Build Go modules
cd aegis-gateway && go build ./cmd/gateway/ && cd ..
cd aegis-control-plane && go build ./cmd/controller/ && cd ..
echo "✓ Go modules built"

# Start dev environment
docker compose -f deploy/docker/docker-compose.yaml up -d
echo "✓ Dev environment started"

echo ""
echo "=== AEGIS is ready ==="
echo "  Sidecar:      cargo run -p aegis-sidecar"
echo "  Gateway:      cd aegis-gateway && go run ./cmd/gateway/"
echo "  Control Plane: cd aegis-control-plane && go run ./cmd/controller/"
echo "  CLI:          cargo run -p aegis-cli -- --help"
echo "  Tests:        cargo test --workspace"
echo "  Benchmarks:   cargo bench --workspace"
