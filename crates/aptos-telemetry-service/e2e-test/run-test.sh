#!/bin/bash
# E2E Test Runner Script
# Sends telemetry data from multiple test accounts
#
# Usage:
#   ./run-test.sh                    # Run full test with 10 additional accounts
#   ./run-test.sh --accounts 5       # Create and test with 5 additional accounts
#   ./run-test.sh --skip-create      # Skip creating new accounts, use existing
#   ./run-test.sh --iterations 3     # Send telemetry 3 times per account

set -e

# Source helpers
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

# Default values
NUM_ACCOUNTS=10
SKIP_CREATE=false
ITERATIONS=1
VERIFY_ONLY=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -a|--accounts)
            NUM_ACCOUNTS="$2"
            shift 2
            ;;
        -n|--iterations)
            ITERATIONS="$2"
            shift 2
            ;;
        --skip-create)
            SKIP_CREATE=true
            shift
            ;;
        --verify)
            VERIFY_ONLY=true
            shift
            ;;
        -h|--help)
            echo "E2E Test Runner"
            echo ""
            echo "Usage:"
            echo "  ./run-test.sh                    Run full test"
            echo "  ./run-test.sh --accounts N       Create N additional accounts (default: 10)"
            echo "  ./run-test.sh --iterations N    Send telemetry N times per account (default: 1)"
            echo "  ./run-test.sh --skip-create      Skip creating accounts, use existing"
            echo "  ./run-test.sh --verify           Only verify metrics (no telemetry send)"
            echo "  ./run-test.sh --help             Show this help"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# ============================================================================
