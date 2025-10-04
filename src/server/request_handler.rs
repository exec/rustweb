use crate::compression::CompressionHandler;
use crate::config::{Config, VirtualHostConfig};
use crate::logging::{AccessLogFormat, AccessLogger, LogEntry};
use crate::metrics::MetricsCollector;
use crate::proxy::ProxyHandler;
use crate::security::SecurityHandler;
use crate::server::response::{ErrorResponse, ResponseBuilder};
use crate::server::static_files::StaticFileHandler;
use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response, StatusCode, Uri};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct RequestHandler {
    config: Arc<Config>,
    static_handler: StaticFileHandler,
    security_handler: SecurityHandler,
    compression_handler: CompressionHandler,
    metrics: Arc<MetricsCollector>,
    proxy_handler: ProxyHandler,
    access_logger: Option<AccessLogger>,
}

impl RequestHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let metrics = Arc::new(MetricsCollector::new());

        // Initialize access logger based on configuration
        let access_logger = if let Some(ref access_log_path) = config.logging.access_log {
            let format = match config.logging.access_log_format.as_str() {
                "json" => AccessLogFormat::Json,
                "common" => AccessLogFormat::CommonLog,
                "combined" | _ => AccessLogFormat::Combined, // Default to combined
            };
            Some(AccessLogger::new(Some(access_log_path), format)?)
        } else {
            None
        };

        Ok(Self {
            static_handler: StaticFileHandler::new(),
            security_handler: SecurityHandler::new(config.clone())?,
            compression_handler: CompressionHandler::new(config.clone()),
            proxy_handler: ProxyHandler::new(config.clone())?,
            access_logger,
            config,
            metrics,
        })
    }

    pub async fn handle_request(
        &self,
        req: Request<Incoming>,
        client_addr: SocketAddr,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let request_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();

        let method = req.method().clone();
        let uri = req.uri().clone();

        // Extract headers for logging before consuming the request
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let referer = req
            .headers()
            .get("referer")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let result = self.process_request(req, client_addr, request_id).await;

        let duration = start_time.elapsed();

        let response = match result {
            Ok(response) => {
                self.metrics
                    .record_request(response.status(), &method, duration);
                response
            }
            Err(e) => {
                tracing::error!(
                    request_id = %request_id,
                    error = %e,
                    "Request processing failed"
                );
                self.metrics.record_error(&e);
                ErrorResponse::internal_server_error().build()
            }
        };

        tracing::info!(
            request_id = %request_id,
            status = %response.status(),
            duration_ms = duration.as_millis(),
            "Request completed"
        );

        // Log access entry
        if let Some(ref access_logger) = self.access_logger {
            let content_length = response
                .headers()
                .get("content-length")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let log_entry = LogEntry {
                request_id,
                remote_addr: client_addr.ip().to_string(),
                method: method.to_string(),
                uri: uri.to_string(),
                status: response.status().as_u16(),
                response_size: content_length,
                duration_ms: duration.as_millis() as f64,
                user_agent,
                referer,
                timestamp: chrono::Utc::now(),
            };

            access_logger.log(log_entry).await;
        }

        Ok(response)
    }

    async fn process_request(
        &self,
        req: Request<Incoming>,
        client_addr: SocketAddr,
        request_id: Uuid,
    ) -> Result<Response<Full<Bytes>>> {
        if !self.security_handler.check_method(req.method()) {
            return Ok(ErrorResponse::method_not_allowed().build());
        }

        if !self.security_handler.check_rate_limit(client_addr).await {
            return Ok(ErrorResponse::too_many_requests().build());
        }

        let host = self.get_host_from_request(&req);
        let vhost_config = self.get_virtual_host_config(&host);

        let path = req.uri().path();
        let location_config = self.find_location_config(vhost_config, path);

        // Save headers for compression before potentially moving req
        let accept_encoding = req
            .headers()
            .get("accept-encoding")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let response = if let Some(proxy_pass) = location_config
            .and_then(|l| l.proxy_pass.as_ref())
            .or_else(|| vhost_config.and_then(|v| v.proxy_pass.as_ref()))
        {
            self.handle_proxy_request(req, proxy_pass).await?
        } else if let Some(document_root) = location_config
            .and_then(|l| l.document_root.as_ref())
            .or_else(|| vhost_config.and_then(|v| v.document_root.as_ref()))
        {
            self.handle_static_request(&req, document_root, vhost_config)
                .await?
        } else {
            ErrorResponse::not_found().build()
        };

        let response = self.security_handler.add_security_headers(response);
        let response = self
            .compression_handler
            .compress_response_with_encoding(response, &accept_encoding)
            .await?;

        Ok(response)
    }

    fn get_host_from_request(&self, req: &Request<Incoming>) -> String {
        req.headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("default")
            .split(':')
            .next()
            .unwrap_or("default")
            .to_string()
    }

    fn get_virtual_host_config(&self, host: &str) -> Option<&VirtualHostConfig> {
        self.config.virtual_hosts.get(host).or_else(|| {
            self.config.virtual_hosts.values().find(|vhost| {
                vhost
                    .server_name
                    .iter()
                    .any(|name| name == "*" || self.matches_wildcard(name, host))
            })
        })
    }

    fn find_location_config<'a>(
        &self,
        vhost_config: Option<&'a VirtualHostConfig>,
        path: &str,
    ) -> Option<&'a crate::config::LocationConfig> {
        vhost_config?
            .locations
            .iter()
            .filter(|(pattern, _)| path.starts_with(pattern.as_str()))
            .max_by_key(|(pattern, _)| pattern.len())
            .map(|(_, config)| config)
    }

    fn matches_wildcard(&self, pattern: &str, host: &str) -> bool {
        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            host.ends_with(suffix) && host.len() > suffix.len()
        } else {
            pattern == host
        }
    }

    async fn handle_static_request(
        &self,
        req: &Request<Incoming>,
        document_root: &str,
        vhost_config: Option<&VirtualHostConfig>,
    ) -> Result<Response<Full<Bytes>>> {
        let default_index = vec!["index.html".to_string()];
        let index_files = vhost_config
            .map(|v| v.index_files.as_slice())
            .unwrap_or(&default_index);

        self.static_handler
            .serve_file(req, document_root, index_files)
            .await
    }

    async fn handle_proxy_request(
        &self,
        req: Request<Incoming>,
        proxy_pass: &str,
    ) -> Result<Response<Full<Bytes>>> {
        self.proxy_handler.proxy_request(req, proxy_pass).await
    }
}
