# AEGIS Recursion Limiter
# Prevents agents from looping on the same operation

package aegis.recursion

default allow = true

# Block if recursion depth exceeds limit
deny[reason] {
    input.recursion_depth > input.max_recursion_depth
    reason := sprintf("recursion limit exceeded: depth %d, max %d", [input.recursion_depth, input.max_recursion_depth])
}

# Block if same tool called repeatedly
deny[reason] {
    tool_repetition := count({op | op := input.recent_operations[_]; op == input.operation})
    tool_repetition >= 3
    reason := sprintf("tool called %d times, max 3 repetitions allowed", [tool_repetition])
}
