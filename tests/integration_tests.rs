use rustweb::config::Config;

#[test]
fn test_config_creation() {
    let config = Config::default();
    assert_eq!(config.server.listen, vec!["0.0.0.0:8080"]);
    assert!(config.security.enable_rate_limiting);
}

#[test]
fn test_config_validation() {
    let config = Config::default();
    assert!(config.validate().is_ok());

    let mut invalid_config = Config::default();
    invalid_config.server.max_connections = 0;
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_listen_address_parsing() {
    let mut config = Config::default();
    config.server.listen = vec!["127.0.0.1:8080".to_string(), "0.0.0.0:8443".to_string()];

    let addresses = config.listen_addresses().unwrap();
    assert_eq!(addresses.len(), 2);
    assert_eq!(addresses[0].port(), 8080);
    assert_eq!(addresses[1].port(), 8443);
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    // Test server defaults
    assert_eq!(config.server.max_connections, 10000);
    assert_eq!(config.server.keep_alive_timeout, 65);
    assert!(config.server.tcp_nodelay);

    // Test security defaults
    assert!(config.security.enable_rate_limiting);
    assert_eq!(config.security.rate_limit_requests_per_second, 100);
    assert_eq!(config.security.rate_limit_burst, 200);

    // Test compression defaults
    assert!(config.compression.enable_gzip);
    assert!(config.compression.enable_brotli);
    assert!(!config.compression.enable_zstd);
    assert_eq!(config.compression.compression_level, 6);
}
