# RustWeb - Development Instructions for Claude Code

This file contains coding standards and project-specific guidance for Claude Code when working on the RustWeb project.

## Project Overview

RustWeb is a high-performance HTTP server written in Rust, designed as a modern alternative to nginx/httpd. See README.md for complete documentation and FEATURES.md for implementation status.

## Coding Standards

### Rust Conventions
- Follow standard Rust formatting: always run `cargo fmt` before committing
- Lint with `cargo clippy -- -D warnings` - warnings are not acceptable
- Write comprehensive tests for new features
- Use `cargo test` to verify all tests pass
- Document public APIs with rustdoc comments

### Code Organization
- Keep modules focused and single-purpose
- Use `src/lib.rs` for library code, `src/main.rs` only for CLI entry point
- Place feature modules in their own directories under `src/`
- Write integration tests in `tests/`, unit tests alongside code

### Performance Considerations
- Profile with `cargo bench` before and after optimizations
- Use zero-copy I/O where possible (Bytes, references)
- Prefer async/await over blocking operations
- Keep allocations minimal in hot paths
- Use `Arc` for shared immutable data, `DashMap` for concurrent collections

### Security Requirements
- Validate and sanitize all user inputs
- Use Rust's type system to prevent security bugs
- Never use `unsafe` without thorough review and documentation
- Run `cargo audit` regularly for dependency vulnerabilities
- Default to secure configurations (deny-by-default)

### Configuration Management
- All features should be configurable via TOML
- Provide sensible defaults requiring minimal config
- Validate configuration at startup and fail fast with clear errors
- Support hot-reload for runtime-changeable settings

### Error Handling
- Use `anyhow::Result` for application errors
- Use `thiserror` for custom error types
- Log errors with appropriate levels (error, warn, info, debug)
- Return meaningful error messages to help operators debug

### Testing
- Write unit tests for all business logic
- Write integration tests for HTTP endpoints
- Use `cargo test` for running all tests
- Keep test coverage above 70% for critical paths

### Commit Practices
- Write clear, descriptive commit messages
- Reference issue numbers where applicable
- Keep commits focused on single changes
- Run tests before committing

## Architecture Guidelines

### Core Principles
- **Do one thing well**: Focus on HTTP serving excellence
- **Composable**: Work seamlessly with other Unix tools
- **Linux philosophy**: Simple, scriptable, with proper exit codes
- **Fail fast**: Clear error messages and validation

### Module Structure
- `server/` - Core HTTP handling and connection management
- `config.rs` - Configuration parsing and validation
- `proxy/` - Reverse proxy and load balancing
- `security/` - Rate limiting and security headers
- `compression/` - Response compression algorithms
- `metrics/` - Performance monitoring and Prometheus export

### Dependencies
- Prefer well-maintained crates with active communities
- Minimize dependency tree where possible
- Keep Cargo.toml organized with comments for sections
- Update dependencies regularly but carefully

## When Making Changes

### Adding Features
1. Update configuration schema if needed
2. Implement feature with comprehensive error handling
3. Add unit and integration tests
4. Update FEATURES.md with status
5. Document in README.md if user-facing

### Fixing Bugs
1. Write a failing test that reproduces the bug
2. Fix the bug
3. Verify the test passes
4. Add regression test if appropriate

### Performance Work
1. Profile with benchmarks (`cargo bench`)
2. Make targeted optimizations
3. Re-run benchmarks to verify improvement
4. Document performance characteristics

## Build and Deployment

- Release builds use LTO and single codegen unit for maximum performance
- Support systemd service deployment
- Provide Docker containerization
- Create .deb and .rpm packages for Linux distributions
- Binary should have minimal dependencies for portability

## Signal Handling
- SIGTERM/SIGINT: Graceful shutdown
- SIGQUIT: Immediate shutdown
- SIGHUP: Configuration reload (hot reload)

## Important Notes

- Never commit secrets, keys, or certificates to version control
- Test files (test_results.json, etc.) should not be committed
- Keep configuration examples (docker-config.toml, test_config.toml) for reference
- Run security audits regularly: `cargo audit`
- Follow semver for versioning
