---
title: "Write Move Smart Contracts"
slug: "aptos-move-guides"
---

# Write Smart Contracts with Move

To efficiently write smart contracts in Aptos, we recommend you first:

1. Learn Move concepts
1. Understand Aptos security
1. Create a collection
1. Create an NFT
1. Create resource account

## Aptos Move guides

Start here to learn how the Move language works on the Aptos blockchain. 

- ### [Move on Aptos](move-on-aptos.md)
- ### [Gas and Transaction Fees](../../concepts/gas-txn-fee.md)
- ### [How Base Gas Works](../../concepts/base-gas.md)
- ### [Interact with the Move VM](../interacting-with-the-blockchain.md)
- ### [Your First Move module](../../tutorials/first-move-module.md)
- ### [Mint NFT with Aptos CLI](./mint-nft-cli.md)
- ### [Upgrading Move Code](upgrading-move-code.md)
- ### [Aptos Move Examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples)
- ### [End-to-End Aptos Move Tests](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/e2e-move-tests/src/tests)

## Aptos Move reference

Then review our auto-generated contents for more [Move References](../../reference/move.md).

## Move tools

## Tools

Use these tools to enhance your Move development experience.

### Aptos CLI

The [Aptos command line interface](https://aptos.dev/cli-tools/aptos-cli-tool/) (CLI) helps you test development iteratively, as many of the functions in our SDKs have corresponding commands.


### Petra Wallet

Although you may use any wallet developed for Aptos, the documents here reflect Petra Wallet. We recommend [installing](https://aptos.dev/guides/install-petra-wallet-extension) and [using](https://petra.app/docs/petra-intro) it.


### Aptos Simulation API

Use the [Aptos Simulation API](https://aptos.dev/concepts/gas-txn-fee/#estimating-the-gas-units-via-simulation) to test your apps, understanding the blockchain is in an ever-changing state. For example, an auction where people are selling, bidding, and buying will return varying results second by second. Gambling apps may generate wildly different results in a short time. So depending upon your application, you should expect some randomness. So guard your users by keeping simulations realistic.


### Move Prover

Install the [Move Prover](https://aptos.dev/cli-tools/install-move-prover) dependencies after installing the Aptos CLI If you want to use the Move Prover to validate your Move code.


### IDEs for Move

Install the [IDE plugins for the Move language](https://aptos.dev/guides/getting-started#ide-plugins-for-move-language) for even more handy features.

TODO: Consider moving from Getting Started to here and instead link from there.

## Supporting Move resources

Use these external resources to learn about the core Move programming language.

* [IMCODING Move Tutorials](https://imcoding.online/courses/move-language)
* [Pontem Move Playground](https://playground.pontem.network/)
* [Collection of nestable Move resources](https://github.com/taoheorg/taohe)
* [Move-Lang tag on Stack Overflow](https://stackoverflow.com/questions/tagged/move-lang)
* [Move Book](https://move-language.github.io/move/)
* [Move Tutorial](https://github.com/move-language/move/tree/main/language/documentation/tutorial)
* [Move language repository](https://github.com/move-language/move)
* [Move by example](https://move-book.com/)
