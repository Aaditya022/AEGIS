use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::debug;

struct TraceState {
    depth: u32,
    max_depth: u32,
    tool_calls: Vec<(String, Instant)>,
    last_reset: Instant,
}

impl TraceState {
    fn new(max_depth: u32) -> Self {
        Self {
            depth: 0,
            max_depth,
            tool_calls: Vec::with_capacity(max_depth as usize + 1),
            last_reset: Instant::now(),
        }
    }

    fn is_stale(&self) -> bool {
        self.last_reset.elapsed() > Duration::from_secs(3600)
    }
}

pub struct RecursionDetector {
    max_depth: u32,
    traces: Arc<RwLock<HashMap<String, TraceState>>>,
}

impl RecursionDetector {
    pub fn new(max_depth: u32) -> Self {
        Self {
            max_depth,
            traces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns true if recursion limit exceeded
    pub async fn check(&self, trace_id: &str) -> bool {
        let mut traces = self.traces.write().await;

        if traces.len() > 10_000 {
            // LRU-style eviction of stale entries
            traces.retain(|_, v| !v.is_stale());
        }

        let state = traces
            .entry(trace_id.to_string())
            .or_insert_with(|| TraceState::new(self.max_depth));

        state.depth += 1;
        state.last_reset = Instant::now();

        let exceeded = state.depth > state.max_depth;

        debug!(
            trace_id,
            depth = state.depth,
            max = state.max_depth,
            exceeded,
            "Recursion check"
        );

        exceeded
    }

    /// Track a specific tool call for repetition detection
    pub async fn track_tool(&self, trace_id: &str, tool: &str) {
        let mut traces = self.traces.write().await;
        if let Some(state) = traces.get_mut(trace_id) {
            state.tool_calls.push((tool.to_string(), Instant::now()));
            // Keep only recent calls
            let cutoff = Instant::now() - Duration::from_secs(60);
            state.tool_calls.retain(|(_, t)| *t > cutoff);
        }
    }

    /// Check if a tool has been called too many times recently
    pub async fn tool_repetition(&self, trace_id: &str, tool: &str) -> bool {
        let traces = self.traces.read().await;
        if let Some(state) = traces.get(trace_id) {
            let count = state.tool_calls.iter().filter(|(t, _)| t == tool).count();
            count >= self.max_depth as usize
        } else {
            false
        }
    }

    pub async fn reset(&self, trace_id: &str) {
        let mut traces = self.traces.write().await;
        traces.remove(trace_id);
        debug!(trace_id, "Recursion state reset");
    }

    pub async fn reset_all(&self) {
        let mut traces = self.traces.write().await;
        traces.clear();
        debug!("All recursion states reset");
    }
}
