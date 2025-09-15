# Pepper Service Developer Guide

## Requirements

The Pepper Service requires an account recovery database, which is deployed as a Firestore instance on GCP.
If you wish to run the Pepper Service locally, you will need to set up a Firestore emulator. This requires:
1. **GCP Project**: You must have a GCP project already set up.
2. **Service Account**: You need to create a service account with Firestore access and download the credential file 
   in JSON format.

Both of these steps can be done via the GCP console. Once you have completed these steps, you can proceed.

## Start the Firestore Emulator

First, you will need to start the Firestore emulator locally. This can be done by running the
`start-firestore-emulator.sh` script in this repository.

```bash
cd keyless/pepper/scripts
./start-firestore-emulator.sh 8081
````

The script arguments are:
- `<FIRESTORE_PORT>`: The port where the Firestore emulator will run.

The command above will start the Firestore emulator on port `8081`. The emulator will run in the foreground, so you
will need to keep this terminal window open while you work with the Pepper Service.

### Stop the Emulator

If you ever need to stop the Firestore emulator, you can do so by running the `stop-firestore-emulator.sh` script:
```bash
cd keyless/pepper/scripts
./stop-firestore-emulator.sh 8081
```

The script arguments are:
- `<FIRESTORE_PORT>`: The port where the existing Firestore emulator is already running.

When stopping the Firestore emulator, be sure to enter the correct port number.

## Start the Pepper Service

Next, you can start the Pepper Service by running the `start-pepper-service.sh` script in this repository.

```bash
cd keyless/pepper/scripts
./start-pepper-service.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <GOOGLE_PROJECT_ID>
```

The script arguments are:
- `<FIRESTORE_EMULATOR_HOST>`: The host and port of the Firestore emulator, e.g., `http://localhost:8081`.
- `<GOOGLE_APPLICATION_CREDENTIALS>`: The path to your service account credential file (in JSON format).
- `<GOOGLE_PROJECT_ID>`: The ID of your GCP project.

Note that the Pepper Service will run in the foreground, so you will need to keep this terminal
window open while you interact with the Pepper Service.

## Run the Pepper Client Example

Finally, you can run the example client to interact with the Pepper Service. The client will send a
pepper request and verify the response. It also connects to the account recovery database and
verifies that it was correctly updated by the Pepper Service.

To run the example client, execute the `start-pepper-client.sh` script in this repository.

```bash
cd keyless/pepper/scripts
./start-pepper-client.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PEPPER_SERVICE_URL> <GOOGLE_PROJECT_ID> <FIRESTORE_DATABASE_ID>"
```

The script arguments are:
- `<FIRESTORE_EMULATOR_HOST>`: The host and port of the Firestore emulator, e.g., `http://localhost:8081`.
- `<GOOGLE_APPLICATION_CREDENTIALS>`: The path to your service account credential file (in JSON format).
- `<PEPPER_SERVICE_URL>`: The host and port of the Pepper Service, e.g., `http://localhost:8000`.
- `<GOOGLE_PROJECT_ID>`: The ID of your GCP project.
- `<FIRESTORE_DATABASE_ID>`: The ID of your Firestore database.

Note: the client is an interactive console program, so you will need to follow the instructions
to manually complete a session with the Pepper Service.

### Frontend Developer Guide

If you are a frontend developer, you should use the Pepper Client example as a reference to
understand how to interact with the Pepper Service API. Your frontend application should
implement the same user flows and API calls in the example.

## Additional Testing

If you wish to test the `v0/verify` API endpoint of the Pepper Service, you can do so manually.
Since this endpoint requires fetching blockchain resources from an Aptos fullnode, you will
need to set up a mock HTTP server to serve the following two resources:
1. `0x1::keyless_account::Groth16VerificationKey`: This is the verification key for the Groth16 proof.
2. `0x1::keyless_account::Configuration`: This is the configuration for the keyless account.

To test the `v0/verify` endpoint, first, run the Pepper Service with the following environment variables:

```bash
export ONCHAIN_GROTH16_VK_URL=http://localhost:4444/groth16_vk.json
export ONCHAIN_KEYLESS_CONFIG_URL=http://localhost:4444/keyless_config.json
./start-pepper-service.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PROJECT_ID>
```

The export commands above will override the default location where the Pepper Service fetches these resources
(i.e., instead of connecting to an Aptos fullnode, it will connect to the service running on localhost at
port `4444`, which will be the mock HTTP server we deploy below.)

To verify that the resources have not yet been fetched from the mocked Aptos node and cached by the pepper service,
ensure the following curl commands return a 404:

```bash
curl -v http://localhost:8000/cached/groth16-vk
curl -v http://localhost:8000/cached/keyless-config
```

These commands will connect to the Pepper Service (running on port `8000`) and attempt to fetch the cached resources.

Next, to mock an Aptos fullnode, run a naive HTTP server to serve the resources to the Pepper Service:
```bash
cd keyless/pepper/service/src/tests/test_resources
python3 -m http.server 4444
```

The HTTP server above will simply provide access to all files in the `test_resources/` directory.

After waiting a while (e.g., 10 seconds), the Pepper Service should have fetched and cached the resources.
You can verify this by running the same curl commands again, which should now return the cached data:

```bash
curl -v http://localhost:8000/cached/groth16-vk
curl -v http://localhost:8000/cached/keyless-config
```
