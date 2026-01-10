#!/bin/bash
# E2E Test Setup Script
# Sets up local test environment for Aptos Telemetry Service with custom contract authentication
#
# Usage:
#   ./setup.sh           # Run all steps (1-7)
#   ./setup.sh --step 5  # Run only step 5
#   ./setup.sh -s 3      # Run only step 3
#   ./setup.sh --from 4  # Run steps 4-7
#   ./setup.sh --help    # Show help

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DIR="$SCRIPT_DIR/test-data"
MOVE_DIR="$SCRIPT_DIR/move"

# Default values
RUN_STEP=""
RUN_FROM=""

# Parse arguments
show_help() {
    echo "E2E Test Setup Script"
    echo ""
    echo "Usage:"
    echo "  ./setup.sh              Run all steps (1-8)"
    echo "  ./setup.sh --step N     Run only step N"
    echo "  ./setup.sh -s N         Run only step N"
    echo "  ./setup.sh --from N     Run steps N through 8"
    echo "  ./setup.sh --skip-node  Skip starting the Aptos test node"
    echo "  ./setup.sh --help       Show this help"
    echo ""
    echo "Steps:"
    echo "  1 - Check prerequisites"
    echo "  2 - Start Docker services (VictoriaMetrics, Prometheus, Loki, Grafana)"
    echo "  3 - Start Aptos test node"
    echo "  4 - Create test accounts"
    echo "  5 - Deploy Move contract"
    echo "  6 - Add test member to registry"
    echo "  7 - Set up environment variables"
    echo "  8 - Start telemetry service (background)"
    echo ""
    echo "Examples:"
    echo "  ./setup.sh --step 5     # Redeploy the Move contract"
    echo "  ./setup.sh --from 4     # Recreate accounts and redeploy"
    echo "  ./setup.sh -s 2         # Restart Docker services only"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--step)
            RUN_STEP="$2"
            shift 2
            ;;
        --from)
            RUN_FROM="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        --skip-node)
            SKIP_NODE="true"
            shift 1
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            exit 1
            ;;
    esac
done

# Determine which steps to run
should_run_step() {
    local step=$1
    if [ -n "$RUN_STEP" ]; then
        [ "$step" -eq "$RUN_STEP" ]
    elif [ -n "$RUN_FROM" ]; then
        [ "$step" -ge "$RUN_FROM" ]
    else
        return 0  # Run all steps
    fi
}

# Load environment if it exists (for running individual steps)
# Falls back to reading from config files if .env doesn't exist
load_env() {
    if [ -f "$TEST_DIR/.env" ]; then
        echo -e "${BLUE}Loading existing environment from $TEST_DIR/.env${NC}"
        source "$TEST_DIR/.env"
    else
        echo -e "${BLUE}Loading environment from config files...${NC}"
        # Load from .aptos/config.yaml if it exists
        if [ -f "$TEST_DIR/.aptos/config.yaml" ]; then
            export DEPLOYER_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test.account' "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"' || echo "")
            export TEST_ACCOUNT_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test-member.account' "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"' || echo "")
            echo "  DEPLOYER_ADDRESS=$DEPLOYER_ADDRESS"
            echo "  TEST_ACCOUNT_ADDRESS=$TEST_ACCOUNT_ADDRESS"
        fi
        # Load keys from key files if they exist
        if [ -f "$TEST_DIR/deployer.key" ]; then
            export DEPLOYER_KEY_HEX="0x$(cat $TEST_DIR/deployer.key)"
        fi
        if [ -f "$TEST_DIR/test-member.key" ]; then
            export TEST_ACCOUNT_KEY_HEX="0x$(cat $TEST_DIR/test-member.key)"
        fi
    fi
}

# Set up APTOS_CMD (called once at startup)
setup_aptos_cmd() {
    if [ -n "$APTOS_CMD" ]; then
        return
    fi
    if command -v aptos &> /dev/null; then
        export APTOS_CMD="aptos"
    else
        echo -e "${YELLOW}Warning: Aptos CLI not found in PATH, using cargo${NC}"
        export APTOS_CMD="cargo run -p aptos --"
    fi
}

