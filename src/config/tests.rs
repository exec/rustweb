#[cfg(test)]
mod tests {
    use crate::config::*;
    use std::collections::HashMap;
    use std::net::SocketAddr;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.server.listen, vec!["0.0.0.0:8080"]);
        assert_eq!(config.server.max_connections, 10000);
        assert!(config.security.enable_rate_limiting);
        assert_eq!(config.security.rate_limit_requests_per_second, 100);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_max_connections() {
        let mut config = Config::default();
        config.server.max_connections = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_worker_threads() {
        let mut config = Config::default();
        config.server.worker_threads = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_empty_upstream() {
        let mut config = Config::default();
        config.upstream.insert(
            "test".to_string(),
            UpstreamConfig {
                servers: vec![], // Empty servers list
                load_balancing: LoadBalancingMethod::RoundRobin,
                health_check: None,
                connection_timeout: 5000,
                read_timeout: 30000,
                max_connections: None,
            },
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_listen_addresses_parsing() {
        let mut config = Config::default();
        config.server.listen = vec!["127.0.0.1:8080".to_string(), "0.0.0.0:8443".to_string()];

        let addresses = config.listen_addresses().unwrap();
        assert_eq!(addresses.len(), 2);

        let expected_addr1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let expected_addr2: SocketAddr = "0.0.0.0:8443".parse().unwrap();

        assert_eq!(addresses[0], expected_addr1);
        assert_eq!(addresses[1], expected_addr2);
    }

    #[test]
    fn test_listen_addresses_parsing_invalid() {
        let mut config = Config::default();
        config.server.listen = vec!["invalid-address".to_string()];

        assert!(config.listen_addresses().is_err());
    }

    #[test]
    fn test_default_with_host_port() {
        let config = Config::default_with_host_port("192.168.1.1", 9090);
        assert_eq!(config.server.listen, vec!["192.168.1.1:9090"]);
    }

    #[test]
    fn test_load_balancing_methods() {
        // Test that all load balancing methods can be created
        let methods = vec![
            LoadBalancingMethod::RoundRobin,
            LoadBalancingMethod::LeastConnections,
            LoadBalancingMethod::IpHash,
            LoadBalancingMethod::Random,
        ];

        // This is mainly to ensure they can be constructed and used
        for method in methods {
            let upstream = UpstreamConfig {
                servers: vec!["http://127.0.0.1:3000".to_string()],
                load_balancing: method,
                health_check: None,
                connection_timeout: 5000,
                read_timeout: 30000,
                max_connections: None,
            };
            assert!(!upstream.servers.is_empty());
        }
    }

    #[test]
    fn test_ssl_config_equality() {
        let ssl_config1 = SslConfig {
            certificate: "cert.pem".to_string(),
            private_key: "key.pem".to_string(),
            certificate_chain: None,
            protocols: vec!["TLSv1.2".to_string(), "TLSv1.3".to_string()],
            ciphers: None,
            auto_generate_self_signed: false,
        };

        let ssl_config2 = SslConfig {
            certificate: "cert.pem".to_string(),
            private_key: "key.pem".to_string(),
            certificate_chain: None,
            protocols: vec!["TLSv1.2".to_string(), "TLSv1.3".to_string()],
            ciphers: None,
            auto_generate_self_signed: false,
        };

        assert_eq!(ssl_config1, ssl_config2);
    }

    #[test]
    fn test_virtual_host_config_creation() {
        let vhost = VirtualHostConfig {
            server_name: vec!["example.com".to_string(), "www.example.com".to_string()],
            document_root: Some("/var/www/example".to_string()),
            index_files: vec!["index.html".to_string(), "index.htm".to_string()],
            proxy_pass: None,
            ssl: None,
            locations: HashMap::new(),
        };

        assert_eq!(vhost.server_name.len(), 2);
        assert!(vhost.document_root.is_some());
        assert_eq!(vhost.index_files.len(), 2);
        assert!(vhost.locations.is_empty());
    }

    #[test]
    fn test_health_check_config() {
        let health_check = HealthCheckConfig {
            path: "/health".to_string(),
            interval: 30,
            timeout: 5,
            healthy_threshold: 2,
            unhealthy_threshold: 3,
        };

        assert_eq!(health_check.path, "/health");
        assert_eq!(health_check.interval, 30);
        assert_eq!(health_check.healthy_threshold, 2);
    }

    #[test]
    fn test_compression_config_defaults() {
        let compression = CompressionConfig::default();

        assert!(compression.enable_gzip);
        assert!(compression.enable_brotli);
        assert!(!compression.enable_zstd);
        assert_eq!(compression.compression_level, 6);
        assert_eq!(compression.min_compress_size, 1024);
        assert!(compression
            .compress_types
            .contains(&"text/html".to_string()));
        assert!(compression
            .compress_types
            .contains(&"application/json".to_string()));
    }

    #[test]
    fn test_security_config_defaults() {
        let security = SecurityConfig::default();

        assert!(security.enable_rate_limiting);
        assert_eq!(security.rate_limit_requests_per_second, 100);
        assert_eq!(security.rate_limit_burst, 200);
        assert!(security.security_headers.contains_key("X-Frame-Options"));
        assert!(security
            .security_headers
            .contains_key("Strict-Transport-Security"));
        assert!(security.allowed_methods.contains(&"GET".to_string()));
        assert!(security.allowed_methods.contains(&"POST".to_string()));
    }
}
