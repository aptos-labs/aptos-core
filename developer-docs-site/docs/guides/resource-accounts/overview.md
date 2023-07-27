---
title: "Overview"
id: "overview"
---

# Overview

## What is a resource account?

A [resource account](../../move/move-on-aptos/resource-accounts.md) is an [account](../../concepts/accounts/) that's used to store and manage [resources](../../concepts/resources/) independent of a user. It can be used as a simple storage account or it can be utilized to programmatically manage resources from within a smart contract. 

In a more general sense, the programmable management of resource accounts facilitates a trustless exchange model between two parties. Resource accounts act as trustless, programmable third-party escrows, which are a fundamental building block in the creation of smart contracts.

## How are resource accounts used?

Resource accounts are used in two ways:

1. General resource management, like using them to [publish smart contracts to a non-user account](./managing-resource-accounts#publishing-modules-with-resource-accounts), and
2. Automating resource management in a smart contract by generating the [signer](../../move/book/signer.md) for an account without a private key

Sometimes when writing smart contracts, developers need to programmatically automate [the approval of an account](../../concepts/accounts.md#access-control-with-signers) delegated to resource management in order to create seamless smart contracts that only require one call.

Resource accounts can relinquish their ability to generate a signature with a [private key](https://en.wikipedia.org/wiki/Public-key_cryptography) by delegating it to a [**SignerCapability**](./managing-resource-accounts#using-a-signercapability) resource, which is used to generate an on-chain `signer` primitive for the account.

:::tip
As we may seem to refer to them interchangeably, it's important to note that an account's signature for a transaction is not exactly the same thing as a `signer`. In the context of a Move module (aka a smart contract), a `signer` is the on-chain, primitive data type used to represent an account's approval to process a given transaction, whereas a signature is the off-chain representation of this approval.
:::

Think of it this way: the ***signature*** for a transaction function `public entry fun foo(sender: &signer)` is converted to a ***signer*** and represented as the input argument `sender` in an [entry function.](../../move/book/functions#entry-modifier)

## A real-world use case: a vesting contract

A [vesting contract](https://github.com/aptos-labs/aptos-core/blob/49400cbf0bc63d5e86a54d0c0a1ee2b74c5ea7ec/aptos-move/framework/aptos-framework/sources/vesting.move#L929) is a smart contract that automates the timed disbursement of allotted funds to a designated receiver.

A real world analogous example of this is a trust fund where a child receives money over time from their grandparents, or a compensation package offered by a company that rewards an employee with vested stock options over time.

To explore conceptually how we'd create something like this with a smart contract, let's evaluate the analogous equivalents in terms of who is sending and receiving funds.

In a trust fund, there is a sending party (the grantor), the receiving party (the beneficiary), and the third-party (the trustee) that handles the disbursement of funds. Smart contracts can be employed to programmatically manage funds, effectively acting as the third-party trustee in this example.

The general process for this is:

1. The grantor locks up the initial funds and sets rules on how much the beneficiary can receive over time
2. As time passes, the beneficiary requests funds
3. If there are funds available based on the amount of time that has passed, the funds are transferred to the beneficiary
4. The beneficiary repeats step 3 periodically until the funds are depleted

The two active parties here are the trustee and the beneficiary. Conceptually, we can think of these two parties as two separate accounts on-chain. Managing an account's resources requires the signed approval of the account, meaning a smart contract that facilitates periodically releasing vested funds would require the approval of both the sender and the receiver accounts whenever funds are to be dispersed. 

Requiring multiple signers to call an [entry function](../../move/book/functions/#entry-modifier) is logistically complex not only for the developer but for the end users as well. Both of the users would need to sign off on the transfer transaction every vesting period. Since these would be asynchronous, manual approvals, one user calling the function will always be waiting on the other.

However, with resource accounts, we can integrate programmatic control of these funds, gating access to the funds with the smart contract's internal logic. Whenever the receiver requests to receive funds, the smart contract evaluates if there are any funds to disperse based on time and the amount of funds left. If there are funds ready to be dispersed, the contract internally generates a signer for the resource account holding the funds in order to withdraw them from it.

This internally generated `signer` primitive is created with the [**SignerCapability**](./managing-resource-accounts#using-a-signercapability) resource, thus fully automating the process for the receiver- completing the exchange of funds with only one account's signature!

## More examples of how to use resource accounts
- [Defi swap contract](https://github.com/aptos-labs/aptos-core/blob/c1d87b8a3f17059311d4bdf83b953d12c61c14c0/aptos-move/move-examples/resource_account/sources/simple_defi.move#L39): an automated escrow contract where the resource account acts as an automated and trustless 3rd party escrow.

- [Automated token minting contract](https://github.com/aptos-labs/aptos-core/blob/49400cbf0bc63d5e86a54d0c0a1ee2b74c5ea7ec/aptos-move/move-examples/post_mint_reveal_nft/sources/minting.move#L181C49-L181C65): a minting contract where the resource account is the collection creator and mints/sends out tokens on request.

- [Multi-signature accounts](https://github.com/aptos-labs/aptos-core/blob/4f9b69b6592f58e57691944b888461c2a93ffe7a/aptos-move/framework/aptos-framework/sources/multisig_account.move#L993): using a resource account as a shared account for multiple users, controlled through decentralized voting mechanisms.

- [Coin disbursement through a shared account](https://github.com/aptos-labs/aptos-core/blob/49400cbf0bc63d5e86a54d0c0a1ee2b74c5ea7ec/aptos-move/move-examples/shared_account/sources/shared_account.move#L48): a coin disbursement contract where coins sent to a resource account are distributed to multiple accounts according to a fixed percentage.
 
- [Vesting contract](https://github.com/aptos-labs/aptos-core/blob/49400cbf0bc63d5e86a54d0c0a1ee2b74c5ea7ec/aptos-move/framework/aptos-framework/sources/vesting.move#L929): a vesting contract that disburses a fixed % of coins over a set amount of time.
