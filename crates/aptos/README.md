# Aptos Command Line Interface (CLI)

The `aptos` CLI is a command line tool for developing Move smart contracts, interacting with the
Aptos blockchain, and operating Aptos nodes.

This crate builds the `aptos` binary. For conceptual guides and tutorials, see the
[Aptos CLI documentation](https://aptos.dev/build/cli).

## Installation

Prebuilt binaries are the recommended way to install the CLI:

- [macOS](https://aptos.dev/build/cli/install-cli/install-cli-mac)
- [Linux](https://aptos.dev/build/cli/install-cli/install-cli-linux)
- [Windows](https://aptos.dev/build/cli/install-cli/install-cli-windows)
- [asdf](https://aptos.dev/build/cli/install-cli/install-cli-asdf)
- [A specific version](https://aptos.dev/build/cli/install-cli/install-cli-specific-version)

Once installed, update the CLI in place with:

```bash
aptos update aptos
```

To build from this repository instead, see [Building from source](#building-from-source).

## Getting started

Initialize a profile, which stores a key pair and network settings in `.aptos/config.yaml` in the
current directory:

```bash
aptos init
```

Then create and publish a Move package:

```bash
aptos move init --name my_package   # Scaffold a new package
aptos move compile                  # Compile it
aptos move test                     # Run its unit tests
aptos move publish                  # Publish it on chain
```

Run a local network to develop against, instead of a public network:

```bash
aptos node run-localnet
```

See [Setting up the CLI](https://aptos.dev/build/cli/setup-cli) for profile and network
configuration details.

## Command groups

Every command is grouped under a top-level subcommand:

| Group | Purpose |
| --- | --- |
| `account` | Create and fund accounts, inspect resources, transfer APT, and rotate keys |
| `config` | Manage CLI profiles, global settings, and shell completions |
| `genesis` | Set up a genesis transaction for a new chain |
| `governance` | Propose, vote on, and verify on-chain governance proposals |
| `info` | Show build information about the CLI |
| `init` | Initialize the current directory for use with the CLI |
| `key` | Generate and inspect keys, and extract peer information |
| `move` | Compile, test, publish, and interact with Move packages |
| `multisig` | Create and approve transactions for multisig accounts |
| `node` | Operate validators and full nodes, and run a local network |
| `stake` | Manage stake, staking contracts, and delegated voters |
| `update` | Update the CLI and the tools it depends on |

## Discovering commands

The CLI is self-documenting. Append `--help` at any level to list the available subcommands and
arguments:

```bash
aptos --help                # All command groups
aptos move --help           # Commands within a group
aptos move publish --help   # Arguments for a single command
```

Use `-h` for a short summary and `--help` for the full description of each argument.

Shell completions make the commands easier to explore interactively:

```bash
aptos config generate-shell-completions --shell zsh --output-file <path>
```

Supported shells are `bash`, `elvish`, `fish`, `powershell`, and `zsh`.

## Building from source

Build the CLI from a checkout of `aptos-core`:

```bash
cargo build -p aptos            # Debug binary at target/debug/aptos
cargo build -p aptos --release  # Release binary at target/release/aptos
```

Helper scripts for release and minimal builds live in [`scripts/cli`](../../scripts/cli) at the
repository root.

## Further reading

- [`CHANGELOG.md`](CHANGELOG.md) — released versions and their changes
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — design decisions and guidelines for adding commands
- [`e2e/README.md`](e2e/README.md) — the CLI end-to-end test suite
- [`homebrew/README.md`](homebrew/README.md) — the Homebrew formula for the CLI
