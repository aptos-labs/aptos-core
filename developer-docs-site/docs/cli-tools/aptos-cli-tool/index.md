---
title: "Install the Aptos CLI"
id: "install-cli"
---

# Install the Aptos CLI

The Aptos command line interface (CLI) helps you develop apps, debug issues, and operate nodes. See [Use Aptos CLI for Configuration](./use-aptos-cli.md) for all possible uses.

You may install the Aptos CLI in any of these ways:

* [Install the Aptos CLI with Homebrew](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/homebrew/README.md) - Recommended for macOS.
* [Install the Aptos CLI by script](./automated-install-aptos-cli.md) - Recommended for Linux, Windows (NT), and Windows (WSL). MacOS is also supported.

These methods are recommended only if you have issues with the above methods:

* [Download the prebuilt Aptos CLI binaries](./install-aptos-cli.md) - Ensures you get a stable version of the Aptos CLI built on a regular cadence from the `main` upstream development branch.
* [Build the Aptos CLI from source code](../build-from-source.md) - Allows you to build from any of the Aptos branches, including `devnet`, `testnet`, `mainnet`, and the latest code in the `main` upstream development branch.

## (Optional) Installing the Move Prover
Optionally, you can [install the Move Prover](../install-move-prover.md); however, most users will not need it and it is
not supported on all platforms.