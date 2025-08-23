# Justfile for Aptos Core with Nix build system

# Set the shell to bash
set shell := ["bash", "-c"]

# Default target
default: build

# Build the project using Nix
build:
    @echo "Building Aptos Core with Nix..."
    nix build .#aptos-core

# Enter the development environment
dev:
    @echo "Entering development environment..."
    nix develop

# Run tests
test:
    @echo "Running tests..."
    nix develop -c cargo test

# Check code formatting
fmt:
    @echo "Checking code formatting..."
    nix develop -c cargo fmt -- --check

# Run clippy
clippy:
    @echo "Running clippy..."
    nix develop -c cargo clippy -- --deny warnings

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    rm -rf result target

# Update flake.lock
update:
    @echo "Updating flake.lock..."
    nix flake update

# Build aptos-node binary
aptos-node:
    @echo "Building aptos-node with Nix..."
    nix build .#aptos-core
    @echo "Binary available at result/bin/aptos-node"

# Build Docker image using Nix
docker-nix:
    @echo "Building Docker image with Nix..."
    nix build .#aptos-core-docker
    @echo "Docker image available as result.tar.gz"

# Build Docker image
docker:
    @echo "Building Docker image..."
    docker build -f nix/Dockerfile.nix -t aptos-core .

# Help - list available recipes
help:
    @just --list