#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the required arguments are provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <FIRESTORE_PORT>"
    exit 1
fi

# Port for Firestore emulator (the first argument to the script)
FIRESTORE_PORT=$1

# Start the Firestore emulator
gcloud emulators firestore start --host-port=localhost:${FIRESTORE_PORT}
