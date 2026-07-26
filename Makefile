.PHONY: all build test lint clean docker docker-sidecar docker-gateway docker-control-plane \
        proto dev-env ci bench security-audit docs help install

SHELL := /bin/bash
CARGO := cargo
GO := go

help: ## Display this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: proto build test lint ## Build, test, and lint all components

# ── Rust Workspace ──────────────────────────────────────────────────────

build: ## Build all Rust crates
	$(CARGO) build --release --workspace

build-debug: ## Build all Rust crates (debug)
	$(CARGO) build --workspace

test: ## Run all Rust tests
	$(CARGO) test --workspace -- --nocapture

test-sidecar: ## Run sidecar-specific tests
	$(CARGO) test -p aegis-sidecar -- --nocapture

test-policy: ## Run policy engine tests
	$(CARGO) test -p aegis-policy-engine -- --nocapture

lint: ## Run clippy and rustfmt checks
	$(CARGO) clippy --workspace -- -D warnings
	$(CARGO) fmt --all --check

clean: ## Clean all build artifacts
	$(CARGO) clean
	rm -rf aegis-gateway/bin/ aegis-control-plane/bin/

# ── Go Modules ───────────────────────────────────────────────────────────

build-gateway: ## Build Go gateway
	cd aegis-gateway && $(GO) build -o bin/gateway ./cmd/gateway/

build-control-plane: ## Build Go control plane
	cd aegis-control-plane && $(GO) build -o bin/controller ./cmd/controller/

test-go: ## Run all Go tests
	cd aegis-gateway && $(GO) test ./... && cd ../aegis-control-plane && $(GO) test ./...

# ── Protocol Buffers ────────────────────────────────────────────────────

PROTO_DIR := proto/aegis/v1
PROTO_OUT_RUST := aegis-common/src
PROTO_OUT_GO := .

proto: ## Compile protobuf definitions (Rust/tonic only; Go does not use gRPC)
	protoc --proto_path=$(PROTO_DIR) \
		--prost_out=$(PROTO_OUT_RUST) \
		--tonic_out=$(PROTO_OUT_RUST) \
		$(PROTO_DIR)/*.proto

# ── Docker ──────────────────────────────────────────────────────────────

docker-sidecar: ## Build sidecar Docker image
	docker build -t aegis/sidecar:latest -f deploy/docker/sidecar.Dockerfile .

docker-gateway: ## Build gateway Docker image
	docker build -t aegis/gateway:latest -f deploy/docker/gateway.Dockerfile .

docker-control-plane: ## Build control plane Docker image
	docker build -t aegis/control-plane:latest -f deploy/docker/control-plane.Dockerfile .

docker-all: docker-sidecar docker-gateway docker-control-plane ## Build all Docker images

# ── Development ──────────────────────────────────────────────────────────

dev-env: ## Start local development dependencies
	docker compose -f deploy/docker/docker-compose.yaml up -d

dev-env-stop: ## Stop local development dependencies
	docker compose -f deploy/docker/docker-compose.yaml down

dev-env-logs: ## View dev dependency logs
	docker compose -f deploy/docker/docker-compose.yaml logs -f

# ── Benchmarking ────────────────────────────────────────────────────────

bench: ## Run Rust benchmarks
	$(CARGO) bench --workspace

bench-sidecar: ## Run sidecar-specific benchmarks
	$(CARGO) bench -p aegis-sidecar

bench-policy: ## Run policy engine benchmarks
	$(CARGO) bench -p aegis-policy-engine

# ── Quality ──────────────────────────────────────────────────────────────

security-audit: ## Run cargo-audit for vulnerability scanning
	$(CARGO) audit

ci: proto lint build test ## Full CI pipeline

# ── Documentation ───────────────────────────────────────────────────────

docs: ## Generate documentation
	$(CARGO) doc --workspace --no-deps

serve-docs: ## Serve documentation locally
	$(CARGO) doc --workspace --no-deps --open

# ── Kubernetes ──────────────────────────────────────────────────────────

kind-up: ## Create local kind cluster
	kind create cluster --name aegis --config deploy/kubernetes/kind-config.yaml

kind-down: ## Delete local kind cluster
	kind delete cluster --name aegis

kind-load: docker-all ## Load Docker images into kind cluster
	kind load docker-image aegis/sidecar:latest --name aegis
	kind load docker-image aegis/gateway:latest --name aegis
	kind load docker-image aegis/control-plane:latest --name aegis

deploy-dev: kind-up kind-load ## Deploy AEGIS to local kind cluster
	helm install aegis deploy/kubernetes/charts/aegis

undeploy-dev: ## Uninstall AEGIS from local kind cluster
	helm uninstall aegis
	kind delete cluster --name aegis

# ── Installation ────────────────────────────────────────────────────────

install: ## Install AEGIS CLI locally
	$(CARGO) install --path aegis-cli
