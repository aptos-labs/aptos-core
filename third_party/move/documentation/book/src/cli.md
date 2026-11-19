# CLI Overview

The `aptos` command-line tool ships every developer-facing utility for working with Move on Aptos. Compiling, testing, publishing, calling functions, debugging — they all live under `aptos move`.

This chapter covers the flags that appear across many `aptos move` subcommands. The chapters that follow describe the individual subcommands and only call out the flags that are specific to each.

For the authoritative list of flags on any subcommand, run:

```shellscript filename="Terminal"
aptos move <subcommand> --help
```

## A typical Move project flow

Most developers move through these subcommands in roughly this order:

1. `aptos move init` — scaffold a new package.
2. `aptos move compile` — compile sources to bytecode.
3. `aptos move test` — run unit tests; add `--coverage` to record line coverage.
4. `aptos move lint` and `aptos move fmt` — style and quality gates before commit.
5. `aptos move publish` (or `deploy-object` for code objects) — push to a network.
6. `aptos move run`, `view`, or `simulate` — invoke the published code.

## Package options

The flags below come from `MovePackageOptions` and are accepted by most package-related subcommands: `compile`, `compile-script`, `test`, `lint`, `fmt`, `prove`, `document`, `clean`, `publish`, `deploy-object`, `upgrade-object`, `verify-package`, `disassemble`, and `decompile`.

| Flag | Meaning |
|---|---|
| `--package-dir <PATH>` | Root of the Move package (the directory containing `Move.toml`). Defaults to the current directory. |
| `--output-dir <PATH>` | Where to write build artifacts. Defaults to `<package-dir>/build`. |
| `--named-addresses <NAME=ADDR,...>` | Bind named addresses declared in `Move.toml`. Example: `--named-addresses alice=0x1234,bob=0x5678`. |
| `--dev` | Use `[dev-addresses]` and `[dev-dependencies]` from `Move.toml`. Implied during `test`. |
| `--override-std <NETWORK>` | Pin the standard library version to a specific network: `mainnet`, `testnet`, or `devnet`. Useful when third-party dependencies disagree on framework versions. |
| `--skip-fetch-latest-git-deps` | Don't refresh git dependencies. Lets the build proceed offline. |
| `--language-version <X>` | Target Move language version (e.g., `2.4`). Defaults to the latest stable. |
| `--compiler-version <X>` | Target compiler version. Must be at least 2. Defaults to the latest stable. |
| `--bytecode-version <N>` | Target bytecode version. Inferred from `--language-version` if omitted. |
| `--skip-attribute-checks` | Don't error on unknown `#[...]` attributes in source. |

## Transaction options

The flags below are accepted by every subcommand that submits a transaction: `publish`, `deploy-object`, `upgrade-object`, `run`, `run-script`, `view`, `simulate`.

| Flag | Meaning |
|---|---|
| `--profile <NAME>` | Use a named profile from `~/.aptos/config.yaml` (account address, key, network URL). Profiles are created with `aptos init`. |
| `--url <URL>` | Override the REST endpoint from the profile. |
| `--sender-account <ADDR>` | Override the sender address (useful when the authentication key has been rotated). |
| `--max-gas <N>` | Cap the transaction's gas. |
| `--gas-unit-price <N>` | Set the gas unit price in Octa. |
| `--assume-yes` / `-y` | Skip interactive confirmations. |
| `--local` | Simulate the transaction locally instead of submitting it. |
| `--profile-gas` | Locally simulate and emit a gas-usage flamegraph. |
| `--session <PATH>` | Execute or dry-run against a local simulation session directory (see `aptos move sim`). |
| `--unauthenticated` | With `--session`, build a `NoAccountAuthenticator` transaction and run it through the simulation VM. Requires `--sender-account`. Cannot be submitted on-chain. |
| `--sponsor-gas` | With `--session` (and `--unauthenticated` for `run` / `run-script`), use fee payer `@0x0` to skip charging gas from the sender. Conflicts with `--fee-payer-account`. |
| `--fee-payer-account <ADDR>` | With `--session` (and `--unauthenticated` for `run` / `run-script`), charge gas to this account instead of the sender. No key needed. |
| `--secondary-signer-accounts <ADDR>...` | With `--session` (and `--unauthenticated` for `run` / `run-script`), extra signers for multi-agent entry functions or scripts. No keys needed. |

## Output and exit code

`aptos move` emits JSON to stdout on success and a non-zero exit code on failure. Most subcommands accept the standard Aptos CLI flags `--output-yaml` (human-readable YAML) and `--output-json` (default).
