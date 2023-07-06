---
title: "Fungible Asset"
id: "fungible-asset"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Fungible Asset

Fungible Assets (FAs) is a core framework component within Aptos that enables the tokenization of various assets, including commodities, real estate, and financial instruments, and facilitate the creation of decentralized financial applications.

Tokenization of securities and commodities provides fractional ownership, making these markets more accessible to a broader range of investors.
Fungible tokens can also represent ownership of real estate, enabling fractional ownership and providing liquidity to the traditionally illiquid market.
In-game assets such as virtual currencies and characters can be tokenized, enabling players to own and trade their assets, creating new revenue streams for game developers and players.

Besides aforementioned features, fungible asset is a superset of cryptocurrency as coin is just one type of fungible asset. Coin module in Move could be replaced by fungible asset framework.

The [fungible asset module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/fungible_asset.move) provides a standard, type-safe framework for defining FAs within the Aptos Move ecosystem.

The standard is built upon [Aptos object model](./aptos-object.md) so all the resources defined here are included in object resource group and stored inside objects. There are two types of objects related to FA.

- `Object<Metadata>`: include information about the FA such as name, symbol and decimals.
- `Object<FungibleStore>`: store a specific amount of FA units. FAs are units that are interchangeable with others of the same metadata. They can be stored in objects that contain a FungibleStore resource. These store objects can be freely created and FAs can be moved, split, combined between them easily.

The standard also supports minting new units and burning existing units with appropriate controls.

The different objects involved - `Object<Metadata>` and `Object<FungibleStore>` objects, and their relationships to accounts are shown in the diagram below:

<div style={{textAlign:"center"}}>
<ThemedImage
alt="fungible asset architecture"
sources={{
    light: useBaseUrl('/img/docs/fungible-asset.svg'),
    dark: useBaseUrl('/img/docs/fungible-asset-dark.svg'),
  }}
/>
</div>

## Difference with Aptos Coin

FA is a broader category than just coins. While fungible coins are just one possible use case of FA, it can represent a wider range of fungible items, such as in-game assets like gems or rocks, event tickets, and partial ownership of real-world assets. FA provides the flexibility for customizable, detailed management and offers a new programming model based on objects.
For Aptos coin, a `Coin` uses a generic, or the `CoinType`, to support distinct typing within the Coin framework. For example, `Coin<A>` and `Coin<B>` are two distinct coins, if `A != B`. In contract, FA does not have generic in struct definition but uses the metadata reference to distinguish the type which will be further explained later.
Minimally, [Aptos coin](./aptos-coin.md) should be interchangeable with FA. The migration plan is under discussion.

## Structures

### Metadata Object

Metadata objects with unique addresses define the type of the FAs. Even if `Metadata` structs of two `Object<Metadata>` are exactly the same, as long as their addresses are different, the FAs points to them would be different. In short, the address of the metadata object can be used as **unique identifier** of the FA type.

```rust
#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct Metadata has key {
    supply: Option<Supply>,
    /// Name of the fungible metadata, i.e., "USDT".
    name: String,
    /// Symbol of the fungible metadata, usually a shorter version of the name.
    /// For example, Singapore Dollar is SGD.
    symbol: String,
    /// Number of decimals used for display purposes.
    /// For example, if `decimals` equals `2`, a balance of `505` coins should
    /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
    decimals: u8,
}
```

### Fungible Asset and Fungible Store

FA allows typing by allocating an object reference that points at the metadata. Hence, a set of units of FA is represented as an amount and a reference to the metadata, as shown:

```rust
struct FungibleAsset {
    metadata: Object<Metadata>,
    amount: u64,
}
```

The FAs is a struct representing the type and the amount of units held. As the struct does not have either key or store abilities, it can only be passed from one function to another but must be consumed by the end of a transaction. Specifically, it must be deposited back into a fungible store at the end of the transaction, which is defined as:

```rust
#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct FungibleStore has key {
    /// The address of the base metadata object.
    metadata: Object<Metadata>,
    /// The balance of the fungible metadata.
    balance: u64,
    /// FAs transferring is a common operation, this allows for freezing/unfreezing accounts.
    frozen: bool,
}
```

:::tip
FAs are always stored in the top-level `FungibleStore` resource. This makes it much easier to find, analyze and control.
:::

The only extra field added here is `frozen`. if it is `true`, this object is frozen, i.e. deposit and withdraw are both disabled without using `TransferRef` in the next section.

### References

_Reference_ (ref) is the means to implement granular permission control across different standards in Aptos. In different contexts, it may be called _capabilities_. The FA standard has three distinct refs for minting, transferring, and burning FA: `MintRef`, `TransferRef`, and `BurnRef`. Each ref contains a reference to the FA metadata:

```rust
struct MintRef has drop, store {
    metadata: Object<Metadata>
}

struct TransferRef has drop, store {
    metadata: Object<Metadata>
}

struct BurnRef has drop, store {
    metadata: Object<Metadata>
}
```

Ref owners can do the following operations depending on the refs they own:

