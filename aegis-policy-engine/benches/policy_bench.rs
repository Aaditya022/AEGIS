use criterion::{black_box, criterion_group, criterion_main, Criterion};
use aegis_policy_engine::*;
use aegis_common::types::*;

fn benchmark_policy_match(c: &mut Criterion) {
    let ctx = PolicyContext {
        agent_id: "test-agent".into(),
        operation: "tool.call".into(),
        resource: "delete-database".into(),
        environment: "production".into(),
        recursion_depth: 0,
        budget_consumed_usd: 0.0,
        trace_id: "trace-1".into(),
        extra: Default::default(),
    };

    c.bench_function("recursion_detection", |b| {
        let trace = vec!["tool_a".into(), "tool_a".into(), "tool_a".into()];
        b.iter(|| {
            black_box(detect_recursion(&trace, "tool_a", 3))
        })
    });

    c.bench_function("budget_check", |b| {
        b.iter(|| {
            black_box(budget_exceeded(95.0, 100.0))
        })
    });
}

criterion_group!(benches, benchmark_policy_match);
criterion_main!(benches);
