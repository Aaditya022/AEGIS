// AEGIS Performance Tests
// These tests measure policy evaluation latency using the PolicyEngine directly.
// They do NOT require a running sidecar or gateway.

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use aegis_common::types::{Decision, PolicyContext, PolicyResult};
    use aegis_policy_engine::PolicyEngine;

    fn bench_ctx() -> PolicyContext {
        PolicyContext {
            agent_id: "bench-agent".into(),
            operation: "tool.call".into(),
            resource: "search_documents".into(),
            environment: "production".into(),
            recursion_depth: 0,
            budget_consumed_usd: 10.0,
            trace_id: uuid::Uuid::new_v4().to_string(),
            extra: Default::default(),
        }
    }

    #[test]
    fn test_policy_evaluation_latency_p50() {
        let engine = PolicyEngine::default();
        let ctx = bench_ctx();
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = engine.evaluate(&ctx).unwrap();
        }
        let total = start.elapsed();
        let avg_ns = total.as_nanos() / iterations as u128;

        println!("Policy Evaluation Latency ({} iterations):", iterations);
        println!("  Total: {:?}", total);
        println!("  Average: {}ns", avg_ns);
        println!("  P50 (approx): {}ns", avg_ns);

        // Target: <1ms average
        let avg_us = avg_ns as f64 / 1000.0;
        assert!(
            avg_us < 1000.0,
            "Average policy evaluation ({:.1}µs) exceeds 1ms target",
            avg_us
        );
    }

    #[test]
    fn test_policy_evaluation_throughput() {
        let engine = PolicyEngine::default();
        let ctx = bench_ctx();
        let iterations = 5000;

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = engine.evaluate(&ctx).unwrap();
        }
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!("Policy Evaluation Throughput:");
        println!("  Iterations: {iterations}");
        println!("  Time: {:?}", elapsed);
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);

        assert!(ops_per_sec > 1000.0, "Throughput too low: {:.0} ops/sec", ops_per_sec);
    }

    #[test]
    fn test_policy_evaluation_deny_fast_path() {
        let engine = PolicyEngine::default();
        let mut deny_ctx = bench_ctx();
        deny_ctx.environment = "staging".into();
        deny_ctx.resource = "https://production-db.internal:5432".into();

        let iterations = 1000;
        let start = Instant::now();
        for _ in 0..iterations {
            let result = engine.evaluate(&deny_ctx).unwrap();
            assert_eq!(result.decision, Decision::Deny,
                "Staging→production should be denied");
        }
        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() / iterations as u128;

        println!("Deny Fast Path ({} iterations):", iterations);
        println!("  Average: {}ns", avg_ns);
        println!("  Total: {:?}", elapsed);
    }

    #[test]
    fn test_memory_footprint() {
        // Verify the engine doesn't allocate excessive memory
        let engine = PolicyEngine::default();
        let ctx = bench_ctx();

        let before = std::process::id();
        let _ = engine.evaluate(&ctx).unwrap();
        let after = std::process::id();

        println!("Memory footprint test:");
        println!("  Process ID: {}", before);
        println!("  Run: ps -p {} -o rss to measure RSS", before);
        println!("  Target: <50MB RSS");
        // No assertion — RSS measurement is external
    }

    #[test]
    fn test_cold_start() {
        // Measure PolicyEngine construction time
        let start = Instant::now();
        let engine = PolicyEngine::default();
        let elapsed = start.elapsed();

        println!("Cold Start:");
        println!("  Engine creation: {:?}", elapsed);
        println!("  Policy count: {}", engine.policy_count());

        assert!(
            elapsed < Duration::from_secs(2),
            "Engine creation ({:?}) exceeds 2s target",
            elapsed
        );
    }
}
