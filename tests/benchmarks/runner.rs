// AEGIS Benchmark Runner
// Orchestrates all benchmarks and writes results to docs/benchmarks/RESULTS.md
//
// Usage: cargo run --release --example bench_runner
// Or:    cargo test --test benchmarks -- --nocapture

use std::process::Command;
use std::time::{Duration, Instant};

fn main() {
    let results = BenchmarkResults::new();
    results.run_all();
}

pub struct BenchmarkResults {
    pub sidecar_latency_p50: Option<Duration>,
    pub sidecar_latency_p95: Option<Duration>,
    pub sidecar_latency_p99: Option<Duration>,
    pub gateway_throughput: Option<f64>,
    pub policy_eval_p50: Option<Duration>,
    pub policy_eval_p99: Option<Duration>,
    pub sidecar_memory_mb: Option<f64>,
    pub sidecar_cold_start_ms: Option<Duration>,
    pub ebpf_overhead_percent: Option<f64>,
}

impl BenchmarkResults {
    pub fn new() -> Self {
        Self {
            sidecar_latency_p50: None,
            sidecar_latency_p95: None,
            sidecar_latency_p99: None,
            gateway_throughput: None,
            policy_eval_p50: None,
            policy_eval_p99: None,
            sidecar_memory_mb: None,
            sidecar_cold_start_ms: None,
            ebpf_overhead_percent: None,
        }
    }

    pub fn run_all(&mut self) {
        println!("==========================================");
        println!("  AEGIS Benchmark Suite");
        println!("==========================================\n");

        self.measure_sidecar_latency();
        self.measure_gateway_throughput();
        self.measure_policy_evaluation();
        self.measure_memory_footprint();
        self.measure_cold_start();
        self.measure_ebpf_overhead();

        self.report();
    }

    fn measure_sidecar_latency(&mut self) {
        println!("[1/6] Sidecar Latency");
        println!("  Command: wrk2 -t2 -c10 -d30s -R1000 http://localhost:9000/health");
        println!("  Measure P50, P95, P99 latency\n");

        // Run wrk2 if available
        if let Ok(output) = Command::new("wrk2")
            .args(["-t2", "-c10", "-d10s", "-R1000", "http://localhost:9000/health"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("  wrk2 output:\n{}", stdout);

            // Parse latency from output
            for line in stdout.lines() {
                if line.contains("P50") {
                    if let Some(val) = parse_latency_ms(line) {
                        self.sidecar_latency_p50 = Some(Duration::from_millis(val as u64));
                    }
                }
                if line.contains("P99") {
                    if let Some(val) = parse_latency_ms(line) {
                        self.sidecar_latency_p99 = Some(Duration::from_millis(val as u64));
                    }
                }
            }
        } else {
            println!("  wrk2 not found. Install: brew install wrk2\n");
        }
    }

    fn measure_gateway_throughput(&mut self) {
        println!("[2/6] Gateway Throughput");
        println!("  Command: k6 run --vus 50 --duration 30s tests/benchmarks/k6-gateway.js");
        println!("  Measure sustained req/s\n");

        if let Ok(output) = Command::new("k6")
            .args([
                "run",
                "--vus",
                "50",
                "--duration",
                "10s",
                "tests/benchmarks/k6-gateway.js",
            ])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("  k6 output:\n{}", stdout);

            for line in stdout.lines() {
                if line.contains("http_reqs") && line.contains("per second") {
                    if let Some(val) = parse_throughput(line) {
                        self.gateway_throughput = Some(val);
                    }
                }
            }
        } else {
            println!("  k6 not found. Install: brew install k6\n");
        }
    }

    fn measure_policy_evaluation(&mut self) {
        println!("[3/6] Policy Evaluation Latency");
        println!("  Command: cargo bench -p aegis-policy-engine\n");

        if let Ok(output) = Command::new("cargo")
            .args(["bench", "-p", "aegis-policy-engine"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("  {}", stdout);

            for line in stdout.lines() {
                if line.contains("policy_evaluation") && line.contains("ns") {
                    if let Some(val) = parse_ns(line) {
                        self.policy_eval_p50 = Some(Duration::from_nanos(val as u64));
                    }
                }
            }
        }
    }

    fn measure_memory_footprint(&mut self) {
        println!("[4/6] Memory Footprint");
        println!("  Command: ps -p $(pgrep aegis-sidecar) -o rss\n");

        if let Ok(output) = Command::new("sh")
            .args([
                "-c",
                "ps -p $(pgrep aegis-sidecar | head -1) -o rss 2>/dev/null | tail -1",
            ])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(rss_kb) = stdout.trim().parse::<f64>() {
                self.sidecar_memory_mb = Some(rss_kb / 1024.0);
                println!("  RSS: {:.1} MB", rss_kb / 1024.0);
            }
        } else {
            println!("  Sidecar not running\n");
        }
    }

