---
title: "Your First Coin"
slug: "your-first-coin"
sidebar_position: 2
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Coin

This tutorial introduces how one can compile, deploy, and mint their own coin, _MoonCoin_.

## Step 1: Pick an SDK

- [Official Aptos Typescript SDK][typescript-sdk]
- [Official Aptos Python SDK][python-sdk]
- [Official Aptos Rust SDK][rust-sdk]

## Step 2: Acquire the CLI

[Install the CLI from Git][install_cli]

## Step 3: Run the Example

Clone `aptos-core`:

```sh
git clone https://github.com/aptos-labs/aptos-core.git
```

### Step 3.1: SDK-Specific Example

<Tabs groupId="examples">
  <TabItem value="typescript" label="Typescript">

Navigate to the Typescript SDK directory:

```sh
cd ~/aptos-core/ecosystem/typescript/sdk
```

Install the necessary dependencies:

```sh
yarn
```

Run the `your_coin` example:

```sh
yarn your_coin ~/aptos-core/aptos-move/move-examples/moon_coin
```

  </TabItem>
  <TabItem value="python" label="Python">

Navigate to the Python SDK directory:

```sh
cd ~/aptos-core/ecosystem/python/sdk
```

Install the necessary dependencies:

```
curl -sSL https://install.python-poetry.org | python3
poetry update
```

Run the `your-coin` example:

```sh
poetry run python -m examples.your-coin ~/aptos-core/aptos-move/move-examples/moon_coin
```

  </TabItem>
  <TabItem value="rust" label="Rust">

    Coming soon.

  </TabItem>
</Tabs>

### Step 3.2: Building the Package

Half-way through, the demo will pause with the following output:

```sh
=== Addresses ===
Alice: 0x5e603a89cf690d7134cf2f24fdb16ba90c4f5686333721c12e835fb6c76bc7ba
Bob: 0xc8421fa4a99153f955e50f1de2a6acff2f3fd0bb33aa17ba1f5b32b699f6c825

Update the package with Alice's address, compile, and press enter.
```

At this point, open another terminal and change directories to the _MoonCoin_ package's directory:

```sh
cd ~/aptos-core/aptos-move/move-examples/moon_coin
```

Now build the package using your CLI:

```sh
aptos move compile --named-addresses MoonCoin=0x5e603a89cf690d7134cf2f24fdb16ba90c4f5686333721c12e835fb6c76bc7ba --save-metadata
```

The `--named-addresses` is a list of address mappings that must be translated in order for the package to be compiled to be stored in Alice's account. Notice how `MoonCoin` is set to Alice's address printed above. Also `--save-metadata` is required in order to publish package.

### Step 3.3: Completing the Example

Returning to the previous prompt, press enter as the package is now ready to be published.

The application will complete, printing:

```sh

Publishing MoonCoin package.

Bob registers the newly created coin so he can receive it from Alice.
Bob's initial MoonCoin balance: 0.
Alice mints Bob some of the new coin.
Bob's updated MoonCoin balance: 100.
```

## Step 4: _MoonCoin_ in Depth

### Step 4.1: Building and Publishing the _MoonCoin_ Package

Move software or contracts are effectively a set of modules known as a package. When deploying or upgrading a new package, the compiler must be invoked with `--save-metadata` in order to publish the package. In the case of _MoonCoin_, the following output files are critical:

- `build/Examples/package-metadata.bcs` -- the metadata associated with the package
- `build/Examples/bytecode_modules/moon_coin.mv` -- the bytecode for the `moon_coin.move` module

These are read by the example and published to the blockchain:

<Tabs groupId="examples">
  <TabItem value="typescript" label="Typescript">

```typescript
:!: static/sdks/typescript/examples/typescript/your_coin.ts publish
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/your-coin.py publish
```

  </TabItem>
  <TabItem value="rust" label="Rust">

    Coming soon.

  </TabItem>
</Tabs>

### Step 4.2: Understanding the _MoonCoin_ Module

