use crate::config::{Config, LoadBalancingMethod, UpstreamConfig};
use crate::server::response::ErrorResponse;
use anyhow::{Context, Result};
use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, Uri};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub struct ProxyHandler {
    config: Arc<Config>,
    upstreams: HashMap<String, Arc<UpstreamPool>>,
}

pub struct UpstreamPool {
    servers: Vec<UpstreamServer>,
    load_balancer: LoadBalancer,
    config: UpstreamConfig,
}

pub struct UpstreamServer {
    url: String,
    healthy: AtomicUsize, // 1 = healthy, 0 = unhealthy
    connections: AtomicUsize,
    last_check: std::sync::Mutex<Instant>,
}

pub struct LoadBalancer {
    method: LoadBalancingMethod,
    counter: AtomicUsize,
}

impl ProxyHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let mut upstreams = HashMap::new();

        for (name, upstream_config) in &config.upstream {
            let servers = upstream_config
                .servers
                .iter()
                .map(|url| UpstreamServer {
                    url: url.clone(),
                    healthy: AtomicUsize::new(1),
                    connections: AtomicUsize::new(0),
                    last_check: std::sync::Mutex::new(Instant::now()),
                })
                .collect();

            let pool = UpstreamPool {
                servers,
                load_balancer: LoadBalancer {
                    method: upstream_config.load_balancing.clone(),
                    counter: AtomicUsize::new(0),
                },
                config: upstream_config.clone(),
            };

            upstreams.insert(name.clone(), Arc::new(pool));
        }

        Ok(Self { config, upstreams })
    }

    pub async fn proxy_request(
        &self,
        mut req: Request<Incoming>,
        proxy_pass: &str,
    ) -> Result<Response<Full<Bytes>>> {
        let upstream_pool = self
            .upstreams
            .get(proxy_pass)
            .ok_or_else(|| anyhow::anyhow!("Unknown upstream: {}", proxy_pass))?;

        let upstream_server = upstream_pool
            .select_server()
            .ok_or_else(|| anyhow::anyhow!("No healthy upstream servers available"))?;

        upstream_server.connections.fetch_add(1, Ordering::Relaxed);

        let result = self
            .forward_request(req, upstream_server, &upstream_pool.config)
            .await;

        upstream_server.connections.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                // Mark server as unhealthy on error
                upstream_server.healthy.store(0, Ordering::Relaxed);
                tracing::error!("Proxy request failed: {}", e);
                Ok(ErrorResponse::bad_gateway().build())
            }
        }
    }

    async fn forward_request(
        &self,
        mut req: Request<Incoming>,
        upstream_server: &UpstreamServer,
        upstream_config: &UpstreamConfig,
    ) -> Result<Response<Full<Bytes>>> {
        let upstream_uri: Uri = upstream_server
            .url
            .parse()
            .context("Invalid upstream URL")?;

        let original_uri = req.uri().clone();
        let path_and_query = original_uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        let new_uri = format!(
            "{}://{}{}",
            upstream_uri.scheme_str().unwrap_or("http"),
            upstream_uri.authority().unwrap(),
            path_and_query
        )
        .parse::<Uri>()
        .context("Failed to construct upstream URI")?;

        *req.uri_mut() = new_uri;

        // Remove connection-specific headers
        req.headers_mut().remove("connection");
        req.headers_mut().remove("upgrade");
        req.headers_mut().remove("proxy-connection");

        // Add forwarding headers
        if let Some(addr) = req.headers().get("x-forwarded-for").cloned() {
            req.headers_mut().insert("x-forwarded-for", addr);
        } else if let Some(remote_addr) = req.extensions().get::<std::net::SocketAddr>().copied() {
            req.headers_mut().insert(
                "x-forwarded-for",
                remote_addr.ip().to_string().parse().unwrap(),
            );
        }

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response_future = client.request(req);
        let response = timeout(
            Duration::from_millis(upstream_config.read_timeout),
            response_future,
        )
        .await
        .context("Upstream request timeout")?
        .context("Upstream request failed")?;

        // Collect the response body
        let (parts, body) = response.into_parts();
        let body_bytes = body
            .collect()
            .await
            .context("Failed to read response body")?
            .to_bytes();

        let response = Response::from_parts(parts, Full::new(body_bytes));

        Ok(response)
    }

    pub async fn health_check(&self) {
        for (name, pool) in &self.upstreams {
            if let Some(health_config) = &pool.config.health_check {
                for server in &pool.servers {
                    let now = Instant::now();
                    let should_check = {
                        let mut last_check = server.last_check.lock().unwrap();
                        if now.duration_since(*last_check).as_secs() >= health_config.interval {
                            *last_check = now;
                            true
                        } else {
                            false
                        }
                    };

                    if should_check {
                        let is_healthy = self.check_server_health(server, health_config).await;
                        server
                            .healthy
                            .store(if is_healthy { 1 } else { 0 }, Ordering::Relaxed);

                        tracing::debug!(
                            "Health check for {}/{}: {}",
                            name,
                            server.url,
                            if is_healthy { "healthy" } else { "unhealthy" }
                        );
                    }
                }
            }
        }
    }

    async fn check_server_health(
        &self,
        server: &UpstreamServer,
        health_config: &crate::config::HealthCheckConfig,
    ) -> bool {
        let health_url = format!("{}{}", server.url, health_config.path);

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let request = Request::builder()
            .uri(health_url)
            .method("GET")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let response_future = client.request(request);

        match timeout(
            Duration::from_millis(health_config.timeout),
            response_future,
        )
        .await
        {
            Ok(Ok(response)) => response.status().is_success(),
            _ => false,
        }
    }
}

impl UpstreamPool {
    fn select_server(&self) -> Option<&UpstreamServer> {
        let healthy_servers: Vec<_> = self
            .servers
            .iter()
            .filter(|s| s.healthy.load(Ordering::Relaxed) == 1)
            .collect();

        if healthy_servers.is_empty() {
            return None;
        }

        match self.load_balancer.method {
            LoadBalancingMethod::RoundRobin => {
                let index = self.load_balancer.counter.fetch_add(1, Ordering::Relaxed);
                Some(healthy_servers[index % healthy_servers.len()])
            }
            LoadBalancingMethod::LeastConnections => healthy_servers
                .iter()
                .min_by_key(|s| s.connections.load(Ordering::Relaxed))
                .copied(),
            LoadBalancingMethod::Random => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};

                let mut hasher = DefaultHasher::new();
                std::time::SystemTime::now().hash(&mut hasher);
                let index = (hasher.finish() as usize) % healthy_servers.len();
                Some(healthy_servers[index])
            }
            LoadBalancingMethod::IpHash => {
                // For IP hash, we'd need the client IP, which we don't have here
                // Fall back to round robin
                let index = self.load_balancer.counter.fetch_add(1, Ordering::Relaxed);
                Some(healthy_servers[index % healthy_servers.len()])
            }
        }
    }
}
