//! Metrics and Observability for BurrowDB Server
//!
//! Provides real-time metrics collection with atomic counters and histograms.
//! Designed for high performance with minimal overhead.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Atomic counter for thread-safe incrementing
#[derive(Debug, Default)]
pub struct Counter(AtomicU64);

impl Counter {
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, n: u64) {
        self.0.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Latency histogram with predefined buckets (in microseconds)
/// Buckets: 10µs, 50µs, 100µs, 500µs, 1ms, 5ms, 10ms, 50ms, 100ms, 500ms, 1s
#[derive(Debug)]
pub struct LatencyHistogram {
    buckets: [AtomicU64; 12],
    sum: AtomicU64,
    count: AtomicU64,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyHistogram {
    const BUCKET_BOUNDS: [u64; 11] = [
        10,      // 10µs
        50,      // 50µs
        100,     // 100µs
        500,     // 500µs
        1_000,   // 1ms
        5_000,   // 5ms
        10_000,  // 10ms
        50_000,  // 50ms
        100_000, // 100ms
        500_000, // 500ms
        1_000_000, // 1s
    ];

    pub fn new() -> Self {
        Self {
            buckets: Default::default(),
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Record a latency value in microseconds
    pub fn observe(&self, latency_us: u64) {
        self.sum.fetch_add(latency_us, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Find the bucket
        let bucket_idx = Self::BUCKET_BOUNDS
            .iter()
            .position(|&bound| latency_us <= bound)
            .unwrap_or(11);

        self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
    }

    /// Get histogram data
    pub fn snapshot(&self) -> HistogramSnapshot {
        let mut buckets = Vec::with_capacity(12);
        for (i, count) in self.buckets.iter().enumerate() {
            let bound = if i < 11 {
                Self::BUCKET_BOUNDS[i]
            } else {
                u64::MAX
            };
            buckets.push((bound, count.load(Ordering::Relaxed)));
        }

        HistogramSnapshot {
            buckets,
            sum: self.sum.load(Ordering::Relaxed),
            count: self.count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
    pub buckets: Vec<(u64, u64)>,
    pub sum: u64,
    pub count: u64,
}

impl HistogramSnapshot {
    pub fn avg_us(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum as f64 / self.count as f64
        }
    }

    pub fn p50_us(&self) -> u64 {
        self.percentile(0.50)
    }

    pub fn p99_us(&self) -> u64 {
        self.percentile(0.99)
    }

    fn percentile(&self, p: f64) -> u64 {
        let target = (self.count as f64 * p) as u64;
        let mut cumulative = 0u64;

        for (bound, count) in &self.buckets {
            cumulative += count;
            if cumulative >= target {
                return *bound;
            }
        }
        self.buckets.last().map(|(b, _)| *b).unwrap_or(0)
    }
}

/// Server-wide metrics collection
#[derive(Debug, Default)]
pub struct Metrics {
    // Connection metrics
    pub connections_total: Counter,
    pub connections_active: AtomicU64,

    // Request counters by operation
    pub requests_total: Counter,
    pub requests_get: Counter,
    pub requests_put: Counter,
    pub requests_delete: Counter,
    pub requests_keys: Counter,
    pub requests_stats: Counter,

    // Response status counters
    pub responses_ok: Counter,
    pub responses_not_found: Counter,
    pub responses_error: Counter,

    // Latency histograms by operation
    pub latency_get: LatencyHistogram,
    pub latency_put: LatencyHistogram,
    pub latency_delete: LatencyHistogram,

    // Multiplexer metrics
    pub reads_coalesced: Counter,
    pub reads_direct: Counter,

    // Bytes transferred
    pub bytes_received: Counter,
    pub bytes_sent: Counter,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Record a new connection
    pub fn connection_opened(&self) {
        self.connections_total.inc();
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record connection close
    pub fn connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get active connection count
    pub fn active_connections(&self) -> u64 {
        self.connections_active.load(Ordering::Relaxed)
    }

    /// Start timing a request (returns timer)
    pub fn start_timer(&self) -> RequestTimer {
        RequestTimer {
            start: Instant::now(),
        }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let get_lat = self.latency_get.snapshot();
        let put_lat = self.latency_put.snapshot();
        let del_lat = self.latency_delete.snapshot();

        format!(
            r#"# HELP burrowdb_connections_total Total connections since start
# TYPE burrowdb_connections_total counter
burrowdb_connections_total {}

# HELP burrowdb_connections_active Current active connections
# TYPE burrowdb_connections_active gauge
burrowdb_connections_active {}

# HELP burrowdb_requests_total Total requests by operation
# TYPE burrowdb_requests_total counter
burrowdb_requests_total{{op="get"}} {}
burrowdb_requests_total{{op="put"}} {}
burrowdb_requests_total{{op="delete"}} {}
burrowdb_requests_total{{op="keys"}} {}
burrowdb_requests_total{{op="stats"}} {}

# HELP burrowdb_responses_total Responses by status
# TYPE burrowdb_responses_total counter
burrowdb_responses_total{{status="ok"}} {}
burrowdb_responses_total{{status="not_found"}} {}
burrowdb_responses_total{{status="error"}} {}

# HELP burrowdb_latency_us Request latency in microseconds
# TYPE burrowdb_latency_us summary
burrowdb_latency_us{{op="get",quantile="0.5"}} {}
burrowdb_latency_us{{op="get",quantile="0.99"}} {}
burrowdb_latency_us{{op="put",quantile="0.5"}} {}
burrowdb_latency_us{{op="put",quantile="0.99"}} {}
burrowdb_latency_us{{op="delete",quantile="0.5"}} {}
burrowdb_latency_us{{op="delete",quantile="0.99"}} {}

# HELP burrowdb_reads_multiplexed Read coalescing stats
# TYPE burrowdb_reads_multiplexed counter
burrowdb_reads_coalesced {}
burrowdb_reads_direct {}

# HELP burrowdb_bytes_total Bytes transferred
# TYPE burrowdb_bytes_total counter
burrowdb_bytes_total{{direction="received"}} {}
burrowdb_bytes_total{{direction="sent"}} {}
"#,
            self.connections_total.get(),
            self.active_connections(),
            self.requests_get.get(),
            self.requests_put.get(),
            self.requests_delete.get(),
            self.requests_keys.get(),
            self.requests_stats.get(),
            self.responses_ok.get(),
            self.responses_not_found.get(),
            self.responses_error.get(),
            get_lat.p50_us(),
            get_lat.p99_us(),
            put_lat.p50_us(),
            put_lat.p99_us(),
            del_lat.p50_us(),
            del_lat.p99_us(),
            self.reads_coalesced.get(),
            self.reads_direct.get(),
            self.bytes_received.get(),
            self.bytes_sent.get(),
        )
    }

    /// Export metrics as JSON
    pub fn export_json(&self) -> String {
        let get_lat = self.latency_get.snapshot();
        let put_lat = self.latency_put.snapshot();
        let del_lat = self.latency_delete.snapshot();

        format!(
            r#"{{"connections":{{"total":{},"active":{}}},"requests":{{"get":{},"put":{},"delete":{},"keys":{},"stats":{}}},"responses":{{"ok":{},"not_found":{},"error":{}}},"latency_us":{{"get":{{"avg":{:.2},"p50":{},"p99":{},"count":{}}},"put":{{"avg":{:.2},"p50":{},"p99":{},"count":{}}},"delete":{{"avg":{:.2},"p50":{},"p99":{},"count":{}}}}},"multiplexer":{{"coalesced":{},"direct":{}}},"bytes":{{"received":{},"sent":{}}}}}"#,
            self.connections_total.get(),
            self.active_connections(),
            self.requests_get.get(),
            self.requests_put.get(),
            self.requests_delete.get(),
            self.requests_keys.get(),
            self.requests_stats.get(),
            self.responses_ok.get(),
            self.responses_not_found.get(),
            self.responses_error.get(),
            get_lat.avg_us(), get_lat.p50_us(), get_lat.p99_us(), get_lat.count,
            put_lat.avg_us(), put_lat.p50_us(), put_lat.p99_us(), put_lat.count,
            del_lat.avg_us(), del_lat.p50_us(), del_lat.p99_us(), del_lat.count,
            self.reads_coalesced.get(),
            self.reads_direct.get(),
            self.bytes_received.get(),
            self.bytes_sent.get(),
        )
    }
}

/// Timer for measuring request latency
pub struct RequestTimer {
    start: Instant,
}

impl RequestTimer {
    /// Get elapsed microseconds
    pub fn elapsed_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let c = Counter::new();
        assert_eq!(c.get(), 0);
        c.inc();
        assert_eq!(c.get(), 1);
        c.add(10);
        assert_eq!(c.get(), 11);
    }

    #[test]
    fn test_histogram() {
        let h = LatencyHistogram::new();
        h.observe(50);   // 50µs bucket
        h.observe(100);  // 100µs bucket
        h.observe(1000); // 1ms bucket

        let snap = h.snapshot();
        assert_eq!(snap.count, 3);
        assert_eq!(snap.sum, 1150);
    }

    #[test]
    fn test_metrics_export() {
        let m = Metrics::new();
        m.requests_get.inc();
        m.latency_get.observe(100);

        let json = m.export_json();
        assert!(json.contains("\"get\":1"));

        let prom = m.export_prometheus();
        assert!(prom.contains("burrowdb_requests_total"));
    }
}