- `MintRef` offers the capability to mint new FA units.
- `TransferRef` offers the capability to mutate the value of `freeze` in any `FungbibleStore` of the same metadata or transfer FA by ignoring `freeze`.
- `BurnRef` offers the capability to burn or delete FA units.

The three refs collectively act as the building blocks of various permission control system as they have `store` so can be passed around and stored anywhere. Please refer to the source file for `mint()`, `mint_to()`, `burn()`, `burn_from()`, `withdraw_with_ref()`, `deposit_with_ref()`, and `transfer_with_ref()`: These functions are used to mint, burn, withdraw, deposit, and transfer FA using the MintRef, BurnRef, and TransferRef.

Note, these are framework functions and must be combined with business logic to produce a usable system. Developers who want to use these functions should familiarize themselves with the concepts of [Aptos object model](./aptos-object.md) and understand how the reference system enables extensible designs within Aptos move.

### Creators

A FA creator can add fungibility to any **undeletable** object at creation by taking `&ConstructorRef` with required information to
make that object a metadata of the associated FA. Then FA of this metadata can be minted and used. It is noted here that
**undeletable** means the `can_delete` field of `&ConstructorRef` has to be `false`.

```rust   
public fun add_fungibility(
    constructor_ref: &ConstructorRef,
    maximum_supply: Option<u128>,
    name: String,
    symbol: String,
    decimals: u8,
    icon_uri: String,
    project_uri: String,
): Object<Metadata> 
```

The creator has the opportunity to define a name, symbol, decimals, icon uri, project uri, and whether the total supply for the FA has a maximum. The following applies:

- The first three of the above (`name`, `symbol`, `decimals`, `icon_uri`, `project_uri`)  are purely metadata and have no impact for onchain
  applications. Some applications may use decimal to equate a single Coin from fractional coin.
- Maximum supply (`maximum_supply`) helps check the total supply does not exceed a maximum value. However, due to the way the parallel executor
  works, setting the maximum supply will prevent any parallel execution of mint and burn.

### Users

Users are FA holders, who can:

- Merge two FAs of the same metadata object.
- Extract FA partially from another.
- Deposit to and withdraw from a `FungibleStore` and emit events as a result.

### Primitives

At creation, the creator has the option to generate refs from the same `&ConstructorRef` to manage FA. These will need
to be stored in global storage to be used later.

#### Mint

If the manager would like to mint FA, they must retrieve a reference to `MintRef` and call:

```rust
public fun mint(ref: &MintRef, amount: u64): FungibleAsset
```

This will produce a new FA of the metadata in the ref, containing a value as dictated by the `amount`. The supply will also be adjusted. Also, there is a `mint_to` function that deposits to a `FungibleStore`
after minting as a helper.

#### Burn

The opposite operation of minting. Likewise, a reference to `BurnRef` is required and call:

```rust
public fun burn(ref: &BurnRef, fa: FungibleAsset)
```

This will reduce the passed-in `fa` to ashes and adjust the supply. There is also a `burn_from` function that forcibly withdraws FA
from an account first and then burns the withdrawn FA as a helper.

#### Transfer and Freeze/Unfreeze

`TransferRef` has two functions:

- Flip `frozen` in `FungibleStore` holding FA of the same metadata in the `TransferRef`. if
  it is false, the store is "frozen" that nobody can deposit to or withdraw from this store without using the ref.
- Withdraw from or deposit to a store ignoring `frozen` field.

To change `frozen`, call:

```rust
public fun set_frozen_flag<T: key>(
    ref: &TransferRef,
    store: Object<T>,
    frozen: bool,
)
```

:::tip
This function will emit a `FrozenEvent`.
:::

To forcibly withdraw, call:

```Rust
public fun withdraw_with_ref<T: key>(
    ref: &TransferRef,
    store: Object<T>,
    amount: u64
): FungibleAsset
```

:::tip
This function will emit a `WithdrawEvent`.
:::

To forcibly deposit, call:

```rust
public fun deposit_with_ref<T: key>(
    ref: &TransferRef,
    store: Object<T>,
    fa: FungibleAsset
)
```

:::tip
This function will emit a `DepositEvent`.
:::

There is a function named `transfer_with_ref` that combining `withdraw_with_ref` and `deposit_with_ref` together as
a helper.

#### Merging Fungible Assets

Two FAs of the same type can be merged into a single struct that represents the accumulated value of the two  
independently by calling:

```rust
public fun merge(dst_fungible_asset: &mut FungibleAsset, src_fungible_asset: FungibleAsset)
```

After merging, `dst_fungible_asset` will have all the amounts.

#### Extracting Fungible Asset

A FA can have amount deducted to create another FA by calling:

```rust
public fun extract(fungible_asset:& mut FungibleAsset, amount: u64): FungibleAsset
```

:::tip
This function may produce FA with 0 amount, which is not usable. It is supposed to be merged with other FA or destroyed
through `destroy_zero()` in the module.
:::

#### Withdraw

The owner of a `FungibleStore` object that is not frozen can extract FA with a specified amount, by calling:

```rust
public fun withdraw<T: key>(owner: &signer, store: Object<T>, amount: u64): FungibleAsset
```

