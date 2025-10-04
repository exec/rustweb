#!/bin/bash

# RustWeb Installation Script
# This script installs RustWeb HTTP server on Linux systems

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="/usr/bin"
CONFIG_DIR="/etc/rustweb"
LOG_DIR="/var/log/rustweb"
WEB_DIR="/var/www/html"
LIB_DIR="/var/lib/rustweb"
SERVICE_FILE="/etc/systemd/system/rustweb.service"
USER="rustweb"
GROUP="rustweb"

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        BINARY_ARCH="x86_64-unknown-linux-gnu"
        ;;
    aarch64|arm64)
        BINARY_ARCH="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo -e "${RED}Error: Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

# Version to install (can be overridden with RUSTWEB_VERSION env var)
VERSION=${RUSTWEB_VERSION:-"latest"}

print_banner() {
    echo -e "${BLUE}"
    echo "██████╗ ██╗   ██╗███████╗████████╗██╗    ██╗███████╗██████╗ "
    echo "██╔══██╗██║   ██║██╔════╝╚══██╔══╝██║    ██║██╔════╝██╔══██╗"
    echo "██████╔╝██║   ██║███████╗   ██║   ██║ █╗ ██║█████╗  ██████╔╝"
    echo "██╔══██╗██║   ██║╚════██║   ██║   ██║███╗██║██╔══╝  ██╔══██╗"
    echo "██║  ██║╚██████╔╝███████║   ██║   ╚███╔███╔╝███████╗██████╔╝"
    echo "╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝    ╚══╝╚══╝ ╚══════╝╚═════╝ "
    echo -e "${NC}"
    echo -e "${GREEN}High-Performance HTTP Server Installation${NC}"
    echo
}

check_prerequisites() {
    echo -e "${BLUE}Checking prerequisites...${NC}"
    
    # Check if running as root
    if [[ $EUID -ne 0 ]]; then
        echo -e "${RED}Error: This script must be run as root${NC}"
        echo "Please run: sudo $0"
        exit 1
    fi
    
    # Check if systemd is available
    if ! command -v systemctl &> /dev/null; then
        echo -e "${RED}Error: systemd is required but not found${NC}"
        exit 1
    fi
    
    # Check for required commands
    for cmd in curl tar; do
        if ! command -v $cmd &> /dev/null; then
            echo -e "${RED}Error: $cmd is required but not installed${NC}"
            exit 1
        fi
    done
    
    echo -e "${GREEN}✓ Prerequisites check passed${NC}"
}

create_user() {
    echo -e "${BLUE}Creating rustweb user and group...${NC}"
    
    # Create group if it doesn't exist
    if ! getent group $GROUP >/dev/null; then
        groupadd --system $GROUP
        echo -e "${GREEN}✓ Created group: $GROUP${NC}"
    else
        echo -e "${YELLOW}✓ Group $GROUP already exists${NC}"
    fi
    
    # Create user if it doesn't exist
    if ! getent passwd $USER >/dev/null; then
        useradd --system --no-create-home --shell /bin/false --gid $GROUP $USER
        echo -e "${GREEN}✓ Created user: $USER${NC}"
    else
        echo -e "${YELLOW}✓ User $USER already exists${NC}"
    fi
}

create_directories() {
    echo -e "${BLUE}Creating directories...${NC}"
    
    # Create directories with proper permissions
    mkdir -p $CONFIG_DIR
    mkdir -p $LOG_DIR
    mkdir -p $WEB_DIR
    mkdir -p $LIB_DIR
    mkdir -p $CONFIG_DIR/ssl
    
    # Set ownership
    chown -R $USER:$GROUP $LOG_DIR $LIB_DIR
    chown root:$GROUP $CONFIG_DIR
    chmod 755 $CONFIG_DIR
    chmod 755 $WEB_DIR
    chmod 750 $LOG_DIR $LIB_DIR
    chmod 750 $CONFIG_DIR/ssl
    
    echo -e "${GREEN}✓ Directories created${NC}"
}

