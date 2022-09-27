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
<details>
<summary>MacOS</summary>

### MacOS
:::tip
These instructions have been tested on MacOS Monterey (12.6)
:::


1. Go to the [Aptos CLI release page](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true).
2. In the latest release section, you will see the zip files with the filename of the format: `aptos-cli-<version>-<platform>`. These are the platform-specific pre-compiled binaries of the CLI. Download the zip file for your platform.
3. Unzip the downloaded file. This will extract the `aptos` CLI binary file into your default downloads folder. For example, on MacOS it is the `~/Downloads` folder.
4. Move this extracted `aptos` binary file into your preferred local folder. For example, place it in `~/bin/aptos` folder on MacOS.

:::tip Upgrading? Remember to look in the default download folder
When you update the CLI binary with the latest version, note that the newer version binary will be downloaded to your default Downloads folder. Remember to move this newer version binary from the Downloads folder to `~/bin/aptos` folder (overwriting the older version).
:::

5. Make this `~/bin/aptos` as an executable by running this command: 
   - `chmod +x ~/bin/aptos`.
   - On MacOS when you attempt to run the `aptos` tool for the first time, you will be blocked by the MacOS that this app is from an "unknown developer". This is normal. Follow the simple steps recommended by the Apple support in [Open a Mac app from an unidentified developer](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac) to remove this blocker.
6. Type `~/bin/aptos help` to read help instructions.
7. Add `~/bin` to your path in your `.bashrc` or `.zshrc` file for future use.

</details>

<details>
<summary>Linux</summary>

### Linux
:::tip
These instructions have been tested on Ubuntu 20.04.
:::

1. Go to the [Aptos CLI release page](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true).
2. In the latest release section, you will see the zip files with the filename of the format: `aptos-cli-<version>-<platform>`. These are the platform-specific pre-compiled binaries of the CLI. Download the zip file for your platform.
3. Unzip the downloaded file. This will extract the `aptos` CLI binary file into your default downloads folder. 
4. Move this extracted `aptos` binary file into your preferred local folder. 

   :::tip Upgrading? Remember to look in the default download folder
   When you update the CLI binary with the latest version, note that the newer version binary will be downloaded to your default Downloads folder. Remember to move this newer version binary from the Downloads folder to `~/bin/aptos` folder (overwriting the older version).
   :::

5. Make this `~/bin/aptos` as an executable by running this command:
    - `chmod +x ~/bin/aptos`.
6. Type `~/bin/aptos help` to read help instructions.
7. Add `~/bin` to your path in your `.bashrc` or `.zshrc` file for future use.

</details>

<details>
<summary>Windows 10, 11 and Windows Server 2022+</summary>

### Windows 10, 11 and Windows Server 2022+

