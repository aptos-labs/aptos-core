#!/bin/bash
# E2E Test Setup Script
# Sets up local test environment for Aptos Telemetry Service with custom contract authentication

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

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Aptos Telemetry Service E2E Test Setup${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Step 1: Check prerequisites
echo -e "${YELLOW}[1/7]${NC} Checking prerequisites..."

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

# Check if aptos CLI is available
if ! command -v aptos &> /dev/null; then
    echo -e "${YELLOW}Warning: Aptos CLI not found in PATH${NC}"
    echo "Attempting to use cargo to run aptos CLI..."
    APTOS_CMD="cargo run -p aptos --"
else
    APTOS_CMD="aptos"
fi

echo -e "${GREEN}✓ All prerequisites met${NC}"
echo ""

# Step 2: Start Docker services (VictoriaMetrics, Loki, Grafana)
echo -e "${YELLOW}[2/7]${NC} Starting Docker services (VictoriaMetrics, Loki, Grafana)..."
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

# Step 3: Create test directory and start aptos-node in test mode
echo -e "${YELLOW}[3/7]${NC} Starting Aptos test node..."
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
nohup $APTOS_CMD node run-local-testnet --faucet-port 8081 --force-restart --assume-yes --test-dir "$TEST_DIR" --no-txn-stream --skip-metadata-apply > "$TEST_DIR/node.log" 2>&1 &
NODE_PID=$!
echo $NODE_PID > "$TEST_DIR/node.pid"

# Wait for node to be ready
echo "Waiting for node to be ready (this may take a minute)..."
for i in {1..60}; do
    if curl -s http://localhost:8080/v1/-/healthy > /dev/null 2>&1; then
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

sleep 30

# Get the chain ID (should be 4 for testnet by default)
CHAIN_ID=$(curl -s http://localhost:8080/v1 | grep -o '"chain_id":[0-9]*' | cut -d':' -f2 || echo "4")
echo "Node chain ID: $CHAIN_ID"
echo ""

# Step 4: Create test accounts (while still in test-data directory)
echo -e "${YELLOW}[4/7]${NC} Creating test accounts..."
cd "$TEST_DIR"

# Create deployer account profile
echo "Creating deployer account..."
$APTOS_CMD key generate --output-file $TEST_DIR/deployer.key --key-type ed25519
DEPLOYER_KEY_HEX="0x$(cat $TEST_DIR/deployer.key)" # USE FOR TEST ONLY
$APTOS_CMD init --profile telemetry-service-e2e-test --network local --assume-yes --private-key "$DEPLOYER_KEY_HEX" 2>&1 | tee init.log || true
DEPLOYER_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test.account' "$TEST_DIR/.aptos/config.yaml" | tr -d '"' || echo "")
echo "Deployer account address: $DEPLOYER_ADDRESS"

# Create test member account profile
echo "Creating test member account..."
$APTOS_CMD key generate --output-file $TEST_DIR/test-member.key --key-type ed25519
TEST_ACCOUNT_KEY_HEX="0x$(cat $TEST_DIR/test-member.key)" # USE FOR TEST ONLY
$APTOS_CMD init --profile telemetry-service-e2e-test-member --network local --assume-yes --private-key "$TEST_ACCOUNT_KEY_HEX" 2>&1 | tee -a init.log || true
TEST_ACCOUNT_ADDRESS=$(yq e '.profiles.telemetry-service-e2e-test-member.account' "$TEST_DIR/.aptos/config.yaml" | tr -d '"' || echo "")
echo "Test member account address: $TEST_ACCOUNT_ADDRESS"

# Copy config file to move dir for consistency
mkdir -p "$MOVE_DIR/.aptos"
cp "$TEST_DIR/.aptos/config.yaml" "$MOVE_DIR/.aptos/config.yaml"

echo -e "${GREEN}✓ Test accounts created${NC}"
echo ""

# Step 5: Deploy the Move contract
echo -e "${YELLOW}[5/7]${NC} Deploying Move contract..."
cd "$MOVE_DIR"

# Create a temporary Move.toml with the correct address
sed "s/telemetry_deployer = \"_\"/telemetry_deployer = \"$DEPLOYER_ADDRESS\"/" Move.toml > Move.toml.tmp
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
echo ""

# Step 6: Add test member to the registry
echo -e "${YELLOW}[6/7]${NC} Adding test member to registry..."

$APTOS_CMD move run \
    --profile telemetry-service-e2e-test \
    --function-id ${DEPLOYER_ADDRESS}::telemetry_registry::add_member \
    --args address:$TEST_ACCOUNT_ADDRESS string:"127.0.0.1" string:"9000" string:"0xtest123" string:"dc_local" \
    --assume-yes \
    --url http://localhost:8080

echo -e "${GREEN}✓ Test member added to registry${NC}"
echo ""

# Step 7: Set up environment variables
echo -e "${YELLOW}[7/7]${NC} Setting up environment variables..."

# Generate x25519 private key for SERVER_PRIVATE_KEY (32 bytes hex encoded)
SERVER_PRIVATE_KEY=$(openssl rand -hex 32)

# Create dummy GCP credentials file for local testing (BigQuery won't actually be used)
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
LOKI_URL=http://localhost:3100
GRAFANA_URL=http://localhost:3000
EOF

echo -e "${GREEN}✓ Environment file created at $TEST_DIR/.env${NC}"
echo ""

# Summary and next steps
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✓ E2E Test Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${BLUE}Services Running:${NC}"
echo "  • Aptos Node:        http://localhost:8080 (PID: $NODE_PID)"
echo "  • Faucet:            http://localhost:8081"
echo "  • VictoriaMetrics:   http://localhost:8428"
echo "  • Loki:              http://localhost:3100"
echo "  • Grafana:           http://localhost:3000 (admin/admin)"
echo ""
echo -e "${BLUE}Test Data:${NC}"
echo "  • Test directory:    $TEST_DIR"
echo "  • Contract address:  $DEPLOYER_ADDRESS"
echo "  • Test account:      $TEST_ACCOUNT_ADDRESS"
echo "  • Environment file:  $TEST_DIR/.env"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo "  1. Start the telemetry service:"
echo "     cd $PROJECT_ROOT"
echo "     source $TEST_DIR/.env"
echo "     cargo run --release -- -f $SCRIPT_DIR/telemetry-config.yaml"
echo ""
echo "  2. Run the E2E test:"
echo "     cd $SCRIPT_DIR"
echo "     ./run-test.sh"
echo ""
echo "  3. View telemetry data in Grafana:"
echo "     Open http://localhost:3000 in your browser"
echo ""
echo -e "${YELLOW}To stop all services:${NC}"
echo "  cd $SCRIPT_DIR"
echo "  ./cleanup.sh"
echo ""

