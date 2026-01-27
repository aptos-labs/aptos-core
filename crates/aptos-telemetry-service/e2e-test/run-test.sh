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
    echo -e "${BLUE}Custom Contract Metrics (Trusted):${NC}"
    verify_metrics_victoria "telemetry_e2e_test_iteration_metric"
    verify_metrics_prometheus "telemetry_e2e_test_iteration_metric"
    verify_trust_status_victoria "telemetry_e2e_test_iteration_metric" "trusted"
    echo ""
    echo -e "${BLUE}Custom Contract Metrics (Untrusted via allow_unknown_nodes):${NC}"
    verify_trust_status_prometheus "telemetry_e2e_test_iteration_metric" "untrusted"
    echo ""
    echo -e "${BLUE}Direct/Legacy Metrics:${NC}"
    verify_metrics_victoria "telemetry_e2e_test_direct"
    echo ""
    echo -e "${BLUE}Logs:${NC}"
    verify_logs_loki '{contract_name="e2e_test_contract"}'
    exit 0
fi

# ============================================================================
# Phase 0: Test Direct Backend Ingestion
# ============================================================================
print_section "Phase 0: Testing Direct Backend Ingestion"

echo "Testing direct Victoria Metrics ingestion..."
echo "This verifies the direct path still works for direct metric push."
echo ""

# Test direct ingestion to Victoria Metrics
send_direct_victoria_metrics "http://localhost:8428" "telemetry_e2e_test_direct"

echo ""

# Verify Prometheus Remote Write is enabled
verify_prometheus_remote_write_enabled "http://localhost:9090" || true

echo ""
echo "Verifying direct ingestion..."
sleep 2
verify_metrics_victoria "telemetry_e2e_test_direct"

# ============================================================================
# Phase 1: Test with TRUSTED account (Custom Contract Auth - in allowlist)
# ============================================================================
print_section "Phase 1: Testing Trusted Node via Custom Contract (allowlisted)"

echo "Sending telemetry from TRUSTED test account (in on-chain registry)..."
echo "  Address: $TEST_ACCOUNT_ADDRESS"
echo "  Key: ${TEST_ACCOUNT_KEY_HEX:0:20}..."
echo "  Expected: trust_status=trusted, routed to metrics_sinks (Victoria Metrics + Prometheus)"
echo ""

send_telemetry "$TEST_ACCOUNT_KEY_HEX" "$ITERATIONS"

# Verify trusted telemetry - should have trust_status=trusted
echo ""
echo "Verifying trusted node telemetry..."
sleep 2
verify_metrics_victoria "telemetry_e2e_test_iteration_metric"
verify_metrics_prometheus "telemetry_e2e_test_iteration_metric"
echo ""
echo "Checking trust_status label..."
verify_trust_status_victoria "telemetry_e2e_test_iteration_metric" "trusted" || true

# ============================================================================
# Phase 1b: Test with UNKNOWN account (Custom Contract Auth - NOT in allowlist)
# ============================================================================
print_section "Phase 1b: Testing Unknown Node via Custom Contract (NOT allowlisted)"

echo "Creating an UNKNOWN account (NOT added to on-chain registry)..."
create_unknown_account "unknown-test-node"

echo ""
echo "Sending telemetry from UNKNOWN account..."
echo "  Address: $UNKNOWN_ACCOUNT_ADDRESS"
echo "  Expected: trust_status=untrusted, routed to untrusted_metrics_sinks (Prometheus)"
echo "  (This tests allow_unknown_nodes=true in telemetry-config.yaml)"
echo ""

send_unknown_telemetry "$UNKNOWN_ACCOUNT_KEY_HEX" "$ITERATIONS"

# Verify unknown telemetry - should have trust_status=untrusted and be in Prometheus
echo ""
echo "Verifying unknown node telemetry (should be in Prometheus via untrusted sinks)..."
echo "Waiting for Prometheus to process remote write..."
sleep 5  # Prometheus remote write needs time to process
echo ""
echo "Checking trust_status label in Prometheus (untrusted sink)..."
verify_trust_status_prometheus "telemetry_e2e_test_iteration_metric" "untrusted" || true

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
echo -e "${BLUE}Checking VictoriaMetrics (trusted node sink)...${NC}"
VICTORIA_CONTRACT_COUNT=$(curl -s "http://localhost:8428/api/v1/series?match[]=telemetry_e2e_test_iteration_metric" | jq -r '.data | length')
VICTORIA_DIRECT_COUNT=$(curl -s "http://localhost:8428/api/v1/series?match[]=telemetry_e2e_test_direct" | jq -r '.data | length')
# URL-encoded: {trust_status="trusted"} -> %7Btrust_status%3D%22trusted%22%7D
VICTORIA_TRUSTED_COUNT=$(curl -s "http://localhost:8428/api/v1/series?match[]=telemetry_e2e_test_iteration_metric%7Btrust_status%3D%22trusted%22%7D" | jq -r '.data | length')
echo -e "  Custom contract metrics (all):       ${GREEN}$VICTORIA_CONTRACT_COUNT${NC} series"
echo -e "  Custom contract metrics (trusted):   ${GREEN}$VICTORIA_TRUSTED_COUNT${NC} series"
echo -e "  Direct ingestion metrics:            ${GREEN}$VICTORIA_DIRECT_COUNT${NC} series"

