#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the correct arguments are provided
if [ "$#" -ne 0 ]; then
    echo "Usage: $0"
    exit 1
fi

# The VUF private key in hex format. This is a dummy key for testing purposes.
VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff

# The expected VUF public key and derived pepper on startup. These are derived from the private key above.
EXPECTED_VUF_PUBKEY_ON_STARTUP=b601ec185c62da8f5c0402d4d4f987b63b06972c11f6f6f9d68464bda32fa502a5eac0adeda29917b6f8fa9bbe0f498209dcdb48d6a066c1f599c0502c5b4c24d4b057c758549e3e8a89ad861a82a789886d69876e6c6341f115c9ecc381eefd
EXPECTED_DERIVED_PEPPER_ON_STARTUP=72c147eb3457ec5ff32cc540e09ebe739a189b0b194fe269f18d29556dcc2f

# The URLs for the on-chain keyless configuration and groth16 verification key (for local development, we use devnet)
ON_CHAIN_GROTH16_VK_URL=https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey
ON_CHAIN_KEYLESS_CONFIG_URL=https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Configuration

# Start the pepper service
echo "Starting the pepper service in local development mode, connecting to Aptos devnet for on-chain data!"
cargo run -p aptos-keyless-pepper-service -- \
--local-development-mode \
--expected-derived-pepper-on-startup=${EXPECTED_DERIVED_PEPPER_ON_STARTUP} \
--expected-vuf-pubkey-on-startup=${EXPECTED_VUF_PUBKEY_ON_STARTUP} \
--on-chain-groth16-vk-url=${ON_CHAIN_GROTH16_VK_URL} \
--on-chain-keyless-config-url=${ON_CHAIN_KEYLESS_CONFIG_URL} \
--vuf-private-key-seed-hex=${VUF_KEY_SEED_HEX}
