# AEGIS Protocol Specification

## Overview

AEGIS intercepts all agent-to-resource communication through a transparent sidecar proxy. The protocol operates at L4/L7 with semantic awareness of AI agent operations.

## Wire Protocol

### Request Headers

Every agent request MUST include:

```
X-AEGIS-Agent-ID: <spiffe://cluster/ns/ns/sa/sa>
X-AEGIS-Trace-ID: <uuid>
X-AEGIS-Parent-Trace: <trace-id>  // optional, for recursion detection
```

### Response Headers

Every response includes governance decision metadata:

```
X-AEGIS-Decision: ALLOW | DENY | ESCALATE
X-AEGIS-Reason: <human-readable-reason>
X-AEGIS-Event-ID: <uuid>
```

### gRPC Services

Three gRPC services defined in `proto/aegis/v1/`:

1. **SidecarService** — Config retrieval, decision reporting, audit streaming
2. **GatewayService** — LLM routing, rate limiting, metrics
3. **ControlPlaneService** — Agent registration, policy management, compliance

## Two-Plane Verification

1. **Application Plane** (inside agent): SDK-based policy checks
2. **Infrastructure Plane** (kernel): eBPF-based independent verification

Both planes MUST agree for an operation to proceed. Divergence triggers block + alert.

## Audit Event Format

```json
{
  "event_id": "uuid",
  "trace_id": "uuid",
  "agent_id": "spiffe://...",
  "operation": "tool.call",
  "resource": "delete-database",
  "decision": "DENY",
  "timestamp": "2026-07-26T12:00:00Z",
  "signature": "hex-encoded-ed25519-sig",
  "sequence_number": 42,
  "sidecar_id": "sidecar-1",
  "metadata": {"reason": "unauthorized tool"}
}
```
