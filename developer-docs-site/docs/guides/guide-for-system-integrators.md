---
title: "System Integrators' Guide"
slug: "guide-for-system-integrators"
---

:::tip 
This is documentation is currently under construction with more being added on a regular basis.
:::

# System Integrators' Guide

If you provide blockchain services to your customers and wish to add the Aptos blockchain to your platform, then this guide is for you. This system integrators guide will walk you through all you need to integrate the Aptos blockchain into your platform. This guide assumes that you are familiar with the blockchains.

## Overview

This guide will overview the following core concepts for integrating with Aptos:
- Create an account on the blockchain.
- Exchange account identifiers with another entity on the blockchain, for example, to perform swaps.
- Create a transaction.
- Obtain a gas estimate and validate the transaction for correctness.
- Submit the transaction to the blockchain.
- Wait for the outcome of the transaction.
- Query historical transactions and interactions for a given account with a specific account, i.e., withdraws and deposits.

## Accounts on Aptos

An [account](https://aptos.dev/concepts/basics-accounts) represents a resource on the Aptos blockchain that can send transactions. Each account is identified by a particular 32-byte account address and is a container for Move modules and Move resources. On Aptos, accounts must be created on-chain prior to any blockchain operations involving that account. The Aptos framework supports implicitly creating accounts when transferring Aptos coin via `account::transfer` or explicitly via `account::create_account`.

At creation, an [Aptos account]() contains:
* A [resource containing Aptos Coin]() and deposit and withdrawal of coins from that resource.
* An authentication key associated with their current public, private key(s).
* A strictly increasing sequence number that represents the account's next transaction's sequence number to prevent replay attacks.
* An event stream for all new types of coins added to the account.

### Account identifiers

Currently, Aptos only supports a single, unified identifier for an account. Accounts on Aptos are universally represented as a 32-byte hex string. A hex string shorter than 32-bytes should is also valid, in those scenarios, the the hex string is padded with leading zeroes, e.g., `0x1` => `0x0000000000000...01`.

### Creating an account address

Account addresses are defined at creation time as a one-way function from the public key(s) and signature algorithm used for authentication for the account.

:::tip Read more
This is covered in depth in the [Accounts](https://aptos.dev/concepts/basics-accounts/) documentation and demonstrated in the [Typescript SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_account.ts#L66) and [Python SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/account_address.py#L43). Note that currently these SDKs only demonstrate how to generate an address from an Ed25519 single signer.
:::

### Rotating the keys

An Account on Aptos has the ability to rotate keys so that potentially compromised keys cannot be used to access the accounts. 

:::tip Read more
See more in [Account address](http://aptos.dev/concepts/basics-accounts#account-address).
:::

Refreshing the keys is generally regarded as good hygiene in the security field. However, this presents a challenge for system integrators who are used to using a mnemonic to represent both a private key and its associated account. To simplify this for the system integrators, Aptos will provide an on-chain mapping, before the launch of the mainnet. The on-chain data maps an effective account address as defined by the current mnemonic to the actual account address. 

### Preventing replay attacks

When the Aptos blockchain processes the transaction, it looks at the sequence number in the transaction and compares it with the sequence number in the sender’s account (as stored on the blockchain at the current ledger version). The transaction is executed only if the sequence number in the transaction is the same as the sequence number for the sender account, and rejects if they do not match. In this way past transactions, which necessarily contain older sequence numbers, cannot be replayed, hence preventing replay attacks.

:::tip Read more
See more on [Account sequence number here](http://aptos.dev/concepts/basics-accounts#account-sequence-number).
:::

## Transactions

Aptos [transactions](http://aptos.dev/concepts/basics-txns-states) are encoded in [BCS](https://github.com/diem/bcs) (Binary Canonical Serialization). Transactions contain  information such as the sender’s account address, authentication from the sender, the desired operation to be performed on the Aptos blockchain, and the amount of gas the sender is willing to pay to execute the transaction.

### Transaction states

A transaction may end in one of the following states:

1. Committed on the blockchain and executed. This is considered as a successful transaction.
2. Committed on the blockchain and aborted. The abort code indicates why the transaction failed to execute.
3. Discarded during transaction submission due to a validation check such as insufficient gas, invalid transaction format, or incorrect key.
4. Discarded after transaction submission but before attempted execution. This could be due to timeouts or insufficient gas due to other transactions affecting the account.

The sender’s account will be charged gas for any committed transactions. 

During transaction submission, the submitter is notified of successful submission or a reason for failing validations otherwise. 

A transaction that is successfully submitted but ultimately discarded may have no visible state in any accessible Aptos node or within the Aptos network. A user can attempt to resubmit the same transaction to re-validate the transaction. If the submitting node believes that this transaction is still valid, this will return an error stating that there exists an identical transaction already submitted. 

The submitter can try to increase the gas cost by a trivial amount to help make progress and adjust for whatever may have been causing the discarding of the transaction further downstream. 

On the Aptos devnet, the finalization time for a transaction is within seconds. 

:::tip Read more
See [here for a comprehensive description of the transaction lifecycle](https://aptos.dev/guides/basics-life-of-txn).
:::

### Constructing a transaction

Aptos supports two methods for constructing transactions:

- Constructing JSON-encoded objects and interacting with the Web API to generate native transactions.
- Using the Aptos client libraries to generate native transactions.

#### JSON-encoded transactions

JSON-encoded transactions can be generated via the [REST API](https://aptos.dev/rest-api), following these steps:

- First construct an appropriate JSON payload for the `/transactions/signing_message` endpoint as demonstrated in the [Python SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L111).
- The output of the above contains an object containing a `message` and this must be signed with the sender’s private key locally.
- Finally, the original JSON payload is extended with the signature information and posted to the `/transactions` [endpoint](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L127). This will return back a transaction submission result that, if successful, contains a transaction hash in the `hash` [field](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L138).

JSON-encoded transactions allow for rapid development and support seamless ABI conversions of transaction arguments to native types. However, most system integrators prefer to generate transactions within their own tech stack. Both the [TypeScript SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_client.ts#L259) and [Python SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L202) support generating BCS transactions. 

#### BCS-encoded transactions

BCS encoded transactions can be submitted to the `/transactions` endpoint but must specify `Content-Type: application/x.aptos.signed_transaction+bcs` in the HTTP headers. This will return back a transaction submission result that, if successful, contains a transaction hash in the `[hash` [field](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L138).

#### Status of a transaction

Transaction status can be obtained by querying the API `/transactions/{hash}` with the hash returned during the submission of the transaction. 

A reasonable strategy for submitting transactions is to limit their lifetime to 30 to 60 seconds, and polling that API at regular intervals until success or a several seconds after that time has elapsed. If there is no commitment on-chain, the transaction was likely discarded.

:::tip Read more

See the following documentation for generating valid transactions:

- [JSON encoded transactions](http://aptos.dev/tutorials/your-first-transaction) via Typescript, Python, and Rust.
- [Python example of BCS encoded coin transfers](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L240).
- [Typescript example of BCS encoded coin transfers](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_client.test.ts#L122).
- [CLI-based transaction publishing](http://aptos.dev/cli-tools/aptos-cli-tool/use-aptos-cli#publishing-a-move-package-with-a-named-address).
- [Publish your first Move module](https://aptos.dev/tutorials/your-first-move-module) via Typescript, Python, and Rust.

:::

### Evaluating transactions

To facilitate evaluation of transactions, Aptos supports a simulation API that does not require and should not contain valid signatures on transactions. 

The simulation API works identical to the transaction submission API, except that it executes the transaction and returns back the results along with the gas used. The simulation API can be accessed by submitting a transaction to `/transactions/simulate`. 

:::tip Read more
Here's an example showing how to use the simulation API in the [Typescript SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_client.ts#L413). Note that the gas use may change based upon the state of the account. We recommend that the maximum gas amount be larger than the amount quoted by this API.
:::
