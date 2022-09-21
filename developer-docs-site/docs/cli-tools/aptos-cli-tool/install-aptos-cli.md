---
title: "Installing Aptos CLI"
id: "install-aptos-cli"
---

# Installing Aptos CLI

The `aptos` tool is a command line interface (CLI) for developing on the Aptos blockchain, for debugging Move contracts, and for node operations. This document describes how to install the `aptos` CLI tool. See [Use Aptos CLI](use-aptos-cli) for how to use the CLI.

Install the CLI by downloading the precompiled binary for your platform, as described below. 

:::tip Move Prover Dependencies
If you want to use the Move Prover, then, [install the Move Prover dependencies](#optional-install-the-dependencies-of-move-prover) after installing the CLI binary. 
:::

## Download precompiled binary

1. Go to the [Aptos CLI release page](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true).
2. In the latest release section, you will see the zip files with the filename of the format: `aptos-cli-<version>-<platform>`. These are the platform-specific pre-compiled binaries of the CLI. Download the zip file for your platform.
3. Unzip the downloaded file. This will extract the `aptos` CLI binary file into your default downloads folder. For example, on MacOS it is the `~/Downloads` folder.
4. Move this extracted `aptos` binary file into your preferred local folder. For example, place it in `~/bin/aptos` folder on Linux or MacOS.
:::tip Upgrading? Remember to look in the default download folder
When you update the CLI binary with the latest version, note that the newer version binary will be downloaded to your default Downloads folder. Remember to move this newer version binary from the Downloads folder to `~/bin/aptos` folder (overwriting the older version).
:::
1. Make this `~/bin/aptos` as an executable by running this command: 
   - On Linux and MacOS: `chmod +x ~/bin/aptos`.
   - On MacOS when you attempt to run the `aptos` tool for the first time, you will be blocked by the MacOS that this app is from an "unknown developer". This is normal. Follow the simple steps recommended by the Apple support in [Open a Mac app from an unidentified developer](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac) to remove this blocker. 
2. Type `~/bin/aptos help` to read help instructions.
3. Add `~/bin` to your path in your `.bashrc` or `.zshrc` file for future use.

## (Optional) Install the dependencies of Move Prover

If you want to use the Move Prover, install the dependencies by following the below steps:

:::tip Windows is not supported
The Move Prover is not supported on Windows.
:::

1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
2. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
3. Change directory into the `aptos-core` directory: `cd aptos-core`.
4. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`.
5. Source the profile file: `source ~/.profile`.
    :::info
    Note that you have to include environment variable definitions in `~/.profile` into your shell. Depending on your setup, the  `~/.profile` may be already automatically loaded for each login shell, or it may not. If not, you may 
    need to add `. ~/.profile` to your `~/.bash_profile` or other shell configuration manually.
    :::
6. You can now run the Move Prover to prove an example:
    ```bash
    aptos move prove --package-dir aptos-move/move-examples/hello_prover/
    ```

<details>
<summary>(Advanced users only) Build the CLI binary from the source</summary>

If you are an advanced user and would like to build the CLI binary by downloading the source code, follow the below steps. **This is not recommended.**

1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
2. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
3. Change directory into the `aptos-core` directory: `cd aptos-core`.
4. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh`.
5. Update your current shell environment: `source ~/.cargo/env`.
6. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
7. Build the CLI tool: `cargo build --package aptos --release`.
8. The binary will be available in `target/release/aptos` folder.
9. (Optional) Move this executable to a place on your path e.g. `~/bin/aptos`.

</details>