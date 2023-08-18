---
title: "Move on Aptos"
slug: "move-on-aptos"
---

# Move on Aptos

The Aptos blockchain consists of validator nodes that run a consensus protocol. The consensus protocol agrees upon the ordering of transactions and their output when executed on the Move Virtual Machine (MoveVM). Each validator node translates transactions along with the current blockchain ledger state as input into the VM. The MoveVM processes this input to produce a changeset or storage delta as output. Once consensus agrees and commits to the output, it becomes publicly visible. In this guide, we will introduce you to core Move concepts and how they apply to developing on Aptos.

## What is Move?

Move is a safe and secure programming language for Web3 that emphasizes **scarcity** and **access control**. Any assets in Move can be represented by or stored within *resource*. **Scarcity** is enforced by default as structs cannot be accidentally duplicated or dropped. Only structs that have explicitly been defined at the bytecode layer as *copy* can be duplicated and *drop* can be dropped, respectively.

**Access control** comes from both the notion of accounts as well as module access privileges. A module in Move may either be a library or a program that can create, store, or transfer assets. Move ensures that only public module functions may be accessed by other modules. Unless a struct has a public constructor, it can only be constructed within the module that defines it. Similarly, fields within a struct can only be accessed and mutated within its module that or via public accessors and setters. Furthermore, structs defined with *key* can be stored and read from global storage only within the module defines it. Structs with *store* can be stored within another *store* or *key* struct inside or outside the module that defines that struct.

In Move, a transaction's sender is represented by a *signer*, a verified owner of a specific account. The signer has the highest level of permission in Move and is the only entity capable of adding resources into an account. In addition, a module developer can require that a signer be present to access resources or modify assets stored within an account.

## Comparison to other VMs

| | Aptos / Move | Solana / SeaLevel | EVM | Sui / Move |
|---|---|---|---|---|
| Data storage | Stored at a global address or within the owner's account | Stored within the owner's account associated with a program | Stored within the account associated with a smart contract | Stored at a global address |
| Parallelization | Capable of inferring parallelization at runtime within Aptos | Requires specifying all data accessed | Currently serial nothing in production | Requires specifying all data accessed |
| Transaction safety | Sequence number | Transaction uniqueness | nonces, similar to sequence numbers | Transaction uniqueness |
| Type safety | Module structs and generics | Program structs | Contract types | Module structs and generics |
| Function calling | Static dispatch | Static dispatch | Dynamic dispatch | Static dispatch |
| Authenticated Storage | [Yes](../reference/glossary.md#merkle-trees) | No | Yes | No |
| Object accessibility | Guaranteed to be globally accessible | Not applicable | Not applicable | Can be hidden |

## Aptos Move features

Each deployment of the MoveVM has the ability to extend the core MoveVM with additional features via an adapter layer. Furthermore, MoveVM has a framework to support standard operations much like a computer has an operating system.

The Aptos Move adapter features include:
* [Move Objects](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-10.md) that offer an extensible programming model for globally access to heterogeneous set of resources stored at a single address on-chain.
* [Resource accounts](./move-on-aptos/resource-accounts) that offer programmable accounts on-chain, which can be useful for DAOs (decentralized autonomous organizations), shared accounts, or building complex applications on-chain.
* [Tables](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/table.move) for storing key, value data within an account at scale
* Parallelism via [Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba) that enables concurrent execution of transactions without any input from the user

The Aptos framework ships with many useful libraries:
* A [Token standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) that makes it possible to create NFTs and other rich tokens without publishing a smart contract
* A [Coin standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) that makes it possible to create type-safe Coins by publishing a trivial module
* A staking and delegation framework
* A [`type_of`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/type_info.move) service to identify at run-time the address, module, and struct name of a given type
* Multi-signer framework that allows multiple `signer` entities
* A [timestamp service](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/timestamp.move) that provides a monotonically increasing clock that maps to the actual current unixtime

With updates frequently.

## More Resources

To start developing smart contracts on the Aptos blockchain, we recommend the following resources:

- [Aptos Move Examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples)
- [End-to-End Aptos Move Tests](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/e2e-move-tests/src/tests)
- [Move language channel](https://discord.com/channels/945856774056083548/955573698868432896) in [Aptos Discord](https://discord.gg/aptosnetwork).
- [Aptos Move Framework](../reference/move.md).

There are several IDE plugins available for Aptos and the Move language:

- [Syntax highlighting for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
- [Move language plugin for Jetbrains IDEs](https://plugins.jetbrains.com/plugin/14721-move-language): Supports syntax highlighting, code navigation, renames, formatting, type checks and code generation.
- [Remix IDE Plugin](../community/contributions/remix-ide-plugin.md): Offers a web-based development environment. It is a no-setup tool with a graphical interface for developing Move modules.

Use these external resources to learn about the Move programming language:

* [Teach yourself Move on Aptos](https://github.com/econia-labs/teach-yourself-move).
* [Formal Verification, the Move Language, and the Move Prover](https://www.certik.com/resources/blog/2wSOZ3mC55AB6CYol6Q2rP-formal-verification-the-move-language-and-the-move-prover)
* [IMCODING Move Tutorials](https://www.imcoding.online/tutorials?tag=Aptos)
* [Pontem Move Playground](https://playground.pontem.network/)
* [Collection of nestable Move resources](https://github.com/taoheorg/taohe)
* [Move-Lang tag on Stack Overflow](https://stackoverflow.com/questions/tagged/move-lang)
