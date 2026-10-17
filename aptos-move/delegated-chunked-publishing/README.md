# Delegated chunked publishing

This is a **standalone** Move package, deployed separately from `0x7::large_packages` at an address supplied to the CLI with `--module-address`. It is not part of the framework genesis bundle and cannot be used with the existing mainnet `large_packages` deployment.

A (the *stager*) pays to upload a local Move package, in position-keyed chunks, and designates B (the *publisher*) when opening A's staging area. Only B's signature can publish the package to B's account or a code object owned by B. B independently compiles the **same** package and artifacts at the destination address. The Move module verifies `SHA3-256(BCS(PackagePreimage { metadata, code }))`, where `metadata` is the serialized package metadata and `code` is the ordered vector of compiled module bytecode. A hash sent by A alone is not approval by B.

## Deploy on a CLI localnet

Start `aptos node run-localnet`, fund a deployer account, and build/deploy from the repository root:

```bash
aptos move publish \
  --package-dir aptos-move/delegated-chunked-publishing \
  --named-addresses delegated_packages="$MODULE_ADDRESS" \
  --sender-account "$MODULE_ADDRESS" --private-key-file "$DEPLOYER_KEY" \
  --url http://127.0.0.1:8080 --assume-yes
```

The module address is **explicit** in every delegated command; there is no network-specific default. Verify that address belongs to a deployment of the reviewed contract before signing: the CLI checks entry names and the module address, **not** the deployed bytecode's trustworthiness. A malicious contract at a supplied address could ignore the package-hash safeguard. Replace the localnet URL with an intended network only after verifying the deployment, its orderless feature, and gas limits.

## A stages; B finalizes

Both parties compile the *same* package with the same named addresses and `--included-artifacts` setting (default: sparse). A and B must agree on those options out of band; B should obtain the source independently. Stage under A's signer, naming B:

```bash
aptos move stage-delegated-package \
  --module-address "$MODULE_ADDRESS" --publisher "$B" \
  --package-dir "$PACKAGE_DIR" --named-addresses "my_module=$B" \
  --private-key-file "$A_KEY" --sender-account "$A" \
  --url http://127.0.0.1:8080 --assume-yes
```

Stage mode is `auto` by default: on a live chain with `ORDERLESS_TRANSACTIONS` enabled it uses independent nonces, otherwise consecutive account sequence numbers. Force either with `--stage-mode nonce` or `--stage-mode seqnum`; forced nonce fails when the chain does not enable it. Feature lookup errors fail rather than silently falling back. `--session` uses sequence numbers in auto mode. All staging transactions must **commit** before B finalizes. Use `--parallelism` (default 10, maximum 100) and `--chunk-size` (maximum 55,000 *raw chunk bytes*) if needed. The serialized transaction has additional BCS overhead.

After staging, B compiles their own local copy and signs exactly one final transaction:

```bash
aptos move finalize-delegated-package \
  --module-address "$MODULE_ADDRESS" --stager "$A" \
  --package-dir "$PACKAGE_DIR" --named-addresses "my_module=$B" \
  --private-key-file "$B_KEY" --sender-account "$B" \
  --url http://127.0.0.1:8080 --assume-yes
```

Both modes keep position IDs, so rerunning the **same** stage command safely skips identical chunks and submits only missing ones. Existing chunks addressed to another publisher, at unexpected positions, or with different bytes are **not** overwritten. B must pass the same `--chunk-size` used by A to verify positions. Before cleanup, check that another run is not still submitting chunks. Only A can explicitly discard A's staged area. Cleanup is bounded to ten chunks per transaction; the CLI repeats transactions until the resource is gone, so a large upload remains recoverable. Each staging area has a random generation ID; both chunk uploads and cleanup check it on-chain, so delayed transactions cannot write into or delete a replacement staging area:

```bash
aptos move cleanup-delegated-staging --module-address "$MODULE_ADDRESS" \
  --private-key-file "$A_KEY" --sender-account "$A" \
  --url http://127.0.0.1:8080 --assume-yes
```

A pays the initial storage deposits. Removal of the chunks during finalization credits the deletion refund to the **final transaction's fee payer** (normally B; for a sponsored final transaction, its sponsor), not automatically to A. Explicit cleanup is signed and paid for by A, so A gets that refund instead. If B never publishes, A can clean up.

## Code object publication and upgrades

To create a code object, A needs the code-object address **before** compiling. B provides an expected object-seed number equal to B's *current on-chain account sequence number plus one*. Derive the corresponding address with `aptos_types::object_address::create_object_code_deployment_address(B, expected_seed)` (or an equivalent SDK helper), and compile `my_module` at that address. Both commands use `--destination new-object --expected-publisher-sequence "$EXPECTED_SEED"` and `--named-addresses "my_module=$OBJECT_ADDRESS"`. Staging under A does not advance B's sequence. B's final transaction uses B's actual current account sequence (`expected_seed - 1`). This is **not a reservation**: if B sends another sequence-numbered transaction in the meantime, the finalize CLI refuses to submit and leaves A's chunks intact; A may clean up and restage for the new address.

For an upgrade, compile both times with `--named-addresses "my_module=$EXISTING_CODE_OBJECT"`, stage with `--destination upgrade --object-address "$EXISTING_CODE_OBJECT"`, and finalize with the same destination and object address. The framework checks that B actually owns the existing object and that the upgrade is compatible. If the framework rejects it, A's staging remains (the Move transaction rolls back) for retry or explicit cleanup.

## Limits and safety

- A names a single B per staging area. It cannot be reassigned without A-signed cleanup; another signer cannot stage or clean it. B's signed final call checks the bound publisher, full-package hash, exact part counts and every required chunk. A failed Move finalization does not consume the staged area.
- The package digest binds **metadata and ordered module bytes**, not just module names, source text or a digest asserted by A. Wrong named-address choices, artifact settings or chunk size cause a client-side mismatch; the contract also checks the assembled bytes on chain.
- Automatic nonce detection reads `0x1::features::Features`. Offline simulation cannot prove support, and production networks may differ. Verify localnet and gas costs for the *actual* package; this repository's small-package localnet tests do not establish a public-chain maximum size or cost.
- `0x7::large_packages` and the previous `aptos move publish --chunked-publish` path are unchanged. The new commands require the separately deployed `delegated_packages` module's ABI.
