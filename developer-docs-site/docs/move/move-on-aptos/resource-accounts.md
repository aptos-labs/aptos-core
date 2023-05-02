---
title: "Resource Accounts"
slug: "resource-accounts"
---

# Resource Accounts

A [resource account](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move) is a developer feature used to manage resources independent of an account managed by a user, specifically publishing modules and automatically signing for transactions. For example, a developer may use a resource account to manage an account for module publishing, say managing a contract. The contract itself does not require a signer post initialization. A resource account gives you the means for the module to provide a signer to other modules and sign transactions on behalf of the module.

Typically, a resource account is used for two main purposes:

* Store and isolate resources; a module creates a resource account just to host specific resources.
* Publish module as a standalone (resource) account, a building block in a decentralized design where no private keys can control the resource account. The ownership (SignerCap) can be kept in another module, such as governance.

## Restrictions

In Aptos, a resource account is created based upon the SHA3-256 hash of the source's address and additional seed data. A resource account can be created only once; for a given source address and seed, there can be only one resource account. That is because the calculation of the resource account address is fully determined by the former.

An entity may call `create_account` in an attempt to claim an account ahead of the creation of a resource account. But if a resource account is found, Aptos will transition ownership of the account over to the resource account. This is done by validating that the account has yet to execute any transactions and that the `Account::signer_capbility_offer::for` is none. The probability of a collision where someone has legitimately produced a private key that maps to a resource account address is improbably low.

## Setup

The easiest way to set up a resource account is by:

1. Using Aptos CLI: `aptos account create-resource-account` creates a resource account, and `aptos move create-resource-account-and-publish-package` creates a resource account and publishes the specified package under the resource account's adddress.
1. Writing custom smart contracts code: in the `resource_account.move` module, developers can find the resource account creation functions `create_resource_account`, `create_resource_account_and_fund`, and `create_resource_account_and_publish_package`. Developers can then call those functions to create resource accounts in their smart contracts.

Each of those options offers slightly different functionality:
* `create_resource_account` - merely creates the resource account but doesn't fund it, retaining access to the resource account's signer until explicitly calling `retrieve_resource_account_cap`.
* `create_resource_account_and_fund` - creates the resource account and funds it, retaining access to the resource account's signer until explicitly calling `retrieve_resource_account_cap`.
* `create_resource_account_and_publish_package` - creates the resource account and results in loss of access to the resource account by design, because resource accounts are used to make contracts autonomous and immutable.

In this example, you will [initialize](https://github.com/aptos-labs/aptos-core/blob/2e9d8ee759fcd3f6e831034f05c1656b1c48efc4/aptos-move/move-examples/mint_nft/sources/minting.move#L73) the `mint_nft` module and retrieve the signer capability from both the resource account and module account. To do so, call `create_resource_account_and_publish_package` to publish the module under the resource account's address.

1. Initialize the module as shown in the [`minting.move`](https://github.com/aptos-labs/aptos-core/blob/2e9d8ee759fcd3f6e831034f05c1656b1c48efc4/aptos-move/move-examples/mint_nft/sources/minting.move#L73) example.
1. Call `create_resource_account_and_publish_package` to publish the module under the resource account's address, such as in the [`mint_nft.rs`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/e2e-move-tests/src/tests/mint_nft.rs#L62) end-to-end example.
1. Retrieve the signer cap from the resource account + module account as shown in the [`minting.move`](https://github.com/aptos-labs/aptos-core/blob/2e9d8ee759fcd3f6e831034f05c1656b1c48efc4/aptos-move/move-examples/mint_nft/sources/minting.move#L83) example.

Note, if the above `resource_account` signer is **not** already set up as a resource account, retrieving the signer cap will fail. The `source_addr` field in the `retrieve_resource_account_cap` function refers to the the address of the source account, or the account that creates the resource account.

For an example, see the `SignerCapability` employed by the `mint_nft` function in [`minting.move`](https://github.com/aptos-labs/aptos-core/blob/2e9d8ee759fcd3f6e831034f05c1656b1c48efc4/aptos-move/move-examples/mint_nft/sources/minting.move#L143-L181).

For more details, see the "resource account" references in [`resource_account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move) and [`account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move).
