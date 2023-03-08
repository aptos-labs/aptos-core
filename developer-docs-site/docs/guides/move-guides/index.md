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

- ### [Aptos Move Book](book/SUMMARY.md)
- ### [Move on Aptos](./move-on-aptos.md)
- ### [Gas and Transaction Fees](../../concepts/gas-txn-fee.md)
- ### [How Base Gas Works](../../concepts/base-gas.md)
- ### [Interact with the Move VM](../interacting-with-the-blockchain.md)
- ### [Your First Move module](../../tutorials/first-move-module.md)
- ### [Mint NFT with Aptos CLI](./mint-nft-cli.md)
- ### [Upgrading Move Code](upgrading-move-code.md)
- ### [Aptos Move Examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples)
- ### [End-to-End Aptos Move Tests](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/e2e-move-tests/src/tests)
- ### [Move language channel](https://discord.com/channels/945856774056083548/955573698868432896) in [Aptos Discord](https://discord.gg/aptoslabs).

## Aptos Move reference

Then review our auto-generated contents for more [Move References](../../reference/move.md).

## Move tools

Use these tools to enhance your Move development experience.

### Aptos CLI

The [Aptos command line interface](../../cli-tools/aptos-cli-tool/index.md) (CLI) helps you test development, as many of the functions in our SDKs have corresponding commands.


### Petra Wallet

Although you may use any wallet developed for Aptos, the documents here reflect Petra Wallet. We recommend [installing](../../guides/install-petra-wallet.md) and [using](https://petra.app/docs/petra-intro) it.


### Aptos Simulation API

Use the [Aptos Simulation API](../../concepts/gas-txn-fee.md#estimating-the-gas-units-via-simulation) to test your apps, understanding the blockchain is in an ever-changing state. For example, an auction where people are selling, bidding, and buying will return varying results second by second. Gambling apps may generate wildly different results in a short time. So depending upon your application, you should expect some randomness. So guard your users by keeping simulations realistic.


### Move Prover

Install the [Move Prover](../../cli-tools/install-move-prover.md) dependencies after installing the Aptos CLI If you want to use the Move Prover to validate your Move code.

### Move Debugger

To run the Move Debugger, issue: `MOVE_VM_STEP=1 aptos move test`

Generate the Move Virtual Machine execution trace with: `MOVE_VM_TRACE=1`


### IDEs for Move

Install these IDE plugins for the Move language to gain some handy features:

- [Syntax highlighting for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
- [Move analyzer for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=move.move-analyzer): Supports advanced code navigation and syntax highlighting.
- [Move language plugin for Jetbrains IDEs](https://plugins.jetbrains.com/plugin/14721-move-language): Supports syntax highlighting, code navigation, renames, formatting, type checks and code generation.
- [Remix IDE Plugin](../../community/contributions/remix-ide-plugin.md): Offers a web-based development environment. It is a no-setup tool with a graphical interface for developing Move modules.

## Supporting Move resources

Use these external resources to learn about the core Move programming language.

* [Teach yourself Move on Aptos](https://github.com/econia-labs/teach-yourself-move).
* [Formal Verification, the Move Language, and the Move Prover](https://www.certik.com/resources/blog/2wSOZ3mC55AB6CYol6Q2rP-formal-verification-the-move-language-and-the-move-prover)
* [IMCODING Move Tutorials](https://imcoding.online/courses/move-language)
* [Pontem Move Playground](https://playground.pontem.network/)
* [Collection of nestable Move resources](https://github.com/taoheorg/taohe)
* [Move-Lang tag on Stack Overflow](https://stackoverflow.com/questions/tagged/move-lang)
* [Move Book](https://move-language.github.io/move/)
* [Move Tutorial](https://github.com/move-language/move/tree/main/language/documentation/tutorial)
* [Move language repository](https://github.com/move-language/move)
* [Move by example](https://move-book.com/)
* [Awesome Move resources](https://github.com/MystenLabs/awesome-move)

Add your own recommended Move resources here. Simply click *Edit this page* below to go to the source and trigger editing there. See [Markdown syntax](https://www.markdownguide.org/basic-syntax/) for help.