# Main Test Execution
# ============================================================================

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Aptos Telemetry Service E2E Test     ${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Load environment
print_section "Loading Environment"
load_env || exit 1

echo "Contract Address: $DEPLOYER_ADDRESS"
echo "Test Account:     $TEST_ACCOUNT_ADDRESS"
echo ""

# Initialize aptos command
setup_aptos_cmd

# Skip to verification if requested
if [ "$VERIFY_ONLY" = true ]; then
    print_section "Verifying Metrics"
    verify_metrics_victoria "test_iteration_metric"
    verify_metrics_prometheus "test_iteration_metric"
    verify_logs_loki '{contract_name="e2e_test_contract"}'
    exit 0
fi

# ============================================================================
# Phase 1: Test with existing account
# ============================================================================
print_section "Phase 1: Testing with Existing Account"

echo "Sending telemetry from existing test account..."
echo "  Address: $TEST_ACCOUNT_ADDRESS"
echo "  Key: ${TEST_ACCOUNT_KEY_HEX:0:20}..."
echo ""

send_telemetry "$TEST_ACCOUNT_KEY_HEX" "$ITERATIONS"

# Verify initial telemetry
echo ""
echo "Verifying telemetry data..."
sleep 2  # Give backends time to ingest
verify_metrics_victoria "test_iteration_metric"
verify_metrics_prometheus "test_iteration_metric"

# ============================================================================
# Phase 2: Create additional accounts
# ============================================================================
if [ "$SKIP_CREATE" = false ] && [ "$NUM_ACCOUNTS" -gt 0 ]; then
    print_section "Phase 2: Creating $NUM_ACCOUNTS Additional Accounts"

    # Store account info for later
    ADDITIONAL_ACCOUNTS=()
    ADDITIONAL_KEYS=()

    for i in $(seq 1 $NUM_ACCOUNTS); do
        echo ""
        echo -e "${YELLOW}Creating account $i/$NUM_ACCOUNTS...${NC}"
        
        # Create account with unique name
        create_account "test-member-$i"
        
        # Store for later use
        ADDITIONAL_ACCOUNTS+=("$CREATED_ACCOUNT_ADDRESS")
        ADDITIONAL_KEYS+=("$CREATED_ACCOUNT_KEY_HEX")
    done

    echo ""
    echo -e "${GREEN}✓ Created $NUM_ACCOUNTS accounts${NC}"

    # ============================================================================
    # Phase 3: Add all accounts to registry
    # ============================================================================
    print_section "Phase 3: Adding Accounts to Registry"

    for i in $(seq 0 $((NUM_ACCOUNTS - 1))); do
        local_addr="${ADDITIONAL_ACCOUNTS[$i]}"
        local_port=$((9001 + i))
        local_dc="dc_test_$((i % 3))"  # Rotate through 3 datacenters
        
        echo ""
        echo -e "${YELLOW}Adding account $((i + 1))/$NUM_ACCOUNTS to registry...${NC}"
        add_member_to_registry "$local_addr" "127.0.0.1" "$local_port" "0xnode$i" "$local_dc"
    done

    echo ""
    echo -e "${GREEN}✓ All accounts added to registry${NC}"

    # ============================================================================
    # Phase 4: Send telemetry from all new accounts
    # ============================================================================
    print_section "Phase 4: Sending Telemetry from All New Accounts"

    # Wait for allowlist cache to refresh (configured at 10s in telemetry-config.yaml)
    echo "Waiting for allowlist cache to refresh..."
    sleep 12

    SUCCESS_COUNT=0
    FAIL_COUNT=0

    for i in $(seq 0 $((NUM_ACCOUNTS - 1))); do
        local_key="${ADDITIONAL_KEYS[$i]}"
        local_addr="${ADDITIONAL_ACCOUNTS[$i]}"
        
        echo ""
        echo -e "${YELLOW}[$((i + 1))/$NUM_ACCOUNTS]${NC} Sending telemetry from ${local_addr:0:16}..."
        
        if send_telemetry "$local_key" "$ITERATIONS"; then
            ((SUCCESS_COUNT++))
        else
            ((FAIL_COUNT++))
        fi
    done

    echo ""
    echo -e "${BLUE}Results:${NC} $SUCCESS_COUNT succeeded, $FAIL_COUNT failed"
fi

# ============================================================================
# Phase 5: Final Verification
# ============================================================================
print_section "Phase 5: Final Verification"

echo "Waiting for metrics to be ingested..."
sleep 3

echo ""
echo "Checking VictoriaMetrics..."
VICTORIA_COUNT=$(curl -s "http://localhost:8428/api/v1/series?match[]=test_iteration_metric" | jq -r '.data | length')
echo -e "  Found ${GREEN}$VICTORIA_COUNT${NC} time series in VictoriaMetrics"

echo ""
echo "Checking Prometheus..."
PROMETHEUS_COUNT=$(curl -s "http://localhost:9090/api/v1/series?match[]=test_iteration_metric" | jq -r '.data | length')
echo -e "  Found ${GREEN}$PROMETHEUS_COUNT${NC} time series in Prometheus"

echo ""
echo "Checking Loki..."
LOKI_COUNT=$(curl -s 'http://localhost:3100/loki/api/v1/query?query=%7Bcontract_name%3D%22e2e_test_contract%22%7D' | jq -r '.data.result | length')
echo -e "  Found ${GREEN}$LOKI_COUNT${NC} log streams in Loki"

# ============================================================================
# Summary
# ============================================================================
print_section "Test Summary"

TOTAL_ACCOUNTS=$((1 + ${NUM_ACCOUNTS:-0}))
if [ "$SKIP_CREATE" = true ]; then
    TOTAL_ACCOUNTS=1
fi

echo -e "${BLUE}Test Configuration:${NC}"
echo "  • Total accounts tested: $TOTAL_ACCOUNTS"
echo "  • Iterations per account: $ITERATIONS"
echo "  • Contract address: $DEPLOYER_ADDRESS"
echo ""

echo -e "${BLUE}Metrics Backends:${NC}"
echo "  • VictoriaMetrics: $VICTORIA_COUNT series"
echo "  • Prometheus:      $PROMETHEUS_COUNT series"
echo "  • Loki:            $LOKI_COUNT streams"
echo ""

# Determine overall pass/fail
if [ "$VICTORIA_COUNT" -gt 0 ] && [ "$PROMETHEUS_COUNT" -gt 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  ✓ E2E TEST PASSED                    ${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}  ✗ E2E TEST FAILED                    ${NC}"
    echo -e "${RED}========================================${NC}"
    echo ""
    echo "Check telemetry service logs:"
    echo "  tail -f $TEST_DIR/telemetry.log"
    exit 1
fi

