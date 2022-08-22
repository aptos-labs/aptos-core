---
title: "Install Aptos CLI"
id: "install-aptos-cli"
---

# Install Aptos CLI

The `aptos` tool is a command line interface (CLI) for debugging, development, and node operations.
This document describes how to install the `aptos` CLI tool. See [Use Aptos CLI](use-aptos-cli) for how to use the CLI.

## Install precompiled binary (easy mode)

1. Navigate to the [release page](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true) for Aptos CLI.
2. From the latest release section, download the binary zip file for your platform. The binary zip files contain the platform name in the filename.
3. Unzip the downloaded file. This will extract the `aptos` CLI tool.
4. Place this extracted `aptos` file at a location for you to run it. For example, place it in `~/bin/aptos` in Linux.
5. On Linux and Mac, make this `~/bin/aptos` as an executable by running this command: `chmod +x ~/bin/aptos`.
6. Type `~/bin/aptos help` to read help instructions.
7. Add `~/bin` to your path in your appropriate `.bashrc` or `.zshrc` for future use.

## Install with cargo

### Step 1: Install cargo

You will need the `cargo` package manager to install the `aptos` CLI tool.  Follow the below steps.

1. Follow the `cargo` [installation instructions on this page](https://doc.rust-lang.org/cargo/getting-started/installation.html)
   and install `cargo`.  Proceed only after you successfully install `cargo`.
2. Execute the below step to ensure that your current shell environment knows where `cargo` is.
```bash
source $HOME/.cargo/env
```

### Step 2: Install the `aptos` CLI

1. Install dependencies before compiling:
   1. For Debian or Ubuntu: `sudo apt install build-essential pkg-config openssl libssl-dev libclang-dev`.
   2. For RHEL or Centos: `sudo yum install pkgconfig openssl openssl-devel clang`.
   3. For others: Manually install `pkg-config` `openssl`, `libssl` and `libclang`:
      - `pkg-config`:
         - Download and unzip the source code at https://pkgconfig.freedesktop.org/releases/
         - `./configure --prefix=/usr/local/pkg_config/0_29_2 --with-internal-glib`
         - `sudo make && sudo make install`
      - `openssl` and `libssl`:
         - Check https://wiki.openssl.org/index.php/Compilation_and_Installation for full instructions.
      - `libclang`:
         - Check https://clang.llvm.org/get_started.html for full instructions.
2. Install the `aptos` CLI tool by running the below command.  **For AIT-3 use `testnet` instead of `devnet`.** You can run this command from any directory.  The `aptos` CLI tool will be installed into your `CARGO_HOME`, usually `~/.cargo`:
```bash
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos --branch devnet
```
3. Confirm that the `aptos` CLI tool is installed successfully by running the below command.  The terminal will display
   the path to the `aptos` CLI's location.
```bash
which aptos
```
