#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the required arguments are provided
if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PROJECT_ID>"
    exit 1
fi

# Export the Firestore emulator host and port (passed as the first argument to the script)
export FIRESTORE_EMULATOR_HOST=$1

# Export the Google application credentials (passed as the second argument to the script).
# Note: this should point to the service account credential JSON file.
export GOOGLE_APPLICATION_CREDENTIALS=$2

# Specify the account recovery DB location (passed as the third argument to the script).
export PROJECT_ID=$3
export DATABASE_ID='(default)' # the default name of a local firestore emulator

# Start the pepper client example
cargo run -p aptos-keyless-pepper-example-client-rust
