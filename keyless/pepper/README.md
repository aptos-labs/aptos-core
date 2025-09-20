# Pepper Service Developer Guide

## Overview

This directory contains the **Pepper Service**, and a Rust-based **Pepper Client** example. 

Note: if you are a frontend developer, you can use the Pepper Client example as a reference to
understand how to interact with the Pepper Service API.

## Local Development
The simplest way to run the Pepper Service and Pepper Client locally are to use the example scripts
in this repository. Follow the steps below to get started.

### Start the Pepper Service

First, start the Pepper Service using the commands below:

```bash
cd keyless/pepper/scripts
./start-pepper-service.sh
```

This will start the Pepper Service in local development mode on port `8000`, using a temporary
file-based database and connecting to the Aptos **devnet** to fetch on-chain resources. 

Note: the Pepper Service will run in the foreground, so you will need to keep this terminal window
open while you interact with the Pepper Service.

### Start the Pepper Client

Next, you can run the Pepper Client example to interact with the Pepper Service:

```bash
cd keyless/pepper/scripts
./start-pepper-service.sh
```

This will start the Pepper Client, which makes several API calls to the Pepper Service to
demonstrate example user flows.

Note: the Pepper Client is an interactive console program, so you will need to follow the
instructions to manually complete a session with the Pepper Service.

## Local Development with Firestore Emulator

If you wish to test the Pepper Service with a Firestore database, you can do so by running
a Firestore emulator locally. The steps below will guide you through the process.

### Prerequisites

To run the Pepper Service with a Firestore database, this requires:
1. **GCP Project**: You must have a GCP project already set up.
2. **Service Account**: You need to create a service account with Firestore access and download the credential file 
   in JSON format.

Both of these steps can be done via the GCP console. Once you have completed these steps, you can proceed.

### Start the Firestore Emulator

First, you will need to start the Firestore emulator locally. This can be done by running:

```bash
cd keyless/pepper/scripts
./start-firestore-emulator.sh 8081
```

The script arguments are:
- `<FIRESTORE_PORT>`: The port where the Firestore emulator will run.

The command above will start the Firestore emulator on port `8081`. The emulator will run in the foreground, so you
will need to keep this terminal window open while you work with the Pepper Service.

### Stop the Emulator

If you ever need to stop the Firestore emulator, you can do so by running:
```bash
cd keyless/pepper/scripts
./stop-firestore-emulator.sh 8081
```

The script arguments are:
- `<FIRESTORE_PORT>`: The port where the existing Firestore emulator is already running.

When stopping the Firestore emulator, be sure to enter the correct port number.

### Start the Pepper Service with Firestore

Next, you can start the Pepper Service with Firestore by running:

```bash
cd keyless/pepper/scripts
./start-pepper-service-with-firestore.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <GOOGLE_PROJECT_ID>
```

The script arguments are:
- `<FIRESTORE_EMULATOR_HOST>`: The host and port of the Firestore emulator, e.g., `http://localhost:8081`.
- `<GOOGLE_APPLICATION_CREDENTIALS>`: The path to your service account credential file (in JSON format).
- `<GOOGLE_PROJECT_ID>`: The ID of your GCP project.

### Start the Pepper Client with Firestore

To run the example client, execute the following commands:

```bash
cd keyless/pepper/scripts
./start-pepper-client-with-firestore.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PEPPER_SERVICE_URL> <GOOGLE_PROJECT_ID> <FIRESTORE_DATABASE_ID>"
```

The script arguments are:
- `<FIRESTORE_EMULATOR_HOST>`: The host and port of the Firestore emulator, e.g., `http://localhost:8081`.
- `<GOOGLE_APPLICATION_CREDENTIALS>`: The path to your service account credential file (in JSON format).
- `<PEPPER_SERVICE_URL>`: The host and port of the Pepper Service, e.g., `http://localhost:8000`.
- `<GOOGLE_PROJECT_ID>`: The ID of your GCP project.
- `<FIRESTORE_DATABASE_ID>`: The ID of your Firestore database.
