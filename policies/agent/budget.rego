# AEGIS Budget Circuit Breaker
# Prevents cost overruns by halting agents that exceed budget

package aegis.budget

default allow = true

# Block if budget exceeded
deny[reason] {
    input.budget_consumed > input.budget_limit
    reason := sprintf("budget limit exceeded: $%.2f, limit $%.2f", [input.budget_consumed, input.budget_limit])
}

# Warn at 80% of budget
escalate[reason] {
    input.budget_consumed > (input.budget_limit * 0.8)
    input.budget_consumed <= input.budget_limit
    reason := sprintf("budget warning: $%.2f used of $%.2f (%.0f%%)", [
        input.budget_consumed,
        input.budget_limit,
        (input.budget_consumed / input.budget_limit) * 100,
    ])
}
