# AEGIS Identity Verification Policy
# Requires valid SPIFFE-compatible identity for all operations

package aegis.identity

default allow = false

# Allow if agent has valid identity
allow {
    input.agent_id != ""
    input.identity_valid == true
    input.identity_expiry > time.now_ns()
}

# Deny with reason
deny[reason] {
    not allow
    reason := sprintf("identity verification failed for agent %s", [input.agent_id])
}