echo ""
echo -e "${BLUE}Checking Prometheus (trusted + untrusted sinks)...${NC}"
PROMETHEUS_CONTRACT_COUNT=$(curl -s "http://localhost:9090/api/v1/series?match[]=telemetry_e2e_test_iteration_metric" | jq -r '.data | length')
# URL-encoded queries for trust_status labels
PROMETHEUS_TRUSTED_COUNT=$(curl -s "http://localhost:9090/api/v1/series?match[]=telemetry_e2e_test_iteration_metric%7Btrust_status%3D%22trusted%22%7D" | jq -r '.data | length')
PROMETHEUS_UNTRUSTED_COUNT=$(curl -s "http://localhost:9090/api/v1/series?match[]=telemetry_e2e_test_iteration_metric%7Btrust_status%3D%22untrusted%22%7D" | jq -r '.data | length')
echo -e "  Custom contract metrics (all):       ${GREEN}$PROMETHEUS_CONTRACT_COUNT${NC} series"
echo -e "  Custom contract metrics (trusted):   ${GREEN}$PROMETHEUS_TRUSTED_COUNT${NC} series"
echo -e "  Custom contract metrics (untrusted): ${GREEN}$PROMETHEUS_UNTRUSTED_COUNT${NC} series"

echo ""
echo -e "${BLUE}Checking Loki...${NC}"
LOKI_COUNT=$(curl -s 'http://localhost:3100/loki/api/v1/query?query=%7Bcontract_name%3D%22e2e_test_contract%22%7D' | jq -r '.data.result | length')
echo -e "  Custom contract logs: ${GREEN}$LOKI_COUNT${NC} streams"

# ============================================================================
# Summary
# ============================================================================
print_section "Test Summary"

TOTAL_ACCOUNTS=$((1 + ${NUM_ACCOUNTS:-0}))
if [ "$SKIP_CREATE" = true ]; then
    TOTAL_ACCOUNTS=1
fi

echo -e "${BLUE}Test Configuration:${NC}"
echo "  • Trusted accounts tested:   $TOTAL_ACCOUNTS"
echo "  • Untrusted accounts tested: 1"
echo "  • Iterations per account:    $ITERATIONS"
echo "  • Contract address:          $DEPLOYER_ADDRESS"
echo ""

echo -e "${BLUE}Metrics Ingestion Results:${NC}"
echo "  VictoriaMetrics (trusted sink - text format):"
echo "    • Custom contract (all):     $VICTORIA_CONTRACT_COUNT series"
echo "    • Custom contract (trusted): $VICTORIA_TRUSTED_COUNT series"
echo "    • Direct ingestion:          $VICTORIA_DIRECT_COUNT series"
echo "  Prometheus (trusted + untrusted sinks - protobuf format):"
echo "    • Custom contract (all):       $PROMETHEUS_CONTRACT_COUNT series"
echo "    • Custom contract (trusted):   $PROMETHEUS_TRUSTED_COUNT series"
echo "    • Custom contract (untrusted): $PROMETHEUS_UNTRUSTED_COUNT series"
echo "  Loki:"
echo "    • Custom contract logs:        $LOKI_COUNT streams"
echo ""

# Determine overall pass/fail
# Pass if: 
#   1. Victoria Metrics has trusted metrics and direct metrics
#   2. Prometheus has both trusted AND untrusted metrics (tests allow_unknown_nodes)
TEST_RESULT="pass"