download_binary() {
    echo -e "${BLUE}Downloading RustWeb binary...${NC}"
    
    if [[ "$VERSION" == "latest" ]]; then
        # Get latest release info from GitHub API
        DOWNLOAD_URL=$(curl -s https://api.github.com/repos/your-org/rustweb/releases/latest | \
                       grep "browser_download_url.*rustweb-$BINARY_ARCH\"" | \
                       cut -d '"' -f 4)
    else
        DOWNLOAD_URL="https://github.com/your-org/rustweb/releases/download/v$VERSION/rustweb-$BINARY_ARCH"
    fi
    
    if [[ -z "$DOWNLOAD_URL" ]]; then
        echo -e "${RED}Error: Could not find download URL for architecture $BINARY_ARCH${NC}"
        exit 1
    fi
    
    echo "Downloading from: $DOWNLOAD_URL"
    curl -L -o /tmp/rustweb "$DOWNLOAD_URL"
    
    # Make executable and move to install directory
    chmod +x /tmp/rustweb
    mv /tmp/rustweb $INSTALL_DIR/rustweb
    
    echo -e "${GREEN}✓ RustWeb binary installed to $INSTALL_DIR/rustweb${NC}"
}

install_config() {
    echo -e "${BLUE}Installing configuration files...${NC}"
    
    # Create default configuration if it doesn't exist
    if [[ ! -f "$CONFIG_DIR/rustweb.toml" ]]; then
        cat > $CONFIG_DIR/rustweb.toml << 'EOF'
# RustWeb HTTP Server - Production Configuration
# This configuration follows FHS (Filesystem Hierarchy Standard) conventions

[server]
listen = ["0.0.0.0:80", "0.0.0.0:443"]
listen_quic = ["0.0.0.0:443"]
enable_http3 = true
max_connections = 10000
keep_alive_timeout = 65
request_timeout = 60
send_timeout = 60
client_body_timeout = 60
client_header_timeout = 60
client_max_body_size = 1048576
tcp_nodelay = true
tcp_fastopen = false

[logging]
log_level = "info"
access_log = "/var/log/rustweb/access.log"
error_log = "/var/log/rustweb/error.log"
access_log_format = "$remote_addr - $remote_user [$time_local] \"$request\" $status $body_bytes_sent \"$http_referer\" \"$http_user_agent\""

[security]
enable_rate_limiting = true
rate_limit_requests_per_second = 1000
rate_limit_burst = 2000
max_request_size = 10485760
allowed_methods = ["GET", "POST", "HEAD", "PUT", "DELETE", "PATCH", "OPTIONS"]

[security.security_headers]
"X-Frame-Options" = "DENY"
"X-Content-Type-Options" = "nosniff"
"X-XSS-Protection" = "1; mode=block"
"Strict-Transport-Security" = "max-age=31536000; includeSubDomains"

[compression]
enable_gzip = true
enable_brotli = true
enable_zstd = true
compression_level = 6
min_compress_size = 1024
compress_types = [
    "text/html",
    "text/css", 
    "text/javascript",
    "text/plain",
    "application/javascript",
    "application/json",
    "application/xml",
    "image/svg+xml"
]

[upstream]

[virtual_hosts]
[virtual_hosts.default]
server_name = ["_"]  # Default catch-all
document_root = "/var/www/html"
index_files = ["index.html", "index.htm"]

[virtual_hosts.default.locations]

[virtual_hosts.default.ssl]
certificate = "/etc/rustweb/ssl/cert.pem"
private_key = "/etc/rustweb/ssl/key.pem"
auto_generate_self_signed = false  # Use real certificates in production
protocols = ["TLSv1.2", "TLSv1.3"]
EOF
        chown root:$GROUP $CONFIG_DIR/rustweb.toml
        chmod 640 $CONFIG_DIR/rustweb.toml
        echo -e "${GREEN}✓ Created configuration file: $CONFIG_DIR/rustweb.toml${NC}"
    else
        echo -e "${YELLOW}✓ Configuration file already exists: $CONFIG_DIR/rustweb.toml${NC}"
    fi
    
    # Create default index.html if it doesn't exist
    if [[ ! -f "$WEB_DIR/index.html" ]]; then
        cat > $WEB_DIR/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Welcome to RustWeb</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            margin: 0;
            padding: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .container {
            text-align: center;
            max-width: 600px;
            padding: 2rem;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 15px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }
        h1 {
            font-size: 3rem;
            margin-bottom: 1rem;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.3);
        }
        p {
            font-size: 1.2rem;
            margin-bottom: 2rem;
        }
        .features {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-top: 2rem;
        }
        .feature {
            background: rgba(255, 255, 255, 0.1);
            padding: 1rem;
            border-radius: 10px;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }
        .version {
            margin-top: 2rem;
            font-size: 0.9rem;
            opacity: 0.8;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 RustWeb</h1>
        <p>Your high-performance HTTP server is up and running!</p>
        
        <div class="features">
            <div class="feature">
                <h3>⚡ HTTP/1.1 & HTTP/2</h3>
                <p>Full protocol support with ALPN negotiation</p>
            </div>
            <div class="feature">
                <h3>🔒 Security First</h3>
                <p>Built-in rate limiting and security headers</p>
            </div>
            <div class="feature">
                <h3>🗜️ Compression</h3>
                <p>Gzip, Brotli, and Zstandard support</p>
            </div>
            <div class="feature">
                <h3>📊 Monitoring</h3>
                <p>Comprehensive logging and metrics</p>
            </div>
        </div>
        
        <div class="version">
            <p>Powered by Rust 🦀 | Configuration: /etc/rustweb/rustweb.toml</p>
        </div>
    </div>
</body>
</html>
EOF
        chown root:root $WEB_DIR/index.html
        chmod 644 $WEB_DIR/index.html
        echo -e "${GREEN}✓ Created default web page: $WEB_DIR/index.html${NC}"
    else
        echo -e "${YELLOW}✓ Web page already exists: $WEB_DIR/index.html${NC}"
    fi
}

