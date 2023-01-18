---
title: "Develop on Aptos"
slug: "developers-index"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Develop on Aptos

If you're looking to build a project on Aptos, this is your starting point. This page summarizes the development process and links to resources for integrating your project with the Aptos framework.

# Overview

1. [Setting up your development environment](#set-up-your-development-environment)
2. 

## Set up your development environment
### Install Aptos CLI
The [Aptos command line interface (CLI)](../cli-tools/aptos-cli-tool/index.md) provides a Move compiler, a test framework, and deployment
and operational tools to interact with the Aptos blockchain. We recommend downloading [a prebuilt binary](../cli-tools/aptos-cli-tool/install-aptos-cli.md)
or using the [Homebrew](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/homebrew/README.md) package manager.

Follow [Using the Aptos CLI](../cli-tools/aptos-cli-tool/use-aptos-cli.md) to see how the CLI can be used to create accounts, transfer coins, publish modules, and more.

### Use SDKs

Aptos currently provides three SDKs to access the blockchain:
1. [Typescript](../sdks/ts-sdk/index.md)
2. [Python](../sdks/python-sdk.md)
3. [Rust](../sdks/rust-sdk.md)

Additionally, these SDKs are available from the community:
* TBD Go, Unity, etc.

### Employ testnet and mainnet node providers

These node providers offer testnet and mainnet access to Aptos:

* [BlockEden](https://blockeden.xyz/)
* [NodeReal](https://nodereal.io/)
* [Chainbase](https://chainbase.online/)
* [BlastAPI](https://blastapi.io/)
  * [Testnet](https://aptos-testnet.public.blastapi.io)
  * [Mainnet](https://aptos-mainnet.public.blastapi.io)
* [Aptos Labs](https://aptoslabs.com)
  * [Devnet](https://fullnode.devnet.aptoslabs.com)
  * [Testnet](https://fullnode.testnet.aptoslabs.com)
  * [Mainnet](https://fullnode.mainnet.aptoslabs.com)

### Indexers

Aptos provides an [indexer](../guides/indexing.md), and multiple other providers also provide indexing services.  The Aptos labs one
is provided for testnet.

You may also run your own [Aptos Indexer Fullnode](../nodes/indexer-fullnode.md).

### Blockchain Explorer

There are multiple explorers across the ecosystem for the Aptos blockchain.  Some of them are:
* https://explorer.aptoslabs.com/

### Other ecosystem projects

TODO: Put other projects that may provide SDKs are frameworks

There are many more that may be missed here, and you can find a list of projects provided by the Aptos Foundation:
* https://github.com/aptos-foundation/ecosystem-projects

### Getting started

TODO: Add integrators guide link / straightforward tutorials for dapp & how to process events etc.
