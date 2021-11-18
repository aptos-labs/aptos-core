# Shuffle
Welcome to Shuffle! Shuffle is a CLI tool for Move development on the Diem blockchain.

## Setup
From diem directory:

- Install Diem dependencies including Rust, Clang, Deno, etc, by running the following script in `diem` root directory:
```
./scripts/dev_setup.sh
```
- Install shuffle binary
```
cargo install --path shuffle/cli
```
- Install deno package needed to run shuffle console
```
brew install deno
```

## Commands Overview
1. `shuffle new`: Creates a new shuffle project for Move development
2. `shuffle node`: Runs a local devnet
3. `shuffle account`: Creates a private key and creates the corresponding account on-chain
4. `shuffle build`: Compiles the Move package and generates typescript files
5. `shuffle deploy`: Publishes all move modules inside the `/main` directory using the account as publisher
6. `shuffle console`: Starts a REPL for onchain inspection
7. `shuffle test`: Runs end to end .ts tests in the `/e2e` project directory
8. `shuffe transactions`: Prints the last 10 transactions and continuously polls for new transactions from the account
9. `shuffle help`: Prints commands overview or the help of the given subcommand

Note that for local development, `shuffle` is replaced with `cargo run -p shuffle --`:

```bash
shuffle new /tmp/helloblockchain # is replaced by
cargo run -p shuffle -- new /tmp/helloblockchain
```

## Tutorials

To start, follow the [Hello Blockchain](https://github.com/diem/diem/tree/main/shuffle/cli/tutorials/HelloBlockchain.md) tutorial.

If you are a genesis move module developer, follow [Genesis Tutorial](https://github.com/diem/diem/tree/main/shuffle/cli/tutorials/Genesis.md).

## Forge Testing

```
RUST_BACKTRACE=1 cargo xtest -p shuffle-integration-tests
```