install_systemd_service() {
    echo -e "${BLUE}Installing systemd service...${NC}"
    
    cat > $SERVICE_FILE << 'EOF'
[Unit]
Description=RustWeb HTTP Server
Documentation=https://github.com/your-org/rustweb
After=network.target
Wants=network.target

[Service]
Type=simple
User=rustweb
Group=rustweb
ExecStart=/usr/bin/rustweb --config /etc/rustweb/rustweb.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/rustweb /var/www
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_BIND_SERVICE

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Working directory
WorkingDirectory=/var/lib/rustweb

# Environment
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
    
    # Reload systemd and enable service
    systemctl daemon-reload
    systemctl enable rustweb
    
    echo -e "${GREEN}✓ Systemd service installed and enabled${NC}"
}

generate_certificates() {
    echo -e "${BLUE}Generating self-signed certificates...${NC}"
    
    if [[ ! -f "$CONFIG_DIR/ssl/cert.pem" ]]; then
        # Generate self-signed certificate for localhost
        openssl req -x509 -newkey rsa:4096 \
            -keyout $CONFIG_DIR/ssl/key.pem \
            -out $CONFIG_DIR/ssl/cert.pem \
            -days 365 -nodes \
            -subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"
        
        # Set proper permissions
        chown root:$GROUP $CONFIG_DIR/ssl/cert.pem $CONFIG_DIR/ssl/key.pem
        chmod 644 $CONFIG_DIR/ssl/cert.pem
        chmod 640 $CONFIG_DIR/ssl/key.pem
        
        echo -e "${GREEN}✓ Self-signed certificates generated${NC}"
        echo -e "${YELLOW}  For production, replace with real certificates:${NC}"
        echo -e "${YELLOW}  - Certificate: $CONFIG_DIR/ssl/cert.pem${NC}"
        echo -e "${YELLOW}  - Private key: $CONFIG_DIR/ssl/key.pem${NC}"
    else
        echo -e "${YELLOW}✓ SSL certificates already exist${NC}"
    fi
}

print_installation_complete() {
    echo
    echo -e "${GREEN}🎉 RustWeb installation completed successfully!${NC}"
    echo
    echo -e "${BLUE}Next steps:${NC}"
    echo -e "  1. Review configuration: ${YELLOW}$CONFIG_DIR/rustweb.toml${NC}"
    echo -e "  2. Start the service: ${YELLOW}systemctl start rustweb${NC}"
    echo -e "  3. Check status: ${YELLOW}systemctl status rustweb${NC}"
    echo -e "  4. View logs: ${YELLOW}journalctl -u rustweb -f${NC}"
    echo
    echo -e "${BLUE}Web server will be available at:${NC}"
    echo -e "  • HTTP:  ${YELLOW}http://your-server${NC}"
    echo -e "  • HTTPS: ${YELLOW}https://your-server${NC}"
    echo
    echo -e "${BLUE}Configuration files:${NC}"
    echo -e "  • Main config: ${YELLOW}$CONFIG_DIR/rustweb.toml${NC}"
    echo -e "  • Web root: ${YELLOW}$WEB_DIR${NC}"
    echo -e "  • Logs: ${YELLOW}$LOG_DIR${NC}"
    echo
    echo -e "${YELLOW}Note: For production use, replace self-signed certificates with real ones!${NC}"
    echo
}

# Main installation process
main() {
    print_banner
    check_prerequisites
    create_user
    create_directories
    download_binary
    install_config
    install_systemd_service
    generate_certificates
    print_installation_complete
}

# Run installation
main "$@"