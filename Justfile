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

<<<<<<< Updated upstream
# Build any binary by package name
build-bin package:
    @echo "Building {{package}} with Nix development shell..."
    nix develop -c cargo build --release -p {{package}}
    @echo "Binary available at target/release/{{package}}"
=======
# Build container using Docker buildx with Nix (works on macOS/Apple Silicon)
container-buildx container="aptos-node" tag="":
    #!/usr/bin/env bash
    # Default tag to git short SHA if not provided
    if [ -z "{{tag}}" ]; then
        TAG=$(git rev-parse --short HEAD)
    else
        TAG="{{tag}}"
    fi

    echo "Building {{container}} container using Docker buildx with Nix..."
    echo "Tag: $TAG"
    echo ""
    echo "This builds inside a Linux container using Nix, then creates a minimal runtime image."
    echo "Works on macOS/Apple Silicon by building inside Docker's Linux VM."
    echo ""

    # Check for Dockerfile.nix
    if [ ! -f "docker/{{container}}/Dockerfile.nix" ]; then
        echo "Error: docker/{{container}}/Dockerfile.nix not found"
        exit 1
    fi

    # Build using buildx for linux/amd64 platform
    docker buildx build \
        --platform linux/amd64 \
        -f docker/{{container}}/Dockerfile.nix \
        --build-arg BUILD_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --build-arg GIT_SHA="$(git rev-parse HEAD)" \
        --build-arg GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD)" \
        --build-arg GIT_TAG="$(git describe --tags --always 2>/dev/null || echo 'none')" \
        -t ghcr.io/movementlabsxyz/{{container}}:$TAG \
        --load \
        .

    echo ""
    echo "Container built successfully!"
    echo "  Image: ghcr.io/movementlabsxyz/{{container}}:$TAG"
    echo ""
    echo "To test: docker run --rm ghcr.io/movementlabsxyz/{{container}}:$TAG --version"

# ==============================================================================
# Local Validator Commands (Docker Compose)
# ==============================================================================

# Start a local validator node using Docker Compose
start-local-validator config_dir="./docker/config":
    #!/usr/bin/env bash
    echo "Starting local validator node..."
    echo "Config directory: {{config_dir}}"
    echo ""

    # Check if config directory exists
    if [ ! -d "{{config_dir}}" ]; then
        echo "Error: Configuration directory '{{config_dir}}' not found."
        echo ""
        echo "To set up, create the directory and add required files:"
        echo "  mkdir -p {{config_dir}}"
        echo "  cp docker/config-example/validator.yaml.example {{config_dir}}/validator.yaml"
        echo "  # Download genesis.blob and waypoint.txt for your network"
        echo ""
        echo "See docker/config-example/README.md for more details."
        exit 1
    fi

    # Check for required config files
    for file in validator.yaml genesis.blob waypoint.txt; do
        if [ ! -f "{{config_dir}}/$file" ]; then
            echo "Error: Required file '{{config_dir}}/$file' not found."
            echo "See docker/config-example/README.md for setup instructions."
            exit 1
        fi
    done

    CONFIG_DIR="{{config_dir}}" docker compose -f docker/docker-compose.yml up -d

    echo ""
    echo "Validator started!"
    echo ""
    echo "REST API: http://localhost:8080/v1"
    echo "Metrics:  http://localhost:9101/metrics"
    echo ""
    echo "Use 'just validator-logs' to view logs"
    echo "Use 'just stop-local-validator' to stop"

# Stop the local validator node
stop-local-validator:
    @echo "Stopping local validator..."
    docker compose -f docker/docker-compose.yml down
    @echo "Validator stopped."

# View local validator logs
validator-logs:
    @echo "Streaming validator logs (Ctrl+C to exit)..."
    docker compose -f docker/docker-compose.yml logs -f

# Check local validator status
validator-status:
    #!/usr/bin/env bash
    echo "Local Validator Status"
    echo "======================"
    echo ""

    # Check container status
    if docker compose -f docker/docker-compose.yml ps --format json 2>/dev/null | grep -q "aptos-validator"; then
        echo "Container: RUNNING"
        docker compose -f docker/docker-compose.yml ps
    else
        echo "Container: NOT RUNNING"
        echo ""
        echo "Use 'just start-local-validator' to start the validator."
        exit 0
    fi

    echo ""

    # Check health
    HEALTH=$(docker inspect aptos-validator --format='{{`{{.State.Health.Status}}`}}' 2>/dev/null || echo "unknown")
    echo "Health: $HEALTH"

    # Try to get node info
    echo ""
    echo "REST API Status:"
    if curl -sf http://localhost:8080/v1 2>/dev/null | head -c 200; then
        echo ""
    else
        echo "  Unable to connect (node may still be starting)"
    fi

