use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    pub compression: CompressionConfig,
    pub upstream: HashMap<String, UpstreamConfig>,
    pub virtual_hosts: HashMap<String, VirtualHostConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub listen: Vec<String>,
    pub listen_quic: Option<Vec<String>>, // HTTP/3 QUIC listeners
    pub worker_threads: Option<usize>,
    pub max_connections: usize,
    pub keep_alive_timeout: u64,
    pub request_timeout: u64,
    pub send_timeout: u64,
    pub client_body_timeout: u64,
    pub client_header_timeout: u64,
    pub client_max_body_size: usize,
    pub tcp_nodelay: bool,
    pub tcp_fastopen: bool,
    pub enable_http3: bool,
    pub http3_max_concurrent_streams: Option<u64>,
    pub http3_max_frame_size: Option<u64>,
    pub http3_initial_connection_window_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub access_log: Option<String>,
    pub error_log: Option<String>,
    pub log_level: String,
    pub access_log_format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub enable_rate_limiting: bool,
    pub rate_limit_requests_per_second: u32,
    pub rate_limit_burst: u32,
    pub security_headers: HashMap<String, String>,
    pub allowed_methods: Vec<String>,
    pub max_request_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionConfig {
    pub enable_gzip: bool,
    pub enable_brotli: bool,
    pub enable_zstd: bool,
    pub compression_level: u32,
    pub min_compress_size: usize,
    pub compress_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    pub servers: Vec<String>,
    pub load_balancing: LoadBalancingMethod,
    pub health_check: Option<HealthCheckConfig>,
    pub connection_timeout: u64,
    pub read_timeout: u64,
    pub max_connections: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LoadBalancingMethod {
    RoundRobin,
    LeastConnections,
    IpHash,
    Random,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    pub path: String,
    pub interval: u64,
    pub timeout: u64,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VirtualHostConfig {
    pub server_name: Vec<String>,
    pub document_root: Option<String>,
    pub index_files: Vec<String>,
    pub proxy_pass: Option<String>,
    pub ssl: Option<SslConfig>,
    pub locations: HashMap<String, LocationConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SslConfig {
    pub certificate: String,
    pub private_key: String,
    pub certificate_chain: Option<String>,
    pub protocols: Vec<String>,
    pub ciphers: Option<String>,
    #[serde(default)]
    pub auto_generate_self_signed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocationConfig {
    pub document_root: Option<String>,
    pub proxy_pass: Option<String>,
    pub return_code: Option<u16>,
    pub return_url: Option<String>,
    pub auth_basic: Option<String>,
    pub auth_basic_user_file: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
            compression: CompressionConfig::default(),
            upstream: HashMap::new(),
            virtual_hosts: HashMap::new(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: vec!["0.0.0.0:8080".to_string()],
            listen_quic: Some(vec!["0.0.0.0:8443".to_string()]), // Default HTTP/3 on 8443
            worker_threads: None,
            max_connections: 10000,
            keep_alive_timeout: 65,
            request_timeout: 60,
            send_timeout: 60,
            client_body_timeout: 60,
            client_header_timeout: 60,
            client_max_body_size: 1024 * 1024, // 1MB
            tcp_nodelay: true,
            tcp_fastopen: false,
            enable_http3: true,
            http3_max_concurrent_streams: Some(256),
            http3_max_frame_size: Some(16384), // 16KB
            http3_initial_connection_window_size: Some(1048576), // 1MB
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            access_log: Some("/var/log/rustweb/access.log".to_string()),
            error_log: Some("/var/log/rustweb/error.log".to_string()),
            log_level: "info".to_string(),
            access_log_format: "$remote_addr - $remote_user [$time_local] \"$request\" $status $body_bytes_sent \"$http_referer\" \"$http_user_agent\"".to_string(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut headers = HashMap::new();
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert("X-XSS-Protection".to_string(), "1; mode=block".to_string());
        headers.insert(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains".to_string(),
        );

        Self {
            enable_rate_limiting: true,
            rate_limit_requests_per_second: 100,
            rate_limit_burst: 200,
            security_headers: headers,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "HEAD".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
            ],
            max_request_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enable_gzip: true,
            enable_brotli: true,
            enable_zstd: false,
            compression_level: 6,
            min_compress_size: 1024,
            compress_types: vec![
                "text/html".to_string(),
                "text/css".to_string(),
                "text/javascript".to_string(),
                "application/javascript".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "text/xml".to_string(),
                "text/plain".to_string(),
            ],
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        config.validate()?;
        Ok(config)
    }

    pub fn default_with_host_port(host: &str, port: u16) -> Self {
        let mut config = Config::default();
        config.server.listen = vec![format!("{}:{}", host, port)];
        config
    }

    pub fn validate(&self) -> Result<()> {
        for listen_addr in &self.server.listen {
            listen_addr
                .parse::<SocketAddr>()
                .with_context(|| format!("Invalid listen address: {}", listen_addr))?;
        }

        if self.server.max_connections == 0 {
            return Err(anyhow::anyhow!("max_connections must be greater than 0"));
        }

        if self.server.worker_threads.map_or(false, |n| n == 0) {
            return Err(anyhow::anyhow!("worker_threads must be greater than 0"));
        }

        for (name, upstream) in &self.upstream {
            if upstream.servers.is_empty() {
                return Err(anyhow::anyhow!(
                    "Upstream '{}' has no servers configured",
                    name
                ));
            }
        }

        Ok(())
    }

    pub fn listen_addresses(&self) -> Result<Vec<SocketAddr>> {
        self.server
            .listen
            .iter()
            .map(|addr| {
                addr.parse()
                    .with_context(|| format!("Invalid listen address: {}", addr))
            })
            .collect()
    }

    pub fn quic_listen_addresses(&self) -> Result<Vec<SocketAddr>> {
        if let Some(ref quic_addresses) = self.server.listen_quic {
            quic_addresses
                .iter()
                .map(|addr| {
                    addr.parse()
                        .with_context(|| format!("Invalid QUIC listen address: {}", addr))
                })
                .collect()
        } else {
            Ok(vec![])
        }
    }
}
