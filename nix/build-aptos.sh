#!/usr/bin/env bash

# Build script for Aptos Core using Nix

set -e  # Exit on any error

echo "Building Aptos Core with Nix..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: This script must be run from the aptos-core root directory"
    exit 1
fi

# Build using Nix flake
if command -v nix &> /dev/null; then
    echo "Using Nix to build aptos-core..."
    
    # Build the package
    nix build .#aptos-core
    
    echo "Build completed successfully!"
    echo "Binary location: ./result/bin/aptos-node"
    
else
    echo "Nix is not installed. Please install Nix to use this build system."
    echo "Visit https://nixos.org/download.html for installation instructions."
    exit 1
fi

# Optional: Copy binary to a more accessible location
if [ -d "./result/bin" ]; then
    echo "Copying aptos-node binary to current directory..."
    cp ./result/bin/aptos-node ./aptos-node
    echo "aptos-node binary is now available in the current directory"
fi