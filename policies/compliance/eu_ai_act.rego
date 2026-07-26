# AEGIS EU AI Act Compliance Policies
# Maps EU AI Act articles to enforceable policies

package aegis.compliance.eu_ai_act

# Article 9: Risk Management
# High-risk AI systems must have risk management
article_9_compliant {
    input.risk_assessment_completed == true
}

# Article 12: Automatic Logging
# Systems must maintain automatic logs
article_12_compliant {
    input.audit_enabled == true
}

# Article 14: Human Oversight
# Systems must allow human intervention
article_14_compliant {
    input.human_oversight_enabled == true
}

# Article 15: Accuracy, Robustness, Cybersecurity
article_15_compliant {
    input.cybersecurity_controls == true
}

default overall_compliant = false

overall_compliant = true {
    article_9_compliant
    article_12_compliant
    article_14_compliant
    article_15_compliant
}