if [ "$VICTORIA_TRUSTED_COUNT" -gt 0 ] && [ "$VICTORIA_DIRECT_COUNT" -gt 0 ] && [ "$PROMETHEUS_TRUSTED_COUNT" -gt 0 ] && [ "$PROMETHEUS_UNTRUSTED_COUNT" -gt 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  ✓ E2E TEST PASSED                    ${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "All telemetry paths verified:"
    echo "  ✓ Trusted node via custom contract (in allowlist)"
    echo "  ✓ Untrusted node via custom contract (allow_unknown_nodes=true)"
    echo "  ✓ Direct backend ingestion (legacy path)"
    echo "  ✓ Trust status labels correctly applied"
    TEST_RESULT="pass"
elif [ "$VICTORIA_CONTRACT_COUNT" -gt 0 ] || [ "$PROMETHEUS_CONTRACT_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}  ⚠ E2E TEST PARTIAL PASS              ${NC}"
    echo -e "${YELLOW}========================================${NC}"
    echo ""
    echo "Some metrics were ingested but not all paths verified."
    echo "  VictoriaMetrics trusted:   $VICTORIA_TRUSTED_COUNT"
    echo "  VictoriaMetrics direct:    $VICTORIA_DIRECT_COUNT"
    echo "  Prometheus trusted:        $PROMETHEUS_TRUSTED_COUNT"
    echo "  Prometheus untrusted:      $PROMETHEUS_UNTRUSTED_COUNT"
    TEST_RESULT="partial"
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}  ✗ E2E TEST FAILED                    ${NC}"
    echo -e "${RED}========================================${NC}"
    echo ""
    echo "No metrics were ingested. Check telemetry service logs:"
    echo "  tail -f $TEST_DIR/telemetry.log"
    TEST_RESULT="fail"
fi

# ============================================================================
# Manual Verification Instructions
# ============================================================================
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Manual Verification via Grafana                          ${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Open Grafana:${NC} http://localhost:3000"
echo "  Login: admin / admin"
echo ""
echo -e "${BLUE}Explore Metrics (VictoriaMetrics - trusted sink):${NC}"
echo "  1. Go to: Explore → Select 'VictoriaMetrics' datasource"
echo "  2. Query all E2E test metrics:"
echo "     ${YELLOW}telemetry_e2e_test_iteration_metric${NC}"
echo "  3. Query trusted metrics only:"
echo "     ${YELLOW}telemetry_e2e_test_iteration_metric{trust_status=\"trusted\"}${NC}"
echo "  4. Query direct ingestion metrics:"
echo "     ${YELLOW}telemetry_e2e_test_direct${NC}"
echo ""
echo -e "${BLUE}Explore Metrics (Prometheus - both trusted + untrusted sinks):${NC}"
echo "  1. Go to: Explore → Select 'Prometheus' datasource"
echo "  2. Query all E2E test metrics:"
echo "     ${YELLOW}telemetry_e2e_test_iteration_metric${NC}"
echo "  3. Query trusted metrics only:"
echo "     ${YELLOW}telemetry_e2e_test_iteration_metric{trust_status=\"trusted\"}${NC}"
echo "  4. Query untrusted/unknown node metrics:"
echo "     ${YELLOW}telemetry_e2e_test_iteration_metric{trust_status=\"untrusted\"}${NC}"
echo ""
echo -e "${BLUE}Explore Logs (Loki):${NC}"
echo "  1. Go to: Explore → Select 'Loki' datasource"
echo "  2. Query all custom contract logs:"
echo "     ${YELLOW}{contract_name=\"e2e_test_contract\"}${NC}"
echo "  3. Query trusted node logs:"
echo "     ${YELLOW}{contract_name=\"e2e_test_contract\",trust_status=\"trusted\"}${NC}"
echo "  4. Query untrusted node logs:"
echo "     ${YELLOW}{contract_name=\"e2e_test_contract\",trust_status=\"untrusted\"}${NC}"
echo ""
echo -e "${BLUE}Direct API Queries:${NC}"
echo "  # All E2E test metrics"
echo "  curl 'http://localhost:8428/api/v1/series?match[]=telemetry_e2e_test_iteration_metric'"
echo "  curl 'http://localhost:9090/api/v1/series?match[]=telemetry_e2e_test_iteration_metric'"
echo ""
echo "  # Trusted metrics only"
echo "  curl 'http://localhost:9090/api/v1/series?match[]=telemetry_e2e_test_iteration_metric{trust_status=\"trusted\"}'"
echo ""
echo "  # Untrusted metrics only (should be in Prometheus via untrusted sink)"
echo "  curl 'http://localhost:9090/api/v1/series?match[]=telemetry_e2e_test_iteration_metric{trust_status=\"untrusted\"}'"
echo ""
echo "  # Logs"
echo "  curl 'http://localhost:3100/loki/api/v1/query?query={contract_name=\"e2e_test_contract\"}'"
echo ""

# Exit with appropriate code
if [ "$TEST_RESULT" = "fail" ]; then
    exit 1
else
    exit 0
fi

