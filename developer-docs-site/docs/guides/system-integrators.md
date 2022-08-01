---
title: "System Integrators Guide"
slug: "system-integrators"
---

# System Integrators Guide

This guide will walk you through all you need to successfully integrate with the Aptos blockchain. This guide assumes familiarity with blockchains, command-line interfaces, and other concepts discussed throughout the developer site. Where possible links will be made to relevant topics.

An integrator may follow some or all of these steps:
* Create an account on the blockchain
* Exchange account identifiers with another entity on the blockchain to perform swaps, for example
* Create a transaction
* Obtain a gas estimate and validate the transaction for correctness
* Ask the user to verify the intent and cost of the transaction
* Submit the transaction to the blockchain
* Wait for the outcome of the transaction
* Show the completed transaction to the user
* The ability to see all interactions for a given account with a specific account -- that is withdraws and deposits

## Accounts on Aptos

An [account](../concepts/basics-accounts) represents a resource on the Aptos Blockchain that can send transactions. Each account is identified by a particular 32-byte account address and is a container for Move modules and Move resources. On Aptos, accounts must be created on-chain prior to any operations involving that account. With the caveat that the framework supports implicitly creating accounts when tranfering Aptos coin via `account::transfer`.

Account addresses are defined at creation time as a one way function from the public key(s) and signature algorithm used for authentication for the account. This is covered in depth in [account](..concepts/basics-accounts) as well as demonstrated in the [Typescript SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_account.ts#L66) and [Python SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/account_address.py#L43) -- note currently both these SDKs only demonstrate how to generate an address from an Ed25519 single signer.

An Account on Aptos has the ability to rotate keys so that potentially compromised keys cannot be used to access accounts. Refreshing keys has been generally regarded as good hygiene in the security field. This presents a challenge for system integrators who are familiar with using a mnemonic to represent both a private key and its associated account. Before launch, Aptos will have an on-chain mapping that makes to maintain this simplicity. The on-chain data maps an effective account address as defined by the current mnemonic to the actual account address. The details are forthcoming.

Currently, Aptos only supports a single, unified identifier for an account. Accounts on Aptos are universally regarded as a 32-byte hex string. A hex string shorter than 32-bytes should be consider padded with leading zeroes, e.g., 0x1 => 0x0000000000000...01.

## A (Not So) Brief Overview of Transactions

Aptos [transactions](../concepts/basics-txns-states) are encoded in [*BCS*](https://github.com/diem/bcs) or *Binary Canonical Serialization*. Transactions contain relevant information such as the sender, authentication from the sender, the desired operation to perform on the blockchain, and the amount of gas the sender is willing to pay to execute the transaction.

A transaction may end in one of the following states:
1. Committed on the blockchain and executed, this would be considered a successful transaction
2. Committed on the blockchain and aborted, the abort code would indicate why the transaction failed to execute
3. Discarded during transaction submission due to a validation check such as insufficient gas, invalid transaction format, or incorrect key
4. Discarded after transaction submission but before attempted execution, this could be due to timeouts or insufficient gas due to other transactions affecting the account

The account will be charged gas for any committed transactions. During transaction submission, the submitter is notified of successful submission or a reason for failing validations otherwise. A transaction that is successfully submitted but ultimately discarded may have no visible state in any accessible node or within the network. A user can attempt to resubmit the same transaction to re-validate the transaction. If the submitting node believes that this transaction is still valid, this will return an error stating that there exists an identical transaction already submitted. The submitter can try to increase the gas cost by a trivial amount to help make progress and adjust for whatever may have been causing the dicarding of the transaction further downstream. On Devnet, the finalization time for a transaction is within seconds. The entire lifecycle of a transaction is covered [here](../guides/basics-life-of-txn).

Aptos supports two methods for constructing transactions:
* Constructing JSON-encoded objects and interacting with the Web API to generate native transactions
* Client libraries to generate native transactions

JSON-encoded transactions can be generated via the [REST API](https://aptos.dev/rest-api). First construct an appropriate JSON payload for the `/transactions/signing_message` endpoint as demonstrated in the [Python SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L111). The output of this contains an object containing a `message`, this must be signed with the private key locally. Finally, the original JSON payload is extended with the relevant signature information and posted to the [`/transactions` endpoint](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L127). This returns back a transaction submission result that if successful contains a transaction hash in the [`hash` field](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L138).

JSON-encoded transactions allow for rapid development and support seamless ABI conversions of transaction arguments to native types. However, most system integrators prefer to generate transactions within their own tech stack. Both the [TypeScript SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_client.ts#L259) and [Python SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L202) support generating BCS transactions. BCS encoded transactions can be submitted to the `/transactions` endpoint but must specify `Content-Type: application/x.aptos.signed_transaction+bcs` in the HTTP headers. This returns back a transaction submission result that if successful contains a transaction hash in the [`hash` field](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L138).

Transactions status can be obtained by querying the API `/transactions/{hash}` with the hash returned during submission. A reasonable strategy for submitting transactions is to limit their lifetime to 30 to 60 seconds and polling that API at regular intervals until success or a several seconds after that time has elapsed. If there's no commitment on-chain, the transaction likely was discarded.

Please bear with us as our documentation is currently undergoing an overhaul. We have the following support for generating valid transactions:
* [JSON encoded transactions](../tutorials/your-first-transaction) via Typescript, Python, and Rust
* [Python example of BCS encoded coin transfers](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/python/sdk/aptos_sdk/client.py#L240)
* [Typescript example of BCS encoded coin transfers](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_client.test.ts#L122)
* [CLI-based transaction publishing](../cli-tools/aptos-cli-tool/use-aptos-cli#publishing-a-move-package-with-a-named-address)
* [Introductory module publishing](../tutorials/your-first-move-module) via Typescript, Python, and Rust

To facilitate evaluation of transactions, Aptos also supports a simulation API that does not require and should not contain valid signatures on transactions. It works identically to the transaction submission API, except that it executes the transaction and returns back the results along with the gas used. It can be accessed by submitting a transaction to `/transactions/simulate`. Here's an example in the [Typescript SDK](https://github.com/aptos-labs/aptos-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/aptos_client.ts#L413). Note: the gas use may change based upon the state of the account, it is recommended that the maximum gas amount be larger than the amount quoted by this API. Further understanding is pending to make a stronger recommendation.

## Viewing Transactions and Transaction History
