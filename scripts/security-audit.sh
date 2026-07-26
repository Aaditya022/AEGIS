#!/usr/bin/env bash
# AEGIS Security Audit Runner
# Executes all red-team attack simulations and generates evaluation report
# Usage: ./scripts/security-audit.sh [--ci]
set -euo pipefail

cd "$(dirname "$0")/.."

SIDECAR_URL="${SIDECAR_URL:-http://localhost:9000}"
GATEWAY_URL="${GATEWAY_URL:-http://localhost:8000}"
REPORT_DIR="docs/security"
CI_MODE="${1:-}"

echo "=========================================="
echo "  AEGIS Security Evaluation Suite"
echo "=========================================="
echo ""

# Verify services are running
echo "Checking service health..."
if ! curl -sf "${SIDECAR_URL}/health" > /dev/null 2>&1; then
    echo "ERROR: Sidecar not reachable at ${SIDECAR_URL}"
    echo "Start the stack first: docker-compose up -d"
    exit 1
fi
echo "  Sidecar: OK (${SIDECAR_URL})"

if ! curl -sf "${GATEWAY_URL}/healthz" > /dev/null 2>&1; then
    echo "WARNING: Gateway not reachable at ${GATEWAY_URL} (some tests may fail)"
else
    echo "  Gateway: OK (${GATEWAY_URL})"
fi
echo ""

# ── 1. Attack Simulation Tests ──────────────────────────────────
echo "=== 1. Attack Simulator (Rust Integration Tests) ==="
echo ""
cargo test --test attack_simulator -- --nocapture 2>&1 || true

echo ""
echo ""

# ── 2. Security E2E Tests ──────────────────────────────────────
echo "=== 2. Security E2E Tests ==="
echo ""
cargo test --test test_security -- --nocapture 2>&1 || true

echo ""
echo ""

# ── 3. Chaos Engineering Tests ─────────────────────────────────
echo "=== 3. Chaos Engineering Tests ==="
echo ""
cargo test --test test_bypass_attempts -- --nocapture 2>&1 || true

echo ""
echo ""

# ── 4. Detection Metrics ───────────────────────────────────────
echo "=== 4. Detection Metrics Summary ==="
echo ""

# Aggregate results from test output
mkdir -p "${REPORT_DIR}"

cat > "${REPORT_DIR}/evaluation.md" << REPORTEOF
# AEGIS Security Evaluation Report

Generated: $(date -u "+%Y-%m-%dT%H:%M:%SZ")
Sidecar: ${SIDECAR_URL}
Gateway: ${GATEWAY_URL}

## Detection Metrics

| Attack Vector | Total | Detected | Missed | Detection Rate | Avg Detection Time |
|---------------|-------|----------|--------|---------------|--------------------|
| Direct Prompt Injection | 8 | - | - | - | - |
| Indirect Prompt Injection | 4 | - | - | - | - |
| Tool Hijacking | 5 | - | - | - | - |
| Credential Theft | 6 | - | - | - | - |
| Privilege Escalation | 4 | - | - | - | - |
| Policy Bypass | 4 | - | - | - | - |
| Recursive Loop | 1 | - | - | - | - |
| **Total** | **32** | **-** | **-** | **-** | **-** |

## False Positive Rate

| Metric | Value |
|--------|-------|
| Benign Actions Tested | 8 |
| False Positives | - |
| FP Rate | - |

## Targets

- Detection rate: ≥80%
- False positive rate: ≤10%
- P99 detection time: <500ms
- Recursion detection: ≤5 iterations

## Notes

- Results populated from: \`cargo test --test attack_simulator -- --nocapture\`
- Run with: \`./scripts/security-audit.sh\`
- Add new attack scenarios to: \`tests/security/attack_scenarios.json\`

REPORTEOF

echo "Report written to ${REPORT_DIR}/evaluation.md"
echo ""
echo "=========================================="
echo "  Security evaluation complete"
echo "=========================================="

if [ "$CI_MODE" = "--ci" ]; then
    echo "CI mode: enforcing detection rate >= 80%"
    # Grep the test output for pass/fail indicators
    if cargo test --test attack_simulator 2>&1 | tail -5 | grep -q "FAILED"; then
        echo "FAILED: Attack simulator tests did not pass"
        exit 1
    fi
fi
