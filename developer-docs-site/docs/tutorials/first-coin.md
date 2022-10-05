---
title: "Your First Coin"
slug: "your-first-coin"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Coin

This tutorial introduces how you can compile, deploy, and mint your own coin, named MoonCoin.

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [Typescript SDK][typescript-sdk]
* [Python SDK][python-sdk]
* [Rust SDK][rust-sdk]

---

## Step 2: Install the CLI

[Install the precombiled binary for the Aptos CLI][install_cli].

---

## Step 3: Run the example

Clone the `aptos-core` repo:

```bash
git clone https://github.com/aptos-labs/aptos-core.git
```

<Tabs groupId="examples">
  <TabItem value="typescript" label="Typescript">

Navigate to the Typescript SDK directory:

```bash
cd ~/aptos-core/ecosystem/typescript/sdk
```

Install the necessary dependencies:

```bash
yarn
```

Run the Typescript [`your_coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/your_coin.ts) example:

```bash
yarn your_coin ~/aptos-core/aptos-move/move-examples/moon_coin
```

  </TabItem>
  <TabItem value="python" label="Python">

Navigate to the Python SDK directory:

```bash
cd ~/aptos-core/ecosystem/python/sdk
```

Install the necessary dependencies. Also see [Aptos Developer Resources](/aptos-developer-resources):

```bash
curl -sSL https://install.python-poetry.org | python3
poetry update
```

Run the Python [`your-coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/your-coin.py) example:

```bash
poetry run python -m examples.your-coin ~/aptos-core/aptos-move/move-examples/moon_coin
```

  </TabItem>
  <TabItem value="rust" label="Rust">

    Coming soon.

  </TabItem>
</Tabs>

---

### Step 3.1: Build the package

The example run will pause with the following output:

```bash
=== Addresses ===
Alice: 0x5e603a89cf690d7134cf2f24fdb16ba90c4f5686333721c12e835fb6c76bc7ba
Bob: 0xc8421fa4a99153f955e50f1de2a6acff2f3fd0bb33aa17ba1f5b32b699f6c825

Update the package with Alice's address, compile, and press enter.
```

At this point, open another terminal and change directories to the MoonCoin package's directory:

```bash
cd ~/aptos-core/aptos-move/move-examples/moon_coin
```

Next, build the package using the CLI:

```bash
aptos move compile --named-addresses MoonCoin=0x5e603a89cf690d7134cf2f24fdb16ba90c4f5686333721c12e835fb6c76bc7ba --save-metadata
```

The `--named-addresses` is a list of address mappings that must be translated in order for the package to be compiled to be stored in Alice's account. Notice how `MoonCoin` is set to Alice's address printed above. Also `--save-metadata` is required to publish the package.

---

### Step 3.2: Completing the example

Returning to the previous prompt, press ENTER as the package is now ready to be published.

The application will complete, printing:

```bash

Publishing MoonCoin package.

Bob registers the newly created coin so he can receive it from Alice.
Bob's initial MoonCoin balance: 0.
Alice mints Bob some of the new coin.
Bob's updated MoonCoin balance: 100.
```
---

## Step 4: MoonCoin in depth

### Step 4.1: Building and publishing the MoonCoin package

Move contracts are effectively a set of Move modules known as a package. When deploying or upgrading a new package, the compiler must be invoked with `--save-metadata` to publish the package. In the case of MoonCoin, the following output files are critical:

- `build/Examples/package-metadata.bcs`: Contains the metadata associated with the package.
- `build/Examples/bytecode_modules/moon_coin.mv`: Contains the bytecode for the `moon_coin.move` module.

These are read by the example and published to the Aptos blockchain:

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

---

### Step 4.2: Understanding the MoonCoin module

The MoonCoin module defines the `MoonCoin` struct, or the distinct type of coin type. In addition, it contains a function called `init_module`. The `init_module` function is called when the module is published. In this case, MoonCoin initializes the `MoonCoin` coin type as a `ManagedCoin`, which is maintained by the owner of the account. 

:::tip ManagedCoin framework
[`ManagedCoin`](https://github.com/aptos-labs/aptos-core/blob/f81ccb01f00227f9c0f36856fead4879f185a9f6/aptos-move/framework/aptos-framework/sources/managed_coin.move#L1) is a simple coin management framework for coins directly managed by users. It provides convenience wrappers around `mint` and `burn`.
:::

```rust
:!: static/move-examples/moon_coin/sources/MoonCoin.move moon
```

---

### Step 4.3: Understanding coins

Coins have several primitives:

- **Minting**: Creating new coins.
- **Burning**: Deleting coins.
- **Freezing**: Preventing an account from storing coins in `CoinStore`.
- **Registering**: Creating a `CoinStore` resource on an account for storing coins.
- **Transferring**: Withdrawing and depositing coins into `CoinStore`.

:::tip

The entity that creates a new coin gains the capabilities for minting, burning, and freezing.
:::

In order to transfer, withdraw, or deposit coins, you must have a `CoinStore` registered for the specific coin. In this tutorial, this is `CoinStore<MoonCoin>`.

---

#### Step 4.3.1: Initializing a coin

Once a coin type has been published to the Aptos blockchain, the entity that published that coin type can initialize it:

```rust showLineNumbers
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

This ensures that this coin type has never been initialized before. Notice the check on lines 10 and 15 to ensure that the caller to `initialize` is the same one that actually published this module, and that there is no `CoinInfo` stored on their account. If both those conditions check, then a `CoinInfo` is stored and the caller obtains capabilities for burning, freezing, and minting.

:::tip
MoonCoin calls this `initialize` function automatically upon package publishing.
:::

---

#### Step 4.3.2: Registering a coin

To use a coin, an entity must register a `CoinStore` for it on their account:

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

MoonCoin uses `ManagedCoin` that provides an entry function wrapper: `managed_coin::register`.

---

#### Step 4.3.3: Minting a coin

Minting coins requires the mint capability that was produced during initialization. the function `mint` (see below) takes in that capability and an amount, and returns back a `Coin<T>` struct containing that amount of coins. If the coin tracks supply, it will be updated.

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

---

#### Step 4.3.4: Transferring a coin

Aptos provides several building blocks to support coin transfers:

- `coin::deposit<CoinType>`: Allows any entity to deposit a coin into an account that has already called `coin::register<CoinType>`.
- `coin::withdraw<CoinType>`: Allows any entity to extract a coin amount from their account.
- `coin::transfer<CoinType>`: Leverages withdraw and deposit to perform an end-to-end transfer.

:::tip important
Aptos does not emit transfer events, but instead it leverages withdraw and deposit events.
:::

[typescript-sdk]: /sdks/ts-sdk/index
[python-sdk]: /sdks/python-sdk
[rust-sdk]: /sdks/rust-sdk
[install_cli]: /cli-tools/aptos-cli-tool/install-aptos-cli
