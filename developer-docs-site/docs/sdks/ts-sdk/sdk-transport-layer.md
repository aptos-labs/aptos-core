---
title: "Typescript SDK Transport Layer"
slug: "typescript-sdk-transport-layer"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

The transport layer in the SDK provides a robust and reliable communication channel between the client-side application and the blockchain server. It exports a [Provider](./sdk-transport-layer.md) class that extends both the Aptos REST API and the Aptos Indexer.
The transport layer acts as a mediator between the client-side application and the blockchain server, ensuring reliable and efficient communication.

These classes provide a high-level interface for the application to interact with the blockchain server. The classes are designed to be easy to use and understand, allowing developers to quickly integrate the SDK into their applications.

### Provider class

To help developers and provide a better development experience, the SDK offers a [Provider](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/providers/provider.ts) class that extends both `AptosClient` and `IndexerClient` classes and gives the end user the option to simply create a `Provider` instance and call a method by hiding the underlying implementation.

The `Provider` class accepts:

- `network` - network enum type `mainnet | testnet | devnet`.
- `CustomEndpoints` of type `{fullnodeUrl: string, indexerUrl: string}` - this is to support devs who run their own nodes/indexer or to use local development against local testnet.
- `Config` - an optional argument the AptosClient accepts.
- `doNotFixNodeUrl` - an optional argument the AptosClient accepts.

An example of how to use the `Provider` class:

```
import { Provider, Network } from "aptos";

const provider = new Provider(Network.DEVNET)
const account = await provider.getAccount("0x123");
const accountNFTs = await provider.getAccountNFTs("0x123");
```

#### AptosClient class

The [AptosClient](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html) uses the [OpenAPI](https://aptos-labs.github.io/ts-sdk-doc/) specification to generate a set of classes that represent the various endpoints and operations of the Aptos REST API.

The `AptosClient` is used to communicate with the Aptos REST API and handling of errors and exceptions.
In addition, the `AptosClient` component supports submitting transactions in BCS format, which prepares and signs the raw transactions on the client-side. This method leverages the BCS Library and Transaction Builder for constructing the transaction payloads. See the guide [Creating a Signed Transaction](../../guides/sign-a-transaction.md) for instructions.

You can use the `AptosClient` class directly or the `Provider` class (preferred).

#### IndexerClient class

The [IndexerClient](../../guides/indexing.md) is responsible for handling the communication between the client-side application and the blockchain network. It uses the Hasura framework to generate a set of GraphQL queries that can be used to retrieve data from the blockchain. The queries are optimized for performance and can retrieve data in real-time.

The class provides a high-level interface for the application to interact with the blockchain. It encapsulates the complexity of the blockchain network, making it easy for developers to integrate the SDK into their applications.

You can use the `IndexerClient` class directly or the `Provider` class (preferred).