The MoonCoin module defines the `MoonCoin` struct, or the distinct type of coin type. In addition, it contains a function called `init_module`. `init_module` is called when the module is published. In this case, _MoonCoin_ initializes the `MoonCoin` coin type as a `ManagedCoin`, which is maintained by the owner of the account. [`ManagedCoin`](https://github.com/aptos-labs/aptos-core/blob/f81ccb01f00227f9c0f36856fead4879f185a9f6/aptos-move/framework/aptos-framework/sources/managed_coin.move#L1) is a simple coin management framework for coins directly managed by users. It provides convenience wrappers around `mint` and `burn`.

```rust
:!: static/move-examples/moon_coin/sources/MoonCoin.move moon
```

### Step 4.3: Understanding Coins

Coins have several primitives:

- Minting -- creating new coins
- Burning -- deleting coins
- Freezing -- preventing an account from storing coins in `CoinStore`
- Registering -- creating a `CoinStore` resource on an account for storing coins
- Transferring -- withdrawing and depositing coins into `CoinStore`

The entity that creates a new coin gains the capabilities for minting, burning, and freezing.

In order to transfer, withdraw, or deposit coins, one must have a `CoinStore` registered for the specific coin. In this tutorial, this is `CoinStore<MoonCoin>`.

#### Step 4.3.1: Initializing a Coin

Once a coin type has been published to the blockchain, the entity that published that coin type can initialize it:

```rust
public fun initialize<CoinType>(
    account: &signer,
    name: string::String,
    symbol: string::String,
    decimals: u8,
    monitor_supply: bool,
): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
    let account_addr = signer::address_of(account);

    assert!(
        coin_address<CoinType>() == account_addr,
        error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),
    );

    assert!(
        !exists<CoinInfo<CoinType>>(account_addr),
        error::already_exists(ECOIN_INFO_ALREADY_PUBLISHED),
    );

    let coin_info = CoinInfo<CoinType> {
        name,
        symbol,
        decimals,
        supply: if (monitor_supply) { option::some(optional_aggregator::new(MAX_U128, false)) } else { option::none() },
    };
    move_to(account, coin_info);

    (BurnCapability<CoinType>{ }, FreezeCapability<CoinType>{ }, MintCapability<CoinType>{ })
}
```

This ensures that this coin type has never been initialized before, notice the check to ensure that the caller to `initialize` actually published this module and that there is no `CoinInfo` stored on their account. If both those conditions check, then a `CoinInfo` is stored and the caller obtains capabilities for burning, freezing, and minting.

_MoonCoin_ calls this function automatically upon package publishing.

#### Step 4.2.2: Registering a Coin

In order to use a coin, an entity must register a `CoinStore` for it on their account:

```rust
public fun register<CoinType>(account: &signer) {
    let account_addr = signer::address_of(account);
    assert!(
        !is_account_registered<CoinType>(account_addr),
        error::already_exists(ECOIN_STORE_ALREADY_PUBLISHED),
    );

    account::register_coin<CoinType>(account_addr);
    let coin_store = CoinStore<CoinType> {
        coin: Coin { value: 0 },
        frozen: false,
        deposit_events: account::new_event_handle<DepositEvent>(account),
        withdraw_events: account::new_event_handle<WithdrawEvent>(account),
    };
    move_to(account, coin_store);
}
```

As this is a `public fun` and not a `public entry fun`, coins will need to provide their own means for registering or users can construct Move `scripts` to call the function.

_MoonCoin_ uses `ManagedCoin` that provides an entry function wrapper: `managed_coin::register`.

#### Step 4.2.3: Minting a Coin

Mint coins requires the mint capability that was produced during initialization. It takes in that capability and an amount and returns back a `Coin<T>` struct containing that amount of coins. If the coin tracks supply, it will be updated.

```rust
public fun mint<CoinType>(
    amount: u64,
    _cap: &MintCapability<CoinType>,
): Coin<CoinType> acquires CoinInfo {
    if (amount == 0) {
        return zero<CoinType>()
    };

    let maybe_supply = &mut borrow_global_mut<CoinInfo<CoinType>>(coin_address<CoinType>()).supply;
    if (option::is_some(maybe_supply)) {
        let supply = option::borrow_mut(maybe_supply);
        optional_aggregator::add(supply, (amount as u128));
    };

    Coin<CoinType> { value: amount }
}
```

`ManagedCoin` makes this easier by providing a entry function `managed_coin::mint`.

#### Step 4.2.4: Transferring a Coin

Aptos provides several building blocks to support transfers:

- `coin::deposit<CoinType>` allows any entity to deposit a coin into an account that has already called `coin::register<CoinType>`.
- `coin::withdraw<CoinType>` allows any entity to extract a coin amount from their account.
- `coin::transfer<CoinType>` leverages withdraw and deposit to perform an end-to-end transfer.

Aptos does not emit transfer events, but instead it leverages withdraw and deposit events.

[typescript-sdk]: /sdks/typescript-sdk
[python-sdk]: /sdks/python-sdk
[rust-sdk]: /sdks/rust-sdk
[install_cli]: /cli-tools/aptos-cli-tool/install-aptos-cli
