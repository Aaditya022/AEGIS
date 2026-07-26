# AEGIS Environment Scoping Policy
# Prevents staging credentials from accessing production resources

package aegis.environment

default allow = true

# Block production access with non-production credentials
deny[reason] {
    input.environment == "staging"
    contains(input.resource, "production")
    reason := "staging credentials cannot access production resources"
}

deny[reason] {
    input.environment == "development"
    contains(input.resource, "production")
    reason := "development credentials cannot access production resources"
}

deny[reason] {
    input.environment == "production"
    not contains(input.resource, "production")
    reason := "production credentials should only access production resources"
}
