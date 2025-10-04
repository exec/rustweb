#!/bin/bash

# RustWeb Uninstallation Script
# This script removes RustWeb HTTP server from Linux systems

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

print_banner() {
    echo -e "${RED}"
    echo "██╗   ██╗███╗   ██╗██╗███╗   ██╗███████╗████████╗ █████╗ ██╗     ██╗     "
    echo "██║   ██║████╗  ██║██║████╗  ██║██╔════╝╚══██╔══╝██╔══██╗██║     ██║     "
    echo "██║   ██║██╔██╗ ██║██║██╔██╗ ██║███████╗   ██║   ███████║██║     ██║     "
    echo "██║   ██║██║╚██╗██║██║██║╚██╗██║╚════██║   ██║   ██╔══██║██║     ██║     "
    echo "╚██████╔╝██║ ╚████║██║██║ ╚████║███████║   ██║   ██║  ██║███████╗███████╗"
    echo " ╚═════╝ ╚═╝  ╚═══╝╚═╝╚═╝  ╚═══╝╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚══════╝"
    echo -e "${NC}"
    echo -e "${RED}RustWeb HTTP Server Uninstallation${NC}"
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
    
    echo -e "${GREEN}✓ Prerequisites check passed${NC}"
}

confirm_uninstall() {
    echo -e "${YELLOW}This will completely remove RustWeb from your system, including:${NC}"
    echo "  • RustWeb binary ($INSTALL_DIR/rustweb)"
    echo "  • Configuration files ($CONFIG_DIR)"
    echo "  • Log files ($LOG_DIR)"
    echo "  • Service files ($SERVICE_FILE)"
    echo "  • RustWeb user and group"
    echo
    echo -e "${YELLOW}Web files in $WEB_DIR will be preserved.${NC}"
    echo
    
    read -p "Are you sure you want to continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Uninstallation cancelled.${NC}"
        exit 0
    fi
}

stop_and_disable_service() {
    echo -e "${BLUE}Stopping and disabling RustWeb service...${NC}"
    
    if systemctl is-active --quiet rustweb; then
        systemctl stop rustweb
        echo -e "${GREEN}✓ Service stopped${NC}"
    else
        echo -e "${YELLOW}✓ Service was not running${NC}"
    fi
    
    if systemctl is-enabled --quiet rustweb; then
        systemctl disable rustweb
        echo -e "${GREEN}✓ Service disabled${NC}"
    else
        echo -e "${YELLOW}✓ Service was not enabled${NC}"
    fi
}

remove_service_file() {
    echo -e "${BLUE}Removing systemd service file...${NC}"
    
    if [[ -f "$SERVICE_FILE" ]]; then
        rm -f "$SERVICE_FILE"
        systemctl daemon-reload
        echo -e "${GREEN}✓ Service file removed${NC}"
    else
        echo -e "${YELLOW}✓ Service file was not found${NC}"
    fi
}

remove_binary() {
    echo -e "${BLUE}Removing RustWeb binary...${NC}"
    
    if [[ -f "$INSTALL_DIR/rustweb" ]]; then
        rm -f "$INSTALL_DIR/rustweb"
        echo -e "${GREEN}✓ Binary removed${NC}"
    else
        echo -e "${YELLOW}✓ Binary was not found${NC}"
    fi
}

remove_config_and_logs() {
    echo -e "${BLUE}Removing configuration and log files...${NC}"
    
    if [[ -d "$CONFIG_DIR" ]]; then
        rm -rf "$CONFIG_DIR"
        echo -e "${GREEN}✓ Configuration directory removed${NC}"
    else
        echo -e "${YELLOW}✓ Configuration directory was not found${NC}"
    fi
    
    if [[ -d "$LOG_DIR" ]]; then
        rm -rf "$LOG_DIR"
        echo -e "${GREEN}✓ Log directory removed${NC}"
    else
        echo -e "${YELLOW}✓ Log directory was not found${NC}"
    fi
    
    if [[ -d "$LIB_DIR" ]]; then
        rm -rf "$LIB_DIR"
        echo -e "${GREEN}✓ Library directory removed${NC}"
    else
        echo -e "${YELLOW}✓ Library directory was not found${NC}"
    fi
}

remove_user() {
    echo -e "${BLUE}Removing RustWeb user and group...${NC}"
    
    # Remove user if it exists
    if getent passwd $USER >/dev/null; then
        userdel $USER
        echo -e "${GREEN}✓ User $USER removed${NC}"
    else
        echo -e "${YELLOW}✓ User $USER was not found${NC}"
    fi
    
    # Remove group if it exists and has no other users
    if getent group $GROUP >/dev/null; then
        if ! getent group $GROUP | grep -q ":.*[^:]"; then
            groupdel $GROUP
            echo -e "${GREEN}✓ Group $GROUP removed${NC}"
        else
            echo -e "${YELLOW}✓ Group $GROUP has other users, not removed${NC}"
        fi
    else
        echo -e "${YELLOW}✓ Group $GROUP was not found${NC}"
    fi
}

print_uninstall_complete() {
    echo
    echo -e "${GREEN}✅ RustWeb has been successfully uninstalled!${NC}"
    echo
    echo -e "${BLUE}The following items were removed:${NC}"
    echo "  • RustWeb binary"
    echo "  • Configuration files"
    echo "  • Log files"
    echo "  • Systemd service"
    echo "  • RustWeb user and group"
    echo
    echo -e "${YELLOW}Web files in $WEB_DIR were preserved.${NC}"
    echo
    echo -e "${BLUE}Thank you for using RustWeb! 🦀${NC}"
    echo
}

# Main uninstallation process
main() {
    print_banner
    check_prerequisites
    confirm_uninstall
    stop_and_disable_service
    remove_service_file
    remove_binary
    remove_config_and_logs
    remove_user
    print_uninstall_complete
}

# Run uninstallation
main "$@"