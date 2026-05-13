# Publish

Subcommands that publish a Move package to a network. All of them accept the [shared package options](./cli.md#package-options) and [shared transaction options](./cli.md#transaction-options); only the command-specific flags are listed below.

## Picking a publication pattern

Two patterns are supported side by side; pick based on how you want the package's address to behave:

- **Account publishing** (`publish` / `deploy`): the modules live at the **signer's account address**. Simplest pattern. Upgrades happen by re-publishing from the same account. The signer owns upgrade authority and can't transfer it.
- **Code-object publishing** (`deploy-object` / `upgrade-object`): the modules live at a **separate, derived object address**. Upgrade authority is held in a code object and can be transferred. Use this when modules need an address independent of any single account.

For code review and reproducibility, [`verify-package`](#aptos-move-verify-package) checks that on-chain bytecode matches a local source tree.

## `aptos move publish` (alias: `deploy`)

Publish a package to the signer's account.

```shellscript filename="Terminal"
aptos move publish --profile devnet
aptos move publish --named-addresses example=0x42 --included-artifacts sparse
```

| Flag | Meaning |
|---|---|
| `--included-artifacts <none\|sparse\|all>` | What to embed in the on-chain package metadata (drives gas cost). Default `sparse`. |
| `--override-size-check` | Skip the local size pre-check. Doesn't bypass on-chain size limits. |
| `--chunked-publish` | Publish in chunks via the `large_packages` framework module. Use for packages exceeding the single-transaction size limit. |
| `--chunk-size <BYTES>` | Override chunk size for chunked publishing. |

Re-publishing this command upgrades the package in place, subject to the [package upgrade rules](./modules-and-packages.md).

_See also: [Working With Move Contracts](https://aptos.dev/build/cli/working-with-move-contracts), [Package Upgrades](./modules-and-packages.md)._

## `aptos move deploy-object`

Publish the package as a **code object** at a freshly derived object address. The chosen named address (passed by `--address-name`) is auto-bound to that object's address before compile.

```shellscript filename="Terminal"
aptos move deploy-object \
  --address-name example \
  --profile devnet
```

| Flag | Meaning |
|---|---|
| `--address-name <NAME>` | The named address in `Move.toml` whose binding will be set to the new object's address. |
| `--included-artifacts`, `--override-size-check`, `--chunked-publish`, `--chunk-size` | Same as `publish`. |

_See also: [Object Code Deployment](https://aptos.dev/build/smart-contracts/deployment), [Package Upgrades](./modules-and-packages.md)._

## `aptos move upgrade-object`

Upgrade a package previously published with `deploy-object`. The original `--address-name` is rebound to the existing object address (passed via `--object-address`) so the rebuild produces the same module ids. The new bytecode must satisfy the [package upgrade rules](./modules-and-packages.md) against the version currently on-chain.

```shellscript filename="Terminal"
aptos move upgrade-object \
  --address-name example \
  --object-address 0xABC...123 \
  --profile devnet
```

| Flag | Meaning |
|---|---|
| `--address-name <NAME>` | Same named address used at deploy time. |
| `--object-address <ADDR>` | Address of the existing code object. |
| `--included-artifacts`, `--override-size-check`, `--chunked-publish`, `--chunk-size` | Same as `publish`. |

_See also: [Object Code Deployment](https://aptos.dev/build/smart-contracts/deployment), [Package Upgrades](./modules-and-packages.md)._

## `aptos move verify-package`

Build the package locally and verify that the on-chain copy matches.

```shellscript filename="Terminal"
aptos move verify-package --account 0xABC...123
```

| Flag | Meaning |
|---|---|
| `--account <ADDR>` | Address of the on-chain package to verify against. |
| `--included-artifacts <none\|sparse\|all>` | Match what was used at publish time. |

A package published with `upgrade_policy = "arbitrary"` cannot be verified — its content can change at any time, so the verifier refuses to depend on it.

## `aptos move build-publish-payload`

Compile the package and write a serialized publish-transaction payload to a JSON file. Used for offline signing, governance proposals, and chunked-publish workflows.

```shellscript filename="Terminal"
aptos move build-publish-payload \
  --json-output-file payload.json \
  --included-artifacts sparse
```

| Flag | Meaning |
|---|---|
| `--json-output-file <PATH>` | Where to write the payload JSON. |
| Plus all `publish` flags. | |

The resulting JSON can be submitted later with `aptos governance` or signed offline.

## `aptos move clear-staging-area`

Remove the staging-area resource left on-chain by an aborted chunked publish. Run this when a `--chunked-publish` flow fails partway and you want to start over.

```shellscript filename="Terminal"
aptos move clear-staging-area --profile devnet
```

No command-specific flags beyond the [transaction options](./cli.md#transaction-options).
