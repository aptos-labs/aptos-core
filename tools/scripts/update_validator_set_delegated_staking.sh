#!/bin/bash

# Delegated validator setup using Movement CLI
# This script assumes a *delegated* setup only.
# It does NOT create a self-hosted stake::StakePool (no create-staking-contract).
#
# Usage:
#   ./update_validator_set_delegated.sh \
#       <operator-identity-file> \
#.      <validator-owner-identity-file> \
#       <network> \
#       <stake-amount> \
#       <validator-host> \
#       [dry-run]
#
# Network options: testnet | mainnet | previewnet | custom:https://your-api-url.com

set -e
set -o pipefail

# Configuration
MOVEMENT_CLI="movement"
PROFILE="PROFILE_SHOULD_NOT_BE_USED"

# Global stake validation constants
MIN_STAKE=0        # Minimum allowed stake (0 for now, will be non-zero later)
MAX_STAKE_RATIO=30 # Maximum stake as percentage of total voting power

# Delegation / delegated pool configuration (for "Validator 4"-style setup)
SEED=2563
DELEGATION_SEED="aptos_framework::delegation_pool${SEED}"
DELEGATION_COMMISSION_BPS=1000 # 1000 = 10%

# Network presets
TESTNET_API="https://testnet.movementnetwork.xyz"
MAINNET_API="https://mainnet.movementnetwork.xyz"
PREVIEWNET_API="https://previewnet.devnet.movementnetwork.xyz"

# Operator identity
OPERATOR_IDENTITY_FILE="$1"
VALIDATOR_OWNER_IDENTITY_FILE="$2"
NETWORK_INPUT="$3"
STAKE_AMOUNT="$4"
VALIDATOR_HOST="$5"
DRY_RUN="${6:-true}"

help_message_and_exit() {
	echo "Usage: $0 <operator-identity-file> <validator-owner-identity-file> <network> <stake-amount> <validator-host> [dry-run]"
	echo "Network options:"
	echo "  testnet               - Movement testnet ($TESTNET_API)"
	echo "  mainnet               - Movement mainnet ($MAINNET_API)"
	echo "  previewnet            - Movement previewnet ($PREVIEWNET_API)"
	echo "  custom:https://url    - Custom API endpoint"
	echo "Example:"
	echo "  $0 operator-identity.yaml owner-identity.yaml testnet 1000000 validator.example.com:6180 true"
	echo "  $0 operator-identity.yaml owner-identity.yaml custom:https://my-node.com 5000000 validator.example.com:6180 false"
	exit 1
}

# Functions
get_network_pub_key() {
	local private_key="$1"
	local temp_file
	temp_file=$(mktemp)

	$MOVEMENT_CLI key extract-public-key \
		--private-key "$private_key" \
		--key-type "x25519" \
		--encoding hex \
		--output-file "$temp_file" \
		--assume-yes >/dev/null 2>&1
	local exit_code=$?

	if [ $exit_code -ne 0 ]; then
		rm -f "$temp_file" "${temp_file}.pub"
		echo "Error: Failed to extract public key (exit code: $exit_code)" >&2
		exit 1
	fi

	local public_key
	public_key=$(cat "${temp_file}.pub")
	rm -f "$temp_file" "${temp_file}.pub"

	echo "$public_key"
}

get_consensus_keys() {
	local private_key="$1"
	local temp_file
	temp_file=$(mktemp)

	local resp
	resp=$($MOVEMENT_CLI key extract-public-key \
		--private-key "$private_key" \
		--key-type "bls12381" \
		--encoding hex \
		--output-file "$temp_file" \
		--assume-yes 2>&1)
	local exit_code=$?

	if [ $exit_code -ne 0 ]; then
		rm -f "$temp_file" "${temp_file}.pub" "${temp_file}.pop"
		echo "Error: Failed to extract consensus keys (exit code: $exit_code)" >&2
		echo "Details: $resp" >&2
		exit 1
	fi

	if [ ! -f "${temp_file}.pub" ]; then
		rm -f "$temp_file" "${temp_file}.pub" "${temp_file}.pop"
		echo "Error: Consensus public key file was not created" >&2
		exit 1
	fi

	if [ ! -f "${temp_file}.pop" ]; then
		rm -f "$temp_file" "${temp_file}.pub" "${temp_file}.pop"
		echo "Error: Consensus proof of possession file was not created" >&2
		exit 1
	fi

	local public_key
	local pop_key
	public_key=$(cat "${temp_file}.pub")
	pop_key=$(cat "${temp_file}.pop")

	rm -f "$temp_file" "${temp_file}.pub" "${temp_file}.pop"

	echo "$public_key $pop_key"
}

