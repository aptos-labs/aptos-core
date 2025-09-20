#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the correct arguments are provided
if [ "$#" -ne 0 ]; then
    echo "Usage: $0"
    exit 1
fi

# Start the pepper client example
echo "Starting the pepper client example connecting to a local pepper service at http://localhost:8000!"
cargo run -p aptos-keyless-pepper-example-client-rust -- --pepper-service-url="http://localhost:8000"
