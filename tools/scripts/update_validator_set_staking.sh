#!/bin/bash

# Script to update the validator set using movement CLI
# Usage: ./update_validator_set.sh <validator-identity-file> <network> <stake-amount> <validator-host> [dry-run]
# Network options: testnet | mainnet | custom:https://your-api-url.com

set -e
set -o pipefail # Add this at the top

# Configuration
MOVEMENT_CLI="movement"
PROFILE="PROFILE_SHOULD_NOT_BE_USED"

# Global stake validation constants
MIN_STAKE=0        # Minimum allowed stake (0 for now, will be non-zero later)
MAX_STAKE_RATIO=30 # Maximum stake as percentage of total voting power

# Network presets
TESTNET_API="https://testnet.movementnetwork.xyz"
MAINNET_API="https://mainnet.movementnetwork.xyz"

# Operator identity
OPERATOR_IDENTITY_FILE="$1"
NETWORK_INPUT="$2"
STAKE_AMOUNT="$3"
VALIDATOR_HOST="$4"
VALIDATOR_OWNER_ACCOUNT="$5"
VALIDATOR_OWNER_PRIVATE_KEY="$6"
DRY_RUN="${7:-true}"

help_message_and_exit() {
	echo "Usage: $0 <operator-identity-file> <network> <stake-amount> <validator-host> <validator-owner-account> <validator-owner-private-key> [dry-run]"
	echo "Network options:"
	echo "  testnet               - Movement testnet ($TESTNET_API)"
	echo "  mainnet               - Movement mainnet ($MAINNET_API)"
	echo "  custom:https://url    - Custom API endpoint"
	exit 1
}

# Functions
get_network_pub_key() {
	local private_key="$1"
	local temp_file=$(mktemp)

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

	# The public key is written to <output-file>.pub
	local public_key=$(cat "${temp_file}.pub")
	rm -f "$temp_file" "${temp_file}.pub"

	echo "$public_key"
}

get_consensus_keys() {
	local private_key="$1"
	local temp_file=$(mktemp)

	$MOVEMENT_CLI key extract-public-key \
		--private-key "$private_key" \
		--key-type "bls12381" \
		--encoding hex \
		--output-file "$temp_file" \
		--assume-yes >/dev/null 2>&1
	local exit_code=$?

	if [ $exit_code -ne 0 ]; then
		rm -f "$temp_file" "${temp_file}.pub" "${temp_file}.pop"
		echo "Error: Failed to extract consensus keys (exit code: $exit_code)" >&2
		exit 1
	fi

	# Check if .pub and .pop files were created
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

	# Read both the public key and proof of possession
	local public_key=$(cat "${temp_file}.pub")
	local pop_key=$(cat "${temp_file}.pop")

	# Clean up all temp files
	rm -f "$temp_file" "${temp_file}.pub" "${temp_file}.pop"

	# Return both values separated by space
	echo "$public_key $pop_key"
}

dependency_check() {
	if ! command -v $MOVEMENT_CLI &>/dev/null; then
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

	if [ -z "$NETWORK_INPUT" ]; then
		echo "Error: Network not provided"
		help_message_and_exit
	fi

	# Parse network input and set NETWORK_API_ADDRESS
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
		NETWORK_API_ADDRESS="${NETWORK_API_ADDRESS%/}" # Remove trailing slash
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

	if [ -z "$VALIDATOR_OWNER_ACCOUNT" ]; then
		echo "Error: Validator account not provided"
		help_message_and_exit
	fi

	if [ -z "$VALIDATOR_OWNER_PRIVATE_KEY" ]; then
		echo "Error: Validator account private key not provided"
		help_message_and_exit
	fi
}

