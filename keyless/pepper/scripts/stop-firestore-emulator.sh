#!/bin/bash

# Exit immediately if a command fails
set -e

# Port for Firestore emulator (default 8081)
FIRESTORE_PORT=8081

# Stop the Firestore emulator (by killing the process using the port)
lsof -t -i:${FIRESTORE_PORT} | xargs kill -9
