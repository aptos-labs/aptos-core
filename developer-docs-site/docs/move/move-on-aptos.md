---
title: "Move on Aptos"
slug: "move-on-aptos"
---

# Move on Aptos

The Aptos blockchain consists of validator nodes that run a consensus protocol. The consensus protocol agrees upon the ordering of transactions and their output when executed on the Move Virtual Machine (MoveVM). Each validator node translates transactions along with the current blockchain ledger state as input into the VM. The MoveVM processes this input to produce a changeset or storage delta as output. Once consensus agrees and commits to the output, it becomes publicly visible. In this guide, we will introduce you to core Move concepts and how they apply to developing on Aptos.

## What is Move?

Move is a safe and secure programming language for Web3 that emphasizes **scarcity** and **access control**. Any assets in Move can be represented by or stored within *resource*. **Scarcity** is enforced by default as structs cannot be duplicated. Only structs that have explicitly been defined at the bytecode layer as *copy* can be duplicated.

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

## Aptos Move features

Each deployment of the MoveVM has the ability to extend the core MoveVM with additional features via an adapter layer. Furthermore, MoveVM has a framework to support standard operations much like a computer has an operating system.

The Aptos Move adapter features include:
* Fine grained storage that decouples the amount of data stored in an account affecting the gas fees for transactions associated with the account
* [Tables](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/table.move) for storing key, value data within an account at scale
* Parallelism via [Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba) that enables concurrent execution of transactions without any input from the user


The Aptos framework ships with many useful libraries:
* A [Token standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) that makes it possible to create NFTs and other rich tokens without publishing a smart contract
* A [Coin standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) that makes it possible to create type-safe Coins by publishing a trivial module
* A staking and delegation framework
* A [`type_of`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/type_info.move) service to identify at run-time the address, module, and struct name of a given type
* Multi-signer framework that allows multiple `signer` entities
* A [timestamp service](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/timestamp.move) that provides a monotonically increasing clock that maps to the actual current unixtime

With much more coming soon...

## Key Concepts in Aptos Move

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

Data flow should have minimal constraints with an emphasis on ecosystem usability.

Assets can be programmed to be only accessible within a module by making it such that no interface ever presents the struct in a value form and instead only provides functions for manipulating the data defined within the module (encapsulation). This constrains direct read+write access of the struct to the defining module, which in turn prevents interoperability with other modules. Specifically, one could imagine a purchase contract that takes as input some `Coin<T>` and returns a `Ticket`. If `Coin<T>` is only defined within the module and is not exportable outside, then the applications for that `Coin<T>` are limited to whatever the module has defined.

Contrast the following two functions of implementing a coin transfer using deposit and withdraw:

