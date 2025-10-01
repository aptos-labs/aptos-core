#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the correct arguments are provided
if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <GOOGLE_PROJECT_ID>"
    exit 1
fi

# Export the Firestore emulator host and port (passed as the first argument to the script)
export FIRESTORE_EMULATOR_HOST=$1

# Export the Google application credentials (passed as the second argument to the script).
# Note: this should point to the service account credential JSON file.
export GOOGLE_APPLICATION_CREDENTIALS=$2

# Specify the account recovery DB location (passed as the third argument to the script).
GOOGLE_PROJECT_ID=$3
FIRESTORE_DATABASE_ID='(default)' # the default name of a local firestore emulator

# Specify an account manager (e.g., Google, Facebook, Apple). The example below is for Google.
export ACCOUNT_MANAGER_0_ISSUER=https://accounts.google.com
export ACCOUNT_MANAGER_0_AUD=407408718192.apps.googleusercontent.com

# To specify more account managers, do the following:
#   export ACCOUNT_MANAGER_1_ISSUER=https://www.facebook.com
#   export ACCOUNT_MANAGER_1_AUD=999999999.apps.fbusercontent.com
#   export ACCOUNT_MANAGER_2_ISSUER=https://appleid.apple.com
#   export ACCOUNT_MANAGER_2_AUD=88888888.apps.appleusercontent.com

# The VUF private key in hex format. This is a dummy key for testing purposes.
VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff

# The URLs for the on-chain keyless configuration and groth16 verification key (for local development, we use devnet)
ON_CHAIN_GROTH16_VK_URL=https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey
ON_CHAIN_KEYLESS_CONFIG_URL=https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Configuration

# Start the pepper service
echo "Starting the pepper service with a Firestore emulator, connecting to Aptos devnet for on-chain data!"
cargo run -p aptos-keyless-pepper-service -- --vuf-private-key-seed-hex=${VUF_KEY_SEED_HEX}
cargo run -p aptos-keyless-pepper-service -- \
--google-project-id=${GOOGLE_PROJECT_ID} \
--firestore-database-id=${FIRESTORE_DATABASE_ID} \
--on_chain-groth16-vk-url=${ON_CHAIN_GROTH16_VK_URL} \
--on-chain-keyless-config-url=${ON_CHAIN_KEYLESS_CONFIG_URL} \
--vuf-private-key-seed-hex=${VUF_KEY_SEED_HEX}