# ============================================================================
# Step 1: Check prerequisites
# ============================================================================
step1_check_prerequisites() {
    echo -e "${YELLOW}[1/8]${NC} Checking prerequisites..."

    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Error: Docker is not installed${NC}"
        echo "Please install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo -e "${RED}Error: Docker Compose is not installed${NC}"
        echo "Please install Docker Compose: https://docs.docker.com/compose/install/"
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Cargo is not installed${NC}"
        echo "Please install Rust: https://rustup.rs/"
        exit 1
    fi

    echo -e "${GREEN}✓ All prerequisites met${NC}"
    echo ""
}

# ============================================================================
# Step 2: Start Docker services
# ============================================================================
step2_start_docker() {
    echo -e "${YELLOW}[2/8]${NC} Starting Docker services (VictoriaMetrics, Prometheus, Loki, Grafana)..."
    cd "$SCRIPT_DIR"
    docker-compose up -d

    # Wait for services to be healthy
    echo "Waiting for services to be ready..."
    sleep 5

    # Check VictoriaMetrics health
    for i in {1..30}; do
        if curl -s http://localhost:8428/health > /dev/null; then
            echo -e "${GREEN}✓ VictoriaMetrics is ready${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}Error: VictoriaMetrics failed to start${NC}"
            docker-compose logs victoria-metrics
            exit 1
        fi
        sleep 1
    done

    # Check Prometheus health
    for i in {1..30}; do
        if curl -s http://localhost:9090/-/healthy > /dev/null; then
            echo -e "${GREEN}✓ Prometheus is ready${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}Error: Prometheus failed to start${NC}"
            docker-compose logs prometheus
            exit 1
        fi
        sleep 1
    done

    # Check Loki health
    for i in {1..30}; do
        if curl -s http://localhost:3100/ready > /dev/null; then
            echo -e "${GREEN}✓ Loki is ready${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}Error: Loki failed to start${NC}"
            docker-compose logs loki
            exit 1
        fi
        sleep 1
    done

    echo -e "${GREEN}✓ All Docker services started${NC}"
    echo ""
}

