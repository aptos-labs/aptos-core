# Justfile for Aptos Core with Nix build system

# Set the shell to bash
set shell := ["bash", "-c"]

# Default target
default: build

# Build the project or a specific binary using Nix development shell
build binary="all":
    #!/usr/bin/env bash
    if [ "{{binary}}" = "all" ]; then
        echo "Building Aptos Core with Nix development shell..."
        nix develop -c cargo build --release
    else
        case "{{binary}}" in
            "aptos-node")
                package="aptos-node"
                ;;
            "aptos")
                package="crates/aptos"
                ;;
            "aptos-debugger")
                package="crates/aptos-debugger"
                ;;
            "aptos-backup-cli")
                package="storage/backup/backup-cli"
                ;;
            "aptos-keygen")
                package="crates/aptos-keygen"
                ;;
            "transaction-emitter")
                package="crates/transaction-emitter"
                ;;
            "aptos-node-checker")
                package="ecosystem/node-checker"
                ;;
            *)
                echo "Error: Unknown binary '{{binary}}'. Use 'just list-binaries' to see available binaries."
                exit 1
                ;;
        esac
        
        echo "Building {{binary}} with Nix development shell..."
        nix develop -c cargo build --release -p "$package"
        echo "Binary available at target/release/{{binary}}"
    fi

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

# Build Docker image
docker:
    @echo "Building Docker image..."
    docker build -f nix/Dockerfile.nix -t aptos-node .

# Build any binary by package name
build-bin package:
    @echo "Building {{package}} with Nix development shell..."
    nix develop -c cargo build --release -p {{package}}
    @echo "Binary available at target/release/{{package}}"

# List available binary build targets
list-binaries:
    @echo "Available binary build targets:"
    @echo "  Generic: just build <binary-name>"
    @echo "  Common binaries:"
    @echo "    aptos-node          - Main Aptos node"
    @echo "    aptos               - Aptos CLI tool"
    @echo "    aptos-debugger      - Debugging tool"
    @echo "    aptos-backup-cli    - Backup CLI tool"
    @echo "    aptos-keygen        - Key generation tool"
    @echo "    transaction-emitter - Transaction emitter"
    @echo "    aptos-node-checker  - Node checker tool"
    @echo ""
    @echo "Use 'just build' to build all packages"
    @echo "Use 'just build-bin <package-name>' for custom package builds"

# Help - list available recipes
help:
    @just --list
    @echo ""
    @echo "Binary Build Options:"
    @echo "  Use 'just list-binaries' to see available binary build targets"
    @echo "  Use 'just build <binary-name>' for common binary builds"
    @echo "  Use 'just build' to build all packages"
    @echo "  Use 'just build-bin <package-name>' for custom package builds"