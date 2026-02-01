#!/bin/bash
set -e

# Installation script for flat CLI tool
# https://github.com/zkoranges/flat

echo "Installing flat CLI tool..."
echo ""

# Detect OS
OS=$(uname -s)
if [ "$OS" != "Darwin" ]; then
    echo "❌ Error: This installer is currently for macOS only"
    echo "   For other platforms, please build from source:"
    echo "   https://github.com/zkoranges/flat#installation"
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64)
        BINARY_URL="https://github.com/zkoranges/flat/releases/latest/download/flat-aarch64-apple-darwin.tar.gz"
        PLATFORM="Apple Silicon (arm64)"
        ;;
    x86_64)
        BINARY_URL="https://github.com/zkoranges/flat/releases/latest/download/flat-x86_64-apple-darwin.tar.gz"
        PLATFORM="Intel (x86_64)"
        ;;
    *)
        echo "❌ Error: Unsupported architecture: $ARCH"
        echo "   Please build from source:"
        echo "   https://github.com/zkoranges/flat#installation"
        exit 1
        ;;
esac

echo "Detected platform: macOS - $PLATFORM"
echo ""

# Check if we need to build from source (releases not yet available)
if ! curl -fsSL -I "$BINARY_URL" > /dev/null 2>&1; then
    echo "⚠️  Pre-built binaries not yet available"
    echo ""
    echo "Building from source instead..."
    echo ""

    # Check for cargo
    if ! command -v cargo &> /dev/null; then
        echo "❌ Error: cargo not found"
        echo "   Please install Rust first: https://rustup.rs/"
        exit 1
    fi

    # Clone and build
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"

    echo "Cloning repository..."
    git clone https://github.com/zkoranges/flat.git
    cd flat

    echo "Building (this may take a minute)..."
    cargo build --release

    echo "Installing to /usr/local/bin..."
    if [ -w /usr/local/bin ]; then
        cp target/release/flat /usr/local/bin/
    else
        echo "Need sudo permission to install to /usr/local/bin"
        sudo cp target/release/flat /usr/local/bin/
    fi

    # Cleanup
    cd /
    rm -rf "$TEMP_DIR"

    echo ""
    echo "✅ flat installed successfully!"
    flat --version
    exit 0
fi

# Download pre-built binary
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo "Downloading flat..."
if ! curl -fsSL "$BINARY_URL" -o flat.tar.gz; then
    echo "❌ Error: Failed to download binary"
    echo "   Please try building from source:"
    echo "   https://github.com/zkoranges/flat#installation"
    exit 1
fi

echo "Extracting..."
tar xzf flat.tar.gz

echo "Installing to /usr/local/bin..."
if [ -w /usr/local/bin ]; then
    mv flat /usr/local/bin/
else
    echo "Need sudo permission to install to /usr/local/bin"
    sudo mv flat /usr/local/bin/
fi

# Cleanup
cd /
rm -rf "$TEMP_DIR"

echo ""
echo "✅ flat installed successfully!"
flat --version
echo ""
echo "Try: flat --help"