# ============================================================================
# Step 3: Start Aptos test node
# ============================================================================
step3_start_node() {
    if [ -n "$SKIP_NODE" ]; then
        echo -e "${YELLOW}[3/8]${NC} Skipping Aptos test node...${NC}"
        echo ""
        return
    fi

    echo -e "${YELLOW}[3/8]${NC} Starting Aptos test node..."
    mkdir -p "$TEST_DIR"

    # Kill any existing aptos-node process on port 8080
    if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo "Killing existing process on port 8080..."
        kill $(lsof -t -i:8080) 2>/dev/null || true
        sleep 2
    fi

    # Start aptos-node in test mode in the background
    export TELEMETRY_SERVICE_URL=http://localhost:8082
    cd "$PROJECT_ROOT/../.."  # Navigate to aptos-core root
    echo "Starting aptos local testnet..."
    ulimit -n unlimited 2>/dev/null || true # For the node
    nohup $APTOS_CMD node run-local-testnet --faucet-port 8081 --force-restart --assume-yes --test-dir "$TEST_DIR" --no-txn-stream --skip-metadata-apply > "$TEST_DIR/node.log" 2>&1 &
    NODE_PID=$!
    echo $NODE_PID > "$TEST_DIR/node.pid"

    # Wait for node to be ready
    echo "Waiting for node to be ready (this may take a minute)..."
    for i in {1..60}; do
        if curl -s http://localhost:8080/v1/-/healthy > /dev/null 2>&1 && curl -s http://localhost:8080/v1/consensus_health_check > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Aptos node is ready${NC}"
            break
        fi
        if [ $i -eq 60 ]; then
            echo -e "${RED}Error: Aptos node failed to start${NC}"
            echo "Check logs at: $TEST_DIR/node.log"
            cat "$TEST_DIR/node.log"
            exit 1
        fi
        sleep 5
    done

    echo "Waiting for DB to bootstrap..."
    for i in {1..60}; do
        RESPONSE=$(curl -s http://localhost:8080/v1 2>/dev/null || echo "")
        if [[ "$RESPONSE" != *"bootstrapping"* ]] && [[ -n "$RESPONSE" ]]; then
            echo -e "${GREEN}✓ DB bootstrap complete${NC}"
            break
        fi
        if [ $i -eq 60 ]; then
            echo -e "${RED}Error: DB bootstrap timed out${NC}"
            exit 1
        fi
        echo "Still bootstrapping..."
        sleep 5
    done

    # Get the chain ID
    CHAIN_ID=$(curl -s http://localhost:8080/v1 | grep -o '"chain_id":[0-9]*' | cut -d':' -f2 || echo "4")
    echo "Node chain ID: $CHAIN_ID"
    echo ""
}

# ============================================================================
# Step 4: Create test accounts
# ============================================================================
step4_create_accounts() {
    echo -e "${YELLOW}[4/8]${NC} Creating test accounts..."

    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    # Create deployer account profile
    echo "Creating deployer account..."
    $APTOS_CMD key generate --output-file $TEST_DIR/deployer.key --key-type ed25519
    DEPLOYER_KEY_HEX="0x$(cat $TEST_DIR/deployer.key)"
    $APTOS_CMD init --profile telemetry-service-e2e-test --network local --assume-yes --private-key "$DEPLOYER_KEY_HEX" 2>&1 | tee init.log || true
    DEPLOYER_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test.account' "$TEST_DIR/.aptos/config.yaml" | tr -d '"' || echo "")
    echo "Deployer account address: $DEPLOYER_ADDRESS"

    # Create test member account profile
    echo "Creating test member account..."
    $APTOS_CMD key generate --output-file $TEST_DIR/test-member.key --key-type ed25519
    TEST_ACCOUNT_KEY_HEX="0x$(cat $TEST_DIR/test-member.key)"
    $APTOS_CMD init --profile telemetry-service-e2e-test-member --network local --assume-yes --private-key "$TEST_ACCOUNT_KEY_HEX" 2>&1 | tee -a init.log || true
    TEST_ACCOUNT_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test-member.account' "$TEST_DIR/.aptos/config.yaml" | tr -d '"' || echo "")
    echo "Test member account address: $TEST_ACCOUNT_ADDRESS"

    # Copy config file to move dir for consistency
    mkdir -p "$MOVE_DIR/.aptos"
    cp "$TEST_DIR/.aptos/config.yaml" "$MOVE_DIR/.aptos/config.yaml"

    # Export for subsequent steps
    export DEPLOYER_ADDRESS
    export DEPLOYER_KEY_HEX
    export TEST_ACCOUNT_ADDRESS
    export TEST_ACCOUNT_KEY_HEX

    echo -e "${GREEN}✓ Test accounts created${NC}"
    echo ""
}

# ============================================================================
# Step 5: Deploy Move contract
# ============================================================================
step5_deploy_contract() {
    echo -e "${YELLOW}[5/8]${NC} Deploying Move contract..."
    
    # Load env if running standalone
    if [ -z "$DEPLOYER_ADDRESS" ]; then
        load_env
    fi

    if [ -z "$DEPLOYER_ADDRESS" ]; then
        echo -e "${RED}Error: DEPLOYER_ADDRESS not set. Run step 4 first or ensure .env exists.${NC}"
        exit 1
    fi

    cd "$MOVE_DIR"

    # Create a temporary Move.toml with the correct address
    sed "s/telemetry_deployer = \".*\"/telemetry_deployer = \"$DEPLOYER_ADDRESS\"/" Move.toml > Move.toml.tmp
    mv Move.toml.tmp Move.toml

    echo "Compiling Move contract..."
    $APTOS_CMD move compile --named-addresses telemetry_deployer=$DEPLOYER_ADDRESS --dev --skip-checks-on-test-code --language-version 2.3

    echo "Publishing Move contract..."
    $APTOS_CMD move publish \
        --profile telemetry-service-e2e-test \
        --named-addresses telemetry_deployer=$DEPLOYER_ADDRESS \
        --assume-yes \
        --url http://localhost:8080

    echo "Initializing registry..."
    $APTOS_CMD move run \
        --profile telemetry-service-e2e-test \
        --function-id ${DEPLOYER_ADDRESS}::telemetry_registry::initialize \
        --assume-yes \
        --url http://localhost:8080

    echo -e "${GREEN}✓ Contract deployed and initialized at $DEPLOYER_ADDRESS${NC}"
    echo -e "${YELLOW}View it at https://explorer.aptoslabs.com/account/$DEPLOYER_ADDRESS/modules/packages/TelemetryRegistry?network=local${NC}"
    echo ""
}

# ============================================================================
# Step 6: Add test member to registry
# ============================================================================
step6_add_member() {
    echo -e "${YELLOW}[6/8]${NC} Adding test member to registry..."

    # Load env if running standalone
    if [ -z "$DEPLOYER_ADDRESS" ] || [ -z "$TEST_ACCOUNT_ADDRESS" ]; then
        load_env
    fi

    if [ -z "$DEPLOYER_ADDRESS" ] || [ -z "$TEST_ACCOUNT_ADDRESS" ]; then
        echo -e "${RED}Error: DEPLOYER_ADDRESS or TEST_ACCOUNT_ADDRESS not set. Run steps 4-5 first.${NC}"
        exit 1
    fi

    $APTOS_CMD move run \
        --profile telemetry-service-e2e-test \
        --function-id ${DEPLOYER_ADDRESS}::telemetry_registry::add_member \
        --args address:$TEST_ACCOUNT_ADDRESS string:"127.0.0.1" string:"9000" string:"0xtest123" string:"dc_local" \
        --assume-yes \
        --url http://localhost:8080

    echo -e "${GREEN}✓ Test member added to registry${NC}"
    echo -e "${YELLOW}View it at https://explorer.aptoslabs.com/account/$DEPLOYER_ADDRESS/modules/packages/TelemetryRegistry?network=local${NC}"
    echo ""
}

# ============================================================================
# Step 7: Set up environment variables
# ============================================================================
step7_setup_env() {
    echo -e "${YELLOW}[7/8]${NC} Setting up environment variables..."

    # Load existing env values if running standalone
    if [ -z "$DEPLOYER_ADDRESS" ]; then
        if [ -f "$TEST_DIR/.aptos/config.yaml" ]; then
            DEPLOYER_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test.account' "$TEST_DIR/.aptos/config.yaml" | tr -d '"' || echo "")
            TEST_ACCOUNT_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test-member.account' "$TEST_DIR/.aptos/config.yaml" | tr -d '"' || echo "")
        fi
        if [ -f "$TEST_DIR/deployer.key" ]; then
            DEPLOYER_KEY_HEX="0x$(cat $TEST_DIR/deployer.key)"
        fi
        if [ -f "$TEST_DIR/test-member.key" ]; then
            TEST_ACCOUNT_KEY_HEX="0x$(cat $TEST_DIR/test-member.key)"
        fi
    fi

    # Get chain ID from node
    CHAIN_ID=$(curl -s http://localhost:8080/v1 | grep -o '"chain_id":[0-9]*' | cut -d':' -f2 || echo "4")

    # Generate x25519 private key for SERVER_PRIVATE_KEY (32 bytes hex encoded)
    SERVER_PRIVATE_KEY=$(openssl rand -hex 32)

    # Generate JWT signing key (32 bytes hex encoded)
    JWT_SIGNING_KEY=$(openssl rand -hex 32)

    # Create dummy GCP credentials file for local testing
    mkdir -p "$TEST_DIR"
    cat > "$TEST_DIR/dummy-gcp-credentials.json" << 'GCPEOF'
{
  "type": "service_account",
  "project_id": "local-test",
  "private_key_id": "dummy",
  "private_key": "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBALRiMLAHHLOflX0lPJj+D2Sc2wfBxAFmX93u5gWJ0xgqPr3v9y5d\ndkJP3CJG3F3r3d3s4F5hXq3A4p5yZ2r3XM0CAwEAAQJANLr8FxlMg3xbYPsS3fE3\nN7E9rEDBIh8bKL+RYSQ3ZhB3X2Gxf4qLX5p1X5E5X5K5X5L5X5M5X5N5X5O5X5P5\nQQIhAORD3X2Z5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5AiEAyF3X5X5X5X\n5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5UCIQCe3X5X5X5X5X5X5X5X5X5X5X5X5X\n5X5X5X5X5X5X5QIgW3X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5UCIBt3\nX5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5X5U=\n-----END RSA PRIVATE KEY-----\n",
  "client_email": "dummy@local-test.iam.gserviceaccount.com",
  "client_id": "000000000000000000000",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token"
}
GCPEOF

    # Create .env file
    cat > "$TEST_DIR/.env" << EOF
# E2E Test Environment Variables
# Generated by setup.sh

# Server private key for telemetry service (x25519, hex encoded)
SERVER_PRIVATE_KEY=$SERVER_PRIVATE_KEY

# JWT signing key for token generation (32 bytes hex encoded)
JWT_SIGNING_KEY=$JWT_SIGNING_KEY

# GCP credentials (dummy for local testing - BigQuery not used)
GOOGLE_APPLICATION_CREDENTIALS=$TEST_DIR/dummy-gcp-credentials.json

# Contract deployer account
DEPLOYER_ADDRESS=$DEPLOYER_ADDRESS
DEPLOYER_KEY_HEX=$DEPLOYER_KEY_HEX

# Test account for authentication
TEST_ACCOUNT_ADDRESS=$TEST_ACCOUNT_ADDRESS
TEST_ACCOUNT_KEY_HEX=$TEST_ACCOUNT_KEY_HEX
TEST_CONTRACT_ADDRESS=$DEPLOYER_ADDRESS

# Chain ID
CHAIN_ID=$CHAIN_ID

# Metrics keys (empty for local testing - no auth)
TEST_METRICS_KEYS={"local":""}

# Loki token (empty for local testing - no auth)
TEST_LOKI_TOKEN=

# Service endpoints (telemetry on 8082, faucet on 8081)
TELEMETRY_SERVICE_URL=http://localhost:8082
FAUCET_URL=http://localhost:8081
NODE_REST_API=http://localhost:8080
VICTORIA_METRICS_URL=http://localhost:8428
PROMETHEUS_URL=http://localhost:9090
LOKI_URL=http://localhost:3100
GRAFANA_URL=http://localhost:3000
EOF

    echo -e "${GREEN}✓ Environment file created at $TEST_DIR/.env${NC}"
    echo ""
}
# ============================================================================
# Step 8: Start the telemetry service (background)
# ============================================================================
step8_start_telemetry() {
    echo -e "${YELLOW}[8/8]${NC} Starting telemetry service..."

    # Load environment variables
    if [ -f "$TEST_DIR/.env" ]; then
        set -a
        source "$TEST_DIR/.env"
        set +a
    else
        echo -e "${RED}Error: .env file not found. Run step 7 first.${NC}"
        exit 1
    fi

    # Stop any existing telemetry service
    if [ -f "$TEST_DIR/telemetry.pid" ]; then
        EXISTING_PID=$(cat "$TEST_DIR/telemetry.pid")
        if ps -p $EXISTING_PID > /dev/null 2>&1; then
            echo "Stopping existing telemetry service (PID: $EXISTING_PID)..."
            kill $EXISTING_PID 2>/dev/null || true
            sleep 2
            # Force kill if still running
            if ps -p $EXISTING_PID > /dev/null 2>&1; then
                kill -9 $EXISTING_PID 2>/dev/null || true
                sleep 1
            fi
            echo -e "${GREEN}✓ Existing telemetry service stopped${NC}"
        fi
        rm -f "$TEST_DIR/telemetry.pid"
    fi

    # Also check if something else is using port 8082
    if lsof -Pi :8082 -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo "Stopping process on port 8082..."
        kill $(lsof -t -i:8082) 2>/dev/null || true
        sleep 2
        # Force kill if still running
        if lsof -Pi :8082 -sTCP:LISTEN -t >/dev/null 2>&1; then
            kill -9 $(lsof -t -i:8082) 2>/dev/null || true
            sleep 1
        fi
        echo -e "${GREEN}✓ Process on port 8082 stopped${NC}"
    fi

    cd "$PROJECT_ROOT"

    # Start telemetry service in background, logging to file
    echo "Starting telemetry service..."
    RUST_LOG=debug nohup cargo run -- -f "$SCRIPT_DIR/telemetry-config.yaml" > "$TEST_DIR/telemetry.log" 2>&1 &
    TELEMETRY_PID=$!
    echo $TELEMETRY_PID > "$TEST_DIR/telemetry.pid"

    # Wait for telemetry service to be ready
    echo "Waiting for telemetry service to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:8082/api/v1/health > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Telemetry service is ready (PID: $TELEMETRY_PID)${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}Error: Telemetry service failed to start${NC}"
            echo "Check logs at: $TEST_DIR/telemetry.log"
            tail -50 "$TEST_DIR/telemetry.log"
            exit 1
        fi
        sleep 2
    done

    echo -e "${BLUE}Telemetry service logs:${NC} $TEST_DIR/telemetry.log"
    echo -e "${YELLOW}Tail logs with: tail -f $TEST_DIR/telemetry.log${NC}"
    echo ""
}

# ============================================================================
# Print summary
# ============================================================================
print_summary() {
    # Get node PID if available
    NODE_PID=""
    if [ -f "$TEST_DIR/node.pid" ]; then
        NODE_PID=$(cat "$TEST_DIR/node.pid")
    fi

    # Get telemetry PID if available
    TELEMETRY_PID=""
    if [ -f "$TEST_DIR/telemetry.pid" ]; then
        TELEMETRY_PID=$(cat "$TEST_DIR/telemetry.pid")
    fi

    # Load env for display
    load_env 2>/dev/null || true

    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}✓ E2E Test Setup Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo -e "${BLUE}Services Running:${NC}"
    echo "  • Aptos Node:        http://localhost:8080 ${NODE_PID:+(PID: $NODE_PID)}"
    echo "  • Faucet:            http://localhost:8081"
    echo "  • Telemetry Service: http://localhost:8082 ${TELEMETRY_PID:+(PID: $TELEMETRY_PID)}"
    echo "  • VictoriaMetrics:   http://localhost:8428"
    echo "  • Prometheus:        http://localhost:9090"
    echo "  • Loki:              http://localhost:3100"
    echo "  • Grafana:           http://localhost:3000 (admin/admin)"
    echo ""
    echo -e "${BLUE}Module deployed at:${NC}"
    echo "  • TelemetryRegistry: https://explorer.aptoslabs.com/account/$DEPLOYER_ADDRESS/modules/packages/TelemetryRegistry?network=local"
    echo "  • View functions:    https://explorer.aptoslabs.com/account/$DEPLOYER_ADDRESS/modules/view/telemetry_registry/get_all_members?network=local"
    echo ""
    echo -e "${BLUE}Test Data:${NC}"
    echo "  • Test directory:    $TEST_DIR"
    echo "  • Contract address:  ${DEPLOYER_ADDRESS:-<not set>}"
    echo "  • Test account:      ${TEST_ACCOUNT_ADDRESS:-<not set>}"
    echo "  • Environment file:  $TEST_DIR/.env"
    echo ""
    echo -e "${BLUE}Log Files:${NC}"
    echo "  • Node logs:         $TEST_DIR/node.log"
    echo "  • Telemetry logs:    $TEST_DIR/telemetry.log"
    echo ""
    echo -e "${BLUE}Next Steps:${NC}"
    echo "  1. Run the E2E test (in another terminal):"
    echo "     cd $SCRIPT_DIR"
    echo "     ./run-test.sh"
    echo ""
    echo "  2. View telemetry data in Grafana:"
    echo "     Open http://localhost:3000 in your browser"
    echo ""
    echo -e "${YELLOW}To stop all services:${NC}"
    echo "  cd $SCRIPT_DIR"
    echo "  ./cleanup.sh"
    echo ""
}

# ============================================================================
# Main execution
# ============================================================================

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Aptos Telemetry Service E2E Test Setup${NC}"
echo -e "${BLUE}========================================${NC}"

if [ -n "$RUN_STEP" ]; then
    echo -e "${BLUE}Running step $RUN_STEP only${NC}"
elif [ -n "$RUN_FROM" ]; then
    echo -e "${BLUE}Running steps $RUN_FROM through 8${NC}"
else
    echo -e "${BLUE}Running all steps (1-8)${NC}"
fi
echo ""

# Set up APTOS_CMD globally (used by steps 3-6)
setup_aptos_cmd

# Run the requested steps
should_run_step 1 && step1_check_prerequisites
should_run_step 2 && step2_start_docker
should_run_step 3 && step3_start_node
should_run_step 4 && step4_create_accounts
should_run_step 5 && step5_deploy_contract
should_run_step 6 && step6_add_member
should_run_step 7 && step7_setup_env

# Step 8 starts the telemetry service in background
should_run_step 8 && step8_start_telemetry

# Print summary if running all steps or the last step
if [ -z "$RUN_STEP" ] || [ "$RUN_STEP" -eq 8 ]; then
    print_summary
fi
