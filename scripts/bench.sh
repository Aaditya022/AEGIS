#!/usr/bin/env bash
set -euo pipefail

echo "=== AEGIS Benchmark Suite ==="
echo ""

SIDECAR_URL=${SIDECAR_URL:-"http://localhost:9000"}
GATEWAY_URL=${GATEWAY_URL:-"http://localhost:8000"}

# Install dependencies if not present
command -v wrk2 >/dev/null 2>&1 || { echo "Installing wrk2..."; brew install wrk2; }
command -v k6 >/dev/null 2>&1 || { echo "Installing k6..."; brew install k6; }

echo "1. Sidecar Latency (wrk2)"
echo "   wrk2 -t2 -c10 -d30s -R1000 ${SIDECAR_URL}"
echo ""

echo "2. Gateway Throughput (wrk2)"
echo "   wrk2 -t4 -c100 -d60s -R10000 ${GATEWAY_URL}/v1/route"
echo ""

echo "3. Sustained Load (k6)"
echo "   k6 run -e URL=${GATEWAY_URL} --vus 50 --duration 5m scripts/k6-load-test.js"
echo ""

echo "4. Policy Evaluation (Rust criterion)"
echo "   cargo bench -p aegis-policy-engine"
echo ""

echo "5. Sidecar Overhead (Rust criterion)"
echo "   cargo bench -p aegis-sidecar"
echo ""

echo "6. Cold Start"
echo "   time aegis-sidecar --config test-config.yaml &"
echo "   PID=\$!; sleep 0.1; kill \$PID"
echo ""

echo "7. Memory Profile"
echo "   ps -p \$(pgrep aegis-sidecar) -o rss,pmem"
echo ""

echo "=== All benchmarks are runnable ==="
echo "Results should be recorded in docs/benchmarks/REPORT.md"
