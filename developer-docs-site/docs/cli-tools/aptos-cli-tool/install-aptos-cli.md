---
title: "Install Aptos CLI"
id: "install-aptos-cli"
---

# Install Aptos CLI

The `aptos` tool is a command line interface (CLI) for debugging, development, and node operations. This document describes how to install the `aptos` CLI tool. See [Use Aptos CLI](use-aptos-cli) for how to use the CLI.

## Install precompiled binary

1. Navigate to the [release page](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true) for Aptos CLI.
2. From the latest release section, download the binary zip file for your platform. The binary zip files contain the platform name in the filename.
3. Unzip the downloaded file. This will extract the `aptos` CLI tool.
4. Place this extracted `aptos` file at a location for you to run it. For example, place it in `~/bin/aptos` in Linux.
5. On Linux and Mac, make this `~/bin/aptos` as an executable by running this command: `chmod +x ~/bin/aptos`.
6. Type `~/bin/aptos help` to read help instructions.
7. Add `~/bin` to your path in your appropriate `.bashrc` or `.zshrc` for future use.

### Step 2 (optional): Install the dependencies of Move Prover

1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git
2. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`
3. Change directory into `aptos-core` directory: `cd aptos-core`
4. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`
5. Source the profile file `source ~/.profile`

This command should work on MacOS and Linux flavors like Ubuntu or CentOS. (Windows is currently not supported).

Notice that you have to include environment variable definitions in `~/.profile` into your shell. Depending on your
setup, the  `~/.profile` may be already automatically loaded for each login shell, or it may not. If not, you may
need to add `. ~/.profile` to your `~/.bash_profile` or other shell configuration manually.

6. You should now be able to prove an example
```bash
aptos move prove --package-dir aptos-move/move-examples/hello_prover/
```

## Install from Git
### Step 1: Install from Git

Start by cloning the `aptos-core` GitHub repo from [GitHub](https://github.com/aptos-labs/aptos-core).

1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git
2. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`
3. Change directory into `aptos-core` directory: `cd aptos-core`
4. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh`
5. Update your current shell environment: `source ~/.cargo/env`
6. Checkout the correct branch `git checkout --track origin/branch`, where branch is
    - `devnet` for building on Devnet
    - `testnet` for building on Testnet
    - `main` for the current development branch
7. Build the CLI tool: `cargo build --package aptos --release`
8. The binary will be available in `target/release/aptos`
9. (Optional) Move this executable to a place on your path e.g. `~/bin/aptos`

### Step 2 (optional): Install the dependencies of Move Prover

1. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`
2. Source the profile file `source ~/.profile`

This command should work on MacOS and Linux flavors like Ubuntu or CentOS. (Windows is currently not supported).

Notice that you have to include environment variable definitions in `~/.profile` into your shell. Depending on your
setup, the  `~/.profile` may be already automatically loaded for each login shell, or it may not. If not, you may
need to add `. ~/.profile` to your `~/.bash_profile` or other shell configuration manually.

3. You should now be able to prove an example
```bash
aptos move prove --package-dir aptos-move/move-examples/hello_prover/
```