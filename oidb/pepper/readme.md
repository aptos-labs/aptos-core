## Quickstart

Start the pepper service in terminal 1.
```bash
VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff cargo run -p aptos-oidb-pepper-service
```
NOTE: `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00` is a dummy VUF private key seed.

Run the example client in terminal 2.
```bash
cargo run -p aptos-oidb-pepper-example-client-rust
```
This is an interactive console program.
Follow the instruction to manually complete a session with the pepper service.

## NOTE for frontend developers
Sorry for the missing examples in other programming languages.
For now please read through `aptos_oidb_pepper_example_client_rust::main()` implementation and output:
that is what your frontend needs to do.
