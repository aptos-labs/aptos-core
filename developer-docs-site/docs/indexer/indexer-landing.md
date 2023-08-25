---
title: "Learn about Indexing"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Learn about Indexing

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

- Users of a dApp, for example, on an NFT marketplace, interact with the Aptos blockchain via a rich UI presented by the dApp. Behind the scenes, these interactions generate, via smart contracts, the transaction and event data. This raw data is stored in the distributed ledger database, for example, on an Aptos fullnode.
- This raw ledger data is read and indexed using an application-specific data model, in this case an NFT marketplace-specific data model (”Business logic” in the above diagram). This NFT marketplace-specific index is then stored in a separate database (”Indexed database” in the above diagram).
- The dApp sends NFT-specific GraphQL queries to this indexed database and receives rich data back, which is then served to the users.

## Indexer API

Aptos supports the following ways to access indexed data.

1. [Labs hosted Indexer API](api/labs-hosted): This API is rate-limited and is intended only for lightweight applications such as wallets. This option is not recommended for high-bandwidth applications.
2. [Self hosted Indexer API](api/self-hosted): Run your own deployment of the Labs hosted indexer stack.
3. [Custom processor](custom-processors): Write and deploy a custom processor to index and expose data in a way specific to your needs.

## Transaction Stream Service

The Indexer API and Custom Processors depend on the Transaction Stream Service. In short, this service provides a GRPC stream of transactions that processors consume. Learn more about this service [here](txn-stream/). Aptos Labs offers a [hosted instance of this service](txn-stream/labs-hosted) but you may also [run your own](txn-stream/self-hosted).

## Legacy Indexer
Find information about the legacy indexer [here](legacy/).
