---
title: "Move on Aptos"
slug: "move-on-aptos"
---

# Move on Aptos

The Aptos blockchain consists of validator nodes that run a consensus protocol. The consensus protocol agrees upon the ordering of transactions and their output when executed on the Move Virtual Machine (MoveVM). Each validator node translates transactions along with the current blockchain ledger state as input into the VM. The MoveVM processes this input to produce a changeset or storage delta as output. Once consensus agrees and commits to the output, it becomes publicly visible. In this guide, we will introduce you to core Move concepts and how they apply to developing on Aptos.

## What is Move?

Move is a safe and secure programming language for Web3 that emphasizes **scarcity** and **access control**. An assets in Move can be represented by or stored within *resource*. **Scarcity** is enforced by default as structs cannot be duplicated. Only structs that have explicitly been defined at the bytecode layer as *copy* can be duplicated.

**Access control** comes from both the notion of accounts as well as module access privileges. A module in Move may either be a library or a program that can create, store, or transfer assets. Move ensures that only public module functions may be accessed by other modules. Unless a struct has a public constructor, it can only be constructed within the module that defines it. Similarly, fields within a struct can only be accessed and mutated within its module that or via public accessors and setters.

In Move, a transaction's sender is represented by a *signer*, a verified owner of a specific account. The signer has the highest level of permission in Move and is the only entity capable of adding resources into an account. In addition, a module developer can require that a signer be present to access resources or modify assets stored within an account.

## Comparison to other VMs

| | Aptos / Move | Solana / SeaLevel | EVM |
|---|---|---|---|
| Data storage | Stored within the owner's account | Stored within the owner's account associated with a program | Stored within the account associated with a smart contract |
| Parallelization | Capable of inferring parallelization at runtime within Aptos | Requires specifying within the transaction all accounts and programs accessed | Currently serial nothing in production | 
| Transaction safety | Sequence number | Transaction uniqueness + remembering transactions | nonces, similar to sequence numbers |
| Type safety | Module structs and generics | Program structs | Contract types |
| Function calling | Static dispatch not on generics | Static dispatch | Dynamic dispatch |

## Aptos Move Features

Each deployment of the MoveVM has the ability to extend the core MoveVM with additional features via an adapter layer. Furthermore, MoveVM has a framework to support standard operations much like a computer has an operating system.

The Aptos Move adapter features include:
* Fine grained storage that decouples the amount of data stored in an account affecting the gas fees for transactions associated with the account
* [Tables](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/table.move) for storing key, value data within an account at scale
* Parallelism via [Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba) that enables concurrent execution of transactions without any input from the user


The Aptos framework ships with many useful libraries:
* A [Token standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) that makes it possible to create NFTs and other rich tokens without publishing a smart contract
* A [Coin standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) that makes it possible to create type-safe Coins by publishing a trivial module
* An [iterable Table](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/iterable_table.move) that allows for traversing all the entries within a table
* A staking and delegation framework
* A [`type_of`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/type_info.move) service to identify at run-time the address, module, and struct name of a given type
* Multi-signer framework that allows multiple `signer` entities
* A [timestamp service](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/timestamp.move) that provides a monotonically increasing clock that maps to the actual current unixtime

With much more coming soon...

## Key Concepts in Move

* Data should be stored within the account that owns it not the account that published the module.
* Data flow should have minimal constraints with an emphasis on ecosystem usability
* Prefer static type-safety over run-time safety via generics
* A `signer` should be required to restrict access to adding or removing assets to an account unless it is explicitly clear

### Data ownership

Data should be stored within the account that owns it not the account that published the module.

In Solidity, data is stored within the namespace of the account that created the contract. Typically this is represented by a map of an address to a value or of an instance id to the address of the owner.

In Solana, data is stored within a distinct account associated with a contract.

In Move, data can be stored within the module owner's account, but that creates the issue of ownership ambiguity and implies that two issues:
1. It makes ownership ambiguous as the asset has no resource associated with the owner
2. The module creator takes responsibility for the lifetime of that resources, e.g., rent, reclamation, etc

On the first point, by placing assets within trusted resources within an account, the owner can ensure that even a maliciously programmed module will be unable to modify those assets. In Move, we can program a standard orderbook structure and interface that would let applications built on top be unable to gain backdoor access to an account or its orderbook entries.

Contrast the following two coin storage strategies:

