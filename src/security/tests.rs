#[cfg(test)]
mod tests {
    use crate::security::SecurityHandler;
    use crate::config::{Config, SecurityConfig};
    use hyper::Method;
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.security.enable_rate_limiting = true;
        config.security.rate_limit_requests_per_second = 10;
        config.security.rate_limit_burst = 20;
        config
    }

    #[tokio::test]
    async fn test_security_handler_creation() {
        let config = create_test_config();
        let handler = SecurityHandler::new(Arc::new(config));
        assert!(handler.is_ok());
    }

    #[test]
    fn test_allowed_methods() {
        let config = create_test_config();
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();

        // Test allowed methods
        assert!(handler.check_method(&Method::GET));
        assert!(handler.check_method(&Method::POST));
        assert!(handler.check_method(&Method::HEAD));
        assert!(handler.check_method(&Method::PUT));
        assert!(handler.check_method(&Method::DELETE));
        assert!(handler.check_method(&Method::OPTIONS));

        // Test disallowed methods  
        assert!(!handler.check_method(&Method::TRACE));
        assert!(!handler.check_method(&Method::CONNECT));
    }

    #[test]
    fn test_custom_allowed_methods() {
        let mut config = create_test_config();
        config.security.allowed_methods = vec!["GET".to_string(), "POST".to_string()];
        
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();

        assert!(handler.check_method(&Method::GET));
        assert!(handler.check_method(&Method::POST));
        assert!(!handler.check_method(&Method::PUT));
        assert!(!handler.check_method(&Method::DELETE));
    }

    #[tokio::test]
    async fn test_rate_limiting_enabled() {
        let config = create_test_config();
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();
        
        let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);

        // First few requests should be allowed
        for _ in 0..5 {
            let allowed = handler.check_rate_limit(client_addr).await;
            assert!(allowed, "Request should be allowed within rate limit");
        }
    }

    #[tokio::test]
    async fn test_rate_limiting_disabled() {
        let mut config = create_test_config();
        config.security.enable_rate_limiting = false;
        
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();
        let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);

        // All requests should be allowed when rate limiting is disabled
        for _ in 0..100 {
            let allowed = handler.check_rate_limit(client_addr).await;
            assert!(allowed, "Request should be allowed when rate limiting is disabled");
        }
    }

    #[test]
    fn test_add_security_headers() {
        let config = create_test_config();
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();

        use hyper::{Response, StatusCode};
        use http_body_util::Full;
        use bytes::Bytes;

        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("test")))
            .unwrap();

        let response_with_headers = handler.add_security_headers(response);

        // Check that security headers were added
        assert!(response_with_headers.headers().contains_key("x-frame-options"));
        assert!(response_with_headers.headers().contains_key("x-content-type-options"));
        assert!(response_with_headers.headers().contains_key("x-xss-protection"));
        assert!(response_with_headers.headers().contains_key("strict-transport-security"));

        // Check specific header values
        assert_eq!(
            response_with_headers.headers().get("x-frame-options").unwrap(),
            "DENY"
        );
        assert_eq!(
            response_with_headers.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
    }

    #[test]
    fn test_custom_security_headers() {
        let mut config = create_test_config();
        let mut custom_headers = HashMap::new();
        custom_headers.insert("Custom-Header".to_string(), "CustomValue".to_string());
        custom_headers.insert("X-Frame-Options".to_string(), "SAMEORIGIN".to_string());
        config.security.security_headers = custom_headers;

        let handler = SecurityHandler::new(Arc::new(config)).unwrap();

        use hyper::{Response, StatusCode};
        use http_body_util::Full;
        use bytes::Bytes;

        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("test")))
            .unwrap();

        let response_with_headers = handler.add_security_headers(response);

        // Check custom headers were added
        assert_eq!(
            response_with_headers.headers().get("custom-header").unwrap(),
            "CustomValue"
        );
        
        // Check that custom value overrode default
        assert_eq!(
            response_with_headers.headers().get("x-frame-options").unwrap(),
            "SAMEORIGIN"
        );
    }

    #[test]
    fn test_validate_request_size() {
        let mut config = create_test_config();
        config.security.max_request_size = 1024; // 1KB limit
        
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();

        // Test valid sizes
        assert!(handler.validate_request_size(Some(500)));
        assert!(handler.validate_request_size(Some(1024)));
        assert!(handler.validate_request_size(None)); // No content-length header

        // Test invalid sizes
        assert!(!handler.validate_request_size(Some(2048)));
        assert!(!handler.validate_request_size(Some(10000)));
    }

    #[tokio::test]
    async fn test_rate_limiting_different_clients() {
        let config = create_test_config();
        let handler = SecurityHandler::new(Arc::new(config)).unwrap();
        
        let client1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
        let client2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 12346);

        // Both clients should be able to make requests independently
        for _ in 0..5 {
            let allowed1 = handler.check_rate_limit(client1).await;
            let allowed2 = handler.check_rate_limit(client2).await;
            
            assert!(allowed1, "Client 1 should be allowed");
            assert!(allowed2, "Client 2 should be allowed");
        }
    }

    #[test]
    fn test_security_config_defaults() {
        let security = SecurityConfig::default();
        
        assert!(security.enable_rate_limiting);
        assert_eq!(security.rate_limit_requests_per_second, 100);
        assert_eq!(security.rate_limit_burst, 200);
        assert_eq!(security.max_request_size, 10 * 1024 * 1024); // 10MB
        
        // Check default allowed methods
        let expected_methods = vec!["GET", "POST", "HEAD", "PUT", "DELETE"];
        for method in expected_methods {
            assert!(security.allowed_methods.contains(&method.to_string()));
        }
        
        // Check default security headers
        assert!(security.security_headers.contains_key("X-Frame-Options"));
        assert!(security.security_headers.contains_key("X-Content-Type-Options"));
        assert!(security.security_headers.contains_key("X-XSS-Protection"));
        assert!(security.security_headers.contains_key("Strict-Transport-Security"));
    }
}