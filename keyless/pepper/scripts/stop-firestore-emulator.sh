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

# Stop the Firestore emulator (by killing the process using the port)
lsof -t -i:${FIRESTORE_PORT} | xargs kill -9
