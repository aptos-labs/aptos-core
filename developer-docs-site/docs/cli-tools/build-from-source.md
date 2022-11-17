---
title: "Build CLI from Source"
id: "build-aptos-cli"
---

# Build Aptos CLI from Source Code

If you are an advanced user and would like to build the CLI binary by downloading the source code, follow the below steps. **This is not recommended** unless you are on a platform unsupported by the prebuilt binaries. Otherwise, [install the prebuilt CLI binaries](aptos-cli-tool/install-aptos-cli.md) to ease ramp up and reduce variables in your environment.

:::tip Use setup script
Aptos offers the [`dev_setup.sh`](https://github.com/aptos-labs/aptos-core/blob/main/scripts/dev_setup.sh) script for establishing your development environment. This script currently supports macOS and Ubuntu Linux with other Linux distributions working but untested. The script does not support Windows. See the instructions below to manually install necessary dependencies on Windows. 
:::

<details>
<summary>macOS</summary>

### macOS
#### Setup dependencies

**> Using the automated script**

1. If on Mac, ensure you have `brew` installed https://brew.sh/
1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
1. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
1. Change directory into `aptos-core`: `cd aptos-core`
1. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh`
1. Update your current shell environment: `source ~/.cargo/env`.

**> Manual installation of dependencies**

If the script above doesn't work for you, you can install these manually, but it's **not recommended**.

1. [Rust](https://www.rust-lang.org/tools/install)
1. [Git](https://git-scm.com/download)
1. [CMake](https://cmake.org/download/)
1. [LLVM](https://releases.llvm.org/)
1. [LLD](https://lld.llvm.org/)

#### Building the Aptos CLI

1. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
1. Build the CLI tool: `cargo build --package aptos --release`.
1. The binary will be available in at
    - `target/release/aptos`
1. (Optional) Move this executable to a place on your path e.g. `~/bin/aptos`.
1. You can now get help instructions by running `~/bin/aptos help`

</details>

<details>
<summary>Linux</summary>

### Linux
#### Setup dependencies

**> Using the automated script**

1. If on Mac, ensure you have `brew` installed https://brew.sh/
1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
1. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
1. Change directory into `aptos-core`: `cd aptos-core`
1. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh`
1. Update your current shell environment: `source ~/.cargo/env`

**> Manual installation of dependencies**

If the script above does not work for you, you can install these manually, but it is **not recommended**:

1. [Rust](https://www.rust-lang.org/tools/install).
1. [Git](https://git-scm.com/download).
1. [CMake](https://cmake.org/download/).
1. [LLVM](https://releases.llvm.org/).

#### Building the Aptos CLI

1. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
1. Build the CLI tool: `cargo build --package aptos --release`.
1. The binary will be available in at
   - `target/release/aptos`
1. (Optional) Move this executable to a place on your path e.g. `~/bin/aptos`.
1. You can now get help instructions by running `~/bin/aptos help`

</details>

<details>
<summary>Windows</summary>

### Windows

#### Setup dependencies

:::tip
The aptos-core codebase currently has no script similar to the `dev_setup.sh` script for
Windows.  All dependencies must be manually installed.
:::

**> Manual installation of dependencies**

If on Windows, you must install these manually:

1. [Rust](https://www.rust-lang.org/tools/install).
1. [Git](https://git-scm.com/download).
1. [CMake](https://cmake.org/download/).
1. For Windows ARM, [Visual Studio Preview](https://visualstudio.microsoft.com/vs/preview/).
1. [C++ build tools for Windows](https://visualstudio.microsoft.com/downloads/#microsoft-visual-c-redistributable-for-visual-studio-2022).
1. [LLVM](https://releases.llvm.org/).

#### Building aptos-core

1. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
1. Build the CLI tool: `cargo build --package aptos --release`.
1. The binary will be available at - `target\release\aptos.exe`
1. You can now get help instructions by running `target\release\aptos.exe` in a Powershell window.

</details>
