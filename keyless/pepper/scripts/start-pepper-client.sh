#!/bin/bash

# Exit immediately if a command fails
set -e

# Check if the required arguments are provided
if [ "$#" -ne 5 ]; then
    echo "Usage: $0 <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PEPPER_SERVICE_URL> <GOOGLE_PROJECT_ID> <FIRESTORE_DATABASE_ID>"
    exit 1
fi

# Export the Firestore emulator host and port (passed as the first argument to the script)
export FIRESTORE_EMULATOR_HOST=$1

# Export the Google application credentials (passed as the second argument to the script).
# Note: this should point to the service account credential JSON file.
export GOOGLE_APPLICATION_CREDENTIALS=$2

# Fetch the Google Project ID, Firestore Database ID and Pepper Service URL from the script arguments
PEPPER_SERVICE_URL=$3
GOOGLE_PROJECT_ID=$4
FIRESTORE_DATABASE_ID=$5

# Start the pepper client example
cargo run -p velor-keyless-pepper-example-client-rust -- --pepper-service-url=${PEPPER_SERVICE_URL} --firestore-google-project-id=${GOOGLE_PROJECT_ID} --firestore-database-id=${FIRESTORE_DATABASE_ID}
