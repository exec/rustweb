use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustweb::config::Config;
use std::sync::Arc;

fn bench_config_creation(c: &mut Criterion) {
    c.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = Config::default();
            black_box(config);
        })
    });
}

fn bench_config_validation(c: &mut Criterion) {
    let config = Config::default();

    c.bench_function("config_validation", |b| {
        b.iter(|| {
            let result = config.validate();
            black_box(result);
        })
    });
}

fn bench_listen_addresses_parsing(c: &mut Criterion) {
    let mut config = Config::default();
    config.server.listen = vec![
        "127.0.0.1:8080".to_string(),
        "0.0.0.0:8443".to_string(),
        "192.168.1.1:9090".to_string(),
    ];

    c.bench_function("listen_addresses_parsing", |b| {
        b.iter(|| {
            let addresses = config.listen_addresses().unwrap();
            black_box(addresses);
        })
    });
}

fn bench_config_with_virtual_hosts(c: &mut Criterion) {
    let mut config = Config::default();

    // Add multiple virtual hosts to test scalability
    for i in 0..100 {
        let host_name = format!("example{}.com", i);
        config.virtual_hosts.insert(
            host_name.clone(),
            rustweb::config::VirtualHostConfig {
                server_name: vec![host_name],
                document_root: Some("/var/www/test".to_string()),
                index_files: vec!["index.html".to_string()],
                proxy_pass: None,
                ssl: None,
                locations: std::collections::HashMap::new(),
            },
        );
    }

    c.bench_function("config_with_100_virtual_hosts", |b| {
        b.iter(|| {
            let result = config.validate();
            black_box(result);
        })
    });
}

fn bench_upstream_config_creation(c: &mut Criterion) {
    c.bench_function("upstream_config_creation", |b| {
        b.iter(|| {
            let upstream = rustweb::config::UpstreamConfig {
                servers: vec![
                    "http://127.0.0.1:3000".to_string(),
                    "http://127.0.0.1:3001".to_string(),
                    "http://127.0.0.1:3002".to_string(),
                ],
                load_balancing: rustweb::config::LoadBalancingMethod::RoundRobin,
                health_check: Some(rustweb::config::HealthCheckConfig {
                    path: "/health".to_string(),
                    interval: 30,
                    timeout: 5,
                    healthy_threshold: 2,
                    unhealthy_threshold: 3,
                }),
                connection_timeout: 5000,
                read_timeout: 30000,
                max_connections: Some(100),
            };
            black_box(upstream);
        })
    });
}

criterion_group!(
    benches,
    bench_config_creation,
    bench_config_validation,
    bench_listen_addresses_parsing,
    bench_config_with_virtual_hosts,
    bench_upstream_config_creation
);

criterion_main!(benches);