The following places the coins into a single account with ownership indicated by an index:
```rust
struct CoinStore has key {
    coins: table<address, Coin>,
}
```

Instead prefer the approach that stores the coins in an account:
```rust
struct CoinStore has key {
    coin: Coin,
}
```

This makes ownership explicit.

### Data flow

Data flow should have minimal constraints with an emphasis on ecosystem usability

Assets can be programmed to be entirely constrained within a module by making it such that no interface ever presents the struct in a value form and instead only provides functions for manipulating the data defined within the module. This constrains the data's availability to only within a module and makes it unexportable, which in turn prevents interoperability with other modules. Specifically, one could imagine a purchase contract that takes as input some `Coin<T>` and returns a `Ticket`. If `Coin<T>` is only defined within the module and is not exportable outside, then the applications for that `Coin<T>` are limited to whatever the module has defined.

Contrast the following two functions of implementing a coins transfer using deposit and withdraw:

```rust
public fun transfer<T>(sender: &signer, recipient: address, amount: u64) {
    let coin = withdraw(&signer, amount);
    deposit(recipient, coin);
}
```

The following limits where Coin can be used outside the module:
```rust
fun withdraw<T>(account: &signer, amount: u64): Coin<T>
fun deposit<T>(account: address, coin: Coin<T>)
```

By adding public accessors to withdraw and deposit, the coin can be taken outside of the module, used by other modules, and returned to the module:
```rust
public fun withdraw<T>(account: &signer, amount: u64): Coin<T>
public fun deposit<T>(account: address, coin: Coin<T>)
```

### Type-safety

In Move, given a specific struct, say `A`, different instances can be made distinct in two fashions:
* Internal identifiers, such as [GUID](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/guid.move)s
* Generics such as `A<T>`, where `T` is another struct

Internal identifiers can be convenient due to their simplicity and easier programmability. Generics, however, provide much higher guarantees including explicit compile or validation time checks though with some costs.

Generics allow for completely distinct types and resources and interfaces that expects those types. For example, an order book can state that they expect two currencies for all orders but one of them must be fixed, e.g., `buy<T>(coin: Coin<Aptos>): Coin<T>`. This explicitly states that a user can buy any coin `<T>` but must pay for it with `Coin<Aptos>`.

The complexity with generics arises when it would be desirable to store data on `T`. Move does not support static dispatch on generics, hence in a function like `create<T>(...) : Coin<T>`, T must either be a phantom type, i.e., only used as a type parameter in `Coin` or it must be specified as an input into `Create`. No functions can be called on a `T`, such as `T::function` even if every `T` implements said function.

In addition for structs that may be created in mass, generics results in the creation of a lot of new stores and resources associated with tracking the data and event emitting, which arguably is a lesser concern.

Because of this, we made the difficult choice of creating two "Token" standards, one for tokens associated with currency called `Coin` and another for tokens associated with assets or NFTs called `Token`. `Coin` leverages static type safety via generics but is a far simpler contract. While `Token` leverages dynamic type safety via its own universal identifier and eschews generics due to complexity that impacts the ergonomics of its use.

### Data access

* A *signer* should be required to restrict access to adding or removing assets to an account unless it is explicitly clear

In Move, a module can define how resources can be access and their contents modified regardless of the presence of the account owner's signer. This means that a programmer could accidentally create a resource that allows other users to arbitrarily insert or remove assets from another user's account.

In our development, we have several examples of where we have allowed permission to access and where we have prevented it:

* A Token cannot be directly inserted into another user's account unless they already have some of that Token
* TokenTransfers allows a user to explicitly claim a token stored in another user's resource effectively using an access control list to gain that access
* In Coin a user can directly transfer into another user's account so long as they have a resource for storing that coin

A less rigorous effort on our Token may have allowed users to airdrop tokens directly into another users account that would add additional storage to their accounts as well as make them owners of content that they did not first approve.

As a concrete example, return to the previous Coin case with the withdraw function. If the withdraw function instead were defined like this:
```rust
public fun withdraw<T>(account: address, amount: u64): Coin<T>
```
anyone would be able to remove coins from the `account`

## Additional Resources

* [The Move Book](https://move-language.github.io/move/)
* [The Aptos Framework documentation](https://github.com/aptos-labs/aptos-core/tree/framework-docs)
* [Getting Started](/guides/getting-started)
