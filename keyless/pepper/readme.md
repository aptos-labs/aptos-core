## Dev guide

### Prepare dependency: a firestore instance.

Pepper service now depends on firestore on GCP.
You must have a GCP project in order to run firestore emulator locally
You also need to create a service account, and grant firestore access to it, and download its credential file.
Below we assume the credential file has been saved as `credential.json`.

In terminal 0, start a local firestore emulator.
```bash
gcloud emulators firestore start --host-port=localhost:8081
```

In terminal 1, start the pepper service.
```bash
FIRESTORE_EMULATOR_HOST=localhost:8081 \
  GOOGLE_APPLICATION_CREDENTIALS=credential.json \
  KEYLESS_PROJECT_ID=$(gcloud config get-value project) \
  DATABASE_ID=account-db-devnet \
  ACCOUNT_MANAGER_0_ISSUER=https://accounts.google.com \
  ACCOUNT_MANAGER_0_AUD=407408718192.apps.googleusercontent.com \
  VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
  cargo run -p aptos-keyless-pepper-service
```

Remarks.
- `ACCOUNT_MANAGER_0_ISSUER` and `ACCOUNT_MANAGER_0_AUD` together determines an account manager app which allows [account recovery/discovery](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-61.md#recovery-service).
  - To specify more account managers, give each a short ID `X` and specify envvars `ACCOUNT_MANAGER_X_ISSUER` and `ACCOUNT_MANAGER_X_AUD`.
- `VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00` is a dummy VUF private key seed.
- `GOOGLE_APPLICATION_CREDENTIALS` and `KEYLESS_PROJECT_ID` are required to connect to firestore.
- If `FIRESTORE_EMULATOR_HOST` is specified, pepper service will connect to to specified emulator;
  otherwise (probably the case in production), it connects to the default firestore instance in the GCP project as specified by `KEYLESS_PROJECT_ID`.

Run the example client in terminal 2.
```bash
FIRESTORE_EMULATOR_HOST=localhost:8081 \
  GOOGLE_APPLICATION_CREDENTIALS=credential.json \
  KEYLESS_PROJECT_ID=$(gcloud config get-value project) \
  DATABASE_ID=account-db-devnet \
  cargo run -p aptos-keyless-pepper-example-client-rust
```
This is an interactive console program.
Follow the instruction to manually complete a session with the pepper service.

## NOTE for frontend developers
Sorry for the missing examples in other programming languages.
For now please read through `example-client-rust/src/main.rs` implementation and output:
that is what your frontend needs to do.
