use crate::metrics::MetricsCollector;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::Arc;

pub struct PrometheusExporter {
    metrics: Arc<MetricsCollector>,
}

impl PrometheusExporter {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self { metrics }
    }

    pub fn export_metrics(&self) -> Response<Full<Bytes>> {
        let snapshot = self.metrics.get_metrics_snapshot();
        let mut output = String::new();

        // Help and type information
        output.push_str("# HELP rustweb_requests_total Total number of HTTP requests\n");
        output.push_str("# TYPE rustweb_requests_total counter\n");
        output.push_str(&format!(
            "rustweb_requests_total {}\n",
            snapshot.requests_total
        ));
        output.push('\n');

        output.push_str(
            "# HELP rustweb_request_duration_milliseconds Total time spent processing requests\n",
        );
        output.push_str("# TYPE rustweb_request_duration_milliseconds counter\n");
        output.push_str(&format!(
            "rustweb_request_duration_milliseconds {}\n",
            snapshot.requests_duration_ms
        ));
        output.push('\n');

        output.push_str("# HELP rustweb_active_connections Currently active connections\n");
        output.push_str("# TYPE rustweb_active_connections gauge\n");
        output.push_str(&format!(
            "rustweb_active_connections {}\n",
            snapshot.active_connections
        ));
        output.push('\n');

        output.push_str("# HELP rustweb_bytes_sent_total Total bytes sent to clients\n");
        output.push_str("# TYPE rustweb_bytes_sent_total counter\n");
        output.push_str(&format!(
            "rustweb_bytes_sent_total {}\n",
            snapshot.bytes_sent
        ));
        output.push('\n');

        output.push_str("# HELP rustweb_bytes_received_total Total bytes received from clients\n");
        output.push_str("# TYPE rustweb_bytes_received_total counter\n");
        output.push_str(&format!(
            "rustweb_bytes_received_total {}\n",
            snapshot.bytes_received
        ));
        output.push('\n');

        // Status code metrics
        output.push_str(
            "# HELP rustweb_requests_by_status_total Total requests by HTTP status code\n",
        );
        output.push_str("# TYPE rustweb_requests_by_status_total counter\n");
        for (status, count) in &snapshot.status_codes {
            output.push_str(&format!(
                "rustweb_requests_by_status_total{{status=\"{}\"}} {}\n",
                status, count
            ));
        }
        output.push('\n');

        // Method metrics
        output.push_str("# HELP rustweb_requests_by_method_total Total requests by HTTP method\n");
        output.push_str("# TYPE rustweb_requests_by_method_total counter\n");
        for (method, count) in &snapshot.methods {
            output.push_str(&format!(
                "rustweb_requests_by_method_total{{method=\"{}\"}} {}\n",
                method, count
            ));
        }
        output.push('\n');

        // System metrics
        output.push_str("# HELP rustweb_build_info Build information\n");
        output.push_str("# TYPE rustweb_build_info gauge\n");
        output.push_str(&format!(
            "rustweb_build_info{{version=\"{}\",rustc_version=\"{}\"}} 1\n",
            env!("CARGO_PKG_VERSION"),
            "1.70+" // Could be made dynamic
        ));
        output.push('\n');

        // Process metrics
        output.push_str("# HELP rustweb_start_time_seconds Unix time when the server started\n");
        output.push_str("# TYPE rustweb_start_time_seconds gauge\n");
        output.push_str(&format!(
            "rustweb_start_time_seconds {}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .header("cache-control", "no-cache")
            .body(Full::new(Bytes::from(output)))
            .unwrap()
    }

    pub fn health_check(&self) -> Response<Full<Bytes>> {
        let health_status = serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "active_connections": self.metrics.get_metrics_snapshot().active_connections
        });

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .header("cache-control", "no-cache")
            .body(Full::new(Bytes::from(health_status.to_string())))
            .unwrap()
    }
}
