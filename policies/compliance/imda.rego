# AEGIS Singapore IMDA Framework Compliance
# Enforces five-tier autonomy classification

package aegis.compliance.imda

# IMDA Autonomy Level 1: Human-initiated, approved
# Policy: Log only, full human control
level_1_compliant {
    input.autonomy_level == 1
}

# IMDA Autonomy Level 2: Human-initiated, executed
# Policy: Basic policy compliance, supervisory human role
level_2_compliant {
    input.autonomy_level == 2
    input.identity_valid == true
}

# IMDA Autonomy Level 3: Agent-initiated, approved
# Policy: Approval gate required, review required
level_3_compliant {
    input.autonomy_level == 3
    input.human_approval_required == true
}

# IMDA Autonomy Level 4: Agent-initiated, executed
# Policy: Full policy + audit, exception handling
level_4_compliant {
    input.autonomy_level == 4
    input.audit_enabled == true
    input.full_policy_enforcement == true
}

# IMDA Autonomy Level 5: Fully autonomous
# Policy: Maximum restriction, post-hoc review
level_5_compliant {
    input.autonomy_level == 5
    input.audit_enabled == true
    input.full_policy_enforcement == true
    input.post_hoc_review_enabled == true
}

default overall_compliant = false

overall_compliant = true {
    level_1_compliant
} or {
    level_2_compliant
} or {
    level_3_compliant
} or {
    level_4_compliant
} or {
    level_5_compliant
}