dependency_check() {
	if ! command -v "$MOVEMENT_CLI" &>/dev/null; then
		echo "Error: movement CLI is not installed or not in PATH"
		exit 1
	fi

	if ! command -v curl &>/dev/null; then
		echo "Error: curl is not installed or not in PATH"
		exit 1
	fi

	if ! command -v jq &>/dev/null; then
		echo "Error: jq is not installed or not in PATH"
		exit 1
	fi
}

validate_input() {
	if [ -z "$OPERATOR_IDENTITY_FILE" ]; then
		echo "Error: Validator identity file not provided"
		help_message_and_exit
	fi

	if [ ! -f "$OPERATOR_IDENTITY_FILE" ]; then
		echo "Error: Validator identity file not found: $OPERATOR_IDENTITY_FILE"
		help_message_and_exit
	fi

	if [ -z "$VALIDATOR_OWNER_IDENTITY_FILE" ]; then
		echo "Error: Validator owner identity file not provided"
		help_message_and_exit
	fi

	if [ ! -f "$VALIDATOR_OWNER_IDENTITY_FILE" ]; then
		echo "Error: Validator owner identity file not found: $VALIDATOR_OWNER_IDENTITY_FILE"
		help_message_and_exit
	fi

	if [ -z "$NETWORK_INPUT" ]; then
		echo "Error: Network not provided"
		help_message_and_exit
	fi

	case "$NETWORK_INPUT" in
	"testnet")
		NETWORK_API_ADDRESS="$TESTNET_API"
		;;
	"mainnet")
		NETWORK_API_ADDRESS="$MAINNET_API"
		;;
	"previewnet")
		NETWORK_API_ADDRESS="https://previewnet.devnet.movementnetwork.xyz"
		;;
	"custom:"*)
		NETWORK_API_ADDRESS="${NETWORK_INPUT#custom:}"
		NETWORK_API_ADDRESS="${NETWORK_API_ADDRESS%/}"
		;;
	*)
		echo "Error: Invalid network option: $NETWORK_INPUT"
		help_message_and_exit
		;;
	esac

	if [ -z "$STAKE_AMOUNT" ]; then
		echo "Error: Stake amount not provided"
		help_message_and_exit
	fi

	if [ -z "$VALIDATOR_HOST" ]; then
		echo "Error: Validator host not provided"
		help_message_and_exit
	fi
}