```rust
public fun transfer<T>(sender: &signer, recipient: address, amount: u64) {
    let coin = withdraw(&sender, amount);
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
* Internal identifiers, such as [GUID](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/guid.move)s
* Generics such as `A<T>`, where `T` is another struct

Internal identifiers can be convenient due to their simplicity and easier programmability. Generics, however, provide much higher guarantees including explicit compile or validation time checks though with some costs.

Generics allow for completely distinct types and resources and interfaces that expects those types. For example, an order book can state that they expect two currencies for all orders but one of them must be fixed, e.g., `buy<T>(coin: Coin<APT>): Coin<T>`. This explicitly states that a user can buy any coin `<T>` but must pay for it with `Coin<APT>`.

The complexity with generics arises when it would be desirable to store data on `T`. Move does not support static dispatch on generics, hence in a function like `create<T>(...) : Coin<T>`, T must either be a phantom type, i.e., only used as a type parameter in `Coin` or it must be specified as an input into `create`. No functions can be called on a `T`, such as `T::function` even if every `T` implements said function.

In addition for structs that may be created in mass, generics result in the creation of a lot of new stores and resources associated with tracking the data and event emitting, which arguably is a lesser concern.

Because of this, we made the difficult choice of creating two "Token" standards, one for tokens associated with currency called `Coin` and another for tokens associated with assets or NFTs called `Token`. `Coin` leverages static type safety via generics but is a far simpler contract. While `Token` leverages dynamic type safety via its own universal identifier and eschews generics due to complexity that impacts the ergonomics of its use.

### Data access

* A *signer* should be required to restrict access to adding or removing assets to an account unless it is explicitly clear

In Move, a module can define how resources can be accessed and their contents modified regardless of the presence of the account owner's signer. This means that a programmer could accidentally create a resource that allows other users to arbitrarily insert or remove assets from another user's account.

In our development of the Aptos Core Framework, we have several examples of where we have allowed permission to access and where we have prevented it:

* A `Token` cannot be directly inserted into another user's account unless they already have some of that `Token`
* `TokenTransfers` allows a user to explicitly claim a token stored in another user's resource effectively using an access control list to gain that access
* In `Coin` a user can directly transfer a `Coint<T>` into another user's account as long as the receiving user has already a `CoinStore<Coin<T>>` resource to store that coin.

A less rigorous effort on our `Token` may have allowed users to airdrop tokens directly into another users account, which would add additional storage to their accounts as well as make them owners of content that they did not first approve.

As a concrete example, return to the previous Coin case with the withdraw function. If the withdraw function instead were defined like this (notice the lack of a `signer` argument):
```rust
public fun withdraw<T>(account: address, amount: u64): Coin<T>
```
Anyone would be able to remove coins from the `account`.

### Resource accounts

Since the Move model often requires knowing the signer of a transaction, Aptos provides [resource accounts](../guides/resource-accounts.md) for assigning signer capability. Creating resource accounts enables access to the signer capability for automated use. The signer capability can be retrieved by the resource account's signer in combination with the address of the source account that created the resource account or placed in storage locally in the module. See the [`resource_signer_cap`](https://github.com/aptos-labs/aptos-core/blob/04ef2f2d02435a75dbf904b696d017e1040ecdd4/aptos-move/move-examples/mint_nft/2-Using-Resource-Account/sources/create_nft_with_resource_account.move#L136) reference in `create_nft_with_resource_account.move`.

When you create a resource account you also grant that account the signer capability. The only field inside the signer capability is the `address` of the signer. To see how we create a signer from the signer capability, review the [`let resource_signer`](https://github.com/aptos-labs/aptos-core/blob/916d8b40232040ce1eeefbb0278411c5007a26e8/aptos-move/move-examples/mint_nft/2-Using-Resource-Account/sources/create_nft_with_resource_account.move#L156) function in `create_nft_with_resource_account.move`.

To prevent security breaches, only the module and the resource account can call the signer capability. You cannot reverse generate a signer from a signer capability; instead, you must create a new resource account. You cannot, for instance, generate a signer capability from your private keys.

To further prevent signer vulnerabilities, monitor your calls in your wallet events and confirm:

* The amount of money being deducted is correct.
* The NFT creation event exists.
* No NFT withdrawal events exist.

See [resource accounts](../guides/resource-accounts.md) to learn more.

### Coins

Aptos Tokens (APT) can be sent to any arbitrary address, creating accounts for addresses that do not exist. For example, you have purchased USDC and want to convert it to APT. To protect users, they must accept those tokens.

### Wrapping

Why do we store Balance instead of Coin directly? We add indirection so that you can add wrapper functions.

For example, you may emit withdraw and deposit events from a Coin.

But say for an escrow, you could emit events for holding the Coins too.

Within a module, you may destructure other structs and operate on Coins directly rather than Balances indirectly.

It is up to the individual implementation. If you are defining both Coin and Balance in the same module, you can get a reference to the Coin inside via destructuring, obtaining mutable references to the structs themselves. If instead you rely upon the Coin module, you would need to use the Balance methods for depositing to users or a BalanceWithdraw method to get the actual coin. To add them together, use CoinMerge.

### Generics

You may use generics for both custom tokens and Aptos tokens. The only magic that Aptos offers, is Aptos uses the aggregator. This is not yet available for other coin types.

### Visibility

Functions are by default private, meaning they may be called only by other functions in the same file. You may use visibility modifiers [`public`, `public(entry)`, etc.] to make the function available outside of the file. For example:

* `entry` - isolates calling the function by making it the actual entry function, preventing re-entrancy (resulting in compiler errors)
* `public` - allows anyone to call the function from anywhere
* `public(entry) `- allows only the method defined in the related transaction to call the function
* `public(friend)` - used to declare modules that are trusted by the current module.
* `public(script)` - enables submission, compilation, and execution of arbitrary Move code on the Aptos network

Whenever possible, we recommend using the `entry` (rather than `public(entry)`)  visibility modifier to ensure your code can’t be wrapped with an additional object.

Move prevents re-entrancy in two ways:

1. Without dynamic dispatch, to call another module within your module, you must explicitly depend upon it. So other modules would need to depend upon you.
2. Cyclic dependencies are not allowed. So if A calls B, and then B reciprocally depend upon A, module B cannot be deployed.

Find out more about the Move programming language among the [Move Guides](index.md).
