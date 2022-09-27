---
title: "Getting Started"
slug: "getting-started"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Getting Started

To kickstart your journey as a developer in the Aptos ecosystem, set up your development environment as described in this section.

## Clone the Aptos-core repo

Start by cloning the `aptos-core` GitHub repo from [GitHub](https://github.com/aptos-labs/aptos-core).

1. Clone the Aptos repo.

      ```
      git clone https://github.com/aptos-labs/aptos-core.git
      ```

2. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```

3. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment.

    ```
    ./scripts/dev_setup.sh
    ```

4. Update your current shell environment.

    ```
    source ~/.cargo/env
    ```
5. Skip this below step if you are not installing an Aptos node.

    <Tabs groupId="network">
    <TabItem value="devnet" label="Devnet">

    Checkout the `devnet` branch using:

    ```
    git checkout --track origin/devnet
    ```
    </TabItem>
    <TabItem value="testnet" label="Testnet" default>

    Checkout the `testnet` branch using:

    ```
    git checkout --track origin/testnet-stable
    ```
    </TabItem>
    </Tabs>

## Install the CLI

Install the Aptos CLI following the [Installing Aptos CLI](/cli-tools/aptos-cli-tool/install-aptos-cli.md) guide. 

## Install tools

Make sure you have the below tools installed on your computer. You will need them for running the [Developer Tutorials](/docs/tutorials/index.md), in the order specified. The below list is for macOS:

- **Homebrew**: [https://brew.sh/](https://brew.sh/)
- **Node.js**: Install [Node.js](https://nodejs.org/en/download/), which will install `npm` and `npx`, by executing the below command on your Terminal:
    ```bash
    brew install node
    ```
- **Yarn**: Install the latest [Yarn](https://classic.yarnpkg.com/lang/en/docs/install/#mac-stable) by executing the below command on your Terminal:
    ```bash
    brew install yarn
    ```
- **Poetry**: Install Poetry from [https://python-poetry.org/docs/#installation](https://python-poetry.org/docs/#installation).

Now your basic Aptos development environment is ready.

## Aptos Developer Resources

This section contains links to frequently referred Aptos developer resources. 

### Aptos Explorer

- [Aptos Explorer](https://explorer.aptoslabs.com/): Use the top-right drop-down menu to select the network.
- [Aptos Community](https://aptoslabs.com/community): Links to discussion forum, Discord and AIT.

### Aptos testnet

- **REST API Open API spec**: [https://fullnode.testnet.aptoslabs.com/v1/spec#/](https://fullnode.testnet.aptoslabs.com/v1/spec#/)
- **REST service:** [https://fullnode.testnet.aptoslabs.com/v1](https://fullnode.testnet.aptoslabs.com/v1)
- **Faucet service:** [https://faucet.testnet.aptoslabs.com](https://faucet.testnet.aptoslabs.com)
- **Genesis**: [https://testnet.aptoslabs.com/genesis.blob](https://testnet.aptoslabs.com/genesis.blob)
- **Genesis and waypoint**: [https://github.com/aptos-labs/aptos-genesis-waypoint/tree/main/testnet](https://github.com/aptos-labs/aptos-genesis-waypoint/tree/main/testnet)
- **ChainID**: [Click here to see it on the Aptos Explorer](https://explorer.aptoslabs.com/?network=testnet).

### Aptos devnet

- **REST API Open API spec**: [https://fullnode.devnet.aptoslabs.com/v1/spec#/](https://fullnode.devnet.aptoslabs.com/v1/spec#/)
- **REST service:** [https://fullnode.devnet.aptoslabs.com/v1](https://fullnode.devnet.aptoslabs.com/v1)
- **Faucet service:** [https://faucet.devnet.aptoslabs.com](https://faucet.devnet.aptoslabs.com)
- **Genesis**: [https://devnet.aptoslabs.com/genesis.blob](https://devnet.aptoslabs.com/genesis.blob)
- **Waypoint**: [https://devnet.aptoslabs.com/waypoint.txt](https://devnet.aptoslabs.com/waypoint.txt)
- **ChainID**: [Click here to see it on the Aptos Explorer](https://explorer.aptoslabs.com/?network=devnet).

### Aptos CLI

- [Aptos CLI releases](https://github.com/aptos-labs/aptos-core/releases?q=cli&expanded=true)
- [Aptos CLI Documentation](/cli-tools/aptos-cli-tool/aptos-cli-index)

### Aptos SDK

- [Typescript SDK](https://www.npmjs.com/package/aptos)
- [Python SDK](https://pypi.org/project/aptos-sdk/)
- [Rust SDK](/sdks/rust-sdk.md)

### IDE plugins for Move language

- [Syntax hightlighting for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
- [Move analyzer for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=move.move-analyzer): Supports advanced code navigation and syntax highlighting.
- [Move language plugin for Jetbrains IDEs](https://plugins.jetbrains.com/plugin/14721-move-language): Supports syntax highlighting, code navigation, renames, formatting, type checks and code generation.