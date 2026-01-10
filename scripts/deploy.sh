#!/bin/bash
# Deploy script for Raspberry Pi Zero W
#
# Usage: ./deploy.sh <pi-hostname-or-ip> [user]
#
# Example: ./deploy.sh raspberrypi.local pi

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET="arm-unknown-linux-gnueabihf"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Arguments
PI_HOST="${1:-raspberrypi.local}"
PI_USER="${2:-pi}"
PI_DEST="$PI_USER@$PI_HOST"

BINARY="$PROJECT_DIR/target/$TARGET/release/epaper-display"
REMOTE_DIR="/opt/epaper-display"

echo -e "${GREEN}Deploying to Pi Zero W${NC}"
echo "Host: $PI_HOST"
echo "User: $PI_USER"
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo -e "${YELLOW}Binary not found, building first...${NC}"
    "$SCRIPT_DIR/build.sh"
fi

# Create remote directory
echo "Creating remote directory..."
ssh "$PI_DEST" "sudo mkdir -p $REMOTE_DIR && sudo chown $PI_USER:$PI_USER $REMOTE_DIR"

# Copy binary
echo "Copying binary..."
scp "$BINARY" "$PI_DEST:$REMOTE_DIR/"

# Copy systemd service file
echo "Copying systemd service..."
scp "$PROJECT_DIR/systemd/epaper-display.service" "$PI_DEST:/tmp/"
ssh "$PI_DEST" "sudo mv /tmp/epaper-display.service /etc/systemd/system/"

# Copy example config if no config exists
echo "Setting up configuration..."
ssh "$PI_DEST" "test -f $REMOTE_DIR/config.json || echo '{}' > $REMOTE_DIR/config.json"

# Copy example config
scp "$PROJECT_DIR/config/config.example.json" "$PI_DEST:$REMOTE_DIR/"

# Set permissions
echo "Setting permissions..."
ssh "$PI_DEST" "chmod +x $REMOTE_DIR/epaper-display"

# Reload systemd
echo "Reloading systemd..."
ssh "$PI_DEST" "sudo systemctl daemon-reload"

echo ""
echo -e "${GREEN}Deployment complete!${NC}"
echo ""
echo "To manage the service:"
echo "  Start:   sudo systemctl start epaper-display"
echo "  Stop:    sudo systemctl stop epaper-display"
echo "  Restart: sudo systemctl restart epaper-display"
echo "  Status:  sudo systemctl status epaper-display"
echo "  Logs:    sudo journalctl -u epaper-display -f"
echo ""
echo "To enable at boot:"
echo "  sudo systemctl enable epaper-display"

