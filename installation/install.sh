#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPO="NotCoffee418/power_control_center"
SERVICE_NAME="power_control_center"
BINARY_NAME="power_control_center"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/power_control_center"
DATA_DIR="/var/lib/power_control_center"
SERVICE_USER="power_control"
SERVICE_GROUP="power_control"

echo -e "${GREEN}Power Control Center Installation Script${NC}"
echo "=========================================="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run as root${NC}"
    echo "Please run with: sudo bash install.sh"
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)
        RELEASE_ARCH="x86_64"
        ;;
    aarch64|arm64)
        RELEASE_ARCH="arm64"
        ;;
    armv7l|armhf)
        RELEASE_ARCH="armv7"
        ;;
    *)
        echo -e "${RED}Error: Unsupported architecture: $ARCH${NC}"
        echo "Supported architectures: x86_64, arm64, armv7"
        exit 1
        ;;
esac

echo -e "Detected architecture: ${GREEN}$ARCH${NC} (release: $RELEASE_ARCH)"

# Get latest release tag
echo "Fetching latest release information..."
LATEST_TAG=$(curl -s --user-agent "power_control_center-installer/1.0" "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo -e "${RED}Error: Could not fetch latest release tag${NC}"
    echo "Please check your internet connection and try again."
    exit 1
fi

echo -e "Latest release: ${GREEN}$LATEST_TAG${NC}"

# Construct download URL
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/power_control_center-$LATEST_TAG-linux-$RELEASE_ARCH.tar.gz"
TEMP_DIR=$(mktemp -d)
ARCHIVE_FILE="$TEMP_DIR/power_control_center.tar.gz"

echo "Downloading binary from: $DOWNLOAD_URL"
if ! curl -L -f --user-agent "power_control_center-installer/1.0" -o "$ARCHIVE_FILE" "$DOWNLOAD_URL"; then
    echo -e "${RED}Error: Failed to download release${NC}"
    echo "URL: $DOWNLOAD_URL"
    rm -rf "$TEMP_DIR"
    exit 1
fi

echo "Extracting archive..."
tar -xzf "$ARCHIVE_FILE" -C "$TEMP_DIR"

# Find the binary in the extracted files
BINARY_PATH=$(find "$TEMP_DIR" -name "$BINARY_NAME" -type f | head -n 1)
if [ -z "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Could not find binary in archive${NC}"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Stop service if running
if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo "Stopping existing service..."
    systemctl stop "$SERVICE_NAME"
fi

# Create service user and group if they don't exist
if ! getent group "$SERVICE_GROUP" >/dev/null 2>&1; then
    echo "Creating service group: $SERVICE_GROUP"
    groupadd --system "$SERVICE_GROUP"
fi

if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
    echo "Creating service user: $SERVICE_USER"
    useradd --system --no-create-home --shell /bin/false -g "$SERVICE_GROUP" "$SERVICE_USER"
fi

# Create necessary directories
echo "Creating directories..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"

# Install binary
echo "Installing binary to $INSTALL_DIR/$BINARY_NAME..."
cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Set directory permissions
echo "Setting permissions..."
chown -R "$SERVICE_USER:$SERVICE_GROUP" "$DATA_DIR"
chmod 755 "$DATA_DIR"
chown -R root:root "$CONFIG_DIR"
chmod 755 "$CONFIG_DIR"

# Install systemd service
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME.service"
echo "Installing systemd service to $SERVICE_FILE..."

# Download or copy service file from installation directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "$SCRIPT_DIR/power_control_center.service" ]; then
    cp "$SCRIPT_DIR/power_control_center.service" "$SERVICE_FILE"
else
    # Create service file inline if not found
    cat > "$SERVICE_FILE" << 'EOF'
# /etc/systemd/system/power_control_center.service
[Unit]
Description=Power Control Center Service
After=network.target

[Service]
Type=simple
User=power_control
Group=power_control
ExecStart=/usr/local/bin/power_control_center
Restart=always
RestartSec=5
WorkingDirectory=/var/lib/power_control_center

# Systemd managed directories
StateDirectory=power_control_center
LogsDirectory=power_control_center
ConfigurationDirectory=power_control_center
RuntimeDirectory=power_control_center

# Environment
Environment=RUST_LOG=info

# Basic security
NoNewPrivileges=true
ReadWritePaths=/var/lib/power_control_center /var/log/power_control_center /run/power_control_center
ReadOnlyPaths=/etc/power_control_center

[Install]
WantedBy=multi-user.target
EOF
fi

# Reload systemd
echo "Reloading systemd daemon..."
systemctl daemon-reload

# Cleanup
rm -rf "$TEMP_DIR"

echo ""
echo -e "${GREEN}Installation completed successfully!${NC}"
echo ""
echo "Next steps:"
echo "1. Create configuration file at: $CONFIG_DIR/config.json"
echo "   (You can use config-example.json as a template)"
echo ""
echo "2. Enable and start the service:"
echo "   sudo systemctl enable $SERVICE_NAME"
echo "   sudo systemctl start $SERVICE_NAME"
echo ""
echo "3. Check service status:"
echo "   sudo systemctl status $SERVICE_NAME"
echo ""
echo "4. View logs:"
echo "   sudo journalctl -u $SERVICE_NAME -f"
echo ""
