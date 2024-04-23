## Quickstart

Start the pepper service in terminal 1.
```bash
ACCOUNT_MANAGER_0_ISSUER=https://accounts.google.com \
  ACCOUNT_MANAGER_0_AUD=407408718192.apps.googleusercontent.com \
  VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
  cargo run -p aptos-keyless-pepper-service
```
NOTE: `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00` is a dummy VUF private key seed.

Run the example client in terminal 2.
```bash
cargo run -p aptos-keyless-pepper-example-client-rust
```
This is an interactive console program.
Follow the instruction to manually complete a session with the pepper service.

## NOTE for frontend developers
Sorry for the missing examples in other programming languages.
For now please read through `example-client-rust/src/main.rs` implementation and output:
that is what your frontend needs to do.
