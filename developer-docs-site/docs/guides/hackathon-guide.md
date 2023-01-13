---
title: "How to develop on Aptos for a Hackathon"
slug: "hackathon-developers-guide"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# How to develop on Aptos quickguide

If you're looking to build a project on Aptos, this is the guide for you.  This will walk you through the development
process and how to integrate with the Aptos framework into your project.

# Overview

1. [Setting up your development environment](#Setting-up-your-development-environment)
2. 

## Setting up your development environment
### CLI Tool
The [Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/) provides a Move compiler, a test framework, and deployment
and operational tools to interact with the Aptos blockchain.  It's preferred to [install a prebuilt binary](https://aptos.dev/cli-tools/aptos-cli-tool/install-aptos-cli)
or with [homebrew on Mac](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/homebrew/README.md)

[Using the CLI](../cli-tools/aptos-cli-tool/use-aptos-cli.md) demonstrates how the CLI can
be used to create accounts, transfer coins, publish modules, and more.

### SDKs

Aptos currently provides three SDKs to access the blockchain:
1. [Typescript](../sdks/ts-sdk/index.md)
2. [Python](../sdks/python-sdk.md)
3. [Rust](../sdks/rust-sSDKs and Toolsdk.md)

Additionally, these SDKs are available from the community:
* TBD Go, Unity, etc.

### Testnet / Mainnet Node Providers

These node providers provide testnet and mainnet access

* [BlockEden](https://blockeden.xyz/)
* [Nodereal](https://nodereal.io/)
* [Chainbase](https://chainbase.online/)
* [BlastAPI](https://blastapi.io/)
  * [Testnet](https://aptos-testnet.public.blastapi.io)
  * [Mainnet](https://aptos-mainnet.public.blastapi.io)
* [Aptos Labs](https://aptoslabs.com)
  * [Devnet](https://fullnode.devnet.aptoslabs.com)
  * [Testnet](https://fullnode.testnet.aptoslabs.com)
  * [Mainnet](https://fullnode.mainnet.aptoslabs.com)

### Indexers

We provide an indexer, and multiple other providers also provide indexing services.  The Aptos labs one
is provided for testnet: https://aptos.dev/guides/indexing/

### Blockchain Explorer

There are multiple explorers across the ecosystem for the Aptos blockchain.  Some of them are:
* https://explorer.aptoslabs.com/

### Other ecosystem projects

TODO: Put other projects that may provide SDKs are frameworks

There are many more that may be missed here, and you can find a list of projects provided by the Aptos Foundation:
* https://github.com/aptos-foundation/ecosystem-projects

### Getting started

TODO: Add integrators guide link / straightforward tutorials for dapp & how to process events etc.