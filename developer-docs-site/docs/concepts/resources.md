---
title: "Resources"
id: "resources"
---

# Resources

On Aptos, on-chain state is organized into resources and modules. These are then stored within the individual accounts. This is different from other blockchains, such as Ethereum, where each smart contract maintains their own storage space. See [Accounts](./accounts.md) for more details on accounts.

## Resources vs Instances

Move modules define struct definitions. Struct definitions may include the abilities such as `key` or `store`. Resources are struct instance with The `key` ability that are stored in global storage or directly in an account. The `store` ability allows struct instances to be stored within resources. An example here is how the APT coin is stored: CoinStore is the resource that contains the APT coin, while the Coin itself is an instance:

```rust
/// A holder of a specific coin type and associated event handles.
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

The Coin instance can be taken out of CoinStore with the owning account's permission and easily transferred to another CoinStore resource. It can also be kept in any other custom resource, if the definition allows, for example:

```rust
struct CustomCoinBox<phantom CoinType> has key {
    coin: Coin<CoinType>,
}
```

## Define resources and objects

All instances and resources are defined within a module that is stored at an address. For example `0x1234::coin::Coin<0x1234::coin::SomeCoin>` would be represented as:

```
module 0x1234::coin {
    struct CoinStore<phantom CoinType> has key {
        coin: Coin<CoinType>,
    }

    struct SomeCoin { }
}
```

In this example, `0x1234` is the address, `coin` is the module, `Coin` is a struct that can be stored as a resource, and `SomeCoin` is a struct that is unlikely to ever be represented as an instance. The use of the phantom type allows for there to exist many distinct types of `CoinStore` resources with different `CoinType` parameters.

## Permissions of Instances including Resources

Permissions of resources and other instances are dictated by the module where the struct is defined. For example, an instance within a resource may be accessed and even removed from the resource, but the internal state cannot be changed without permission from the module where the instance's struct is defined.

Ownership, on the other hand, is signified by either storing a resource under an account or by logic within the module that defines the struct.

## Viewing a resource

Resources are stored within accounts. Resources can be located by searching within the owner's account for the resource at its full query path inclusive of the account where it is stored as well as its address and module. Resources can be viewed on the [Aptos Explorer](https://explorer.aptoslabs.com/) by searching for the owning account or be directly fetched from a fullnode's API.

## How resources are stored

The module that defines a struct specifies how instances may be stored. For example, events for depositing a token can be stored in the receiver account where the deposit happens or in the account where the token module is deployed. In general, storing data in individual user accounts enables a higher level of execution efficiency as there would be no state read/write conflicts among transactions from different accounts, allowing for seamless parallel execution.
