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
