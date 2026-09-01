//! Lightweight observability: atomic counters + a /metrics text exporter.
//!
//! ANUBIS tracks a small set of runtime counters for operations, errors, and
//! synthesis work, and exposes them in the Prometheus text format on the
//! WebSocket HTTP server's /_metrics endpoint. The counters are lock-free
//! atomics, so they're cheap to bump on hot paths.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

pub struct Metrics {
    counter: BTreeMap<&'static str, AtomicU64>,
    started_at: std::time::Instant,
    /// Running sum of seconds spent inside synthesis (approx).
    synth_seconds_running: AtomicU64,
    /// Tracks the last synthesis call's duration in ms (gauge).
    last_synth_ms: AtomicI64,
}

fn all_counters() -> &'static [&'static str] {
    &[
        "messages_total",
        "commands_total",
        "callbacks_total",
        "asks_total",
        "asks_errors_total",
        "speak_total",
        "clones_total",
        "clone_errors_total",
        "voice_input_total",
        "watermarks_total",
        "payments_total",
        "marketplace_installs_total",
        "ws_connections_total",
        "memory_cleared_total",
    ]
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            counter: all_counters()
                .iter()
                .map(|&k| (k, AtomicU64::new(0)))
                .collect(),
            started_at: std::time::Instant::now(),
            synth_seconds_running: AtomicU64::new(0),
            last_synth_ms: AtomicI64::new(0),
        }
    }
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc(&self, name: &'static str) {
        if let Some(c) = self.counter.get(name) {
            c.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn add_synth_time(&self, millis: u64) {
        self.synth_seconds_running
            .fetch_add(millis / 1000, Ordering::Relaxed);
        self.last_synth_ms.store(millis as i64, Ordering::Relaxed);
    }

    fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Render all metrics in Prometheus text exposition format.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# HELP anubis_uptime_seconds Process uptime since start."
        );
        let _ = writeln!(out, "# TYPE anubis_uptime_seconds counter");
        let _ = writeln!(out, "anubis_uptime_seconds {}", self.uptime_secs());
        let _ = writeln!(out, "# HELP last synthesis duration ms");
        let _ = writeln!(
            out,
            "anubis_last_synth_ms {}",
            self.last_synth_ms.load(Ordering::Relaxed)
        );
        for &k in all_counters() {
            let name = k.trim_end_matches("_total");
            let _ = writeln!(out, "# TYPE anubis_{name} counter");
            let _ = writeln!(
                out,
                "anubis_{name}_total {}",
                self.counter
                    .get(k)
                    .map(|c| c.load(Ordering::Relaxed))
                    .unwrap_or(0)
            );
        }
        out
    }
}
