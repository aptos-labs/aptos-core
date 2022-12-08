---
title: "See What's New"
slug: "whats-new-in-docs"
---

# See What's New in Aptos

This page shows the key updates to the developer documentation on this site.

## 02 December 2022

- Distributed a survey asking how we can make the Aptos developer experience better: https://aptos.typeform.com/dev-survey

## 29 November 2022

- Increased rate limits of https://indexer.mainnet.aptoslabs.com and https://fullnode.mainnet.aptoslabs.com to 1000 requests/5-minute interval by IP.

## 21 November 2022

- Added conceptual overviews for [blocks](concepts/blocks.md) and [resources](concepts/resources.md) in Aptos, explaining how transactions are batched and resources relate to objects, respectively.

## 18 November 2022

- Increased [Aptos Indexer](/guides/indexing) rate limits from 300 requests per IP per hour to 400 requests every five minutes.

## 17 November 2022

- Published instructions for [updating validator nodes](/nodes/validator-node/operator/update-validator-node) by configuring and failing over to validator fullnode.

## 16 November 2022

Completely overhauled the navigation of Aptos.dev to better reflect our users and their feedback. Here are the highlights:
 * Introduced new *Start Aptos* and *Build Apps* sections to contain information related to setup and app development, respectively.
 * Shifted key concepts up in navigation, included the Aptos White Paper, moved nodes-related materials to the *Run Nodes* section, and gas-related pages to a new *Build Apps > [Write Move Smart Contracts](/guides/move-guides/aptos-move-guides)* section.
 * Placed instructions for the Aptos CLI and other tools under *Start Aptos > [Set Environment](/guides/getting-started)*.
 * Recategorized previous *Guides* across several new subsections, including *Build Apps > [Develop Locally](/nodes/local-testnet/local-testnet-index)*, *[Interact with Blockchain](/guides/aptos-guides)*, and *Run Nodes > [Configure Nodes](/nodes/identity-and-configuration)*.
 * Integrated the [Aptos Node API specification](/nodes/aptos-api-spec#/), [Issues and Workarounds](/issues-and-workarounds) and [Aptos Glossary](/reference/glossary) into a new *Reference* section.

## 12 November 2022

- Recommended performance improvements to [validator source code](/nodes/validator-node/operator/running-validator-node/run-validator-node-using-source) startup instructions by suggesting building the `aptos-node` binary and running it directly instead of using `cargo run`.

## 09 November 2022

- Improved [indexer fullnode](/docs/nodes/indexer-fullnode.md) setup instructions to standardize on one package manager and explain how to restart the database.

## 08 November 2022

- Published links to new auto-generated Move reference files [for all available versions](/guides/move-guides/aptos-move-guides#aptos-move-documentation).

## 07 November 2022

- Created an Aptos tag on [Stack Overflow](https://stackoverflow.com/questions/tagged/aptos) and started populating it with common questions and answers.

## 04 November 2022

- Added a guide on [Resource Accounts](/docs/guides/resource-accounts.md) used by developers to publish modules and automatically sign transactions.

## 03 November 2022

- Added [Aptos API reference files](https://aptos.dev/nodes/aptos-api-spec/#/) directly to Aptos.dev for easy access and clarified available information at various endpoints.

## 02 November 2022

- Created a #docs-feedback channel on [Discord](https://discord.com/channels/945856774056083548/1034215378299133974) seeking input on Aptos.dev and taking action with updates to the documentation.

## 01 November 2022

- Expanded the previous Coin and Token documentation into the [Aptos Token Standard](/docs/concepts/coin-and-token/index.md) with new field descriptions and more and moved it to the [Getting Started](/docs/guides/getting-started.md) section for greater visibility.

## 25 October 2022

- Replaced the outdated auto-generated Move docs formally located at `aptos-core/tree/framework-docs` with new online versions now found at:
  * [Aptos tokens](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/overview.md)
  * [Aptos framework](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/overview.md)
  * [Aptos stdlib](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/doc/overview.md)
  * [Move stdlib](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/move-stdlib/doc/overview.md)

## 13 October 2022

- Added [user documentation](/docs/guides/use-aptos-explorer.md) for [Aptos Explorer](https://explorer.aptoslabs.com/) to Aptos.dev covering common use cases and popular Explorer screen descriptions.

## 12 October 2022

- Added [Node Connections](/docs/nodes/full-node/fullnode-network-connections.md) document that describes how to configure node network connections.

## 11 October 2022

- Added [Data Pruning](/docs/guides/data-pruning.md) document that describes how to change the data pruning settings.

## 10 October 2022

- Added [Staking Pool Operations](/docs/nodes/validator-node/operator/staking-pool-operations.md) document.

## 07 October 2022

- [Using the Petra Wallet](https://petra.app/docs/use) covers common use cases of the Petra Wallet Chrome browser extension and can be found from [Install Petra Extension](guides/install-petra-wallet.md) on Aptos.dev.

## 06 October 2022

- Added [Node Files](/docs/nodes/node-files-all-networks/node-files.md) document that lists all the files required during node deployment process. Includes commands to download each file.

## 05 October 2022

- Related Aptos resources (aptoslabs.com, Twitter, Discord, and more) can be found in the [Have fun](https://aptos.dev/#have-fun) section of the Aptos.dev landing page.

## 03 October 2022

- [How Base Gas Works](/docs/concepts/base-gas.md) describes the types of gas involved in Aptos transactions and offers optimizations for your use.

## 26 September 2022

- [Installing Aptos CLI](/docs/cli-tools/aptos-cli-tool/install-aptos-cli.md) provides detailed guidance for all major operating systems: Linux, macOS, and Windows.

## 25 September 2022

- [Transactions and States](/docs/concepts/txns-states.md) matches the [Aptos Blockchain whitepaper](/docs/aptos-white-paper/index.md) in structure and content.

## 23 September 2022

- [Gas and Transaction Fees](/docs/concepts/gas-txn-fee.md) contains sections on [prioritizing your transaction](/docs/concepts/gas-txn-fee.md#prioritizing-your-transaction), [gas parameters set by governance](/docs/concepts/gas-txn-fee.md#gas-parameters-set-by-governance), and [examples](/docs/concepts/gas-txn-fee.md#examples) for understanding account balances, transaction fees, and transaction amounts.

## 22 September 2022

The [System Integrators Guide](/docs/guides/system-integrators-guide.md) contains a section on [tracking coin balance changes](/docs/guides/system-integrators-guide.md#tracking-coin-balance-changes).

## 21 September 2022

When [installing Aptos CLI](/docs/cli-tools/aptos-cli-tool/install-aptos-cli.md), we recommend [downloading the precompiled binary](/docs/cli-tools/aptos-cli-tool/install-aptos-cli.md#download-precompiled-binary) over [building the CLI binary from the source code](/docs/cli-tools/aptos-cli-tool/install-aptos-cli.md#advanced-users-only-build-the-cli-binary-from-the-source-code) as less error prone and much easier to get started.

## 19 September 2022

When [using the Aptos CLI to publish Move modules](/docs/cli-tools/aptos-cli-tool/use-aptos-cli.md#publishing-a-move-package-with-a-named-address), we note multiple modules in one package must have the same account or publishing will fail at the transaction level.

## 16 September 2022

When [connecting to Aptos Testnet](/docs/nodes/validator-node/operator/connect-to-aptos-network.md), use the `testnet` rather than `testnet-stable` branch. See that document for the latest commit and Docker image tag.

## 15 September 2022

The [hardware requirements](/docs/nodes/validator-node/operator/node-requirements.md#hardware-requirements) for Aptos nodes have grown for both Amazon Web Services and Google Cloud.

## 13 September 2022

- A new guide describing [how to deploy multiple validator nodes and validator fullnodes](/docs/guides/running-a-local-multi-node-network.md) is posted.

## 12 September 2022

- A new set of documents on Aptos [Coin and Token](/concepts/coin-and-token/index.md) are posted. 
- A new document describing how to [bootstrap a new fullnode using data restore](/nodes/full-node/bootstrap-fullnode.md) is posted.

## 06 September 2022

- A new concept document explaining the [State Synchronization](/guides/state-sync.md) is posted.

- The [Staking](/concepts/staking.md) document is updated.

## 29 August 2022

- A new guide, [Leaderboard Metrics](/nodes/leaderboard-metrics), describing the [Aptos Validator Status](https://aptoslabs.com/leaderboard/it3) page is released.

## 25 August 2022

- A new guide describing [Upgrading the Move Code](/guides/move-guides/upgrading-move-code.md) is posted.


## 24 August 2022

- The Korean language version of the [Aptos White Paper](/aptos-white-paper/aptos-white-paper-in-korean) is posted.
- Typescript and Rust are added to the [first transaction tutorial](/tutorials/your-first-transaction-sdk).
- A [new tutorial](/tutorials/your-first-nft-sdk) is added that shows how to use the Typescript SDK and Python SDKs to work with NFT. The tutorial covers topics such as creating your own collection, creating a token in that collection, and how to offer and claim that token.

## 16 August 2022

- A new tutorial showing how to create, submit and verify [your first transaction using the Python SDK](/tutorials/your-first-transaction) is posted.

## 11 August 2022

- The [Aptos White Paper](/aptos-white-paper/aptos-white-paper-index) is released.

- A section explaining the network [Port settings](/nodes/validator-node/operator/node-requirements#ports) for the nodes connecting to an Aptos network is added.

## 08 August 2022

- A new document for the [exploratory Move transactional testing](/guides/move-guides/guide-move-transactional-testing) is posted.

## 07 August 2022

- A new document for [using the Aptos CLI to launch a local testnet](/nodes/local-testnet/using-cli-to-run-a-local-testnet) is posted.

## 02 August 2022

- A new [Guide for System Integrators](/guides/system-integrators-guide) is posted.
