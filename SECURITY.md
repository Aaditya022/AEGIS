# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take the security of AEGIS seriously. If you believe you have found a
security vulnerability, please report it to us as described below.

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, report via email to **security@aegis.ai** (preferred) or create a
confidential issue.

You should receive a response within 48 hours. If you do not, please follow up
to ensure we received your message.

When reporting, include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Disclosure Policy

When we receive a security report, we will:

1. Confirm receipt within 48 hours
2. Assess severity and impact
3. Develop and test a fix
4. Release a security advisory on GitHub
5. Credit the reporter (if desired)

## Security Features

AEGIS employs multiple layers of defense:

- **Two-Plane Verification**: Application-plane (OPA/Rego) + infra-plane (eBPF)
  policy enforcement ensures non-bypassable security
- **Tamper-Evident Audit Log**: Hash-chained audit events with integrity
  verification
- **SPIFFE Identity**: mTLS-based identity for all components
- **WASM Sandboxing**: Policy code runs in isolated WebAssembly runtime
- **Rate Limiting**: Token bucket with Redis backend
- **Network Policy**: Kubernetes NetworkPolicies restrict inter-component
  traffic

## Known Security Considerations

- eBPF requires Linux kernel 5.4+ with BPF support enabled
- WASM evaluation is sandboxed but not memory-safe against malicious policies
- Metadata server access should be blocked by network policy
- Audit log integrity depends on hash chain; key compromise enables forging

## Responsible Disclosure

We will credit security researchers who responsibly disclose vulnerabilities
in our GitHub Security Advisories. We request a 90-day disclosure timeline
from the date of fix release.
