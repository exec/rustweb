use crate::config::Config;
use anyhow::Result;
use bytes::Bytes;
use dashmap::DashMap;
use governor::{Jitter, Quota, RateLimiter};
use http_body_util::Full;
use hyper::{Method, Response};
use nonzero_ext::*;
use std::net::SocketAddr;
use std::sync::Arc;

#[cfg(test)]
mod tests;

pub struct SecurityHandler {
    config: Arc<Config>,
    rate_limiter: Option<
        RateLimiter<
            SocketAddr,
            dashmap::DashMap<SocketAddr, governor::state::InMemoryState>,
            governor::clock::DefaultClock,
        >,
    >,
}

impl SecurityHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let rate_limiter = if config.security.enable_rate_limiting {
            use std::num::NonZeroU32;
            use std::time::Duration;

            // Create a more restrictive quota for testing
            let rps = NonZeroU32::new(config.security.rate_limit_requests_per_second)
                .unwrap_or(NonZeroU32::new(1).unwrap());
            let burst = NonZeroU32::new(config.security.rate_limit_burst)
                .unwrap_or(NonZeroU32::new(1).unwrap());

            tracing::info!("Rate limiting enabled: {} RPS, burst {}", rps, burst);

            let quota = Quota::per_second(rps).allow_burst(burst);
            Some(RateLimiter::dashmap(quota))
        } else {
            tracing::info!("Rate limiting disabled");
            None
        };

        Ok(Self {
            config,
            rate_limiter,
        })
    }

    pub fn check_method(&self, method: &Method) -> bool {
        self.config
            .security
            .allowed_methods
            .iter()
            .any(|allowed| allowed == method.as_str())
    }

    pub async fn check_rate_limit(&self, client_addr: SocketAddr) -> bool {
        if let Some(ref limiter) = self.rate_limiter {
            match limiter.check_key(&client_addr) {
                Ok(_) => true,
                Err(err) => {
                    tracing::warn!(client = %client_addr, error = ?err, "Rate limit exceeded for client");
                    false
                }
            }
        } else {
            true
        }
    }

    pub fn add_security_headers(
        &self,
        mut response: Response<Full<Bytes>>,
    ) -> Response<Full<Bytes>> {
        let headers = response.headers_mut();

        for (name, value) in &self.config.security.security_headers {
            if let Ok(header_name) = name.parse::<hyper::header::HeaderName>() {
                if let Ok(header_value) = value.parse::<hyper::header::HeaderValue>() {
                    headers.insert(header_name, header_value);
                }
            }
        }

        headers.insert("server", "RustWeb/0.1.0".parse().unwrap());

        response
    }

    pub fn validate_request_size(&self, content_length: Option<usize>) -> bool {
        match content_length {
            Some(size) => size <= self.config.security.max_request_size,
            None => true, // Allow requests without content-length (e.g., chunked)
        }
    }
}
