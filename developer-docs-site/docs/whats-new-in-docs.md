---
title: "See What's New"
slug: "whats-new-in-docs"
---

# See What's New in Aptos

This page shows the key updates to the developer documentation on this site. Note, this site is built from the `main` upstream branch of GitHub and so therefore reflects the latest changes to Aptos. If you have checked out [another branch](https://github.com/aptos-labs/aptos-core/branches) to use a [specific network](guides/system-integrators-guide.md#choose-a-network), the code may not yet have all of the features described here.

## 30 January 2023

- Released [Aptos Node v1.2.3](https://github.com/aptos-labs/aptos-core/releases/tag/aptos-node-v1.2.3) to mainnet with enhancements to state synchronization and features added for use of the Move programming language, including:
  * New integer types (u16, u32, u256)
  * [View functions](guides/aptos-apis.md#reading-state-with-the-view-function) to evaluate transactions before execution
  * Compile-time checks for transaction arguments
  * Various updates to smart contracts
  * Several Aptos Improvement Proposals (AIPs)

  See the [Aptos Releases](https://github.com/aptos-labs/aptos-core/releases) list for many more details.

- Created an entirely new tutorial that covers [building an end-to-end todo list dapp](tutorials/build-e2e-dapp/index.md), starting from the smart contract side through the front-end side and use of a wallet to interact with the two.

## 26 January 2023

- Developed and now recommend use of a script to [automate installation of the Aptos command line interface (CLI)](cli-tools/aptos-cli-tool/automated-install-aptos-cli.md) that works on Linux, macOS, Windows Subsystem for Linux (WSL), and Windows NT.

## 25 January 2023

- Split up the sidebars of Aptos.dev into one left navigation menu per topic to ease use. As part of this:

  * Added top-level menu for *Create NFTs* section
  * Renamed *Measure Nodes* section to *Monitor Nodes*
  * Moved [Node Liveness Criteria](nodes/validator-node/operator/node-liveness-criteria.md) to the *Monitor Nodes* section

- Added a new section [Reading state with the View function](guides/aptos-apis.md#reading-state-with-the-view-function) explaining how to use the [View](https://github.com/aptos-labs/aptos-core/blob/main/api/src/view_function.rs) function now available in devnet to test transactions without modifying blockchain state.

## 24 January 2023

- Added [Mint NFT with Aptos CLI](guides/move-guides/mint-nft-cli.md) Move code lab describing the completely revised [Mint NFT](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/mint_nft) Move examples.

## 23 January 2023

- Introduced a *Create NFTs* section in navigation that includes:

  * a [comparison of Aptos tokens](concepts/coin-and-token/aptos-token-overview.md) with other blockchains
  * instructions for [minting fungible tokens with on-chain data](concepts/coin-and-token/mint-onchain-data.md)
  * installation instructions for a new web-based [Aptos NFT Minting Tool](concepts/coin-and-token/nft-minting-tool.md)

## 18 January 2023

- Added a section explaining the nuances of [batch signing](guides/sign-a-transaction.md#batch-signing) to Create a Signed Transaction.

- Enhanced [validator node setup documentation](nodes/validator-node/operator/index.md) to ensure operators first [deploy the nodes](nodes/validator-node/operator/running-validator-node/index.md), then [connect to the Aptos network](nodes/validator-node/operator/connect-to-aptos-network.md), and finally [establish staking pool operations](nodes/validator-node/operator/staking-pool-operations.md).

## 16 January 2023

- Documented how to [create and fund accounts](guides/get-test-funds.md) using the Petra Wallet and Aptos CLI.

## 12 January 2023

- Added [Homebrew](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/homebrew/README.md) support for the Aptos CLI, enabling [easy installation](cli-tools/aptos-cli-tool/index.md) via the macOS, Linux, and WSL package manager.

## 11 January 2023

- Greatly expanded the [First Dapp tutorial](tutorials/first-dapp.md) to include a section on publishing the Move module using the TypeScript SDK, adds labels to output, improve the initialization for Petra wallet and the Aptos CLI, and more.

- Noted in the [Aptos Token Standard](concepts/coin-and-token/aptos-token.md) that limits exist to storing customized token properties on-chain, namely 1000 properties per token with field names limited to 128 characters.

- Added examples for requesting staking commission to [Staking Pool Operations](nodes/validator-node/operator/staking-pool-operations.md).


## 10 January 2023

- Explained in Validator cloud setup docs ([AWS](nodes/validator-node/operator/running-validator-node/using-aws.md), [Azure](nodes/validator-node/operator/running-validator-node/using-azure.md) and [GCP](nodes/validator-node/operator/running-validator-node/using-gcp.md)) how to check for and remove remaining Kubernetes volumes after changing the `era` to reset a deployment's state.

- Expanded support for other networks in [Start Public Fullnode with Aptos Source or Docker](nodes/full-node/fullnode-source-code-or-docker.md) by adding files and instructions for fullnodes in `devnet` and `testnet`, as well as the default `mainnet`.

- Enhanced [Run a Public Fullnode on GCP](nodes/full-node/run-a-fullnode-on-gcp.md) with details on fixing Terraform version mismatches, a link to the Docker image, and example output from `kubectl` commands.

- Added a [Node types](concepts/node-networks-sync.md#node-types) section to Node Networks and Synchronization describing the various forms of nodes supported by Aptos: validator nodes, public fullnodes, validator fullnodes, and archival nodes.

## 06 January 2023

- Added an *Authors* list to the bottom of every page on Aptos.dev giving credit to all contributors to the document, both within Aptos Labs and externally.

## 30 December 2022

- Added [Node Inspection Service](nodes/measure/node-inspection-service.md) document that explains how to access node metrics for validators and fullnodes and highlights key metrics for monitoring.

- Added instructions for [running archival nodes](guides/state-sync.md#running-archival-nodes), specifically avoiding fast syncing and ledger pruning. 

## 29 December 2022

- Improved [Update Aptos Validator Node](nodes/validator-node/operator/update-validator-node.md) with a section on securely running multiple validator fullnodes (VFNs) plus links to [Bootstrap Fullnode from Snapshot](nodes/full-node/bootstrap-fullnode.md) and [state synchronization](guides/state-sync.md).

## 26 December 2022

- Restored and refined [Bootstrap Fullnode from Snapshot](nodes/full-node/bootstrap-fullnode.md) to simplify and expedite Aptos fullnode starts in devnet and testnet environments.

## 23 December 2022

- Added instructions for [manually installing build dependencies on Windows](guides/getting-started.md#set-up-build-dependencies).

## 20 December 2022

- Added [Formal Verification, the Move Language, and the Move Prover](https://www.certik.com/resources/blog/2wSOZ3mC55AB6CYol6Q2rP-formal-verification-the-move-language-and-the-move-prover) blog post from the community to the [Supporting Move resources](guides/move-guides/index.md#supporting-move-resources) list.

## 14 December 2022

- Noted you may employ the [Aptos Name Service](https://www.aptosnames.com/) to secure .apt domains for key [accounts](concepts/accounts.md).

## 12 December 2022

- Released [Node Health Checker](nodes/measure/node-health-checker.md) web interface for evaluating fullnodes at: https://nodetools.aptosfoundation.org/#/node_checker

## 11 December 2022

- [Renamed](https://github.com/aptos-labs/aptos-core/pull/5778) `AptosGovernance::create_proposal` to `aptos_governance::create_proposal` and added information on [Aptos Improvement Proposals (AIPs)](concepts/governance.md#aptos-improvement-proposals-aips) and the [Technical Implementation of Aptos Governance](concepts/governance.md#technical-implementation-of-aptos-governance).

## 09 December 2022

- Added an [Aptos Wallet Adapter overview](concepts/wallet-adapter-concept.md) and instructions for both [dApp](guides/wallet-adapter-for-dapp.md) and [wallet](guides/wallet-adapter-for-wallets.md) builders.

## 08 December 2022

- Released [aptos-node-v1.1.0](https://github.com/aptos-labs/aptos-core/releases/tag/aptos-node-v1.1.0) to mainnet:

  Framework upgrade through governance voting:
  - Testnet upgrade - Nov 30th
  - Mainnet upgrade - Dec 12th - 19th (7 days voting period) required by fullnode operators

  New features and enhancements:
  - Move
    - [New chain_id native function + corresponding new gas schedule entry](https://github.com/aptos-labs/aptos-core/pull/5288).
  - Blockchain
    - Added automatic chain-health based back pressure to improve reliability. Automatic slow-down (through max block size reduction) is triggered in some scenarios.
    - Improved timeouts for state synchronization: (i) lower timeouts for optimistic fetch (to help reduce end-to-end latency); and (ii) exponential back-off for low-bandwidth nodes.

  Resolved issues:
  - Move
    - Explicit error codes in resource account.
    - Improved Leader Election (gated behind feature flag).

  See these resources for more details on the release:
    - [#mainnet-release](https://discord.com/channels/945856774056083548/1042502400507916349) Discord channel for more detailed descriptions of the above changes.
    - [Aptos Releases](https://github.com/aptos-labs/aptos-core/releases) list for all releases.
    - [`testnet`](https://github.com/aptos-labs/aptos-core/commits/testnet) branch commits for the changes landing in mainnet today.

## 05 December 2022

- Moved recently pared down System Integrators Guide to [Use the Aptos REST Read API](guides/aptos-apis.md).
- Resurrected last full version of the [Aptos System Integrators Guide](guides/system-integrators-guide.md).
- Renamed System Integrators Guide to [Integrate Aptos with Your Platform](guides/system-integrators-guide.md) and updated it with our latest guidance.

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
