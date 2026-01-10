#!/bin/bash
# Build script for cross-compiling to Raspberry Pi Zero W
#
# Prerequisites:
# - ARM cross-compilation toolchain installed
# - Rust target arm-unknown-linux-gnueabihf added

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET="arm-unknown-linux-gnueabihf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building E-Paper Display Server for Pi Zero W${NC}"
echo "Target: $TARGET"
echo ""

# Check for cross-compilation tools
if ! command -v arm-linux-gnueabihf-gcc &> /dev/null; then
    echo -e "${YELLOW}Warning: arm-linux-gnueabihf-gcc not found${NC}"
    echo "Install with: sudo apt install gcc-arm-linux-gnueabihf"
fi

# Source cargo environment if needed
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Check if target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
    echo -e "${YELLOW}Adding Rust target $TARGET...${NC}"
    rustup target add "$TARGET"
fi

cd "$PROJECT_DIR"

# Build options
BUILD_OPTS="--release --target $TARGET"

echo "Building..."
cargo build $BUILD_OPTS

# Check build result
BINARY="target/$TARGET/release/epaper-display"
if [ -f "$BINARY" ]; then
    SIZE=$(ls -lh "$BINARY" | awk '{print $5}')
    echo ""
    echo -e "${GREEN}Build successful!${NC}"
    echo "Binary: $BINARY"
    echo "Size: $SIZE"
    
    # Strip binary for smaller size
    if command -v arm-linux-gnueabihf-strip &> /dev/null; then
        echo "Stripping binary..."
        arm-linux-gnueabihf-strip "$BINARY"
        SIZE=$(ls -lh "$BINARY" | awk '{print $5}')
        echo "Size after strip: $SIZE"
    fi
else
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

