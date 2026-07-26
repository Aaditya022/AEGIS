#!/usr/bin/env bash
# eBPF Overhead Benchmark
# Measures latency with and without eBPF enabled
set -euo pipefail

SIDECAR_BIN="${SIDECAR_BIN:-./target/release/aegis-sidecar}"
RESULTS_DIR="${RESULTS_DIR:-./docs/benchmarks}"

echo "=== eBPF Overhead Benchmark ==="
echo ""

# Ensure we have wrk2
command -v wrk2 >/dev/null 2>&1 || { echo "Installing wrk2..."; brew install wrk2; }

# 1. Benchmark without eBPF
echo "1. Benchmark WITHOUT eBPF"
echo "   Starting sidecar..."
$SIDECAR_BIN --config test/config-no-ebpf.yaml &
PID=$!
sleep 2

echo "   Running wrk2..."
wrk2 -t2 -c10 -d30s -R1000 http://localhost:9000/health > "$RESULTS_DIR/latency-no-ebpf.txt" 2>&1

kill $PID 2>/dev/null
wait $PID 2>/dev/null || true

NO_EBPF_P99=$(grep "P99" "$RESULTS_DIR/latency-no-ebpf.txt" | awk '{print $2}')
echo "   P99 without eBPF: ${NO_EBPF_P99}ms"
echo ""

# 2. Benchmark WITH eBPF
echo "2. Benchmark WITH eBPF"
echo "   Starting sidecar with eBPF..."
sudo $SIDECAR_BIN --config test/config-with-ebpf.yaml --enable-ebpf &
PID=$!
sleep 3

echo "   Running wrk2..."
wrk2 -t2 -c10 -d30s -R1000 http://localhost:9000/health > "$RESULTS_DIR/latency-with-ebpf.txt" 2>&1

sudo kill $PID 2>/dev/null
wait $PID 2>/dev/null || true

WITH_EBPF_P99=$(grep "P99" "$RESULTS_DIR/latency-with-ebpf.txt" | awk '{print $2}')
echo "   P99 with eBPF: ${WITH_EBPF_P99}ms"
echo ""

# 3. Calculate overhead
echo "3. Results"
echo "   Without eBPF: P99 = ${NO_EBPF_P99}ms"
echo "   With eBPF:    P99 = ${WITH_EBPF_P99}ms"

# Calculate overhead percentage
OVERHEAD=$(echo "scale=2; (${WITH_EBPF_P99} - ${NO_EBPF_P99}) / ${NO_EBPF_P99} * 100" | bc)
echo "   Overhead:     ${OVERHEAD}%"
echo ""

echo "Results saved to $RESULTS_DIR/"
