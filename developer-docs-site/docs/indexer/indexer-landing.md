---
title: "Learn about Indexing"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Learn about Indexing

## Quick Start

Refer to this role-oriented guide to help you quickly find the relevant docs:

- Core Infra Provider: You want to run your own Transaction Stream Service in addition to the rest of the stack.
  - See docs for [Self-Hosted Transaction Stream Service](/indexer/txn-stream/self-hosted).
- API Operator: You want to run the Indexer API on top of a hosted Transaction Stream Service.
  - See docs for [Self-Hosted Indexer API](/indexer/api/self-hosted).
- Custom Processor Builder: You want to build a custom processor on top of a hosted Transaction Stream Service.
  - See docs for [Custom Processors](/indexer/custom-processors).
- Indexer API Consumer: You want to use a hosted Indexer API.
  - See docs for the [Labs-Hosted Indexer API](/indexer/api/labs-hosted).
  - See the [Indexer API Usage Guide](/indexer/api/usage-guide).

# Architecture Overview

Typical applications built on the Aptos blockchain, on any blockchain for that matter, require the raw blockchain data to be shaped and stored in an application-specific manner. This is essential to supporting low-latency and rich experiences when consuming blockchain data in end-user apps from millions of users. The [Aptos Node API](https://aptos.dev/nodes/aptos-api-spec#/) provides a lower level, stable and generic API and is not designed to support data shaping and therefore cannot support rich end-user experiences directly.

The Aptos Indexer is the answer to this need, allowing the data shaping critical to real-time app use. See this high-level diagram for how Aptos indexing works:

<center>
<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/aptos-indexing.svg'),
    dark: useBaseUrl('/img/docs/aptos-indexing-dark.svg'),
  }}
/>
</center>

At a high level, indexing on the Aptos blockchain works like this:

1. Users of a dApp, for example, on an NFT marketplace, interact with the Aptos blockchain via a rich UI presented by the dApp. Behind the scenes, these interactions generate, via smart contracts, the transaction and event data. This raw data is stored in the distributed ledger database, for example, on an Aptos fullnode.
1. This raw ledger data is read and indexed using an application-specific data model, in this case an NFT marketplace-specific data model (”Business logic” in the above diagram). This NFT marketplace-specific index is then stored in a separate database (”Indexed database” in the above diagram) and exposed via an API.
1. The dApp sends NFT-specific GraphQL queries to this indexed database and receives rich data back, which is then served to the users.

Step 2 is facilitated by the Aptos Indexer. The diagram above is a simplified view of how the system works at a high level. In reality, the system is composed of many components. If you are interested in these details, see the [Detailed Overview](#detailed-overview) below.

## Indexer API

Aptos supports the following ways to access indexed data.

1. [Labs hosted Indexer API](/indexer/api/labs-hosted): This API is rate-limited and is intended only for lightweight applications such as wallets. This option is not recommended for high-bandwidth applications.
2. [Self hosted Indexer API](/indexer/api/self-hosted): Run your own deployment of the Labs hosted indexer stack.
3. [Custom processor](/indexer/custom-processors): Write and deploy a custom processor to index and expose data in a way specific to your needs.

## Transaction Stream Service

The Indexer API and Custom Processors depend on the Transaction Stream Service. In short, this service provides a GRPC stream of transactions that processors consume. Learn more about this service [here](/indexer/txn-stream/). Aptos Labs offers a [hosted instance of this service](/indexer/txn-stream/labs-hosted) but you may also [run your own](/indexer/txn-stream/self-hosted).

## Detailed Overview

This diagram explains how the Aptos Indexer tech stack works in greater detail.

<center>
<div style={{marginBottom: 20}}>
<iframe
  style={{border: "1px solid rgba(0, 0, 0, 0.1);"}}
  width="100%"
  height="750"
  src="https://www.figma.com/embed?embed_host=share&url=https%3A%2F%2Fwww.figma.com%2Ffile%2FsVhSOGR7ZT4CdeUzlXyduD%2FIndexer-Overview%3Ftype%3Dwhiteboard%26node-id%3D0%253A1%26t%3DUnUKeEaBE7ETMksb-1"
  allowfullscreen>
</iframe>
</div>
</center>

<!-- TODO: Write an explanation of this diagram. -->

## Legacy Indexer
Find information about the legacy indexer [here](/indexer/legacy/).
