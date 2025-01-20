## Dev guide

### Prepare dependency: a firestore instance.

Pepper service now depends on an account recovery DB, which is deployed as a firestore on GCP.
You must have a GCP project in order to run firestore emulator locally.
You also need to create a service account, grant firestore access to it, and download its credential file.

In terminal 0, start a local firestore emulator.
```bash
gcloud emulators firestore start --host-port=localhost:8081
```

In terminal 1, set up the environment variables and start the pepper service.
```bash
# If specified, firestore library connects to the local emulator instead of the real GCP API.
export FIRESTORE_EMULATOR_HOST=localhost:8081

export GOOGLE_APPLICATION_CREDENTIALS="<path-to-your-service-account-credential>"

# Specify the account recovery DB location.
export PROJECT_ID=$(gcloud config get-value project)
export DATABASE_ID='(default)' # the default name of a local firestore emulator

# Specify an account manager.
export ACCOUNT_MANAGER_0_ISSUER=https://accounts.google.com
export ACCOUNT_MANAGER_0_AUD=407408718192.apps.googleusercontent.com
# To specify more, do the following:
#   export ACCOUNT_MANAGER_1_ISSUER=https://www.facebook.com
#   export ACCOUNT_MANAGER_1_AUD=999999999.apps.fbusercontent.com
#   export ACCOUNT_MANAGER_2_ISSUER=https://appleid.apple.com
#   export ACCOUNT_MANAGER_2_AUD=88888888.apps.appleusercontent.com

# Specify the VUF private key.
export VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff

# Start the pepper service.
cargo run -p aptos-keyless-pepper-service
```

Run the example client in terminal 2.
```bash
# In addition to sending a pepper request and verify the response,
# the example client also connects to the account recovery DB and verifies that it was correctly updated by the pepper service.
# So here it relies on the same firestore-related parameters as `aptos-keyless-pepper-service` does.
export FIRESTORE_EMULATOR_HOST=localhost:8081
export GOOGLE_APPLICATION_CREDENTIALS="<path-to-your-service-account-credential>"
export PROJECT_ID=$(gcloud config get-value project)
export DATABASE_ID='(default)' # the default name of a local firestore emulator

# Start the example client.
cargo run -p aptos-keyless-pepper-example-client-rust
```
This is an interactive console program.
Follow the instruction to manually complete a session with the pepper service.

## NOTE for frontend developers
Sorry for the missing examples in other programming languages.
For now please read through `example-client-rust/src/main.rs` implementation and output:
that is what your frontend needs to do.

## Extra: manual testing for endpoint `v0/verify`.
NOTE: API `v0/verify` now depends on on-chain resources
`0x1::keyless_account::Groth16VerificationKey` and `0x1::keyless_account::Configuration`,
which need to be fetched via HTTP requests.

In terminal 0, run the pepper service.
```bash
export VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
export ONCHAIN_GROTH16_VK_URL=http://localhost:4444/groth16_vk.json
export ONCHAIN_KEYLESS_CONFIG_URL=http://localhost:4444/keyless_config.json
cargo run -p aptos-keyless-pepper-service
```

In terminal 1, peek the cached resources, they should currently give 404.
```
curl -v http://localhost:8000/cached/groth16-vk
curl -v http://localhost:8000/cached/keyless-config
```

In terminal 2, mock the full node with a naive HTTP server.
```bash
cd keyless/pepper/service/resources
python3 -m http.server 4444
```

Wait for 10 secs then go back to terminal 1 to retry the curl cmds. The cached data should be available.

TODO: how to generate sample request and interact with `v0/verify` endpoint?
