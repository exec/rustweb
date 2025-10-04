# RustWeb - High-Performance HTTP Server

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

RustWeb is a modern, high-performance HTTP server written in Rust, designed to replace nginx/httpd while following Linux philosophy principles. It provides enterprise-grade features with memory safety, high concurrency, and exceptional performance.

## 🚀 Features

### ✅ Implemented
- **HTTP/1.1 & HTTP/2** - Full protocol support with ALPN negotiation
- **Static File Serving** - Efficient static content delivery with MIME type detection
- **Virtual Hosts** - Multiple domains/sites on a single server
- **Reverse Proxy** - Load balancing with health checks and multiple algorithms
- **SSL/TLS Termination** - rustls-based TLS with modern cipher suites
- **Security** - Built-in rate limiting, security headers, and path traversal protection
- **Compression** - Gzip, Brotli, and Zstd compression with content negotiation
- **Configuration** - TOML-based configuration with validation and hot reload
- **Signal Handling** - Graceful shutdown and configuration reload
- **Metrics** - Performance monitoring and request statistics
- **CLI Interface** - Full command-line interface with argument parsing
- **Access Logging** - Structured logging with request IDs and timing
- **Packaging** - Debian (.deb), RPM, and Docker packages
- **Systemd Integration** - Full systemd service support

### 🔄 Planned
- **HTTP/3 & QUIC** - Next-generation protocol support (architecture ready)
- **WebSocket Proxying** - Full WebSocket support
- **Advanced Health Checks** - Custom health check endpoints
- **Prometheus Metrics** - Native Prometheus metrics export

## 🏗️ Architecture

RustWeb is built with a modular architecture following Rust best practices:

```
src/
├── main.rs              # Entry point and CLI
├── config.rs            # Configuration management
├── server/              # Core HTTP server
│   ├── http_server.rs   # Server orchestration
│   ├── request_handler.rs # Request routing and handling
│   ├── static_files.rs  # Static file serving
│   └── response.rs      # Response building
├── security/            # Security features
├── compression/         # Compression algorithms
├── proxy/              # Reverse proxy and load balancing
└── metrics/            # Performance monitoring
```

## 📦 Installation

### 🚀 Quick Install (Linux)
```bash
# Download and run the installation script
curl -sSL https://github.com/your-org/rustweb/raw/main/install.sh | sudo bash

# Or with specific version
RUSTWEB_VERSION=1.0.0 curl -sSL https://github.com/your-org/rustweb/raw/main/install.sh | sudo bash
```

### 📱 Package Managers

#### Debian/Ubuntu (.deb)
```bash
# Download the .deb package
wget https://github.com/your-org/rustweb/releases/latest/download/rustweb_0.1.0_amd64.deb

# Install
sudo dpkg -i rustweb_0.1.0_amd64.deb
sudo apt-get install -f  # Fix dependencies if needed

# Start the service
sudo systemctl start rustweb
sudo systemctl enable rustweb
```

#### Red Hat/CentOS/Fedora (.rpm)
```bash
# Download the .rpm package
wget https://github.com/your-org/rustweb/releases/latest/download/rustweb-0.1.0-1.x86_64.rpm

# Install
sudo rpm -ivh rustweb-0.1.0-1.x86_64.rpm

# Start the service
sudo systemctl start rustweb
sudo systemctl enable rustweb
```

### 🐳 Docker
```bash
# Quick start with Docker
docker run -d \
  --name rustweb \
  -p 80:8080 \
  -p 443:8443 \
  -v /var/www/html:/var/www/html:ro \
  ghcr.io/your-org/rustweb:latest

# Or use Docker Compose
curl -O https://github.com/your-org/rustweb/raw/main/docker-compose.yml
docker-compose up -d
```

### 🛠️ From Source
```bash
# Clone the repository
git clone https://github.com/your-org/rustweb.git
cd rustweb

# Build release binary
cargo build --release

# The binary will be at ./target/release/rustweb
sudo cp target/release/rustweb /usr/bin/
sudo cp rustweb.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable rustweb
```

### Dependencies
- Rust 1.70+ (2021 edition)
- OpenSSL or equivalent for TLS support
- Linux kernel 3.9+ (for modern async I/O)

## 🎯 Quick Start

1. **Create a configuration file** (optional - defaults available):
```toml
# rustweb.toml
[server]
listen = ["0.0.0.0:8080"]

[virtual_hosts.default]
server_name = ["*"]
document_root = "./www"
```

2. **Create content directory**:
```bash
mkdir www
echo "<h1>Hello, RustWeb!</h1>" > www/index.html
```

3. **Start the server**:
```bash
./target/release/rustweb -c rustweb.toml
```

4. **Test it**:
```bash
curl http://localhost:8080/
```

## ⚙️ Configuration

