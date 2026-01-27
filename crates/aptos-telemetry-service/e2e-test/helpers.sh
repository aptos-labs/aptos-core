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

    # Check if key already exists - reuse it for consecutive runs
    if [ -f "$key_file" ]; then
        CREATED_ACCOUNT_KEY_HEX="0x$(cat "$key_file")"
        
        # Check if profile already exists
        local addr_val=$(yq e ".profiles.\"$profile_name\".account" "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"')
        if [ -n "$addr_val" ] && [ "$addr_val" != "null" ]; then
            CREATED_ACCOUNT_ADDRESS="$addr_val"
            echo -e "${GREEN}✓ Reusing account:${NC} $account_name"
            echo "  Address: $CREATED_ACCOUNT_ADDRESS"
            export CREATED_ACCOUNT_ADDRESS
            export CREATED_ACCOUNT_KEY_HEX
            return 0
        fi
    else
        # Generate new key (no prompt since file doesn't exist)
        $APTOS_CMD key generate --output-file "$key_file" --key-type ed25519 2>/dev/null
        CREATED_ACCOUNT_KEY_HEX="0x$(cat "$key_file")"
    fi

    # Initialize profile (this also funds the account via faucet in local testnet)
    $APTOS_CMD init \
        --profile "$profile_name" \
        --network local \
        --assume-yes \
        --private-key "$CREATED_ACCOUNT_KEY_HEX" 2>/dev/null

    # Extract the address from config
    local addr_val=$(yq e ".profiles.\"$profile_name\".account" "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"')
    
    if [ -z "$addr_val" ] || [ "$addr_val" = "null" ]; then
        echo -e "${RED}Error: Failed to create account $account_name${NC}"
        return 1
    fi
    
    CREATED_ACCOUNT_ADDRESS="$addr_val"

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

# ============================================================================
# Send metrics directly to Victoria Metrics (bypass telemetry service)
# This tests the legacy/direct ingestion path and URL backwards compatibility
# Args: $1 = endpoint_url (optional, defaults to localhost:8428)
#       $2 = metric_name (optional, defaults to telemetry_e2e_test_direct)
# ============================================================================
send_direct_victoria_metrics() {
    local endpoint_url="${1:-http://localhost:8428}"
    local metric_name="${2:-telemetry_e2e_test_direct}"
    local timestamp=$(date +%s)000
    
    # Generate sample Prometheus-format metrics
    local metrics="# HELP ${metric_name} Direct test metric bypassing telemetry service
# TYPE ${metric_name} gauge
${metric_name}{source=\"direct_test\",environment=\"e2e_test\"} 42 ${timestamp}
${metric_name}_counter{source=\"direct_test\",environment=\"e2e_test\"} 100 ${timestamp}"

    echo "Sending metrics directly to Victoria Metrics..."
    echo "  Endpoint: $endpoint_url"
    echo "  Metric: $metric_name"
    
    # Try the endpoint - the client should handle path fallback automatically
    # First try with explicit path (new config style)
    local response=$(curl -s -w "\n%{http_code}" -X POST \
        "${endpoint_url}/api/v1/import/prometheus" \
        -H "Content-Type: text/plain" \
        --data-binary "$metrics" 2>&1)
    
    local http_code=$(echo "$response" | tail -n1)
    local body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" = "204" ] || [ "$http_code" = "200" ]; then
        echo -e "${GREEN}✓ Direct Victoria Metrics ingestion successful (HTTP $http_code)${NC}"
        return 0
    else
        echo -e "${RED}✗ Direct Victoria Metrics ingestion failed (HTTP $http_code)${NC}"
        echo "  Response: $body"
        return 1
    fi
}

# ============================================================================
# Verify Prometheus Remote Write is enabled
# Note: Prometheus Remote Write requires protobuf+snappy format, NOT plain text.
# Direct text ingestion to Prometheus is not supported - use Victoria Metrics for that.
# This test just verifies the endpoint is reachable and returns expected error for text.
# Args: $1 = endpoint_url (optional, defaults to localhost:9090)
# ============================================================================
verify_prometheus_remote_write_enabled() {
    local endpoint_url="${1:-http://localhost:9090}"
    
    echo "Verifying Prometheus Remote Write endpoint..."
    echo "  Endpoint: $endpoint_url/api/v1/write"
    echo -e "  ${YELLOW}Note: Prometheus requires protobuf+snappy format (not plain text)${NC}"
    echo "  The telemetry service handles this conversion automatically."
    
    # Check if the endpoint responds (even 400 means it's enabled)
    local http_code=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
        "${endpoint_url}/api/v1/write" \
        -H "Content-Type: application/x-protobuf" \
        --data-binary "" 2>&1)
    
    # 400 = endpoint exists but bad request (expected for empty body)
    # 415 = unsupported media type (endpoint exists)
    # 200/204 = success (unlikely with empty body)
    if [ "$http_code" = "400" ] || [ "$http_code" = "415" ] || [ "$http_code" = "200" ] || [ "$http_code" = "204" ]; then
        echo -e "${GREEN}✓ Prometheus Remote Write endpoint is enabled (HTTP $http_code)${NC}"
        return 0
    elif [ "$http_code" = "404" ]; then
        echo -e "${RED}✗ Prometheus Remote Write not enabled (HTTP 404)${NC}"
        echo "  Add '--web.enable-remote-write-receiver' to Prometheus startup args"
        return 1
    else
        echo -e "${YELLOW}⚠ Prometheus returned unexpected HTTP $http_code${NC}"
        return 1
    fi
}

# ============================================================================
# Create an unknown/untrusted account (NOT added to registry)
# This account can authenticate via custom contract if allow_unknown_nodes is true
# Args: $1 = account_name (used for key file naming)
# Returns: Sets UNKNOWN_ACCOUNT_ADDRESS and UNKNOWN_ACCOUNT_KEY_HEX
# ============================================================================
create_unknown_account() {
    local account_name="${1:-unknown-node}"
    
    setup_aptos_cmd
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    local key_file="$TEST_DIR/${account_name}.key"
    local profile_name="telemetry-e2e-${account_name}"

    # Check if key already exists - reuse it for consecutive runs
    if [ -f "$key_file" ]; then
        echo "  Reusing existing key file: $key_file"
        UNKNOWN_ACCOUNT_KEY_HEX="0x$(cat "$key_file")"
        
        # Check if profile already exists
        local addr_val=$(yq e ".profiles.\"$profile_name\".account" "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"')
        if [ -n "$addr_val" ] && [ "$addr_val" != "null" ]; then
            UNKNOWN_ACCOUNT_ADDRESS="$addr_val"
            echo -e "${GREEN}✓ Reusing unknown account:${NC} $account_name (NOT in registry)"
            echo "  Address: $UNKNOWN_ACCOUNT_ADDRESS"
            export UNKNOWN_ACCOUNT_ADDRESS
            export UNKNOWN_ACCOUNT_KEY_HEX
            return 0
        fi
    else
        # Generate new key (no prompt needed since file doesn't exist)
        $APTOS_CMD key generate --output-file "$key_file" --key-type ed25519 2>/dev/null
        UNKNOWN_ACCOUNT_KEY_HEX="0x$(cat "$key_file")"
    fi

    # Initialize profile (this also funds the account via faucet in local testnet)
    $APTOS_CMD init \
        --profile "$profile_name" \
        --network local \
        --assume-yes \
        --private-key "$UNKNOWN_ACCOUNT_KEY_HEX" 2>/dev/null

    # Extract the address from config
    local addr_val=$(yq e ".profiles.\"$profile_name\".account" "$TEST_DIR/.aptos/config.yaml" 2>/dev/null | tr -d '"')
    
    if [ -z "$addr_val" ] || [ "$addr_val" = "null" ]; then
        echo -e "${RED}Error: Failed to create unknown account $account_name${NC}"
        return 1
    fi
    
    UNKNOWN_ACCOUNT_ADDRESS="$addr_val"

    echo -e "${GREEN}✓ Created unknown account:${NC} $account_name (NOT in registry)"
    echo "  Address: $UNKNOWN_ACCOUNT_ADDRESS"
    
    export UNKNOWN_ACCOUNT_ADDRESS
    export UNKNOWN_ACCOUNT_KEY_HEX
}

# ============================================================================
# Send telemetry as an unknown/untrusted node via custom contract
# This tests the allow_unknown_nodes feature
# Args: $1 = private_key_hex (of account NOT in registry)
#       $2 = iterations (optional, default 1)
# ============================================================================
send_unknown_telemetry() {
    local private_key="$1"
    local iterations="${2:-1}"

    if [ -z "$private_key" ]; then
        echo -e "${RED}Error: private_key required${NC}"
        return 1
    fi

    cd "$SCRIPT_DIR/test-client"
    
    echo "Sending telemetry as unknown/untrusted node..."
    echo "  (Account is NOT in on-chain registry, but allow_unknown_nodes=true)"
    
    # Run test client - same as regular, but account is not in allowlist
    # The telemetry service should accept it and route to untrusted sinks
    cargo run -q -p telemetry-test-client -- \
        -p "$private_key" \
        all --iterations "$iterations" 2>&1

    local exit_code=$?
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✓ Unknown node telemetry sent successfully${NC}"
        echo "  (Routed to untrusted sinks with trust_status=untrusted label)"
    else
        echo -e "${RED}✗ Failed to send unknown node telemetry (exit code: $exit_code)${NC}"
        return 1
    fi
}

# ============================================================================
# Verify trust_status labels in VictoriaMetrics
# Args: $1 = metric_name, $2 = expected_trust_status (trusted/untrusted)
# ============================================================================
verify_trust_status_victoria() {
    local metric_name="$1"
    local trust_status="$2"
    
    # URL-encode the query (braces and quotes need encoding)
    local query="${metric_name}%7Btrust_status%3D%22${trust_status}%22%7D"
    local result=$(curl -s "http://localhost:8428/api/v1/series?match[]=$query" | jq -r '.data | length')
    
    if [ "$result" -gt 0 ]; then
        echo -e "${GREEN}✓ Metric '$metric_name' with trust_status='$trust_status' found ($result series)${NC}"
        return 0
    else
        echo -e "${RED}✗ Metric '$metric_name' with trust_status='$trust_status' not found${NC}"
        return 1
    fi
}

# ============================================================================
# Verify trust_status labels in Prometheus
# Args: $1 = metric_name, $2 = expected_trust_status (trusted/untrusted)
# ============================================================================
verify_trust_status_prometheus() {
    local metric_name="$1"
    local trust_status="$2"
    
    # URL-encode the query (braces and quotes need encoding)
    # { = %7B, } = %7D, = = %3D, " = %22
    local query="${metric_name}%7Btrust_status%3D%22${trust_status}%22%7D"
    local result=$(curl -s "http://localhost:9090/api/v1/series?match[]=$query" | jq -r '.data | length')
    
    if [ "$result" -gt 0 ]; then
        echo -e "${GREEN}✓ Metric '$metric_name' with trust_status='$trust_status' found in Prometheus ($result series)${NC}"
        return 0
    else
        echo -e "${RED}✗ Metric '$metric_name' with trust_status='$trust_status' not found in Prometheus${NC}"
        return 1
    fi
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
export -f send_direct_victoria_metrics
export -f verify_prometheus_remote_write_enabled
export -f create_unknown_account
export -f send_unknown_telemetry
export -f verify_trust_status_victoria
export -f verify_trust_status_prometheus

