use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

#[derive(Default)]
struct MetricCounters {
    requests_total: AtomicU64,
    allowed_total: AtomicU64,
    denied_total: AtomicU64,
    escalated_total: AtomicU64,
    request_duration_ns: AtomicU64,
    request_count: AtomicU64,
}

pub struct MetricsRegistry {
    counters: MetricCounters,
    denied_by_category: Arc<RwLock<HashMap<String, u64>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: MetricCounters::default(),
            denied_by_category: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn clone_handle() -> Self {
        Self::new()
    }

    pub fn record_request(&self, method: &str, path: &str, duration: Duration, status: u16) {
        self.counters.requests_total.fetch_add(1, Ordering::Relaxed);
        self.counters
            .request_duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        self.counters.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_allowed(&self, _category: &str) {
        self.counters.allowed_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_denied(&self, category: &str) {
        self.counters.denied_total.fetch_add(1, Ordering::Relaxed);
        let cat = category.to_string();
        let map = self.denied_by_category.clone();
        tokio::spawn(async move {
            let mut m = map.write().await;
            *m.entry(cat).or_insert(0) += 1;
        });
    }

    pub fn inc_escalated(&self) {
        self.counters
            .escalated_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub async fn snapshot(&self) -> String {
        let total = self.counters.requests_total.load(Ordering::Relaxed);
        let allowed = self.counters.allowed_total.load(Ordering::Relaxed);
        let denied = self.counters.denied_total.load(Ordering::Relaxed);
        let escalated = self.counters.escalated_total.load(Ordering::Relaxed);
        let duration_ns = self.counters.request_duration_ns.load(Ordering::Relaxed);
        let count = self.counters.request_count.load(Ordering::Relaxed);
        let avg_latency = if count > 0 { duration_ns / count } else { 0 };

        let cat_map = self.denied_by_category.read().await;

        let mut output = String::new();
        output.push_str("# HELP aegis_requests_total Total requests processed\n");
        output.push_str("# TYPE aegis_requests_total counter\n");
        output.push_str(&format!("aegis_requests_total {total}\n"));
        output.push_str("# HELP aegis_allowed_total Total allowed requests\n");
        output.push_str("# TYPE aegis_allowed_total counter\n");
        output.push_str(&format!("aegis_allowed_total {allowed}\n"));
        output.push_str("# HELP aegis_denied_total Total denied requests\n");
        output.push_str("# TYPE aegis_denied_total counter\n");
        output.push_str(&format!("aegis_denied_total {denied}\n"));
        output.push_str("# HELP aegis_escalated_total Total escalated requests\n");
        output.push_str("# TYPE aegis_escalated_total counter\n");
        output.push_str(&format!("aegis_escalated_total {escalated}\n"));
        output.push_str(
            "# HELP aegis_request_duration_avg Average request duration in nanoseconds\n",
        );
        output.push_str("# TYPE aegis_request_duration_avg gauge\n");
        output.push_str(&format!("aegis_request_duration_avg {avg_latency}\n"));

        output.push_str("# HELP aegis_denied_by_category Denied requests by category\n");
        output.push_str("# TYPE aegis_denied_by_category counter\n");
        for (cat, count) in cat_map.iter() {
            output.push_str(&format!(
                "aegis_denied_by_category{{category=\"{cat}\"}} {count}\n"
            ));
        }

        output
    }
}
