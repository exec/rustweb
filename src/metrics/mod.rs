pub mod prometheus;

use hyper::{Method, StatusCode};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub requests_duration_seconds: AtomicU64,
    pub active_connections: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub status_codes: DashMap<u16, AtomicU64>,
    pub methods: DashMap<String, AtomicU64>,
}

pub struct MetricsCollector {
    metrics: Arc<Metrics>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Metrics::default()),
        }
    }

    pub fn record_request(&self, status: StatusCode, method: &Method, duration: Duration) {
        self.metrics.requests_total.fetch_add(1, Ordering::Relaxed);
        
        self.metrics.requests_duration_seconds.fetch_add(
            duration.as_millis() as u64,
            Ordering::Relaxed
        );

        let status_counter = self.metrics.status_codes
            .entry(status.as_u16())
            .or_insert_with(|| AtomicU64::new(0));
        status_counter.fetch_add(1, Ordering::Relaxed);

        let method_counter = self.metrics.methods
            .entry(method.to_string())
            .or_insert_with(|| AtomicU64::new(0));
        method_counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self, _error: &anyhow::Error) {
        // In a full implementation, we'd categorize different error types
        let error_counter = self.metrics.status_codes
            .entry(500)
            .or_insert_with(|| AtomicU64::new(0));
        error_counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_active_connections(&self) {
        self.metrics.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active_connections(&self) {
        self.metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_bytes_sent(&self, bytes: u64) {
        self.metrics.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_bytes_received(&self, bytes: u64) {
        self.metrics.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: self.metrics.requests_total.load(Ordering::Relaxed),
            requests_duration_ms: self.metrics.requests_duration_seconds.load(Ordering::Relaxed),
            active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
            bytes_sent: self.metrics.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.metrics.bytes_received.load(Ordering::Relaxed),
            status_codes: self.metrics.status_codes
                .iter()
                .map(|entry| (*entry.key(), entry.value().load(Ordering::Relaxed)))
                .collect(),
            methods: self.metrics.methods
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
                .collect(),
        }
    }

    #[cfg(feature = "metrics")]
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.get_metrics_snapshot();
        let mut output = String::new();

        output.push_str("# HELP rustweb_requests_total Total number of HTTP requests\n");
        output.push_str("# TYPE rustweb_requests_total counter\n");
        output.push_str(&format!("rustweb_requests_total {}\n", snapshot.requests_total));

        output.push_str("# HELP rustweb_active_connections Currently active connections\n");
        output.push_str("# TYPE rustweb_active_connections gauge\n");
        output.push_str(&format!("rustweb_active_connections {}\n", snapshot.active_connections));

        output.push_str("# HELP rustweb_bytes_sent_total Total bytes sent\n");
        output.push_str("# TYPE rustweb_bytes_sent_total counter\n");
        output.push_str(&format!("rustweb_bytes_sent_total {}\n", snapshot.bytes_sent));

        output.push_str("# HELP rustweb_bytes_received_total Total bytes received\n");
        output.push_str("# TYPE rustweb_bytes_received_total counter\n");
        output.push_str(&format!("rustweb_bytes_received_total {}\n", snapshot.bytes_received));

        for (status, count) in &snapshot.status_codes {
            output.push_str(&format!(
                "rustweb_requests_total{{status=\"{}\"}} {}\n",
                status, count
            ));
        }

        for (method, count) in &snapshot.methods {
            output.push_str(&format!(
                "rustweb_requests_total{{method=\"{}\"}} {}\n",
                method, count
            ));
        }

        output
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub requests_total: u64,
    pub requests_duration_ms: u64,
    pub active_connections: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub status_codes: std::collections::HashMap<u16, u64>,
    pub methods: std::collections::HashMap<String, u64>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}