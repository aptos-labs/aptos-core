---
title: "Getting Started"
slug: "getting-started"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Getting Started

To kickstart your journey in the Aptos ecosystem, set up your environment as needed by your role. To interact with Aptos, you may simply [install the Aptos command line interface (CLI)](#install-the-cli). To develop Aptos itself, you will need to [clone the Aptos-core repository](#clone-the-Aptos-core-repo).

See the [Workflows](#workflows) for use cases associated with each path. See the [Aptos developer resources](#aptos-developer-resources) for quick links to Aptos networks, SDKs, and other tools.

## Workflows

Most Aptos users will want to have the Aptos CLI installed. [Install](../cli-tools/aptos-cli-tool/install-aptos-cli.md) and [use](../cli-tools/aptos-cli-tool/use-aptos-cli.md) the Aptos CLI if you will:

* [Run a local testnet](../nodes/local-testnet/using-cli-to-run-a-local-testnet.md).
* [Manage accounts](../cli-tools/aptos-cli-tool/use-aptos-cli.md#account-examples).
* [Generate keys](../cli-tools/aptos-cli-tool/use-aptos-cli.md#key-examples).
* [Compile Move packages](../cli-tools/aptos-cli-tool/use-aptos-cli.md#move-examples).

In addition to installing the CLI, [clone](#clone-the-aptos-core-repo) and [review](https://github.com/aptos-labs/aptos-core) the Aptos repository if you will:

* [Run a fullnode](../nodes/full-node/index.md).
* [Run a validator node](../nodes/validator-node/index.md).
* [Take the starter tutorials](../tutorials/index.md), many of which rely upon Aptos source code.
* [Extend and contribute](https://github.com/aptos-labs/aptos-core) to the Aptos blockchain itself; [pull requests](https://github.com/aptos-labs/aptos-core/pulls) welcome!

Although Docker options exist for many of these configurations, you should download the Aptos source to become familiar with the innerworkings of the blockchain once you are conducting this more advanced work.

:::tip Find information in the source
The [Aptos source files](https://github.com/aptos-labs/aptos-core) themselves also contain a wealth of information in docs comments worth reviewing.
:::

## Install the CLI

[Install Aptos CLI](../cli-tools/aptos-cli-tool/install-aptos-cli.md) to interact with the Aptos network. As a developer in the Aptos ecosystem, set up your development environment as described in the link. See [Installing Aptos CLI](../cli-tools/aptos-cli-tool/install-aptos-cli.md) for the supported operating systems.

## Clone the Aptos-core repo

As described in [Workflows](#workflows), you may interact with Aptos using only the CLI. If you need the source, clone the `aptos-core` GitHub repo from [GitHub](https://github.com/aptos-labs/aptos-core).

1. Clone the Aptos repo.

      ```
      git clone https://github.com/aptos-labs/aptos-core.git
      ```

2. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```

3. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment by installing most of the dependencies needed to build, test and inspect Aptos Core. Note, you may be prompted for your password:

    ```
    ./scripts/dev_setup.sh
    ```
    :::tip
    You can see the available options for the script by running `./scripts/dev_setup.sh --help`
    :::

4. Update your current shell environment to run `cargo build` and other Aptos-related commands:

    ```
    source ~/.cargo/env
    ```

5. Optionally, check out a release branch to install an Aptos node:

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
    </Tabs>

Now your basic Aptos development environment is ready.

## Aptos developer resources

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
- [Aptos CLI documentation](/cli-tools/aptos-cli-tool/aptos-cli-index)

### Aptos SDK

- [Typescript SDK](../sdks/ts-sdk/index.md)
- [Python SDK](../sdks/python-sdk.md)
- [Rust SDK](../sdks/rust-sdk.md)

### IDE plugins for Move language

- [Syntax hightlighting for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
- [Move analyzer for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=move.move-analyzer): Supports advanced code navigation and syntax highlighting.
- [Move language plugin for Jetbrains IDEs](https://plugins.jetbrains.com/plugin/14721-move-language): Supports syntax highlighting, code navigation, renames, formatting, type checks and code generation.
