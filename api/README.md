# Aptos Node API

This module provides a REST API for client applications to query the Aptos blockchain.

See spec source:
- [YAML in doc/spec.yaml](doc/spec.yaml).
- [JSON in doc/spec.json](doc/spec.json).
- [HTML in doc/spec.html](doc/spec.html).

## Regenerating docs / code based on API changes
With our API setup, the spec files (`api/doc/spec.yaml` / `api/doc/spec.json`) are generated from the API in code, and the TS SDK client (`ecosystem/typescript/sdk`) is generated from that spec. We have CI that ensures that all of these are updated together. As such, if you want to make a change to the API, do it in this order.

![API + spec + TS SDK generation diagram](doc/api_spec_ts_sdk_diagram.png)

This process updates the docs at:
- https://fullnode.devnet.aptoslabs.com/v1/spec#/ (and testnet / mainnet, based on the API rollout schedule)
- https://aptos-labs.github.io/ts-sdk-doc/

All commands here are relative to the root of `aptos-core`.

1. Make your changes to the API code, i.e. the code in `api/src/`.
2. Regenerate the API spec `.yaml` and `.json` files by running these commands from the root of `aptos-core`:
```
cargo run -p aptos-openapi-spec-generator -- -f yaml -o api/doc/spec.yaml
cargo run -p aptos-openapi-spec-generator -- -f json -o api/doc/spec.json
```
3. Regenerate the TypeScript SDK client files based upon the new API spec:
```
cd ecosystem/typescript/sdk
pnpm install
pnpm generate-client
```
4. Manually update the helper methods in the TypeScript SDK in: `ecosystem/typescript/sdk/src/aptos_client.ts`. Note: This is necessary because we wrap the generated client, so the docs on the methods in that file are written by hand. For example, if you change `/accounts/<addr>/resources` in the API, the `getAccountResources` method in the generated client will be different. You must therefore then change `getAccountResources` in `ecosystem/typescript/sdk/src/aptos_client.ts`, which wraps the generated method.
5. Update the TS SDK docs site (https://aptos-labs.github.io/ts-sdk-doc/):
```
pnpm generate-ts-docs
```

### Sanity checks
Double check that the spec looks good by running these commands and then visit http://127.0.0.1:8888/spec.html.
```
cd api/
make serve
```

