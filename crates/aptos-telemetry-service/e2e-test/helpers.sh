#!/bin/bash
# Helper functions for E2E testing
# Source this file to use the helper functions

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DIR="$SCRIPT_DIR/test-data"
MOVE_DIR="$SCRIPT_DIR/move"

# ============================================================================
# Set up APTOS_CMD
# ============================================================================
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
# Load environment from .env file
# ============================================================================
load_env() {
    if [ -f "$TEST_DIR/.env" ]; then
        set -a
        source "$TEST_DIR/.env"
        set +a
        return 0
    else
        echo -e "${RED}Error: .env file not found at $TEST_DIR/.env${NC}"
        return 1
    fi
}

# ============================================================================
# Create a new account
# Args: $1 = account_name (used for key file and profile naming)
# Returns: Sets CREATED_ACCOUNT_ADDRESS and CREATED_ACCOUNT_KEY_HEX
# ============================================================================
create_account() {
    local account_name="$1"
    
    if [ -z "$account_name" ]; then
        echo -e "${RED}Error: account_name required${NC}"
        return 1
    fi

    setup_aptos_cmd
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    local key_file="$TEST_DIR/${account_name}.key"
    local profile_name="telemetry-e2e-${account_name}"

    # Generate key
    $APTOS_CMD key generate --output-file "$key_file" --key-type ed25519 2>/dev/null

    # Get the key hex
    CREATED_ACCOUNT_KEY_HEX="0x$(cat "$key_file")"

    # Initialize profile (this also funds the account via faucet in local testnet)
    $APTOS_CMD init \
        --profile "$profile_name" \
        --network local \
        --assume-yes \
        --private-key "$CREATED_ACCOUNT_KEY_HEX" 2>/dev/null

    # Extract the address from config
    CREATED_ACCOUNT_ADDRESS=$(yq e ".profiles.\"$profile_name\".account" "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"')

    if [ -z "$CREATED_ACCOUNT_ADDRESS" ]; then
        echo -e "${RED}Error: Failed to create account $account_name${NC}"
        return 1
    fi

    echo -e "${GREEN}✓ Created account:${NC} $account_name"
    echo "  Address: $CREATED_ACCOUNT_ADDRESS"
    
    export CREATED_ACCOUNT_ADDRESS
    export CREATED_ACCOUNT_KEY_HEX
}

# ============================================================================
# Add a member to the telemetry registry
# Args: $1 = member_address, $2 = ip (optional), $3 = port (optional), 
#       $4 = node_id (optional), $5 = datacenter (optional)
# ============================================================================
add_member_to_registry() {
    local member_address="$1"
    local ip="${2:-127.0.0.1}"
    local port="${3:-9000}"
    local node_id="${4:-0x$(openssl rand -hex 16)}"
    local datacenter="${5:-dc_local}"

    if [ -z "$member_address" ]; then
        echo -e "${RED}Error: member_address required${NC}"
        return 1
    fi

    # Load env if DEPLOYER_ADDRESS not set
    if [ -z "$DEPLOYER_ADDRESS" ]; then
        load_env || return 1
    fi

    setup_aptos_cmd

    $APTOS_CMD move run \
        --profile telemetry-service-e2e-test \
        --function-id "${DEPLOYER_ADDRESS}::telemetry_registry::add_member" \
        --args "address:$member_address" "string:$ip" "string:$port" "string:$node_id" "string:$datacenter" \
        --assume-yes \
        --url http://localhost:8080 \
        2>/dev/null

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Added member to registry:${NC} $member_address"
    else
        echo -e "${RED}✗ Failed to add member:${NC} $member_address"
        return 1
    fi
}

# ============================================================================
# Send telemetry data using test-client
# Args: $1 = private_key_hex, $2 = iterations (optional, default 1)
# ============================================================================
send_telemetry() {
    local private_key="$1"
    local iterations="${2:-1}"

    if [ -z "$private_key" ]; then
        echo -e "${RED}Error: private_key required${NC}"
        return 1
    fi

    cd "$SCRIPT_DIR/test-client"
    
    # Run test client (iterations flag is on the 'all' subcommand)
    cargo run -q -p telemetry-test-client -- \
        -p "$private_key" \
        all --iterations "$iterations" 2>&1

    local exit_code=$?
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✓ Telemetry sent successfully${NC}"
    else
        echo -e "${RED}✗ Failed to send telemetry (exit code: $exit_code)${NC}"
        return 1
    fi
}

# ============================================================================
# Verify metrics in VictoriaMetrics
# Args: $1 = metric_name
# ============================================================================
verify_metrics_victoria() {
    local metric_name="$1"
    
    # Use series endpoint which finds metrics even if not recently updated
    local result=$(curl -s "http://localhost:8428/api/v1/series?match[]=$metric_name" | jq -r '.data | length')
    
    if [ "$result" -gt 0 ]; then
        echo -e "${GREEN}✓ Metric '$metric_name' found in VictoriaMetrics ($result series)${NC}"
        return 0
    else
        echo -e "${RED}✗ Metric '$metric_name' not found in VictoriaMetrics${NC}"
        return 1
    fi
}

# ============================================================================
# Verify metrics in Prometheus
# Args: $1 = metric_name
# ============================================================================
verify_metrics_prometheus() {
    local metric_name="$1"
    
    # Use series endpoint which finds metrics even if not recently updated
    local result=$(curl -s "http://localhost:9090/api/v1/series?match[]=$metric_name" | jq -r '.data | length')
    
    if [ "$result" -gt 0 ]; then
        echo -e "${GREEN}✓ Metric '$metric_name' found in Prometheus ($result series)${NC}"
        return 0
    else
        echo -e "${RED}✗ Metric '$metric_name' not found in Prometheus${NC}"
        return 1
    fi
}

# ============================================================================
# Query Loki logs
# Args: $1 = label query (e.g., {contract_name="e2e_test_contract"})
# ============================================================================
verify_logs_loki() {
    local query="$1"
    
    # URL encode the query
    local encoded_query=$(echo "$query" | jq -sRr @uri)
    local result=$(curl -s "http://localhost:3100/loki/api/v1/query?query=$encoded_query" | jq -r '.data.result | length')
    
    if [ "$result" -gt 0 ]; then
        echo -e "${GREEN}✓ Logs found in Loki ($result streams)${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠ No logs found in Loki for query: $query${NC}"
        return 1
    fi
}

# ============================================================================
# Print a section header
# ============================================================================
print_section() {
    local title="$1"
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $title${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Export all functions
export -f setup_aptos_cmd
export -f load_env
export -f create_account
export -f add_member_to_registry
export -f send_telemetry
export -f verify_metrics_victoria
export -f verify_metrics_prometheus
export -f verify_logs_loki
export -f print_section