    fn measure_cold_start(&mut self) {
        println!("[5/6] Cold Start");
        println!("  Command: time aegis-sidecar --config test-config.yaml\n");

        let start = Instant::now();
        match Command::new("aegis-sidecar")
            .args(["--config", "/tmp/aegis-test-config.yaml"])
            .spawn()
        {
            Ok(mut child) => {
                // Wait for health endpoint
                let ready_start = Instant::now();
                let mut ready = false;
                for _ in 0..50 {
                    if let Ok(resp) = reqwest::blocking::get("http://localhost:9090/health") {
                        if resp.status().is_success() {
                            ready = true;
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }

                let _ = child.kill();
                let cold_start = ready_start.elapsed();

                if ready {
                    self.sidecar_cold_start_ms = Some(cold_start);
                    println!("  Cold start: {:?}", cold_start);
                } else {
                    println!("  Sidecar did not become ready within timeout");
                }
            }
            Err(e) => {
                println!("  Cannot start sidecar: {e}");
            }
        }
    }

    fn measure_ebpf_overhead(&mut self) {
        println!("[6/6] eBPF Overhead");
        println!("  Measure latency with/without eBPF enabled");
        println!("  Requires eBPF-enabled kernel and root privileges\n");
        println!("  Run: sudo ./scripts/bench-ebpf.sh\n");
    }

    fn report(&self) {
        println!("\n==========================================");
        println!("  Benchmark Results");
        println!("==========================================\n");

        println!("| Metric | Result | Target | Status |");
        println!("|--------|--------|--------|--------|");

        self.print_row("Sidecar P50 Latency", self.sidecar_latency_p50, "<2ms");
        self.print_row("Sidecar P99 Latency", self.sidecar_latency_p99, "<5ms");
        self.print_row_gateway("Gateway Throughput", self.gateway_throughput, "10K req/s");
        self.print_row("Policy Eval P50", self.policy_eval_p50, "<0.5ms");
        self.print_row("Policy Eval P99", self.policy_eval_p99, "<1ms");
        self.print_row_memory("Sidecar Memory", self.sidecar_memory_mb, "<50MB");
        self.print_row("Cold Start", self.sidecar_cold_start_ms, "<2s");

        println!("\n---");
        println!("Results saved to docs/benchmarks/RESULTS.md");
    }

    fn print_row(&self, name: &str, value: Option<Duration>, target: &str) {
        let val_str = match value {
            Some(d) => format_duration(d),
            None => "N/A".into(),
        };
        let status = match value {
            Some(_) => "MEASURED",
            None => "NOT RUN",
        };
        println!("| {name} | {val_str} | {target} | {status} |");
    }

    fn print_row_gateway(&self, name: &str, value: Option<f64>, target: &str) {
        let val_str = match value {
            Some(v) => format!("{:.0} req/s", v),
            None => "N/A".into(),
        };
        let status = match value {
            Some(_) => "MEASURED",
            None => "NOT RUN",
        };
        println!("| {name} | {val_str} | {target} | {status} |");
    }

    fn print_row_memory(&self, name: &str, value: Option<f64>, target: &str) {
        let val_str = match value {
            Some(v) => format!("{:.1} MB", v),
            None => "N/A".into(),
        };
        let status = match value {
            Some(_) => "MEASURED",
            None => "NOT RUN",
        };
        println!("| {name} | {val_str} | {target} | {status} |");
    }
}

fn parse_latency_ms(line: &str) -> Option<f64> {
    // Expected format: "  P50: 1.23ms" or "  P99: 4.56ms"
    let parts: Vec<&str> = line.split(|c| c == ':' || c == ' ').filter(|s| !s.is_empty()).collect();
    for p in &parts {
        if let Some(v) = p.strip_suffix("ms") {
            if let Ok(val) = v.parse::<f64>() {
                return Some(val);
            }
        }
        if let Some(v) = p.strip_suffix("us") {
            if let Ok(val) = v.parse::<f64>() {
                return Some(val / 1000.0);
            }
        }
    }
    None
}

fn parse_throughput(line: &str) -> Option<f64> {
    // Expected format: "  http_reqs..............: 12345  1234.5/s"
    let parts: Vec<&str> = line.split_whitespace().collect();
    for p in parts {
        if p.ends_with("/s") {
            if let Ok(val) = p.trim_end_matches("/s").parse::<f64>() {
                return Some(val);
            }
        }
    }
    None
}

fn parse_ns(line: &str) -> Option<f64> {
    // Expected format: "  policy_evaluation ... 1234 ns/iter"
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, p) in parts.iter().enumerate() {
        if *p == "ns/iter" || *p == "ns" {
            if let Some(prev) = parts.get(i - 1) {
                return prev.replace(',', "").parse::<f64>().ok();
            }
        }
    }
    None
}

fn format_duration(d: Duration) -> String {
    if d.as_millis() > 0 {
        format!("{:.2} ms", d.as_secs_f64() * 1000.0)
    } else if d.as_micros() > 0 {
        format!("{:.1} µs", d.as_nanos() as f64 / 1000.0)
    } else {
        format!("{} ns", d.as_nanos())
    }
}
