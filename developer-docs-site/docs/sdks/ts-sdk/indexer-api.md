---
title: "IndexerClient Class"
slug: "typescript-sdk-indexer-client-class"
---

The [IndexerClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/providers/indexer.ts) is responsible for handling the communication between the client-side application and the blockchain network. It uses the Hasura framework to generate a set of GraphQL queries that can be used to retrieve data from the blockchain. The queries are optimized for performance and can retrieve data in real-time.

### Usage

To use the IndexerClient class, you will need to create an instance of the IndexerClient class and call the desired API method. The IndexerClient object will handle the HTTP requests and responses and return the result to your application.

### Configuration

Before using the IndexerClient class, you will need to configure it with the necessary parameters. These parameters may include the network endpoint URL, custom configuration, and any other required settings. You can configure the IndexerClient class by passing in the necessary parameters when you initialize the client object.

### Initialization

To initialize the IndexerClient class, you will need to pass in the necessary configuration parameters. Here is an example:

```js
import { IndexerClient } from "aptos";

const client = new IndexerClient("https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql");
```

### Making API fetch calls

To make an API call, you will need to call the appropriate method on the IndexerClient object. The method name and parameters will depend on the specific API you are using. Here is an example:

```js
const accountNFTs = await provider.getAccountNFTs("0x123");
```

In this example, we are using the `getAccountNFTs()` method to retrieve the NFT of an account with the address `0x123`.

### Use custom queries

### Generate queries

:::tip
You can use the `IndexerClient` class directly or the [Provider](./sdk-client-layer.md) class (preferred).
:::
