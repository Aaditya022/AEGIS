use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tracing::{info, warn};

pub struct CostCircuit {
    current_cost_micro_cents: AtomicU64,
    budget_limit_micro_cents: u64,
    breached: AtomicBool,
    sidecar_id: String,
}

impl CostCircuit {
    pub fn new(budget_limit_usd: f64, sidecar_id: String) -> Self {
        Self {
            current_cost_micro_cents: AtomicU64::new(0),
            budget_limit_micro_cents: (budget_limit_usd * 1_000_000.0) as u64,
            breached: AtomicBool::new(false),
            sidecar_id,
        }
    }

    pub fn record(&self, cost_usd: f64) {
        let micro_cents = (cost_usd * 1_000_000.0) as u64;
        let prev = self.current_cost_micro_cents.fetch_add(micro_cents, Ordering::SeqCst);
        let new_total = (prev + micro_cents) as f64 / 1_000_000.0;

        if new_total > self.budget() {
            if !self.breached.swap(true, Ordering::SeqCst) {
                warn!(
                    sidecar = %self.sidecar_id,
                    total = new_total,
                    limit = self.budget(),
                    "Budget circuit breaker tripped"
                );
            }
        }
    }

    pub fn check(&self) -> Result<(), String> {
        if self.breached.load(Ordering::SeqCst) {
            let total = self.current_cost();
            return Err(format!(
                "budget exhausted: ${:.4} of ${:.2}",
                total,
                self.budget()
            ));
        }
        Ok(())
    }

    pub async fn current_cost(&self) -> f64 {
        self.current_cost_micro_cents.load(Ordering::SeqCst) as f64 / 1_000_000.0
    }

    pub fn budget(&self) -> f64 {
        self.budget_limit_micro_cents as f64 / 1_000_000.0
    }

    pub fn remaining(&self) -> f64 {
        let current = self.current_cost_micro_cents.load(Ordering::SeqCst);
        if current >= self.budget_limit_micro_cents {
            0.0
        } else {
            (self.budget_limit_micro_cents - current) as f64 / 1_000_000.0
        }
    }

    pub fn is_breached(&self) -> bool {
        self.breached.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.current_cost_micro_cents.store(0, Ordering::SeqCst);
        self.breached.store(false, Ordering::SeqCst);
        info!(sidecar = %self.sidecar_id, "Cost circuit reset");
    }
}
