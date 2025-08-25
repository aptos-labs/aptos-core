#!/bin/bash

# Exit immediately if a command fails
set -e

# Port for Firestore emulator (default 8081)
FIRESTORE_PORT=8081

# Start the Firestore emulator
gcloud emulators firestore start --host-port=localhost:${FIRESTORE_PORT}