get_identities() {
	# Read values from YAML file and strip quotes
	OPERATOR_ACCOUNT=$(grep "account_address:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	OPERATOR_ACCOUNT_PRIVATE_KEY=$(grep "account_private_key:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	CONSENSUS_PRIVATE_KEY=$(grep "consensus_private_key:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')
	NETWORK_PRIVATE_KEY=$(grep "network_private_key:" "$OPERATOR_IDENTITY_FILE" | awk '{print $2}' | tr -d '"')

	# Validate all fields are present
	if [ -z "$OPERATOR_ACCOUNT" ] || [ -z "$OPERATOR_ACCOUNT_PRIVATE_KEY" ] || [ -z "$CONSENSUS_PRIVATE_KEY" ] || [ -z "$NETWORK_PRIVATE_KEY" ]; then
		echo "Error: Missing required fields in validator identity file"
		echo "Required fields: account_address, account_private_key, consensus_private_key, network_private_key"
		exit 1
	fi

	# Extract public keys from private keys
	read CONSENSUS_PUBLIC_KEY CONSENSUS_POP <<<$(get_consensus_keys "$CONSENSUS_PRIVATE_KEY")

	NETWORK_PUBLIC_KEY=$(get_network_pub_key "$NETWORK_PRIVATE_KEY")
}

get_current_validator_set() {
	local api_url="${NETWORK_API_ADDRESS}/v1/accounts/0x1/resource/0x1::stake::ValidatorSet"
	local response=$(curl -s "$api_url")

	if [ $? -ne 0 ]; then
		echo "Error: Failed to fetch validator set"
		exit 1
	fi

	# Parse the validator set data
	TOTAL_VOTING_POWER=$(echo "$response" | jq -r '.data.total_voting_power')
	ACTIVE_VALIDATOR_COUNT=$(echo "$response" | jq -r '.data.active_validators | length')
	TOTAL_JOINING_POWER=$(echo "$response" | jq -r '.data.total_joining_power')

	if [ -z "$TOTAL_VOTING_POWER" ] || [ "$TOTAL_VOTING_POWER" = "null" ]; then
		echo "Error: Unable to parse validator set data"
		exit 1
	fi

	# Extract list of active validator addresses
	ACTIVE_VALIDATORS=$(echo "$response" | jq -r '.data.active_validators[].addr')
	PENDING_ACTIVE_VALIDATORS=$(echo "$response" | jq -r '.data.pending_active[].addr // empty')
	PENDING_INACTIVE_VALIDATORS=$(echo "$response" | jq -r '.data.pending_inactive[].addr // empty')
}

check_not_in_validator_set() {
	# Normalize account address to have 0x prefix for comparison
	local normalized_account="0x${OPERATOR_ACCOUNT#0x}"

	# Check if account is already in active validators
	if echo "$ACTIVE_VALIDATORS" | grep -q "^${normalized_account}$"; then
		echo "Error: Account $OPERATOR_ACCOUNT is already an active validator"
		exit 1
	fi

	# Check if account is in pending active validators
	if [ -n "$PENDING_ACTIVE_VALIDATORS" ] && echo "$PENDING_ACTIVE_VALIDATORS" | grep -q "^${normalized_account}$"; then
		echo "Error: Account $OPERATOR_ACCOUNT is already pending to become active validator"
		exit 1
	fi

	# Check if account is in pending inactive validators
	if [ -n "$PENDING_INACTIVE_VALIDATORS" ] && echo "$PENDING_INACTIVE_VALIDATORS" | grep -q "^${normalized_account}$"; then
		echo "Error: Account $OPERATOR_ACCOUNT is pending to become inactive validator"
		exit 1
	fi

	echo "Account $OPERATOR_ACCOUNT is not in current validator set - proceeding"
}

check_account_balance() {
	BALANCE_OUTPUT=$($MOVEMENT_CLI account balance --account $VALIDATOR_OWNER_ACCOUNT --url $NETWORK_API_ADDRESS 2>&1)

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

	# Range check: MIN_STAKE <= STAKE_AMOUNT <= MAX_STAKE
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

# Execution.
# Validator owner operation.
init_stake_owner() {
	$MOVEMENT_CLI stake create-staking-contract \
		--operator $OPERATOR_ACCOUNT \
		--voter $OPERATOR_ACCOUNT \
		--amount $STAKE_AMOUNT \
		--commission-percentage 10 \
		--private-key $VALIDATOR_OWNER_PRIVATE_KEY \
		--url $NETWORK_API_ADDRESS \
		--gas-unit-price 100 \
		--max-gas 20000 \
		--assume-yes
	if [ $? -ne 0 ]; then
		echo "Error: Failed to initialize stake owner"
		exit 1
	fi
}

# Get pool address
get_pool_address() {
	POOL_ADDRESS_OUTPUT=$($MOVEMENT_CLI node get-stake-pool --owner-address $VALIDATOR_OWNER_ACCOUNT --url $NETWORK_API_ADDRESS 2>&1)

	if [ $? -ne 0 ]; then
		echo "Error: Failed to retrieve stake pool address"
		echo "$POOL_ADDRESS_OUTPUT"
		exit 1
	fi

	POOL_ADDRESS=$(echo "$POOL_ADDRESS_OUTPUT" | jq -r '.Result[0].pool_address')

	if [ -z "$POOL_ADDRESS" ] || [ "$POOL_ADDRESS" = "null" ]; then
		echo "Error: Unable to parse stake pool address from response"
		exit 1
	fi
}

# Operator operation.
update_consensus_keys() {
	$MOVEMENT_CLI node update-consensus-key \
		--pool-address $POOL_ADDRESS \
		--consensus-public-key $CONSENSUS_PUBLIC_KEY \
		--proof-of-possession $CONSENSUS_POP \
		--private-key $OPERATOR_ACCOUNT_PRIVATE_KEY \
		--sender-account $OPERATOR_ACCOUNT \
		--url $NETWORK_API_ADDRESS \
		--gas-unit-price 100 \
		--max-gas 20000 \
		--assume-yes
	if [ $? -ne 0 ]; then
		echo "Error: Failed to update consensus keys"
		exit 1
	fi
}

# Operator operation
# TODO: add fullnode host here as well.
update_network_address() {
	$MOVEMENT_CLI node update-validator-network-addresses \
		--pool-address $POOL_ADDRESS \
		--validator-host $VALIDATOR_HOST \
		--validator-network-public-key $NETWORK_PUBLIC_KEY \
		--private-key $OPERATOR_ACCOUNT_PRIVATE_KEY \
		--sender-account $OPERATOR_ACCOUNT \
		--url $NETWORK_API_ADDRESS \
		--gas-unit-price 100 \
		--max-gas 20000 \
		--assume-yes
	if [ $? -ne 0 ]; then
		echo "Error: Failed to update network address"
		exit 1
	fi
}

# Operator operation
join_the_network() {
	$MOVEMENT_CLI node join-validator-set \
		--pool-address $POOL_ADDRESS \
		--private-key $OPERATOR_ACCOUNT_PRIVATE_KEY \
		--sender-account $OPERATOR_ACCOUNT \
		--url $NETWORK_API_ADDRESS \
		--gas-unit-price 100 \
		--max-gas 20000 \
		--assume-yes
	if [ $? -ne 0 ]; then
		echo "Error: Failed to join the validator set"
		exit 1
	fi
}

execute() {
	# validate_config

	if [ "$DRY_RUN" = "true" ]; then
		echo "Dry run enabled. No changes will be made."
	else
		echo "Executing validator set update..."

		# Initialize stake owner
		echo "Initializing stake owner..."
		init_stake_owner

		# Get pool address
		echo "Retrieving stake pool address..."
		get_pool_address

		# Update consensus keys
		echo "Updating consensus keys..."
		update_consensus_keys

		# Update network address
		echo "Updating network address..."
		update_network_address

		echo "Validator set update completed successfully!"
	fi
}

execution_result() {
	if [ "$DRY_RUN" = "true" ]; then
		echo "Dry run completed. No changes were made."
	else
		echo "Execution completed. Validator set update was successful."
		echo "Check the updated validator set at:"
		echo "  $NETWORK_API_ADDRESS/v1/accounts/0x1/resource/0x1::stake::ValidatorSet"
	fi
}

execution_summary() {
	echo ""
	echo "Execution Summary:"
	echo "==========================================="
	echo "Network Information:"
	echo "  Total Voting Power:      $TOTAL_VOTING_POWER octas"
	echo "  Active Validators:       $ACTIVE_VALIDATOR_COUNT"
	echo ""
	echo "Operator Configuration:"
	echo ""
	echo "  Account Address:         $OPERATOR_ACCOUNT"
	echo ""
	echo "Validator Owner Configuration:"
	echo "  Account Address:         $VALIDATOR_OWNER_ACCOUNT"
	echo "  Account Balance:         $BALANCE"
	echo "  Stake Amount:            $STAKE_AMOUNT"
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

# Main execution
dependency_check
validate_input
get_identities
get_current_validator_set
check_not_in_validator_set
check_account_balance
execution_summary
execute
execution_result
