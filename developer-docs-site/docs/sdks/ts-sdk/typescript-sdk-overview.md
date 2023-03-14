---
title: "Typescript SDK Architecture"
slug: "typescript-sdk-overview"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# TypeScript SDK Architecture

This document describes the main features and components of the Aptos TypeScript SDK.

The [Aptos TypeScript SDK](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) provides APIs and interfaces you can use to interact with the Aptos blockchain by connecting to the [Aptos REST API](https://aptos.dev/nodes/aptos-api-spec/#/). The REST API is the means for sending your transaction to the Aptos blockchain and reading the blockchain's state.

See below a high-level architecture diagram of the Aptos TypeScript SDK.

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/ts-sdk-light.svg'),
    dark: useBaseUrl('/img/docs/ts-sdk-dark.svg'),
  }}
/>

## Key SDK features

Here are key features of the Aptos TypeScript SDK:

- **Key generation** - The Aptos SDK provides convenient methods for generating [Ed25519](https://ed25519.cr.yp.to/) key pairs. The Ed25519 public keys can be used to derive the chain account addresses, while the private keys should be kept private for transaction signing. See the [ TransactionBuilderEd25519](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderEd25519.html) class for details.
- **Transaction signing and submission** - Although the Aptos REST APIs support signing a raw transaction on the server-side, signing the transactions on the client side using the Aptos SDK, is more secure and should be the preferred choice.
- **Transaction status querying** - The Aptos SDK supports transaction status queries (success, failure, pending), by transaction hash.
- **BCS library:** The Aptos SDK implements a [Binary Canonical Serialization](https://docs.rs/bcs/latest/bcs/) (BCS) library for transaction signing and submission. The Aptos blockchain uses BCS for data serialization and deserialization. See the [Aptos SDK BCS](https://aptos-labs.github.io/ts-sdk-doc/modules/BCS.html) reference for more information.
- **Methods for information retrieval** - The Aptos SDK lets your dApp retrieve resources, modules, and transactions under a specific account.
- **Faucet client**: The Aptos [FaucetClient](https://aptos-labs.github.io/ts-sdk-doc/classes/FaucetClient.html) is for minting test coins used for development.
- **Token client** - Aptos SDK provides built-in support for NFT minting and querying. See [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) for details.

## Components of the TypeScript SDK

The Aptos TypeScript SDK has three logical layers:

1. Transport layer
2. Core SDK layer
3. Optional application layer.

Refer to the above high-level architecture diagram for their relationship. The transportation layer is responsible for sending payloads to the REST API endpoints.

The core SDK layer exposes the functionalities needed by most applications:

- Key generation
- Transaction signing and submission
- Transaction status querying
- Information retrieval techniques

The optional application layer provides built-in support for **NFT Token** API.

:::tip
You can also use this [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) API as an example of NFT Token API before you start developing your own application APIs with the SDK.
:::

### OpenAPI client

The OpenAPI client is a set of classes generated based on the Aptos REST API spec. See the [TypeScript SDK OpenAPI definition](https://aptos-labs.github.io/ts-sdk-doc/).

### AptosAccount class

The [AptosAccount](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosAccount.html) class has a constructor that creates a new account instance or retrieves an existing account instance. Additionally, this class provides the methods for:

- Generating Ed25519 key pairs
- Signing a bytes buffer with an Ed25519 public key
- Deriving initial account addresses from the public keys
- Retrieving a resource account address by source address and seeds
- Deriving account address, public key, and private key

### BCS Library

This section describes a subset of BCS standards implemented in TypeScript.

### Transaction Builder

The Aptos TypeScript SDK exposes five transaction Builder classes:

- [TransactionBuilder](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilder.html) that takes in a Signing Message (serialized raw transaction) and returns a signature.
- [TransactionBuilderEd25519](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderEd25519.html) extends the TransactionBuilder class and provides a signing method for raw transactions with a single public key.
- [TransactionBuilderMultiEd25519](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderMultiEd25519.html) extends the TransactionBuilder class and provides a signing method for signing a raw transaction with a multisignature public key.
- [TransactionBuilderABI](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderABI.html) builds raw transactions based on ABI.
- [TransactionBuilderRemoteABI](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderRemoteABI.html) downloads JSON ABIs from the fullnodes. It then translates the JSON ABIs to the format accepted by TransactionBuilderABI.

The Transaction Builder contains the TypeScript types for constructing the transaction payloads. The Transaction Builder within the TypeScript SDK supports the following transaction payloads:

1. EntryFunction
2. Script

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

### AptosClient class

The [AptosClient](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html) class exposes the methods for retrieving the account resources, transactions, modules and events.

In addition, the `AptosClient` component supports submitting transactions in BCS format, which prepares and signs the raw transactions on the client-side. This method leverages the BCS Library and Transaction Builder for constructing the transaction payloads. See the guide [Creating a Signed Transaction](../../guides/sign-a-transaction.md) for instructions.

You can use the `AptosClient` class directly or the `Provider` class.

### IndexerClient class

The `IndexerClient` class exposes functions to query the [Aptos Indexer](../../guides/indexing.md). The Aptos Indexer fulfills this need, allowing the data shaping critical to real-time app use.

You can use the `IndexerClient` class directly or the `Provider` class.

The `IndexerClient` class is needed because the Aptos Node API provides a lower level, stable and generic API that is not designed to support data shaping and therefore such rich end-user experiences directly.


### TokenClient class

The [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) class provides methods for creating and querying the NFT collections and tokens.
It covers (1) write methods that support creating, transferring, mutating, and burning tokens on-chain and (2) read methods performing deserialization and returning data in TypeScript objects.

The main write methods supported by the Token SDK are:

- Create Collection
- Create Token
- Offer Token
- Claim Token
- Directly Transfer Token
- Transfer Token with Opt-in
- Mutate Token Properties
- Burn Token by Owner or Creator

The main read methods deserializing on-chain data to TS objects are:

- Get CollectionData
- Get TokenData
- Get Token of an Account

## Validation for the Transaction Builder and BCS

The [BCS](https://docs.rs/bcs/latest/bcs/) is used to assemble and serialize the transaction payloads for signing and submission.

Given that different programming languages have different primitive type constraints (e.g., byte length, value range, etc.) and various composite types support (e.g., enum, struct, class, etc.), the code for data serialization is hard to validate.

The Aptos SDK validates the Transaction Builder and BCS in two ways:

1. First, with the unit tests and end-to-end (e2e) tests.

:::tip

An example of unit tests for the BCS serializer can be found in [`serializer.test.ts`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/bcs/serializer.test.ts).

An example of an e2e test for submitting a BCS transaction can be found in [`aptos_client.test.ts`](https://github.com/aptos-labs/aptos-core/blob/f4a7820a61f22ed8306219621402d96f70379d20/ecosystem/typescript/sdk/src/tests/e2e/aptos_client.test.ts#L78).

:::

2. The second level of validation is fuzzing tests with test vectors. The test vectors are produced by the same code used by the Aptos blockchain. The test vectors are arrays of JSON objects. Each JSON object contains randomized inputs and the expected outputs. The Aptos SDKs can parse and load test vectors to validate their implementations of Transaction Builder and BCS.

There are a two test vectors. Each covers one type of transaction payload:

- [EntryFunction](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_entry_function_payload.json) vector
- [Script](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_script_payload.json) vector

Vector items are self-explanatory. However, a special serialization method is used to save space and avoid data overflow. The details are described below:

- All account address are hex-coded.
- `args` in EntryFunction is hex-coded.
- U64 and U128 numbers are serialized as string literals to avoid data truncation.
- U8 is serialized as a number (not a string).
- `code` in Script and ModuleBundle are hex-coded.

:::tip
See the [`transaction_vector.test.ts`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/transaction_builder/transaction_vector.test.ts) code example for how the TypeScript SDK does vector validation.
:::
