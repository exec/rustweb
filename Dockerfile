# Multi-stage build for optimal container size
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

# Create app directory
WORKDIR /app

# Copy all source files
COPY . .

# Build the application
RUN cargo build --release --locked

# Runtime stage
FROM alpine:3.19 AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tzdata \
    curl

# Create non-root user
RUN addgroup -g 1000 rustweb && \
    adduser -D -s /bin/sh -u 1000 -G rustweb rustweb

# Create directories following FHS standards
RUN mkdir -p /etc/rustweb /var/www/html /var/log/rustweb /var/lib/rustweb /usr/bin && \
    chown -R rustweb:rustweb /var/www/html /var/log/rustweb /var/lib/rustweb

# Copy binary from builder stage
COPY --from=builder /app/target/release/rustweb /usr/bin/rustweb

# Copy default configuration and static files
COPY --chown=root:rustweb rustweb.toml /etc/rustweb/
COPY --chown=rustweb:rustweb www/ /var/www/html/
COPY --chown=rustweb:rustweb static/ /usr/share/rustweb/static/

# Set working directory
WORKDIR /var/lib/rustweb

# Switch to non-root user
USER rustweb

# Expose ports
EXPOSE 8080 8443

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command
CMD ["/usr/bin/rustweb", "--config", "/etc/rustweb/rustweb.toml"]

# Labels for metadata
LABEL maintainer="RustWeb Contributors" \
      description="High-performance HTTP server written in Rust" \
      version="0.1.0" \
      org.opencontainers.image.title="RustWeb" \
      org.opencontainers.image.description="A modern HTTP server replacement for nginx/httpd" \
      org.opencontainers.image.vendor="RustWeb Project" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.created="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"