---
title: "Aptos Coin"
id: "aptos-coin"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos Coin

[Coin](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) provides a standard, typesafe framework for simple, fungible tokens or coins. 

:::tip 
Coin is stored in `0x1::coin`.
:::

## Structures

### Reusability

A coin is defined in Move as:

```rust
struct Coin<phantom CoinType> has store {
    /// Amount of coin this address has.
    value: u64,
}
```

A Coin uses the `CoinType` to support re-usability of the Coin framework for distinct Coins. For example, `Coin<A>` and `Coin<B>` are two distinct coins.

### Global store

Coin also supports a resource for storing coins in global store:

```rust
struct CoinStore<phantom CoinType> has key {
    coin: Coin<CoinType>,
    frozen: bool,
    deposit_events: EventHandle<DepositEvent>,
    withdraw_events: EventHandle<WithdrawEvent>,
}
```

Coin information or metadata is stored in global store under the coin creators account:

```rust
struct CoinInfo<phantom CoinType> has key {
    name: string::String,
    /// Symbol of the coin, usually a shorter version of the name.
    /// For example, Singapore Dollar is SGD.
    symbol: string::String,
    /// Number of decimals used to get its user representation.
    /// For example, if `decimals` equals `2`, a balance of `505` coins should
    /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
    decimals: u8,
    /// Amount of this coin type in existence.
    supply: Option<OptionalAggregator>,
}
```

## Primitives

Coin provides the primitives for users creating and managing the coin and the users who use it.

**Creators**

Coin creators and managers can:

- Initialize a coin and set its metadata and supply monitoring.
- Minting and burning Coin value.
- Burning coins from a `CoinStore`.
- Freezing mobility into and out of a `CoinStore`.

**Users**

Coin users can:

- Merging two Coin structs of the same type.
- Extracting value from a Coin struct into a new Coin struct.
- Ability to deposit and withdraw from a `CoinStore` and emit events as a result.
- Allows for users to register a `CoinStore<CoinType>` in their account to handle coin.

### Creating a new CoinType

A coin creator can publish to an on-chain account a new module that defines a struct to represent a new `CoinType`. The coin creator will then call `coin:initialize<CoinType>` from that account to register this as a valid coin, and in return receive back structs that enable calling the functions to burn and mint coins and freeze `CoinStore`s. These will need to be stored in global storage by the creator to manage the use of the coin.

```rust
public fun initialize<CoinType>(
    account: &signer,
    name: string::String,
    symbol: string::String,
    decimals: u8,
    monitor_supply: bool,
): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
```

The creator has the opportunity to define a name, symbol, decimals, and whether or not the total supply for the coin is monitored. The following applies:

- The first three of the above (`name`, `symbol`, `decimals`)  are purely metadata and have no impact for on-chain applications. Some applications may use decimal to equate a single Coin from fractional coin. 
- Monitoring supply (`monitor_supply`) helps track total coins in supply. However, due to the way the parallel executor works, turning on this option will prevent any parallel execution of mint and burn. If the coin will be regularly minted or burned, consider disabling `monitor_supply`.

### Minting Coins

If the creator or manager would like to mint coins, they must retrieve a reference to their `MintCapability`, which was produced in the `initialize`, and call:

```rust
public fun mint<CoinType>(
    amount: u64,
    _cap: &MintCapability<CoinType>,
): Coin<CoinType> acquires CoinInfo {
```

This will produce a new Coin struct containing a value as dictated by the `amount`. If supply is tracked, then it will also be adjusted.

### Burning Coins

If the creator or manager would like to burn coins, they must retrieve a reference to their `BurnCapability`, which was produced in the `initialize`, and call:

```rust
public fun burn<CoinType>(
    coin: Coin<CoinType>,
    _cap: &BurnCapability<CoinType>,
) acquires CoinInfo {
```

The creator or manager can also burn coins from a `CoinStore`:

```rust
public fun burn_from<CoinType>(
    account_addr: address,
    amount: u64,
    burn_cap: &BurnCapability<CoinType>,
) acquires CoinInfo, CoinStore {
```

:::tip burn vs burn_from
The function `burn` eliminates the total value stored in the coin, while `burn_from` only eliminates a given amount of value from a `CoinStore`. If supply is tracked, then it will also be adjusted.

Burning coins from an account does not emit a `WithdrawEvent` as the `withdraw` function does.
:::



### Freezing Accounts

If the creator or manager would like to freeze a `CoinStore` on a specific account, they must retrieve a reference to their `FreezeCapability`, which was produced in `initialize`, and call:

```rust
public entry fun freeze_coin_store<CoinType>(
    account_addr: address,
    _freeze_cap: &FreezeCapability<CoinType>,
) acquires CoinStore {
```

### Merging Coins

Two coins of the same type can be merged into a single Coin struct that represents the accumulated value of the two coins independently by calling:

```rust
public fun merge<CoinType>(
    dst_coin: &mut Coin<CoinType>,
    source_coin: Coin<CoinType>,
) {
```

### Extracting Coins

A Coin can have value deducted to  create another Coin by calling:

```rust
public fun extract<CoinType>(
		coin: &mut Coin<CoinType>,
		amount: u64,
): Coin<CoinType> {
```

### Withdrawing Coins from CoinStore

A holder of a `CoinStore` can extract a Coin of a specified value by calling:

```rust
public fun withdraw<CoinType>(
    account: &signer,
    amount: u64,
): Coin<CoinType> acquires CoinStore {
```

:::tip
This function will emit a `WithdrawEvent`.
:::

### Depositing Coins into CoinStore

Any entity can deposit coins into an account’s `CoinStore` by calling:

```rust
public fun deposit<CoinType>(
		account_addr: address,
		coin: Coin<CoinType>,
) acquires CoinStore {
```
:::tip
This function will emit a `DepositEvent`.
:::

### Transferring Coins

A holder of a `CoinStore` can directly transfer coins from their account to another account’s `CoinStore` by calling:

```rust
public entry fun transfer<CoinType>(
    from: &signer,
    to: address,
    amount: u64,
) acquires CoinStore {
```

:::tip
This will emit both a `WithdrawEvent` and `DespositEvent` on the respective `CoinStore`s.
:::

## Events

```rust
struct DepositEvent has drop, store {
    amount: u64,
}
```

```rust
struct WithdrawEvent has drop, store {
    amount: u64,
}
```