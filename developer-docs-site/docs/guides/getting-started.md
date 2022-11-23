---
title: "Prepare Aptos Dev Environment"
slug: "getting-started"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Prepare Aptos Dev Environment

To kickstart your journey in the Aptos ecosystem, set up your environment as needed by your role. To interact with Aptos, you may simply [install the Aptos command line interface (CLI)](#install-the-cli). To develop Aptos itself, you will need to [clone the Aptos-core repository](#clone-the-Aptos-core-repo).

See the [Workflows](#workflows) for use cases associated with each path. See the [Aptos developer resources](#aptos-developer-resources) for quick links to Aptos networks, SDKs, and other tools.

Then review the Aptos source code, found in the [Aptos-core](https://github.com/aptos-labs/aptos-core) repository of GitHub, and continue your journey through this site. The source contains READMEs and code comments invaluable to developing on Aptos.

## Use supported operating systems

Aptos can be built on various operating systems, including Linux, macOS. and Windows. Aptos is tested extensively on Linux and macOS, and less so on Windows. Here are the versions we use:

* Linux - Ubuntu version 20.04 and 22.04
* macOS - macOS Monterey and later
* Microsoft Windows - Windows 10, 11 and Windows Server 2022+

See [Installing Aptos CLI](../cli-tools/aptos-cli-tool/install-aptos-cli.md) for instructions by operating system.

## Meet hardware requirements

Aptos requires no specific hardware to develop on the blockchain. To run one of our nodes, see the hardware requirements for:

* [Fullnodes](nodes/full-node/fullnode-source-code-or-docker.md#hardware-requirements)
* [Validators](nodes/validator-node/operator/node-requirements.md#hardware-requirements)

## Workflows

### CLI only

Most Aptos users will want to have the Aptos CLI installed. [Install](../cli-tools/aptos-cli-tool/install-aptos-cli.md) and [use](../cli-tools/aptos-cli-tool/use-aptos-cli.md) the Aptos CLI if you will:

* [Run a local testnet](../nodes/local-testnet/using-cli-to-run-a-local-testnet.md).
* [Manage accounts](../cli-tools/aptos-cli-tool/use-aptos-cli.md#account-examples).
* [Generate keys](../cli-tools/aptos-cli-tool/use-aptos-cli.md#key-examples).
* [Compile Move packages](../cli-tools/aptos-cli-tool/use-aptos-cli.md#move-examples).

### Source code and CLI

In addition to installing the CLI, [clone](#clone-the-aptos-core-repo) and [review](https://github.com/aptos-labs/aptos-core) the Aptos repository if you will:

* [Run a fullnode](../nodes/full-node/index.md).
* [Run a validator node](../nodes/validator-node/index.md).
* [Take the starter tutorials](../tutorials/index.md), many of which rely upon Aptos source code.
* [Extend and contribute](https://github.com/aptos-labs/aptos-core) to the Aptos blockchain itself; [pull requests](https://github.com/aptos-labs/aptos-core/pulls) welcome!

Although Docker options exist for many of these configurations, you should download the Aptos source to become familiar with the inner workings of the blockchain once you are conducting this more advanced work.

:::tip Find information in the source
The [Aptos source files](https://github.com/aptos-labs/aptos-core) themselves also contain a wealth of information in docs comments worth reviewing.
:::

## Install the Aptos CLI

[Install Aptos CLI](../cli-tools/aptos-cli-tool/install-aptos-cli.md) to interact with the Aptos network. As a developer in the Aptos ecosystem, set up your development environment as described in the link.

## Clone the Aptos-core repo

As described in [Workflows](#workflows), you may interact with Aptos using only the CLI. If you need the source, clone the `aptos-core` GitHub repo from [GitHub](https://github.com/aptos-labs/aptos-core).

1. Clone the Aptos repo.

      ```
      git clone https://github.com/aptos-labs/aptos-core.git
      ```

1. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```

1. Update your current shell environment to run `cargo build` and other Aptos-related commands:

    ```
    source ~/.cargo/env
    ```

1. Optionally, check out a release branch to install an Aptos node:

    <Tabs groupId="network">
    <TabItem value="devnet" label="Devnet">

    Check out the `devnet` branch using:

    ```
    git checkout --track origin/devnet
    ```
    </TabItem>
    <TabItem value="testnet" label="Testnet" default>

    Check out the `testnet` branch using:

    ```
    git checkout --track origin/testnet
    ```
    </TabItem>
    <TabItem value="mainnet" label="Mainnet">

    Check out the `mainnet` branch using:

    ```
    git checkout --track origin/mainnet
    ```
    </TabItem>
    </Tabs>

## Prepare development environment

Prepare your developer environment by installing the dependencies needed to build, test and inspect Aptos Core. You can do this either via the Aptos [`dev_setup.sh`](https://github.com/aptos-labs/aptos-core/blob/main/scripts/dev_setup.sh) Bash script or by manually installing all dependencies. as shown below.

No matter your selected mechanism for installing these dependencies, **it is imperative you keep your entire toolchain up-to-date**. If you encounter issues later, update all packages and try again.

### Use the script
The `dev_setup.sh` script currently supports macOS and Ubuntu Linux with other Linux distributions working but untested. The script does not support Windows. Note, you may be prompted for your password:

```
./scripts/dev_setup.sh
```

:::tip
You can see the available options for the script by running `./scripts/dev_setup.sh --help`
:::

### Install packages manually

If the script doesn't work for your operating system, manually install these packages:
* [Rust](https://www.rust-lang.org/tools/install)
* [Git](https://git-scm.com/download)
* [CMake](https://cmake.org/download/)
* [LLVM](https://releases.llvm.org/)
* Linux only - [libssl-dev](https://packages.ubuntu.com/bionic/libssl-dev) and [libclang-dev](https://packages.ubuntu.com/bionic/libclang-dev)
* Windows only - [C++ build tools](https://visualstudio.microsoft.com/downloads/#microsoft-visual-c-redistributable-for-visual-studio-2022)

See [`dev_setup.sh`](https://github.com/aptos-labs/aptos-core/blob/main/scripts/dev_setup.sh) for other potential packages needed in your environment.

Now your basic Aptos development environment is ready. Head over to our [Developer Tutorials](tutorials/index.md) to get started in Aptos.

## Understand the Aptos Token Standard

The [Aptos Token Standard](../concepts/coin-and-token/index.md) lays out the rules for creating and distributing digital assets on the Aptos blockchain.

## Find Aptos developer resources

This section contains links to frequently referred Aptos developer resources. 

### Aptos Explorer

- [Aptos Explorer](https://explorer.aptoslabs.com/): Use the top-right drop-down menu to select the network.
- [Aptos Community](https://aptoslabs.com/community): Links to discussion forum, Discord and AIT.


### Aptos mainnet

- **REST API Open API spec**: [https://fullnode.mainnet.aptoslabs.com/v1/spec#/](https://fullnode.mainnet.aptoslabs.com/v1/spec#/)
- **REST service:** [https://fullnode.mainnet.aptoslabs.com/v1](https://fullnode.mainnet.aptoslabs.com/v1)
- **Genesis and waypoint**: [https://github.com/aptos-labs/aptos-networks/tree/main/mainnet](https://github.com/aptos-labs/aptos-networks/tree/main/mainnet)
- **ChainID**: [Click here to see it on the Aptos Explorer](https://explorer.aptoslabs.com/?network=mainnet).

### Aptos testnet

- **REST API Open API spec**: [https://fullnode.testnet.aptoslabs.com/v1/spec#/](https://fullnode.testnet.aptoslabs.com/v1/spec#/)
- **REST service:** [https://fullnode.testnet.aptoslabs.com/v1](https://fullnode.testnet.aptoslabs.com/v1)
- **Faucet dApp:** [https://aptoslabs.com/testnet-faucet](https://aptoslabs.com/testnet-faucet)
- **Genesis and waypoint**: [https://github.com/aptos-labs/aptos-genesis-waypoint/tree/main/testnet](https://github.com/aptos-labs/aptos-genesis-waypoint/tree/main/testnet)
- **ChainID**: [Click here to see it on the Aptos Explorer](https://explorer.aptoslabs.com/?network=testnet).

### Aptos devnet

- **REST API Open API spec**: [https://fullnode.devnet.aptoslabs.com/v1/spec#/](https://fullnode.devnet.aptoslabs.com/v1/spec#/)
- **REST service:** [https://fullnode.devnet.aptoslabs.com/v1](https://fullnode.devnet.aptoslabs.com/v1)
- **Faucet service:** [https://faucet.devnet.aptoslabs.com](https://faucet.devnet.aptoslabs.com)
- **Genesis and waypoint**: [https://github.com/aptos-labs/aptos-networks/tree/main/devnet](https://github.com/aptos-labs/aptos-networks/tree/main/devnet)
- **ChainID**: [Click here to see it on the Aptos Explorer](https://explorer.aptoslabs.com/?network=devnet).

### Aptos CLI

- [Aptos CLI releases](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true)
- [Aptos CLI documentation](/cli-tools/aptos-cli-tool/use-aptos-cli)

### Aptos SDK

- [Typescript SDK](../sdks/ts-sdk/index.md)
- [Python SDK](../sdks/python-sdk.md)
- [Rust SDK](../sdks/rust-sdk.md)

### IDE plugins for Move language

- [Syntax highlighting for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
- [Move analyzer for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=move.move-analyzer): Supports advanced code navigation and syntax highlighting.
- [Move language plugin for Jetbrains IDEs](https://plugins.jetbrains.com/plugin/14721-move-language): Supports syntax highlighting, code navigation, renames, formatting, type checks and code generation.
