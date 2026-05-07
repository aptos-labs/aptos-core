# Run

Subcommands for invoking already-published code on-chain or against a local simulator. Most of them accept the [shared transaction options](./cli.md#transaction-options); only command-specific flags are listed below.

## `aptos move run`

Call an entry function as a transaction.

```shellscript filename="Terminal"
aptos move run \
  --function-id 0x42::counter::increment \
  --args address:0xABC u64:1 \
  --profile devnet
```

| Flag | Meaning |
|---|---|
| `--function-id <ADDR::MOD::FN>` | Fully qualified entry function name. |
| `--type-args <TYPE>...` | Type arguments separated by spaces, e.g., `--type-args 0x1::aptos_coin::AptosCoin`. |
| `--args <TYPED-ARG>...` | Arguments as `<TYPE>:<VALUE>` pairs, separated by spaces. Supported types: `address`, `bool`, `hex`, `string`, `u8`, `u16`, `u32`, `u64`, `u128`, `u256`, `raw`. Vectors use JSON array syntax: `'u64:[1,2,3]'`. |
| `--json-file <PATH>` | Read function id, type args, and args from a JSON file instead of the command line. |

## `aptos move run-script`

Run a transaction script. Either pass a precompiled `.mv` (`--compiled-script-path`) or a source file the CLI will compile first (`--script-path`, or via a script package's `--package-dir`).

```shellscript filename="Terminal"
# Compile + run a script package
aptos move run-script \
  --package-dir my_script_pkg \
  --args u64:42

# Run a precompiled script
aptos move run-script \
  --compiled-script-path build/MyPkg/bytecode_scripts/my_script.mv \
  --args u64:42
```

| Flag | Meaning |
|---|---|
| `--script-path <PATH>` | Path to a `.move` script source. The CLI compiles it before submitting. |
| `--compiled-script-path <PATH>` | Path to a pre-compiled `.mv` script. |
| `--package-dir <PATH>` | Compile a script package, then run it. |
| `--args <TYPED-ARG>...` / `--type-args <TYPE>...` | Same shape as `aptos move run`. |

_See also: [Running Move Scripts](https://aptos.dev/build/smart-contracts/scripts/running-scripts)._

## `aptos move view`

Call a `#[view]` function. View functions don't change state and don't submit a transaction; the result is read directly from the fullnode.

```shellscript filename="Terminal"
aptos move view \
  --function-id 0x42::counter::get_count \
  --args address:0xABC \
  --profile devnet
```

Same arg/type-arg shape as `aptos move run`. The output is a JSON array of return values.

## `aptos move simulate`

Simulate a transaction without committing it. Useful for estimating gas, checking aborts, and previewing effects.

```shellscript filename="Terminal"
aptos move simulate \
  --function-id 0x42::counter::increment \
  --args address:0xABC u64:1 \
  --profile devnet

aptos move simulate \
  --function-id 0x42::counter::increment \
  --args address:0xABC u64:1 \
  --local                                # local VM instead of remote simulation
```

| Flag | Meaning |
|---|---|
| `--local` | Simulate in a local VM (using the latest state pulled from the network), instead of asking the fullnode to simulate. |
| Plus the same `--function-id` / `--type-args` / `--args` shape as `run`. | |

## `aptos move replay`

Re-execute a historical transaction in a local VM. Lets you reproduce, benchmark, and gas-profile committed transactions.

```shellscript filename="Terminal"
aptos move replay --network mainnet --txn-id 12345678
aptos move replay --network testnet --txn-id 87654321 --profile-gas
```

| Flag | Meaning |
|---|---|
| `--network <NET>` | One of `mainnet`, `testnet`, `devnet`, or a `<URL>` to a fullnode. |
| `--txn-id <N>` | Transaction "version" (the monotonically increasing sequence; sometimes called the txn id). |
| `--benchmark` | Time the replay and report wall-clock numbers. |
| `--profile-gas` | Generate a gas-usage report from the replay. |
| `--fold-unique-stack` | Fold call graphs by stack trace in the gas report (smaller output). Requires `--profile-gas`. |

_See also: [Replaying Past Transactions](https://aptos.dev/build/cli/replay-past-transactions)._

## `aptos move list`

List on-chain packages owned by an account.

```shellscript filename="Terminal"
aptos move list --account 0xABC...123 --profile devnet
```

| Flag | Meaning |
|---|---|
| `--account <ADDR>` | Address whose packages to list. |
| `--query <packages>` | Currently only `packages` is supported. |

The output enumerates each package with its upgrade policy, upgrade number, source digest, and constituent module names.

## `aptos move download`

Download a published package's source (or bytecode) from a network into a local directory. Pair with [`disassemble`](./cli-develop.md#aptos-move-disassemble) or [`decompile`](./cli-develop.md#aptos-move-decompile) for inspection.

```shellscript filename="Terminal"
aptos move download \
  --account 0xABC...123 \
  --package MyPkg \
  --output-dir ./downloads \
  --profile mainnet

# Bytecode-only download (use with disassemble/decompile)
aptos move download --account 0xABC --package MyPkg --bytecode
```

| Flag | Meaning |
|---|---|
| `--account <ADDR>` | Account that hosts the package. |
| `--package <NAME>` | Package name as registered on-chain. |
| `--output-dir <PATH>` | Where to write the package. Defaults to the current directory. |
| `--bytecode` / `-b` | Also download the compiled `.mv` files (not just sources). |
| `--print-metadata` | Print the package's on-chain metadata before saving. |

A package published with `upgrade_policy = "arbitrary"` cannot be downloaded — depending on it isn't safe.
