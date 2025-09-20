#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the correct arguments are provided
if [ "$#" -ne 0 ]; then
    echo "Usage: $0"
    exit 1
fi

# Specify the VUF private key in hex format. This is a dummy key for testing purposes.
export VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff

# Specify the URLs for the on-chain keyless configuration and groth16 verification key (for local development, we use devnet)
export ONCHAIN_GROTH16_VK_URL=https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey
export ONCHAIN_KEYLESS_CONFIG_URL=https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Configuration

# Start the pepper service
echo "Starting the pepper service in local development mode, connecting to Aptos devnet for on-chain data!"
cargo run -p aptos-keyless-pepper-service -- --local-development-mode
