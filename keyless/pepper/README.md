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
./start-firestore-emulator.sh
````

Note that the Firestore emulator will run in the foreground, so you will need to keep this terminal
window open while you work with the Pepper Service.

### Stop the Emulator

If you ever need to stop the Firestore emulator, you can do so by running the `stop-firestore-emulator.sh` script:
```bash
cd keyless/pepper/scripts
./stop-firestore-emulator.sh
```

## Start the Pepper Service

Next, you can start the Pepper Service by running the `start-pepper-service.sh` script in this repository.

```bash
cd keyless/pepper/scripts
./start-pepper-service.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PROJECT_ID>
```

The script arguments are:
- `<FIRESTORE_EMULATOR_HOST>`: The host and port of the Firestore emulator, e.g., `localhost:8081`.
- `<GOOGLE_APPLICATION_CREDENTIALS>`: The path to your service account credential file (in JSON format).
- `<PROJECT_ID>`: The ID of your GCP project.

Note that the Pepper Service will run in the foreground, so you will need to keep this terminal
window open while you interact with the Pepper Service.

## Run the Pepper Client Example

Finally, you can run the example client to interact with the Pepper Service. The client will send a
pepper request and verify the response. It also connects to the account recovery database and
verifies that it was correctly updated by the Pepper Service.

To run the example client, execute the `start-pepper-client.sh` script in this repository.

```bash
cd keyless/pepper/scripts
./start-pepper-client.sh <FIRESTORE_EMULATOR_HOST> <GOOGLE_APPLICATION_CREDENTIALS> <PROJECT_ID>
```

The script arguments are the same as those used for starting the Pepper Service (above).

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

To verify that the resources have not been cached yet, ensure the following curl commands return a 404:

```bash
curl -v http://localhost:8000/cached/groth16-vk
curl -v http://localhost:8000/cached/keyless-config
```

Next, to mock an Aptos fullnode, run a naive HTTP server to serve the resources to the Pepper Service:
```bash
cd keyless/pepper/service/resources
python3 -m http.server 4444
```

After waiting a while (e.g., 10 seconds), the Pepper Service should have fetched and cached the resources.
You can verify this by running the same curl commands again, which should now return the cached data:

```bash
curl -v http://localhost:8000/cached/groth16-vk
curl -v http://localhost:8000/cached/keyless-config
```
