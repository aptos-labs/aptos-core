---
title: "Typescript SDK Overview"
slug: "typescript-sdk-overview"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Typescript SDK Overview

This document describes the main features and components of the Aptos Typescript SDK.

The Aptos Typescript SDK provides APIs and interfaces you can use to interact with the Aptos blockchain by connecting to the Aptos REST API. The REST API is the means for sending your transaction to the Aptos blockchain and reading the blockchain's state.

See below a high-level architecture diagram of the Aptos Typescript SDK.

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/ts-sdk-light.svg'),
    dark: useBaseUrl('/img/docs/ts-sdk-dark.svg'),
  }}
/>

## Key SDK features

The following are a few key features of the Aptos SDK:

- **Key generation:** The Aptos SDK provides convenient methods for generating [Ed25519](https://ed25519.cr.yp.to/) key pairs. The Ed25519 public keys can be used to derive the chain account addresses, while the private keys should be kept private for transaction signing. See the [class TransactionBuilderEd25519](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderEd25519.html).
- **Transaction signing and submission**: Although the Aptos REST APIs support signing a raw transaction on the server-side, signing the transactions on the client side, using the Aptos SDK, is more secure and should be the preferred choice.
- **Transaction status querying**: The Aptos SDK supports transaction status queries (success, failure, pending), by transaction hash.
- **BCS library:** The Aptos SDK implements a [BCS](https://docs.rs/bcs/latest/bcs/) (Binary Canonical Serialization) library for transaction signing and submission. The Aptos blockchain uses BCS for data serialization and deserialization. See [Aptos SDK BCS](https://aptos-labs.github.io/ts-sdk-doc/modules/BCS.html).
- **Methods for information retrieval**: Resources, modules, and transactions under a specific account can be retrieved with the Aptos SDK.
- **Faucet client**: The Aptos [FaucetClient](https://aptos-labs.github.io/ts-sdk-doc/classes/FaucetClient.html) is for minting test coins that are used for development.
- **Token client**: Aptos SDK provides built-in support for NFT minting and querying. See [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html).

## Components of the Typescript SDK

The Aptos Typescript SDK has three logical layers. Refer to the above high-level architecture diagram:

1. The transport layer.
2. The core SDK layer.
3. An optional application layer.

The transportation layer is responsible for sending payloads to the REST API endpoints.

The core SDK layer exposes the functionalities needed by most applications, including:

- The key generation.
- Transaction signing and submission.
- Transaction status querying, and
- Various kinds of information retrieval.

The optional application layer provides built-in support for **NFT token** API.

:::tip
You can also use this [TokenClient API](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) as an example of NFT token API before you start developing your own application APIs using the SDK.
:::

### OpenAPI client

The OpenAPI client is a set of classes that are generated based on the Aptos REST API spec. See the [Typescript SDK OpenAPI definition](https://aptos-labs.github.io/ts-sdk-doc/).

### Aptos Account

The [class AptosAccount](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosAccount.html) provides the methods for:

- Generating Ed25519 key pairs.
- Signing a bytes buffer with an Ed25519 public key, and
- Deriving initial account addresses from the public keys.

### BCS Library

A subset of BCS standards implemented in Typescript.

### Transaction builder

The transaction builder contains the Typescript types for constructing the transaction payloads. The transaction builder within the Typescript SDK supports the following transaction payloads:

1. EntryFunction
2. Script

### Aptos Client

The [class AptosClient](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html) exposes the methods for retrieving the account resources, transactions, modules and events.

In addition, the `AptosClient` component supports two methods for transaction signing and submission.

1. Submitting transactions in JSON format, which delegates the signing message (input to the signing function) creation to the API server. This is applicable when using the REST API and the Aptos server to generate the signing message, the transaction signature and submit the signed transaction to the Aptos blockchain. See the tutorial [Your First Transaction](/tutorials/first-transaction.md).
2. Submitting transactions in BCS format, which prepares and signs the raw transactions on the client-side. This method leverages the BCS Library and Transaction Builder for constructing the transaction payloads. See the guide [Creating a Signed Transaction](/guides/sign-a-transaction.md).

:::tip

The second method, i.e., in BCS format, is the recommended way for submitting transactions to the Aptos blockchain.

:::

### Token Client

The class [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) provides methods for creating and querying the NFT collections and tokens.

## Validation for Transaction Builder and BCS

The [BCS](https://docs.rs/bcs/latest/bcs/) is used to assemble and serialize the transaction payloads for signing and submission.

Given that different programming languages have different primitive type constraints (e.g., byte length, value range, etc.) and various composite types support (e.g., enum, struct, class, etc.), the code for data serialization is hard to validate.

The Aptos SDK validates the Transaction Builder and BCS in two ways:

1. First, with the unit tests and end-to-end (e2e) tests.

:::tip

An example of unit tests for the BCS serializer can be found [here](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.test.ts).

An example of an e2e test for submitting a BCS transaction can be found [here](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/aptos_client.test.ts#L88).

:::

2. The second level of validation is fuzzing tests with test vectors. The test vectors are produced by the same code used by the Aptos blockchain. The test vectors are arrays of JSON objects. Each JSON object contains randomized inputs and the expected outputs. These test vectors can be parsed and loaded by Aptos SDKs to validate their implementations of Transaction Builder and BCS.

There are a total of three test vectors. Each covers one type of transaction payload.

- [EntryFunction vector](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_entry_function_payload.json)
- [Script vector](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_script_payload.json)

Vector items are self-explanatory. However, special serialization method is used to save space and avoid data overflow. Details are described below:

- All account address are hex-coded.
- `args` in EntryFunction is hex-coded.
- U64 and U128 numbers are serialized as string literals to avoid data truncation.
- U8 is serialized as a number (not a string).
- `code` in Script and ModuleBundle are hex-coded.

:::tip
See [this code example](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/transaction_builder/transaction_vector.test.ts) for how Typescript SDK does vector validation.
:::