get_operator_identity() {
	OPERATOR_ACCOUNT=$(grep "account_address:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	OPERATOR_ACCOUNT_PRIVATE_KEY=$(grep "account_private_key:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	CONSENSUS_PRIVATE_KEY=$(grep "consensus_private_key:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	NETWORK_PRIVATE_KEY=$(grep "network_private_key:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')

	if [ -z "$OPERATOR_ACCOUNT" ] || [ -z "$OPERATOR_ACCOUNT_PRIVATE_KEY" ] || [ -z "$CONSENSUS_PRIVATE_KEY" ] || [ -z "$NETWORK_PRIVATE_KEY" ]; then
		echo "Error: Missing required fields in validator identity file"
		echo "Required fields: account_address, account_private_key, consensus_private_key, network_private_key"
		exit 1
	fi

	read CONSENSUS_PUBLIC_KEY CONSENSUS_POP <<<"$(get_consensus_keys "$CONSENSUS_PRIVATE_KEY")"
	NETWORK_PUBLIC_KEY=$(get_network_pub_key "$NETWORK_PRIVATE_KEY")
}

get_validator_owner_identity() {
	VALIDATOR_OWNER_ACCOUNT=$(grep "account_address:" "$VALIDATOR_OWNER_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	VALIDATOR_OWNER_PRIVATE_KEY=$(grep "account_private_key:" "$VALIDATOR_OWNER_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')

	if [ -z "$VALIDATOR_OWNER_ACCOUNT" ] || [ -z "$VALIDATOR_OWNER_PRIVATE_KEY" ]; then
		echo "Error: Missing required fields in validator owner identity file"
		echo "Required fields: account_address, account_private_key"
		exit 1
	fi
}

get_current_validator_set() {
	local api_url="${NETWORK_API_ADDRESS}/v1/accounts/0x1/resource/0x1::stake::ValidatorSet"
	local response
	response=$(curl -s "$api_url")

	if [ $? -ne 0 ]; then
		echo "Error: Failed to fetch validator set"
		exit 1
	fi

	TOTAL_VOTING_POWER=$(echo "$response" | jq -r '.data.total_voting_power')
	ACTIVE_VALIDATOR_COUNT=$(echo "$response" | jq -r '.data.active_validators | length')
	TOTAL_JOINING_POWER=$(echo "$response" | jq -r '.data.total_joining_power')

	if [ -z "$TOTAL_VOTING_POWER" ] || [ "$TOTAL_VOTING_POWER" = "null" ]; then
		echo "Error: Unable to parse validator set data"
		exit 1
	fi

	ACTIVE_VALIDATORS=$(echo "$response" | jq -r '.data.active_validators[].addr')
	PENDING_ACTIVE_VALIDATORS=$(echo "$response" | jq -r '.data.pending_active[].addr // empty')
	PENDING_INACTIVE_VALIDATORS=$(echo "$response" | jq -r '.data.pending_inactive[].addr // empty')
}

check_not_in_validator_set() {
	local normalized_account="0x${DELEGATED_RESOURCE_ACCOUNT#0x}"

	if echo "$ACTIVE_VALIDATORS" | grep -q "^${normalized_account}$"; then
		echo "Error: Account $DELEGATED_RESOURCE_ACCOUNT is already an active validator"
		exit 1
	fi

	if [ -n "$PENDING_ACTIVE_VALIDATORS" ] && echo "$PENDING_ACTIVE_VALIDATORS" | grep -q "^${normalized_account}$"; then
		echo "Error: Account $DELEGATED_RESOURCE_ACCOUNT is already pending to become active validator"
		exit 1
	fi

	if [ -n "$PENDING_INACTIVE_VALIDATORS" ] && echo "$PENDING_INACTIVE_VALIDATORS" | grep -q "^${normalized_account}$"; then
		echo "Error: Account $DELEGATED_RESOURCE_ACCOUNT is pending to become inactive validator"
		exit 1
	fi

	echo "Account $DELEGATED_RESOURCE_ACCOUNT is not in current validator set - proceeding"
}

check_account_balance() {
	BALANCE_OUTPUT=$($MOVEMENT_CLI account balance --account "$VALIDATOR_OWNER_ACCOUNT" --url "$NETWORK_API_ADDRESS" 2>&1)

	if [ $? -ne 0 ]; then
		echo "Error: Failed to retrieve account balance"
		echo "$BALANCE_OUTPUT"
		exit 1
	fi

	BALANCE=$(echo "$BALANCE_OUTPUT" | jq -r '.Result[0].balance')

	if [ -z "$BALANCE" ] || [ "$BALANCE" = "null" ]; then
		echo "Error: Unable to parse balance from response"
		exit 1
	fi
}

validate_config() {
	if [ "$STAKE_AMOUNT" -gt "$BALANCE" ]; then
		echo "Error: Stake amount ($STAKE_AMOUNT) exceeds account balance ($BALANCE)"
		exit 1
	fi

	if [ "$STAKE_AMOUNT" -lt "$MIN_STAKE" ]; then
		echo "Error: Stake amount ($STAKE_AMOUNT) is below minimum required stake ($MIN_STAKE)"
		exit 1
	fi

	local max_allowed_stake=$((TOTAL_VOTING_POWER * MAX_STAKE_RATIO / 100))
	if [ "$STAKE_AMOUNT" -gt "$max_allowed_stake" ]; then
		echo "Error: Stake amount ($STAKE_AMOUNT) exceeds $MAX_STAKE_RATIO% of total voting power ($max_allowed_stake)"
		exit 1
	fi
}

execution_summary() {
	echo ""
	echo "Execution Summary (Delegated Pool Mode):"
	echo "==========================================="
	echo "Network Information:"
	echo "  Total Voting Power:      $TOTAL_VOTING_POWER octas"
	echo "  Active Validators:       $ACTIVE_VALIDATOR_COUNT"
	echo ""
	echo "Operator Configuration:"
	echo "  Account Address:         $OPERATOR_ACCOUNT"
	echo ""
	echo "Validator Owner Configuratio:"
	echo "  Account Address:         $VALIDATOR_OWNER_ACCOUNT"
	echo "  Account Balance:         $BALANCE"
	echo "  (CLI stake-amount arg):  $STAKE_AMOUNT"
	echo "  Validator Host:          $VALIDATOR_HOST"
	echo ""
	echo "  Consensus Public Key:    ${CONSENSUS_PUBLIC_KEY:0:64}"
	echo "                           ${CONSENSUS_PUBLIC_KEY:64}"
	echo ""
	echo "  Consensus POP:           ${CONSENSUS_POP:0:64}"
	echo "                           ${CONSENSUS_POP:64:64}"
	echo "                           ${CONSENSUS_POP:128}"
	echo ""
	echo "  Network Public Key:      $NETWORK_PUBLIC_KEY"
	echo ""
	echo "  Network API Address:     $NETWORK_API_ADDRESS"
	echo "==========================================="
	echo ""
}

# Delegated pool logic                                                        #

# Initialize common CLI args (must be called after validate_input sets the variables)
init_common_cli_args() {
	COMMON_CLI_ARGS=(
		--private-key "$VALIDATOR_OWNER_PRIVATE_KEY"
		--sender-account "$VALIDATOR_OWNER_ACCOUNT"
		--url "$NETWORK_API_ADDRESS"
		--gas-unit-price 100
		--max-gas 20000
		--assume-yes
	)
}

init_operator_cli_args() {
	OPERATOR_CLI_ARGS=(
		--private-key "$OPERATOR_ACCOUNT_PRIVATE_KEY"
		--sender-account "$OPERATOR_ACCOUNT"
		--url "$NETWORK_API_ADDRESS"
		--gas-unit-price 100
		--max-gas 20000
		--assume-yes
	)
}

derive_delegated_resource_account() {
	echo "Deriving delegated pool resource-account address..."
	local output
	output=$($MOVEMENT_CLI account derive-resource-account-address \
		--address "$VALIDATOR_OWNER_ACCOUNT" \
		--seed "$DELEGATION_SEED" \
		--seed-encoding utf8 2>&1)

	if [ $? -ne 0 ]; then
		echo "Error: Failed to derive resource account address for delegated pool"
		echo "$output"
		exit 1
	fi

	DELEGATED_RESOURCE_ACCOUNT=$(echo "$output" | jq -r '.Result')

	if [ -z "$DELEGATED_RESOURCE_ACCOUNT" ]; then
		echo "Error: Failed to parse delegated resource account from CLI output"
		echo "$output"
		exit 1
	fi

	echo "Derived delegated resource account: $DELEGATED_RESOURCE_ACCOUNT"
}

init_delegation_pool() {
	echo "Initializing delegation pool for validator owner account..."
	$MOVEMENT_CLI move run \
		--function-id 0x1::delegation_pool::initialize_delegation_pool \
		--args "u64:${DELEGATION_COMMISSION_BPS}" "string:$SEED" \
		"${COMMON_CLI_ARGS[@]}"

	if [ $? -ne 0 ]; then
		echo "Error: Failed to initialize delegation pool"
		exit 1
	fi
}

add_delegation_stake() {
	echo "Adding initial delegated stake of $STAKE_AMOUNT octas..."
	$MOVEMENT_CLI move run \
		--function-id 0x1::delegation_pool::add_stake \
		--args "address:$DELEGATED_RESOURCE_ACCOUNT" "u64:${STAKE_AMOUNT}" \
		"${COMMON_CLI_ARGS[@]}"

	if [ $? -ne 0 ]; then
		echo "Error: Failed to add initial delegated stake"
		exit 1
	fi
}

extract_delegated_consensus_keys() {
	echo "Extracting consensus pubkey & POP for delegation pool..."
	read DELEGATED_CONSENSUS_PUB DELEGATED_CONSENSUS_POP <<<"$(get_consensus_keys "$CONSENSUS_PRIVATE_KEY")"
}

update_delegated_consensus_keys() {
	echo "Updating delegated pool consensus keys..."
	$MOVEMENT_CLI node update-consensus-key \
		--pool-address "$DELEGATED_RESOURCE_ACCOUNT" \
		--consensus-public-key "$DELEGATED_CONSENSUS_PUB" \
		--proof-of-possession "$DELEGATED_CONSENSUS_POP" \
		"${OPERATOR_CLI_ARGS[@]}"

	if [ $? -ne 0 ]; then
		echo "Error: Failed to update delegated pool consensus keys"
		exit 1
	fi
}

update_delegated_network_address() {
	echo "Updating delegated pool network address..."
	$MOVEMENT_CLI node update-validator-network-addresses \
		--pool-address "$DELEGATED_RESOURCE_ACCOUNT" \
		--validator-host "$VALIDATOR_HOST" \
		--validator-network-public-key "$NETWORK_PUBLIC_KEY" \
		"${OPERATOR_CLI_ARGS[@]}"

	if [ $? -ne 0 ]; then
		echo "Error: Failed to update delegated pool network address"
		exit 1
	fi
}

check_vn_endpoint_healthy() {
	local url="${NETWORK_API_ADDRESS%/}/v1"

	echo "Checking VN endpoint health at: $url ..."
	# -s: silent, -o /dev/null: discard body, -w: print status code only
	local http_code
	http_code=$(curl -s -o /dev/null -w "%{http_code}" "$url" || echo "000")

	if [ "$http_code" != "200" ]; then
		echo "Error: VN endpoint '$url' is not healthy (HTTP $http_code, expected 200)."
		echo "Aborting join-validator-set operation."
		exit 1
	fi

	echo "VN endpoint is healthy (200)."
}

set_pool_operator() {
	# Assign operator
	echo "Setting operator of delegation pool..."
	$MOVEMENT_CLI move run \
		--function-id 0x1::delegation_pool::set_operator \
		--args address:${OPERATOR_ACCOUNT} \
		"${COMMON_CLI_ARGS[@]}"

	if [ $? -ne 0 ]; then
		echo "Error: Failed to set operator of delegation pool"
		exit 1
	fi

	echo "Successfully set operator of delegation pool"

}

setup_delegated_pool_flow() {
	echo ""
	echo "-------------------------------------------"
	echo "Starting delegated pool setup (Validator 4-style)..."
	echo "-------------------------------------------"

	# This pseudo was explicitly for previewnet; guard here.
	if [ "$NETWORK_INPUT" != "previewnet" ]; then
		echo "Delegated pool setup skipped: only configured for 'previewnet'. Current network: $NETWORK_INPUT"
		return
	fi
	if [ "$DRY_RUN" = "true" ]; then
		echo "Dry run enabled. Delegated pool actions will NOT be executed."
		echo "Would run, in order:"
		echo "  - derive_delegated_resource_account"
		echo "  - get_delegated_pool_address"
		echo "  - init_delegation_pool"
		echo "  - add_delegation_stake"
		echo "  - extract_delegated_consensus_keys"
		echo "  - update_delegated_consensus_keys"
		echo "  - update_delegated_network_address"
		echo "  - join_delegated_validator_set"
		echo ""
		echo "Afterwards you should:"
		echo "  - Wait one epoch."
		echo "  - Update validator-identity.yaml account_address with the delegated pool address."
		echo "  - Start the validator process."
		return
	fi

	# Initialize common CLI args before running delegated pool commands
	init_common_cli_args
	init_operator_cli_args

	init_delegation_pool
	set_pool_operator
	add_delegation_stake
	extract_delegated_consensus_keys
	update_delegated_consensus_keys
	update_delegated_network_address

	echo ""
	echo "Delegated pool setup completed."
	echo "TODOs:"
	echo "  - Deploy the validator using the identity below."
	echo "  - Once deployed, run the command below to join the validator set."
	echo "  - Wait one epoch for the validator to become active."
	echo " Netowkrt Key: "
	echo "  /dns/movement-network-previewnet-vn-01.mvmt-previewnet:6180/tcp/6180/noise-ik/$NETWORK_PUBLIC_KEY/handshake/0"
	echo " VALIDATOR IDENTITY YAML FILE "
	echo " -------------------------------------------"
	echo ""
	echo "---"
	echo "account_address: \"$DELEGATED_RESOURCE_ACCOUNT\""
	echo "account_private_key: \"$OPERATOR_ACCOUNT_PRIVATE_KEY\""
	echo "consensus_private_key: \"$CONSENSUS_PRIVATE_KEY\""
	echo "network_private_key: \"$NETWORK_PRIVATE_KEY\""
	echo ""
	echo " -------------------------------------------"
	echo ""
	echo ""
	echo "Command to join validator set after deploying the validator:"
	echo ""
	echo "$MOVEMENT_CLI node join-validator-set \\"
	echo "  --pool-address \"$DELEGATED_RESOURCE_ACCOUNT\" \\"
	echo "  ${OPERATOR_CLI_ARGS[*]}"
	echo ""

}

dependency_check
validate_input
get_operator_identity
get_validator_owner_identity
get_current_validator_set
derive_delegated_resource_account
check_not_in_validator_set
check_account_balance
execution_summary
# validate_config
setup_delegated_pool_flow
