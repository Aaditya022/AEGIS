# AEGIS NIST AI Agent Standards Compliance
# Maps NIST standards to enforceable policies

package aegis.compliance.nist

# NIST Identification: SPIFFE-compatible identities
identity_compliant {
    input.identity_type == "spiffe"
    input.identity_expiry != 0
}

# NIST Authorization: OAuth 2.1 / OIDC
authorization_compliant {
    input.auth_protocol == "oauth2.1"
}

# NIST Access Delegation: NGAC
delegation_compliant {
    input.delegation_chain_length <= 3
}

# NIST Logging and Transparency
logging_compliant {
    input.audit_enabled == true
    input.audit_retention_days >= 180
}

default overall_compliant = false

overall_compliant = true {
    identity_compliant
    authorization_compliant
    delegation_compliant
    logging_compliant
}
