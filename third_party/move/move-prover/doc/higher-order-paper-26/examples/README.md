# Higher-Order Function Verification Examples

This directory is a self-contained Move package containing the examples from the
paper *Formal Verification of Imperative Higher-Order Functions*. The steps below
walk you through installing the Aptos CLI, installing the Move Prover
dependencies, and running the prover on these examples.

## 1. Install the Aptos CLI

The Aptos CLI bundles the Move Prover. Install it in **one** of the following
ways.

### Option A — Download a prebuilt binary (recommended)

Download the binary for your platform from the
[Aptos CLI v9.5.0 release](https://github.com/aptos-labs/aptos-core/releases/tag/aptos-cli-v9.5.0),
or follow the [official installation guide](https://aptos.dev/en/build/cli). Then
confirm it is on your `PATH`:

```shell
aptos --version
```

### Option B — Build from source

1. Install [Rust and Cargo](https://doc.rust-lang.org/cargo/).

2. Clone the repository and check out the pinned commit:

   ```shell
   git clone https://github.com/aptos-labs/aptos-core.git
   cd aptos-core
   git checkout fde503186ef74658abd2c66532a8602eca33a20d
   ```

3. Build the CLI:

   ```shell
   cargo build --package aptos --profile cli
   ```

   The binary is produced at `target/cli/aptos`. Add it to your `PATH`, or use
   the full path in place of `aptos` in the commands below.

## 2. Install the Move Prover dependencies

The prover uses the [Boogie](https://github.com/boogie-org/boogie) verifier and
the [Z3](https://github.com/Z3Prover/z3) SMT solver. The CLI installs them for
you:

```shell
aptos update prover-dependencies
```

## 3. Run the examples

From this directory (`.../examples`), run the prover on the package. Language
version 2.4 or later is required, since the specifications use `@`-state labels
and state quantification:

```shell
aptos move prove --dev --language-version=2.4
```

The prover verifies the specifications under `sources/` and prints the results.

## Learn more

- [Move on Aptos Book](https://aptos-labs.github.io/move-book) — the Move language.
- [Aptos CLI documentation](https://aptos.dev/en/build/cli) — installing and using the CLI.
