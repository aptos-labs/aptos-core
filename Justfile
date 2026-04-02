# Justfile for Aptos Core with Nix build system

# Set the shell to bash
set shell := ["bash", "-c"]

# Default target
default: build

# Build the project or a specific binary using Nix development shell
build binary="all" profile="dev":
    #!/usr/bin/env bash
    if [ "{{profile}}" != "release" ]; then
        PROFILE_ARG="--profile {{profile}}"
    else
        PROFILE_ARG="--release"
    fi
    echo "Using profile: $PROFILE_ARG"
    if [ "{{binary}}" = "all" ]; then
        echo "Building Aptos Core with Nix development shell..."
        nix --extra-experimental-features "nix-command flakes" develop -c cargo build $PROFILE_ARG
    else
        echo "Building {{binary}} with Nix development shell..."
        nix --extra-experimental-features "nix-command flakes" develop -c cargo build $PROFILE_ARG -p {{binary}}
        echo "Binary available at target/release/{{binary}}"
    fi

# Enter the development environment
dev:
    @echo "Entering development environment..."
    nix --extra-experimental-features "nix-command flakes" develop

# Run tests
test:
    @echo "Running tests..."
    nix --extra-experimental-features "nix-command flakes" develop -c cargo test

# Check code formatting
fmt:
    @echo "Checking code formatting..."
    nix --extra-experimental-features "nix-command flakes" develop -c cargo fmt -- --check

# Run clippy
clippy:
    @echo "Running clippy..."
    nix --extra-experimental-features "nix-command flakes" develop -c cargo clippy -- --deny warnings

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    rm -rf result target

# Update flake.lock
update:
    @echo "Updating flake.lock..."
    nix flake update

# Build Docker image
container-build container="aptos-node" tag="latest" profile="release":
    #!/usr/bin/env bash
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        echo "Error: Docker is not installed. Please install Docker to build the image."
        exit 1
    fi

    # Check if Docker Buildx is available
    if ! docker buildx version &> /dev/null; then
        echo "Error: Docker Buildx is not available. Please install Docker Buildx."
        echo "You can typically install it by updating Docker Desktop or installing the buildx plugin."
        exit 1
    fi

    # Validate the docker folder exists
    if [ ! -d "docker/{{container}}" ]; then
        echo "Error: Docker folder 'docker/{{container}}' does not exist."
        exit 1
    fi

    # Build the binary first
    echo "Building {{container}}..."
    # Case for docker containers that need multiple binaries
    case "{{container}}" in
        "aptos-node")
            just build aptos-node {{profile}}
            just build aptos {{profile}}
            just build l1-migration {{profile}}
            ;;
        "aptos-debugger")
            just build aptos-debugger {{profile}}
            ;;
        "aptos-faucet-service")
            just build aptos-faucet-service {{profile}}
            ;;
        *)
            just build {{container}} {{profile}}
            ;;
    esac
    

    # Set binary path based on profile
    if [ "{{profile}}" = "release" ]; then
        BINARY_PATH="target/release"
    elif [ "{{profile}}" = "dev" ]; then
        BINARY_PATH="target/debug"
    else
        BINARY_PATH="target/{{profile}}"
    fi
    
    echo "Building Docker image for '{{container}}' using binary at $BINARY_PATH..."
    docker build \
        --build-arg BINARY_PATH="$BINARY_PATH" \
        -f docker/{{container}}/Dockerfile \
        -t ghcr.io/movementlabsxyz/{{container}}:{{tag}} .
    
    # Clean up the copied binary
    rm -f aptos-test

# Push a container image to GHCR
container-push container="aptos-node" tag="latest":
    docker push ghcr.io/movementlabsxyz/{{container}}:{{tag}}

# Build and push a container image
container-release container="aptos-node" tag="latest" profile="release":
    just container-build {{container}} {{tag}} {{profile}}
    just container-push {{container}} {{tag}}

# List available container targets
list-containers:
    @echo "Available container build targets:"
    @echo "  aptos-node          - L1 blockchain node (aptos-node + movement + l1-migration)"
    @echo "  aptos-debugger      - Database backup and restore tool"
    @echo "  aptos-faucet-service - Token faucet for test networks"
    @echo ""
    @echo "Usage:"
    @echo "  just container-build <container> [tag] [profile]"
    @echo "  just container-push <container> [tag]"
    @echo "  just container-release <container> [tag] [profile]  # build + push"

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
    @echo ""
    @echo "Container Build Options:"
    @echo "  Use 'just list-containers' to see available container targets"
    @echo "  Use 'just container-build <container>' to build a container image"
    @echo "  Use 'just container-push <container>' to push to GHCR"
    @echo "  Use 'just container-release <container>' to build + push"