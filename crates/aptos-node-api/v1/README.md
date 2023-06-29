# Aptos Node API v1

This code provides a REST API for client applications to query the Aptos blockchain.

See spec source:
- [YAML in doc/spec.yaml](doc/spec.yaml).
- [JSON in doc/spec.json](doc/spec.json).
- [HTML in doc/spec.html](doc/spec.html).

## Regenerating docs / code based on API changes
With our API setup, the spec files (`doc/spec.yaml` / `doc/spec.json`) are generated from the API in code. We have CI that ensures that all of these are updated together. As such, if you want to make a change to the API, do it in this order.

This process updates the docs at:
- https://fullnode.devnet.aptoslabs.com/v1/spec#/ (and testnet / mainnet, based on the API rollout schedule)

All commands here are relative to the root of `aptos-core`.

1. Make your changes to the API code, i.e. the code in `api/src/`.
2. Regenerate the API spec `.yaml` and `.json` files by running these commands from the root of `aptos-core`:
```
cargo run -p aptos-node-api-v1-spec-generator -- -f yaml -o crates/aptos-node-api/v1/doc/spec.yaml
cargo run -p aptos-node-api-v1-spec-generator -- -f json -o crates/aptos-node-api/v1/doc/spec.json
```


