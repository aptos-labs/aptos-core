---
title: "Indexing"
slug: "indexing"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Indexing

## Concept

An application built on the Aptos blockchain, on any blockchain for that matter, requires that the raw data from the blockchain be shaped by the application-specific data model before the application can consume it. The [Aptos Node API](https://fullnode.devnet.aptoslabs.com/v1/spec#/), using which a client can interact with the Aptos blockchain, is not designed to support data shaping. Moreover, the ledger data you get back using this API contains the data only for the transactions **initiated by you**. It does not provide the data for the transactions initiated by the others. This data is insufficient and too slow for an application that must access the blockchain data in an omniscient way to serve multiple users of the application. 

Indexer is a solution to this problem. See below a high-level block diagram of how Aptos indexing works. 

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/aptos-indexing.svg'),
    dark: useBaseUrl('/img/docs/aptos-indexing-dark.svg'),
  }}
/>

## Indexing the Aptos blockchain data

Indexing on the Aptos blockchain works like this:

- Users of a dApp, for example, on an NFT marketplace, interact with the Aptos blockchain via a rich UI presented by the dApp. Behind the scenes, these interactions generate, via smart contracts, the transaction and event data. This raw data is stored in the distributed ledger database, for example, on an Aptos fullnode.
- This raw ledger data is read and indexed using an application-specific data model, in this case an NFT marketplace-specific data model (”Business logic” in the above diagram). This NFT marketplace-specific index is then stored in a separate database (”Indexed database” in the above diagram).
- The dApp sends NFT-specific GraphQL queries to this indexed database and receives rich data back, which is then served to the users.

## Options for Aptos indexing service

Aptos supports the following ways to index the Aptos blockchain. 

1. Use the Aptos-provided indexing service with GraphQL API. This API is rate-limited and is intended only for lightweight applications such as wallets. This option is not recommended for high-bandwidth applications. This indexing service supports the following modules:
    1. **Token**: Only tokens that implement the Aptos `0x3::token::Token` standard. This indexer will only support 0x3 operations such as mint, create, transfer, offer, claim, and coin token swap. Also see Coin and Token.
    2. **Coin**: Supports only `0x1::coin::CoinStore`. This indexer will index any coins that appear in Aptos `CoinStore` standard but such coins may not have value unless they implement `0x1::coin::CoinInfo`.
2. Run your own indexer-enabled Aptos fullnode. With this option, the indexer supports, in addition to the above coin and token modules, basic transactions, i.e., each write set, events and signatures. 
3. Lastly, you can define your own data model (”Business Logic” in the above diagram) and set up the database for the index. 

## Use the Aptos-provided indexing service

TBD: Provide GraphQL query examples showing how to use this service. 

## Run an indexer-enabled fullnode

Port this document to aptos.dev: [https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer#installation-guide-for-apple-sillicon](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer#installation-guide-for-apple-sillicon) 

## Define your own data model

TBD

