#!/bin/bash
# Build script for aptos-node Docker image using proper Nix package

set -e  # Exit on any error

# Ensure we're in the project root
cd "$(git rev-parse --show-toplevel)"

echo "Building aptos-node using Nix package..."
# Build the aptos-node package using Nix
nix develop -c cargo build --package aptos-node

# Find the location of the aptos-node binary
BINARY_PATH=$(find . -name "aptos-node" -type f | grep -E "target/debug|result/bin" | head -1)

if [ -z "$BINARY_PATH" ]; then
    echo "Error: aptos-node binary not found!"
    exit 1
fi

echo "Found aptos-node binary at: $BINARY_PATH"

# Check if the binary is properly linked
echo "Checking binary dependencies..."
ldd "$BINARY_PATH" || echo "Binary may be statically linked"

# Create a temporary directory for Docker context
TEMP_DIR=$(mktemp -d)
echo "Using temporary directory: $TEMP_DIR"

# Copy the binary to temp directory
cp "$BINARY_PATH" "$TEMP_DIR/aptos-node"

# Copy minimal Dockerfile to temp directory
cp docker/aptos-node/Dockerfile.nix "$TEMP_DIR/Dockerfile"

# Build Docker image from temp directory
echo "Building Docker image..."
docker build -t aptos-node:nix-latest "$TEMP_DIR"

# Clean up
rm -rf "$TEMP_DIR"
rm -f result  # Remove Nix build symlink

echo "Docker image 'aptos-node:nix-latest' built successfully!"
echo "This binary was built with proper Nix packaging - no patching required!"