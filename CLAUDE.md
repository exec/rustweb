# RustWeb - A High-Performance HTTP Server

RustWeb is a modern, high-performance HTTP server written in Rust, designed to replace nginx/httpd while following Linux philosophy principles: do one thing well, be composable, and prioritize simplicity.

## Project Goals

### Core Principles (Linux Philosophy)
- **Do one thing well**: Focus on HTTP serving excellence
- **Composable**: Work seamlessly with other Unix tools
- **Simple configuration**: Human-readable, version-controllable config files
- **Fail fast**: Clear error messages and proper exit codes
- **Scriptable**: All functionality accessible via CLI

### Performance Targets
- Handle 100k+ concurrent connections (C10K+ problem)
- Sub-millisecond response times for static content
- Memory-efficient (target <50MB base memory usage)
- Zero-copy I/O where possible
- Efficient connection pooling and reuse

## Feature Set

### Phase 1: Core HTTP Server
- [x] HTTP/1.1 compliant server
- [x] Static file serving with proper MIME types
- [x] Basic request routing
- [x] Configuration file support (TOML)
- [x] Comprehensive logging (access + error)
- [x] Graceful shutdown/reload via signals

### Phase 2: Essential Features
- [x] Virtual hosts / server blocks
- [x] Reverse proxy with upstream health checks
- [x] SSL/TLS termination (rustls)
- [x] Compression (gzip, brotli, zstd)
- [x] Rate limiting and DDoS protection
- [x] Basic security headers

### Phase 3: Advanced Features  
- [x] HTTP/2 support
- [x] Load balancing algorithms
- [x] WebSocket proxying
- [x] Performance metrics and monitoring
- [x] Hot configuration reload
- [x] Request/response transformation

### Phase 4: Operations
- [ ] Systemd integration
- [ ] Docker containerization
- [ ] Prometheus metrics export
- [ ] Health check endpoints
- [ ] Configuration validation
- [ ] Performance profiling tools

## Architecture

### Core Components
- **Server Core**: Tokio-based async runtime with connection handling
- **Config Manager**: TOML-based configuration with hot reload
- **Router**: Fast request routing with pattern matching
- **Upstream Manager**: Connection pooling and health checks for proxying
- **Security Module**: Rate limiting, access control, security headers
- **Compression Engine**: Multi-algorithm compression with content negotiation
- **Logger**: Structured logging with configurable formats
- **Metrics**: Performance monitoring and stats collection

### Configuration Philosophy
- Single configuration file (rustweb.toml)
- Environment variable overrides
- Configuration validation on startup
- Hot reload without dropping connections
- Sensible defaults requiring minimal configuration

## Build Requirements

### Dependencies
- Rust 1.70+ (2021 edition)
- Key crates: tokio, hyper, rustls, serde, clap, toml
- Optional: jemalloc for memory management

### Development Commands
- `cargo run` - Start development server
- `cargo test` - Run test suite
- `cargo bench` - Performance benchmarks
- `cargo clippy` - Linting
- `cargo fmt` - Code formatting

### Deployment
- Single static binary
- No external dependencies
- Configuration via `/etc/rustweb/rustweb.toml`
- Logs to stdout/stderr (follows 12-factor principles)

## Security Considerations

### Built-in Security
- Memory safety (Rust guarantees)
- No buffer overflows or use-after-free
- Safe defaults for all configurations
- Input validation and sanitization
- Protection against common web attacks (XSS, CSRF headers)

### Operational Security
- Privilege dropping after binding ports
- Chroot/namespace isolation support
- Resource limits and quotas
- Secure TLS configuration defaults
- Regular security audits via cargo-audit

## Performance Characteristics

### Benchmarks (Target)
- Static file serving: 50k+ RPS on commodity hardware
- Reverse proxy: 30k+ RPS with upstream latency <1ms
- Memory usage: <1MB per 1000 idle connections
- CPU usage: <5% at 10k RPS on modern CPU

### Optimizations
- Zero-copy I/O with io_uring (Linux)
- Connection pooling and reuse
- Efficient parsing with minimal allocations
- SIMD optimizations where applicable
- Custom memory allocator (jemalloc/mimalloc)

## Compatibility

### Standards Compliance
- HTTP/1.1 (RFC 7230-7235)
- HTTP/2 (RFC 7540)
- TLS 1.2/1.3 (RFC 5246, 8446)
- WebSocket (RFC 6455)
- Common MIME types

### Drop-in Replacement
- nginx-style configuration mapping
- Common directive support
- Similar log formats
- Signal handling compatibility
- Exit codes follow conventions