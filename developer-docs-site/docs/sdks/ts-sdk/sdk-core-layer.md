---
title: "Core Layer"
slug: "typescript-sdk-core-layer"
---

The core SDK layer exposes the functionalities needed by most applications:

- Key generation
- Transaction signing and submission
- Transaction status querying
- Information retrieval techniques

## Transaction Builder

The Aptos TypeScript SDK exposes five transaction builder classes:

- [TransactionBuilder](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilder.html) that takes in a Signing Message (serialized raw transaction) and returns a signature.
- [TransactionBuilderEd25519](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderEd25519.html) extends the TransactionBuilder class and provides a signing method for raw transactions with a single public key.
- [TransactionBuilderMultiEd25519](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderMultiEd25519.html) extends the TransactionBuilder class and provides a signing method for signing a raw transaction with a multisignature public key.
- [TransactionBuilderABI](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderABI.html) builds raw transactions based on ABI.
- [TransactionBuilderRemoteABI](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderRemoteABI.html) downloads JSON ABIs from the fullnodes. It then translates the JSON ABIs to the format accepted by TransactionBuilderABI.

The Transaction Builder contains the TypeScript types for constructing the transaction payloads. The Transaction Builder within the TypeScript SDK supports the following transaction payloads:

1. Entry Function
2. Script
3. MultiSig Transaction

### Generate transaction

The TypeScript SDK provides 2 efficient ways to generate a raw transaction that can be signed and submitted to chain

1. Using the `generateTransaction()` method. This methods accepts an `entry function payload` type and is available for entry funtion transaction submission. It uses the [TransactionBuilderRemoteABI](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderRemoteABI.html) to fetch the ABI from chain, serializes the payload arguments based on the entry function argument types and generates and return a raw transaction that can be signed and submitted to chain.
2. Using the `generateRawTransaction()` method. This method accept any transaction payload type (entry, script, multisig) and expects the passed in arguments to be serialized. It then generates and returns a raw transaction that can be signed and submitted to chain.

In addition, The Aptos SDK supports transaction status queries (success, failure, pending), by transaction hash.

## AptosAccount class

The [AptosAccount](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosAccount.html) class has a constructor that creates a new account instance or retrieves an existing account instance. Additionally, this class provides the methods for:

- Generating [Ed25519](https://ed25519.cr.yp.to/) key pairs. The Ed25519 public keys can be used to derive the chain account addresses, while the private keys should be kept private for transaction signing.
- Signing a bytes buffer with an Ed25519 public key.
- Deriving initial account addresses from the public keys.
- Retrieving a resource account address by source address and seeds.
- Deriving account address, public key, and private key.
