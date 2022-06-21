---
title: "Aptos SDK"
slug: "aptos-sdk-overview"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos SDK

This document describes the main features and components of the Aptos SDK.

The Aptos SDK provides APIs and interfaces you can use to interact with the Aptos Blockchain by connecting to the Aptos REST API. The REST API is the means for sending your transaction to the Aptos Blockchain.

See below a high-level architecture diagram of the Aptos Typescript SDK.

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/ts-sdk-overview-light.svg'),
    dark: useBaseUrl('/img/docs/ts-sdk-overview-dark.svg'),
  }}
/>

## Key SDK features

The following are a few key features of the Aptos SDK:

- **Key generation:** The Aptos SDKs provide convenient methods for generating [Ed25519](https://ed25519.cr.yp.to/) key pairs. The Ed25519 public keys can be used to derive the chain account addresses, while the private keys should be kept secret for transaction signing.
- **Transaction signing and submission**: Although the Aptos REST APIs support signing a raw transaction on the server-side, signing the transactions on the client side is more secure and should be the preferred choice.
- **Transaction status querying**: The Aptos SDKs support transaction status queries (success, failure, pending), by transaction hash.
- **BCS library:** SDKs implement a [BCS](https://docs.rs/bcs/latest/bcs/) (Binary Canonical Serialization) library for transaction signing and submission. The Aptos Blockchain uses BCS for data serialization and deserialization.
- **Methods for information retrieval**: Resources, modules, and transactions under a specific account can be retrieved with the SDK.
- **Token APIs**: To reduce the amount of work for NFT minting and querying, Aptos SDKs have built-in NFT support in token standards.
- **Faucet client**: The Aptos Faucet client is for minting test coins that are used for development.

## Components of the Typescript SDK

The Aptos Typescript SDK has three logical layers. Refer to the above high-level architecture diagram:

1. The transportation layer.
2. The core SDK layer.
3. The high-level SDK API layer.

The transportation layer is responsible for sending payloads to the REST API endpoints.

The core SDK layer exposes the functionalities needed by most applications, including:

- The key generation.
- Transaction signing and submission.
- Transaction status querying, and
- Various kinds of information retrieval.

The high-level APIs layer leverages the core SDK to provide convenient methods around **NFT tokens.**

### OpenAPI client

The OpenAPI client is a set of classes that are generated based on the Aptos REST API spec. See the [Typescript SDK OpenAPI definition](https://aptos-labs.github.io/ts-sdk-doc/).

### Aptos Account

Provides the methods for:

- Generating Ed25519 key pairs.
- Signing a bytes buffer with an Ed25519 public key, and 
- Deriving initial account addresses from the public keys.

### BCS Library

A subset of BCS standards implemented in Typescript.

### Transaction builder

The transaction builder contains the Typescript types for constructing the transaction payloads. The Typescript SDK supports three kinds of transaction payloads:

1. ScriptFunction
2. Script
3. ModuleBundle

### Aptos Client

The Aptos Client is the main component of Typescript SDK. It exposes the methods for retrieving the account resources, transactions, modules and events.

The Aptos Client component also supports two methods for transaction signing and submission.

1. Submitting transactions in JSON format, which delegates the signing message (input to the signing function) creation to the API server.
2. Submitting transactions in BCS format, which prepares and signs the raw transactions on the client-side. This method leverages the BCS Library and Transaction Builder for constructing the transaction payloads.

:::note

The second method, i.e., in BCS format, is the recommended way for submitting transactions to the Aptos Blockchain.

:::

### Token Client

The Token Client provides methods for the creation, and querying of the NFT collections and tokens.

## Validation for Transaction Builder and BCS

Signing and submitting the BCS transactions is the core functionality of Aptos SDKs. The Transaction Builder and the BCS are used to assemble and serialize the transaction payloads for signing and submission.

Given that different programming languages have different primitive type constraints (e.g., byte length, value range, etc.) and various composite types support (e.g., enum, struct, class, etc.), the code for data serialization is hard to validate.

The Aptos SDK provides two levels of validation for Transaction Builder and BCS.

1. First, with the unit tests and end-to-end (e2e) tests.

:::note

An example of unit tests for the BCS serializer can be found [here](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.test.ts).

An example of an e2e test for submitting a BCS transaction can be found [here](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/aptos_client.test.ts#L88).

:::

2. The second level of validation is fuzzing tests with test vectors. The test vectors are produced by the same code used by the Aptos Blockchain. The test vectors are arrays of JSON objects. Each JSON object contains randomized inputs and the expected outputs. These test vectors can be parsed and loaded by Aptos SDKs to validate their implementations of Transaction Builder and BCS.

There are a total of three test vectors. Each covers one type of transaction payload.

- [ScriptFunction vector](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_script_function_payload.json)
- [Script vector](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_script_payload.json)
- [ModuleBundle vector](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_module_payload.json)

Vector items are self-explanatory. However, special serialization method is used to save space and avoid data overflow. Details are described below:

- All account address are hex-coded.
- `args` in ScriptFunction is hex-coded.
- U64 and U128 numbers are serialized as string literals to avoid data truncation.
- U8 is serialized as a number (not a string).
- `code` in Script and ModuleBundle are hex-coded.

:::tip
See [this code example](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/transaction_builder/transaction_vector.test.ts) for how Typescript SDK does vector validation.
:::