### Production Configuration
```toml
[server]
listen = ["0.0.0.0:80", "0.0.0.0:443"]
listen_quic = ["0.0.0.0:443"]  # HTTP/3 support
enable_http3 = true
max_connections = 10000
keep_alive_timeout = 65

[logging]
log_level = "info"
access_log = "/var/log/rustweb/access.log"
error_log = "/var/log/rustweb/error.log"

[security]
enable_rate_limiting = true
rate_limit_requests_per_second = 1000
rate_limit_burst = 2000
allowed_methods = ["GET", "POST", "HEAD", "PUT", "DELETE", "PATCH", "OPTIONS"]

[security.security_headers]
"X-Frame-Options" = "DENY"
"X-Content-Type-Options" = "nosniff"
"Strict-Transport-Security" = "max-age=31536000; includeSubDomains"

[compression]
enable_gzip = true
enable_brotli = true
enable_zstd = true
compression_level = 6

[virtual_hosts.default]
server_name = ["_"]
document_root = "/var/www/html"
index_files = ["index.html", "index.htm"]

[virtual_hosts.default.ssl]
certificate = "/etc/rustweb/ssl/cert.pem"
private_key = "/etc/rustweb/ssl/key.pem"
protocols = ["TLSv1.2", "TLSv1.3"]
```

### Virtual Hosts
```toml
[virtual_hosts.example]
server_name = ["example.com", "www.example.com"]
document_root = "/var/www/example.com"
index_files = ["index.html"]

[virtual_hosts.example.ssl]
certificate = "/etc/rustweb/ssl/example.com.pem"
private_key = "/etc/rustweb/ssl/example.com.key"
protocols = ["TLSv1.2", "TLSv1.3"]

[virtual_hosts.example.locations."/api/"]
proxy_pass = "backend"
```

### Reverse Proxy
```toml
[upstream.backend]
servers = ["http://127.0.0.1:3000", "http://127.0.0.1:3001"]
load_balancing = "RoundRobin"

[upstream.backend.health_check]
path = "/health"
interval = 30
timeout = 5
```

## 📋 Command Line Options

```bash
# Basic usage
rustweb -c config.toml

# Test configuration
rustweb -t -c config.toml

# Override listen address
rustweb -h 0.0.0.0 -p 8080

# Enable verbose logging
rustweb -v -c config.toml

# Run as daemon (Unix only)
rustweb -d -c config.toml
```

**Options:**
- `-c, --config <FILE>` - Configuration file path
- `-h, --host <HOST>` - Listen host (default: 0.0.0.0)
- `-p, --port <PORT>` - Listen port (default: 8080)
- `-t, --test-config` - Test configuration and exit
- `-v, --verbose` - Enable verbose logging
- `-d, --daemon` - Run as daemon (Unix only)

## 🔧 Operations

### Signal Handling
- `SIGTERM/SIGINT` - Graceful shutdown
- `SIGQUIT` - Immediate shutdown

### Graceful Shutdown
```bash
# Send SIGTERM for graceful shutdown
kill -TERM $(pidof rustweb)
```

### Health Monitoring
Built-in metrics collection and optional Prometheus export:
```bash
# Access metrics endpoint (if enabled)
curl http://localhost:8080/metrics
```

## 🚀 Performance

### Benchmarks (Target)
- **Static files**: 50k+ RPS on commodity hardware
- **Reverse proxy**: 30k+ RPS with <1ms upstream latency
- **Memory usage**: <1MB per 1000 idle connections
- **CPU usage**: <5% at 10k RPS on modern CPU

### Optimizations
- Zero-copy I/O where possible
- Connection pooling and reuse
- Efficient HTTP parsing
- SIMD optimizations
- Custom memory allocator support

## 🛡️ Security

### Built-in Protection
- **Memory Safety** - Rust's ownership system prevents buffer overflows
- **Path Traversal Protection** - Validates and sanitizes file paths
- **Rate Limiting** - Configurable per-client request limits
- **Security Headers** - Automatic security headers (HSTS, CSP, etc.)
- **Input Validation** - All inputs validated and sanitized

### Security Headers (Default)
- `X-Frame-Options: DENY`
- `X-Content-Type-Options: nosniff`
- `X-XSS-Protection: 1; mode=block`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains`

## 📊 Monitoring

### Request Logging
```
[2025-09-24T09:45:57.310258Z] INFO Request completed 
  request_id=90db1599-500f-4f2b-bd81-9c8ddd8b7ca3 
  status=200 OK 
  duration_ms=0
```

### Metrics Collection
- Request count and timing
- Status code distribution
- Active connections
- Bytes sent/received
- Error rates

## 🧪 Testing

### Manual Testing
```bash
# Test static files
curl http://localhost:8080/

# Test virtual hosts
curl -H "Host: example.com" http://localhost:8080/

# Test compression
curl -H "Accept-Encoding: gzip" http://localhost:8080/

# Test security
curl http://localhost:8080/../../../etc/passwd
```

### Load Testing
```bash
# Example with Apache Bench
ab -n 10000 -c 100 http://localhost:8080/

# Example with wrk
wrk -t12 -c400 -d30s http://localhost:8080/
```

## 🤝 Contributing

RustWeb follows standard Rust development practices:

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Run tests
cargo test

# Build documentation
cargo doc --open
```

## 📝 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🙋‍♀️ Support

- 📖 Documentation: See [CLAUDE.md](CLAUDE.md) for technical specifications
- 🐛 Issues: Report bugs and feature requests via GitHub Issues
- 💬 Discussions: Join the community discussions

---

**RustWeb** - *Fast, Safe, Reliable* 🦀