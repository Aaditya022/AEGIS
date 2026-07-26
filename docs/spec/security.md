# AEGIS Security Model

## Threat Model

### Attacker Capabilities
- Full control of agent process (memory, execution)
- Ability to send arbitrary prompts and tool calls
- Access to agent's network environment
- Knowledge of agent's configuration

### Attacker Limitations
- No kernel-level (ring 0) access
- Cannot modify eBPF programs
- Cannot bypass iptables/nftables rules
- Cannot access HSM/TPM

## Attack Vectors

| Attack Vector | Mitigation |
|--------------|------------|
| Direct prompt injection | App-plane + infra-plane policy checks |
| Indirect prompt injection | Tool output scanning via eBPF |
| Tool hijacking | Schema validation + audit |
| Credential theft | Environment scoping + eBPF file access monitor |
| Privilege escalation | SPIFFE identity + NGAC delegation limits |
| Policy bypass | Two-Plane Verification divergence detection |
| Recursive agent loops | Recursion depth tracking |

## Security Properties

1. **Non-bypassability**: Even with full agent compromise, infra-plane (eBPF) enforces policies
2. **Tamper-evident audit**: Hash chain guarantees audit immutability
3. **Least privilege**: Every agent gets minimal required permissions
4. **Defense in depth**: App-plane + infra-plane + gateway + control-plane
5. **Zero trust**: Every request verified regardless of source
