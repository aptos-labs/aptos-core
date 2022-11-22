---
title: "Resources"
id: "resources"
---

# Resources

On Aptos, smart contract states are sharded by accounts. All on-chain states have to be organized into resources and associated
with specific accounts. This is different from other blockchains, such as Ethereum, where each smart contract maintains
their own storage space. Accounts on Aptos can contain associated modules and resources. [Events](./events.md) are stored
inside resources.
See [Accounts](./accounts.md) for more details on accounts.

## Resources vs Objects
Resources refer to top-level objects that are stored directly with an account on the blockchain. Objects can be resources but
can also be individual units of state that are stored inside a resource. An example here is how the APT coin is stored: CoinStore
is the resource that contains the APT coin while the Coin itself is an object:

```rust
/// A holder of a specific coin types and associated event handles.
/// These are kept in a single resource to ensure locality of data.
struct CoinStore<phantom CoinType> has key {
    coin: Coin<CoinType>,
}

/// Main structure representing a coin/token in an account's custody.
struct Coin<phantom CoinType> has store {
    /// Amount of coin this address has.
    value: u64,
}
```

The Coin object can be taken out of CoinStore with the owning account's permission and easily transferred to another CoinStore
resource. It can also be kept in any other custom resource, if the definition allows, for example:

```rust
struct CustomCoinBox<phantom CoinType> has key {
    coin: Coin<CoinType>,
}
```

## Dual ownership of objects, including resources
Objects (including resources) on Aptos are owned by both:
1. The account where the object is stored, and
2. The module that defines the object.

Creating a new resource and storing it into an account requires both the owning account's signature and the module's code.
But modifying and deleting the resource/object requires only the module's code and the owning account's address. The fields of
an object also can be read only directly by the module's code, which can be offered as public utilities for other modules.

This dual-ownership design is one of the bases of state safety in Aptos Move and enables powerful but safe functionalities to be built around resources and objects.

## Viewing a resource
Resources are stored within specific accounts. To locate a resource, the owning account must first be identified.
Resources can be viewed on the [Aptos Explorer](https://explorer.aptoslabs.com/) by searching for the owning account or be directly
fetched from a fullnode's API. See [Interacting with the blockchain](../guides/interacting-with-the-blockchain.md) for more information.

## How resources are stored
It's up to the smart contract developers to decide how and where a specific state is stored. For example, events for depositing
a token can be stored in the receiver account where the deposit happens or in the account the token module is deployed at.
In general, storing data in individual user accounts enables a higher level of execution efficiency as there would be no
state read/write conflicts among transactions, and they can be executed in parallel.

## Parallel processing
Sharding of state across accounts via resources allows efficient processing of transactions on the Aptos blockchain. The more
developers neatly organize resources in end user accounts, the more efficiency the network gains collectively. The only trade-off
of sharding is that it's more difficult when data is needed from multiple different accounts. This problem can be solved through
[indexing](../guides/indexing.md).
