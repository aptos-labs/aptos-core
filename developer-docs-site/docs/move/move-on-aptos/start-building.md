---
title: "Start Building"
slug: "start-building"
---

# Start Building

To start developing smart contracts on the Aptos blockchain, we recommend the following resources:

- [The Move Book](../book/SUMMARY.md)
- [Aptos CLI Move commands](../../tools/aptos-cli-tool/use-aptos-cli.md#move-examples)
- [Aptos Move Examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples)
- [End-to-End Aptos Move Tests](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/e2e-move-tests/src/tests)
- [Move language channel](https://discord.com/channels/945856774056083548/955573698868432896) in [Aptos Discord](https://discord.gg/aptoslabs).
- [Aptos Move Framework](../../reference/move.md).

## Move tools

Use these tools to enhance your Move development experience.

### Aptos CLI

The [Aptos command line interface](../../tools/install-cli/index.md) (CLI) helps you test development, as many of the functions in our SDKs have corresponding commands.


### Aptos Simulation API

Use the [Aptos Simulation API](../../concepts/gas-txn-fee.md#estimating-the-gas-units-via-simulation) to test your apps, understanding the blockchain is in an ever-changing state. For example, an auction where people are selling, bidding, and buying will return varying results second by second. Gambling apps may generate wildly different results in a short time. So depending upon your application, you should expect some randomness. So guard your users by keeping simulations realistic.

### Move Prover

Install the [Move Prover](../../tools/install-cli/install-move-prover.md) dependencies after installing the Aptos CLI If you want to use the Move Prover to validate your Move code.

### Move Debugger

To run the Move Debugger, issue: `MOVE_VM_STEP=1 aptos move test`

Generate the Move Virtual Machine execution trace with: `MOVE_VM_TRACE=1`

### IDEs for Move

Install these IDE plugins for the Move language to gain some handy features:

- [Syntax highlighting for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
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
* [Move language repository](https://github.com/move-language/move)
