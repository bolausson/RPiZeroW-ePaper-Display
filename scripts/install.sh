#!/bin/bash
# Installation script to run on the Raspberry Pi Zero W
#
# This script should be run on the Pi after the binary has been deployed

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

INSTALL_DIR="/opt/epaper-display"
SERVICE_NAME="epaper-display"

echo -e "${GREEN}Installing E-Paper Display Server${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Please run as root (use sudo)${NC}"
    exit 1
fi

# Enable SPI
echo "Enabling SPI..."
if ! grep -q "^dtparam=spi=on" /boot/config.txt 2>/dev/null && \
   ! grep -q "^dtparam=spi=on" /boot/firmware/config.txt 2>/dev/null; then
    # Try to find the correct config.txt location
    if [ -f /boot/firmware/config.txt ]; then
        echo "dtparam=spi=on" >> /boot/firmware/config.txt
    elif [ -f /boot/config.txt ]; then
        echo "dtparam=spi=on" >> /boot/config.txt
    fi
    echo -e "${YELLOW}SPI enabled (reboot required)${NC}"
else
    echo "SPI already enabled"
fi

# Create installation directory
echo "Creating installation directory..."
mkdir -p "$INSTALL_DIR"

# Check if binary exists
if [ ! -f "$INSTALL_DIR/epaper-display" ]; then
    echo -e "${YELLOW}Binary not found at $INSTALL_DIR/epaper-display${NC}"
    echo "Please copy the binary first using deploy.sh"
fi

# Create default config if not exists
if [ ! -f "$INSTALL_DIR/config.json" ]; then
    echo "Creating default configuration..."
    cat > "$INSTALL_DIR/config.json" << 'EOF'
{
    "image_url": "",
    "refresh_interval_min": 60,
    "rotation": 0,
    "mirror_h": false,
    "mirror_v": false,
    "scale_to_fit": true,
    "web_port": 8888,
    "verbose": false
}
EOF
fi

# Install systemd service
if [ -f "$INSTALL_DIR/../systemd/epaper-display.service" ]; then
    echo "Installing systemd service..."
    cp "$INSTALL_DIR/../systemd/epaper-display.service" /etc/systemd/system/
elif [ -f "/tmp/epaper-display.service" ]; then
    mv /tmp/epaper-display.service /etc/systemd/system/
fi

# Reload systemd
systemctl daemon-reload

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Edit configuration: sudo nano $INSTALL_DIR/config.json"
echo "2. Start the service: sudo systemctl start $SERVICE_NAME"
echo "3. Enable at boot: sudo systemctl enable $SERVICE_NAME"
echo "4. Check status: sudo systemctl status $SERVICE_NAME"
echo ""
echo "Access the web interface at: http://$(hostname).local:8888/"
echo ""

# Check if reboot is needed
if [ -f /var/run/reboot-required ] || grep -q "SPI enabled" /tmp/install_log 2>/dev/null; then
    echo -e "${YELLOW}A reboot is recommended to apply hardware changes.${NC}"
    echo "Run: sudo reboot"
fi