# ==============================================================================
# Utility Commands
# ==============================================================================

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    rm -rf result target

# Update flake.lock
update:
    @echo "Updating flake.lock..."
    nix flake update
>>>>>>> Stashed changes

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
<<<<<<< Updated upstream
    @echo "Use 'just build' to build all packages"
    @echo "Use 'just build-bin <package-name>' for custom package builds"
=======
    @echo "================================================================================"
    @echo "                        Available Binary Build Targets"
    @echo "================================================================================"
    @echo ""
    @echo "NIX BUILD TARGETS (reproducible, cached via Cachix):"
    @echo "  just build-nix aptos-node              - Main Aptos node binary"
    @echo "  just build-nix movement                - Movement CLI (renamed from aptos)"
    @echo "  just build-nix l1-migration            - L1 migration tool"
    @echo "  just build-nix aptos-faucet-service    - Faucet service for test networks"
    @echo "  just build-nix aptos-transaction-emitter - Transaction testing tool"
    @echo "  just build-all-nix                     - Build all five binaries"
    @echo ""
    @echo "CARGO BUILD TARGETS (faster iteration, uses dev shell):"
    @echo "  just build aptos-node                  - Build with cargo (dev profile)"
    @echo "  just build aptos-node release          - Build with cargo (release profile)"
    @echo "  just build                             - Build entire workspace"
    @echo "  just build-bin <package-name>          - Build any cargo package"
    @echo ""
    @echo "CONTAINER TARGETS (Nix-based containers, Linux only):"
    @echo "  just container-nix aptos-node          - Build aptos-node container"
    @echo "  just container-nix aptos-faucet-service - Build faucet container"
    @echo "  just container-load <name>             - Load container into Docker"
    @echo "  just container-push <name> [tag]       - Push container to GHCR"
    @echo ""
    @echo "CONTAINER TARGETS (Docker buildx, works on macOS/Apple Silicon):"
    @echo "  just container-buildx aptos-node       - Build container using buildx"
    @echo "  just container-buildx aptos-node v1.0  - Build with specific tag"
    @echo ""
    @echo "LOCAL VALIDATOR (Docker Compose):"
    @echo "  just start-local-validator             - Start validator with docker-compose"
    @echo "  just stop-local-validator              - Stop validator container"
    @echo "  just validator-logs                    - Stream validator logs"
    @echo "  just validator-status                  - Check validator health"
    @echo ""
    @echo "CACHIX (share builds with team):"
    @echo "  just cache-push <binary>               - Build and push to Cachix"
    @echo "  just cache-push-all                    - Build and push all binaries"
    @echo "  just cache-status                      - Check Cachix setup status"
    @echo ""
    @echo "================================================================================"
>>>>>>> Stashed changes

# Help - list available recipes
help:
    @just --list
    @echo ""
<<<<<<< Updated upstream
    @echo "Binary Build Options:"
    @echo "  Use 'just list-binaries' to see available binary build targets"
    @echo "  Use 'just build <binary-name>' for common binary builds"
    @echo "  Use 'just build' to build all packages"
    @echo "  Use 'just build-bin <package-name>' for custom package builds"
=======
    @echo "================================================================================"
    @echo "                              Quick Reference"
    @echo "================================================================================"
    @echo ""
    @echo "DEVELOPMENT:"
    @echo "  just dev                 - Enter Nix development shell"
    @echo "  just build               - Build all with cargo (fast iteration)"
    @echo "  just test                - Run tests"
    @echo ""
    @echo "PRODUCTION BUILDS:"
    @echo "  just build-nix <binary>  - Build single binary (reproducible, cached)"
    @echo "  just build-all-nix       - Build all binaries (reproducible, cached)"
    @echo ""
    @echo "CONTAINERS:"
    @echo "  just container-buildx <name> - Build container (macOS/Apple Silicon)"
    @echo "  just container-nix <name>    - Build container with Nix (Linux only)"
    @echo "  just container-push <name>   - Push to GHCR"
    @echo ""
    @echo "LOCAL VALIDATOR:"
    @echo "  just start-local-validator - Start validator with docker-compose"
    @echo "  just stop-local-validator  - Stop validator"
    @echo "  just validator-status      - Check health"
    @echo ""
    @echo "SHARE BUILDS (Cachix):"
    @echo "  just cache-push-all        - Build all & push to cache"
    @echo "  just cache-status          - Check Cachix setup"
    @echo ""
    @echo "Use 'just list-binaries' for complete list of build targets"
    @echo "================================================================================"
>>>>>>> Stashed changes