:::tip
This function will emit a `WithdrawEvent`.
:::

#### Deposit

Any entity can deposit FA into a `FungibleStore` object that is not frozen, by calling:

```rust
public fun deposit<T: key>(store: Object<T>, fa: FungibleAsset)
```

:::tip
This function will emit a `DepositEvent`.
:::

#### Transfer

The owner of a `FungibleStore` can directly transfer FA from that store to another if neither is frozen by calling:

```rust
public entry fun transfer<T: key>(sender: &signer, from: Object<T>, to: Object<T>, amount: u64)
```

:::tip
This will emit both `WithdrawEvent` and `DepositEvent` on the respective `Fungibletore`s.
:::

## Events

- `DepositEvent`: Emitted when FAs are deposited into a store.
- `WithdrawEvent`: Emitted when FAs are withdrawn from a store.
- `FrozenEvent`: Emitted when the frozen status of a fungible store is updated.

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

```rust
struct FrozenEvent has drop, store {
    frozen: bool,
}
```

# Primary and secondary `FungibleStore`s

Each `FungibleStore` object has an owner. However, an owner may possess more than one store. When Alice sends FA to
Bob, how does she determine the correct destination? Additionally, what happens if Bob doesn't have a store yet?

To address these questions, the standard has been expanded to define primary and secondary stores.

- Each account owns only one undeletable primary store for each type of FA, the address of which is derived in a deterministic
  manner from the account address and metadata object address. If primary store does not exist, it will be created if
  FA is going to be deposited by calling functions defined in `primary_fungible_store.move`
- Secondary stores do not have deterministic address and theoretically deletable. Users are able to create as many
  secondary stores as they want using the provided functions but there is a caveat that addressing secondary stores
  on-chain may need extra work.

The vast majority of users will have primary store as their only store for a specific type of FAs. It is
expected that secondary stores would be useful in complicated defi or other asset management contracts that will be
introduced in other tutorials using FA.

## How to enable Primary `FungibleStore`?

To add primary store support, when creating metadata object, instead of aforementioned `add_fungibility()`, creator
has to call:

```rust
public fun create_primary_store_enabled_fungible_asset(
    constructor_ref: &ConstructorRef,
    maximum_supply: Option<u128>,
    name: String,
    symbol: String,
    decimals: u8,
    icon_uri: String,
    project_uri: String,
) 
```

The parameters are the same as those of `add_fungibility()`.

## Primitives

### Get Primary `FungibleStore`

To get the primary store object of a metadata object belonging to an account, call:

```rust
public fun primary_store<T: key>(owner: address, metadata: Object<T>): Object<FungibleStore>
```

:::tip
There are other utility functions. `primary_store_address` returns the deterministic address the primary store,
and `primary_store_exists` checks the existence, etc.
:::

### Manually Create Primary `FungibleStore`

If a primary store does not exist, any entity is able to create it by calling:

```rust
public fun create_primary_store<T: key>(owner_addr: address, metadata: Object<T>): Object<FungibleStore>
```

### Check Balance and Frozen Status

To check the balance of a primary store, call:

```rust
public fun balance<T: key>(account: address, metadata: Object<T>): u64
```

To check whether the given account's primary store is frozen, call:

```rust
public fun is_frozen<T: key>(account: address, metadata: Object<T>): bool
```

### Withdraw

An owner can withdraw FA from their primary store by calling:

```rust
public fun withdraw<T: key>(owner: &signer, metadata: Object<T>, amount: u64): FungibleAsset
```

### Deposit

An owner can deposit FA to their primary store by calling:

```rust
public fun deposit(owner: address, fa: FungibleAsset)
```

### Transfer

An owner can deposit FA from their primary store to that of another account by calling:

```rust
public entry fun transfer<T: key>(sender: &signer, metadata: Object<T>, recipient: address, amount: u64)
```

## Secondary `FungibleStore`

Secondary stores are not commonly used by normal users but prevailing for smart contracts to manage assets owned by
contracts. For example, an asset pool may have to manage multiple fungible stores for one or more types of FA. Those
stores do not necessarily have to have deterministic addresses and a user may have multiple stores for a given kind of
FA. So primary fungible store is not a good fit for the needs where secondary store plays a vital role.

The way to create secondary store is to create an object first and get its `ConstructorRef`. Then call:

```rust
public fun create_store<T: key>(
    constructor_ref: &ConstructorRef,
    metadata: Object<T>,
): Object<FungibleStore>
```

It will turn make the newly created object a `FungibleStore`. Sometimes an object can be reused as a store. For example,
a metadata object can also be a store to hold some FA of its own type or a liquidity pool object can be a store of the
issued liquidity pool's token/coin.

## Ownership of `FungibleStore`

It is crucial to set correct owner of a `FungibleStore` object for managing the FA stored inside. By default, the owner
of a newly created object is the creator whose `signer` is passed into the creation function. For `FungibleStore`
objects managed by smart contract itself, usually they shouldn't have an owner out of the control of this contract. For
those cases, those objects could make themselves as their owners and keep their object `ExtendRef` at the proper place
to create `signer` as needed by the contract logic.