:::tip
These instructions have been tested on Windows 11 and Windows Server 2022. Windows support is new and some features may be not complete. Open [Github issues](https://github.com/aptos-labs/aptos-core/issues) for bugs.
:::

1. Go to the [Aptos CLI release page](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true).
2. In the latest release section, you will see the zip files with the filename of the format: `aptos-cli-<version>-<platform>`. These are the platform-specific pre-compiled binaries of the CLI. Download the zip file for your platform.
3. Unzip the downloaded file. This will extract the `aptos` CLI binary file into your default downloads folder. For example, on Windows it is the `\Users\user\Downloads` folder.
4. Move this extracted `aptos` binary file into your preferred local folder.
   :::tip Upgrading? Remember to look in the default download folder
   When you update the CLI binary with the latest version, note that the newer version binary will be downloaded to your default Downloads folder. Remember to move this newer version binary from the Downloads folder to your preferred location.
   :::
5. Open a powershell terminal via the windows start menu
6. In the powershell terminal, you can get help instructions by running the command with help.  For example ` .\Downloads\aptos-cli-0.3.5-Windows-x86_64\aptos.exe help` to read help instructions.

</details>

## (Optional) Install the dependencies of Move Prover

If you want to use the Move Prover, install the dependencies by following the below steps:

:::tip
Currently Windows is not supported for the prover.
:::

<details>
<summary>Prover MacOS installation</summary>

### MacOS

:::tip
These instructions have been tested on MacOS Monterey (12.6)
:::

1. Ensure you have `brew` installed https://brew.sh/.
2. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
3. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
4. Change directory into the `aptos-core` directory: `cd aptos-core`.
5. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`.
6. Source the profile file: `source ~/.profile`.

   :::info
   Note that you have to include environment variable definitions in `~/.profile` into your shell. Depending on your setup, the  `~/.profile` may be already automatically loaded for each login shell, or it may not. If not, you may
   need to add `. ~/.profile` to your `~/.bash_profile` or other shell configuration manually.
   :::

7. You can now run the Move Prover to prove an example:
    ```bash
    aptos move prove --package-dir aptos-move/move-examples/hello_prover/
    ```
   
</details>

<details>
<summary>Prover Linux installation</summary>

### Linux

:::tip 
Some Linux distributions are not supported. Currently, OpenSUSE and Amazon Linux do not support the automatic installation via the `dev_setup.sh` script.
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

</details>

## (Advanced users only) Build the CLI binary from the source code

If you are an advanced user and would like to build the CLI binary by downloading the source code, follow the below steps. **This is not recommended** unless you are on a platform unsupported by the prebuilt binaries.

<details>
<summary>MacOS</summary>

### MacOS
#### Setup dependencies

**> Using the automated script**

1. If on Mac, ensure you have `brew` installed https://brew.sh/
2. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
3. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
4. Change directory into the `aptos-core` directory: `cd aptos-core`.
5. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh`.
6. Update your current shell environment: `source ~/.cargo/env`.

**> Manual installation of dependencies**

If the script above doesn't work for you, you can install these manually, but it's **not recommended**.

1. Install [Rust](https://www.rust-lang.org/tools/install)
2. Install [Git](https://git-scm.com/download)
3. Install [CMake](https://cmake.org/download/)
4. Install [LLVM](https://releases.llvm.org/)

#### Building the Aptos CLI

1. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
2. Build the CLI tool: `cargo build --package aptos --release`.
3. The binary will be available in at
    - `target/release/aptos`
4. (Optional) Move this executable to a place on your path e.g. `~/bin/aptos`.
5. You can now get help instructions by running `~/bin/aptos help`

</details>

<details>
<summary>Linux</summary>

### Linux
#### Setup dependencies

**> Using the automated script**

1. If on Mac, ensure you have `brew` installed https://brew.sh/
2. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
3. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
4. Change directory into the `aptos-core` directory: `cd aptos-core`.
5. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh`.
6. Update your current shell environment: `source ~/.cargo/env`.

**> Manual installation of dependencies**

If the script above does not work for you, you can install these manually, but it is **not recommended**.

1. Install [Rust](https://www.rust-lang.org/tools/install).
2. Install [Git](https://git-scm.com/download).
3. Install [CMake](https://cmake.org/download/).
4. Install [LLVM](https://releases.llvm.org/).

#### Building the Aptos CLI

1. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
2. Build the CLI tool: `cargo build --package aptos --release`.
3. The binary will be available in at
   - `target/release/aptos`
4. (Optional) Move this executable to a place on your path e.g. `~/bin/aptos`.
5. You can now get help instructions by running `~/bin/aptos help`

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

1. Install [Rust](https://www.rust-lang.org/tools/install).
2. Install [Git](https://git-scm.com/download).
3. Install [CMake](https://cmake.org/download/).
4. If on Windows ARM, install [Visual Studio Preview](https://visualstudio.microsoft.com/vs/preview/).
5. Install [C++ build tools for Windows](https://visualstudio.microsoft.com/downloads/#microsoft-visual-c-redistributable-for-visual-studio-2022).
6. Install [LLVM](https://releases.llvm.org/).

#### Building aptos-core

1. Checkout the correct branch `git checkout --track origin/<branch>`, where `<branch>` is:
    - `devnet` for building on the Aptos devnet.
    - `testnet` for building on the Aptos testnet.
    - `main` for the current development branch.
2. Build the CLI tool: `cargo build --package aptos --release`.
3. The binary will be available in at
   - `target\release\aptos.exe`
4. You can now get help instructions by running `target\release\aptos.exe` in a Powershell window.

</details>