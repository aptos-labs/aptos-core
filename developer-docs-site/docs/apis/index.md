---
title: "Aptos APIs"
---

The Aptos Blockchain network can be accessed by several APIs, depending on your use-case.

1. #### [Aptos Fullnode-embedded REST API](./fullnode-rest-api.md). 
    
    This API - embedded into Fullnodes - provides a simple, low latency, yet low-level way of _reading_ state and _submitting_ transactions to the Aptos Blockchain. It also supports transaction simulation.

2. #### [Aptos Indexer-powered GraphQL API](../indexer/indexer-landing.md). 
    
    This API provides a high-level, opinionated GraphQL API to _read_ state from the Aptos Blockchain. If your app needs to interact with high level constructs, such as NFTs, Aptos Objects or custom Move contracts, you likely want to incorporate the Aptos GraphQL Indexer API in some fashion. Learn more about the Indexer-powered GraphQL API here.

3. #### [Aptos GRPC Transaction Stream API](../indexer/txn-stream/index.md)

   This API provides a way to stream historical and current transaction data in real-time to an indexing processor. This API is used by the Aptos Core Indexing infrastructure itself but also can be used to build app-specific custom indexing processors that process blockchain data in real-time. Learn more here.

4. #### Faucet API (Only Testnet/Devnet)
   
   This API provides the ability to mint coins on the Aptos Labs operated devnet and testnet and it's primary purpose is development and testing of your apps and Move contracts before deploying them to mainnet.


The code of each of the above mentioned APIs is open-sourced on [GitHub](https://github.com/aptos-labs/aptos-core). As such anyone can operate these APIs and many independent operators and builders worldwide choose to do so.


### Aptos Labs operated API Deployments

For convenience [Aptos Labs](https://aptoslabs.com) operates a deployment of these APIs for each Aptos Network and makes them available for public consumption.

At the moment there are 2 sets of Aptos Labs API deployments:

1. [APIs with anonymous access and IP-based rate-limiting](../nodes/networks.md)
2. [[Beta] APIs with authentication and developer-account based rate limiting through the Aptos Labs Developer Portal](./aptos-labs-developer-portal.md)
