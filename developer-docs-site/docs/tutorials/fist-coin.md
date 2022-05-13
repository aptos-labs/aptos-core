---
title: "Your first Coin"
slug: "your-first-coin"
sidebar_position: 2
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your first Coin

This tutorial details how to deploy and manage a new Coin. The steps are:

1. Deploy MoonCoin module. Let's call it MoonCoin.
2. Initialize MoonCoin via the standard Coin framework module.
3. Register a recipient account to receive MoonCoin.
4. Mint MoonCoin to the recipient as the owner of the MoonCoin.

This tutorial builds on [Your first transaction](/tutorials/your-first-transaction) as a library for this example. The following tutorial contains example code that can be downloaded in its entirety below:

<Tabs>
  <TabItem value="python" label="Python" default>

For this tutorial, will be focusing on `first_coin.py` and re-using the `first_transaction.py` library from the previous tutorial.

You can find the python project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python)

  </TabItem>
  <TabItem value="rust" label="Rust" default>

For this tutorial, will be focusing on `first_coin.rs` and re-using the `first_transaction.rs` library from the previous tutorial.

You can find the rust project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/rust)

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

For this tutorial, will be focusing on `first_coin.ts` and re-using the `first_transaction.ts` library from the previous tutorial.

You can find the typescript project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript)

  </TabItem>
</Tabs>

## Step 1) Deploy MoonCoin module

### Step 1.1) Download Aptos-core

For the simplicity of this exercise, Aptos-core has a `move-examples` directory that makes it easy to build and test Move modules without downloading additional resources. Over time, we will expand this section to describe how to leverage [Move](https://github.com/move-language/move/tree/main/language/documentation/tutorial) tools for development.

For now, download and prepare Aptos-core:

```bash
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
./scripts/dev_setup.sh
source ~/.cargo/env
git checkout origin/devnet
```

Install Aptos Commandline tool. Learn more about the [Aptos command line tool](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos)
```bash
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos
```

### Step 1.2) Review the Module

In this terminal, change directories to `aptos-move/move-examples/moon_coin`. Keep this terminal window for the rest of this tutorial- we will refer to it later as the "Move Window". The rest of this section will review the file `sources/MoonCoinType.move`.

This module enables users to create a new MoonCoinType::MoonCoin::MoonCoin that can be used to register with the framework Coin module (0x1::Coin) to create a standard Coin. Developers can write their own functionalities in the MoonCoin module if they want to do more than what's provided by the standard 0x1::Coin or 0x1::ManagedCoin (adds mint/burn functionalities).  

```rust
module MoonCoinType::MoonCoin {
    struct MoonCoin {}
}
```

The code is very simple as we are not adding more functionalities to MoonCoin beyond the standard ones provided by the framework Coin (transfer, deposit, withdraw, mint, burn). The most important part is struct MoonCoin, which defines a new type of coin that can be registered with 0x1::Coin.

### Step 1.3) Deploying the Move module containing MoonCoin type

<Tabs>
<TabItem value="python" label="Python" default>
For Python3:

* Download the [example project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python)
* Open your favorite terminal and navigate to where you downloaded the above example project
* Install the required libraries: `pip3 install -r requirements.txt`.
* Execute the example: `python3 first_coin.py MoonCoin.mv`

</TabItem>
<TabItem value="rust" label="Rust">
For Rust:

* Download the [example project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/rust)
* Open your favorite terminal and navigate to where you downloaded the above example project
* Execute the example: `cargo run --bin first_coin -- MoonCoin.mv`

</TabItem>
<TabItem value="typescript" label="Typescript">
For Typescript:

* Download the [example project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript)
* Open your favorite terminal and navigate to where you downloaded the above example project
* Install the required libraries: `yarn install`
* Execute the example: `yarn first_coin MoonCoin.mv`

</TabItem>
</Tabs>

### Step 1.4) Verify output

* After a few moments it will mention that "Update the module with Alice's address, build, copy to the provided path,
  and press enter."
* In the "Move Window" terminal, and for the Move file we had previously looked at:
  * Copy Alice's address
  * Compile the modules with Alice's address by `aptos move compile --package-dir . --named-addresses MoonCoinType=0x{alice_address_here}`. Here, we replace the generic named address `MoonCoinType='_'` in `moon_coin/move.toml` with Alice's Address
  * Copy `build/Examples/bytecode_modules/MoonCoin.mv` to the same folder as this tutorial project code
* Return to your other terminal window, and press "enter" at the prompt to continue executing the rest of the code


The output should look like the following:

```
=== Addresses ===
Alice: 11c32982d04fbcc79b694647edff88c5b5d5b1a99c9d2854039175facbeefb40
Bob: 7ec8f962139943bc41c17a72e782b7729b1625cf65ed7812152a5677364a4f88

Update the module with Alice's address, build, copy to the provided path, and press enter.
```

## Step 2) Initialize MoonCoin

The MoonCoin module has alreayd been deployed. The next step is to initialize MoonCoin. In this example, we'll be using 0x1::ManagedCoin::initialize since we want the ability to mint/burn our new MoonCoin. This adds standard functionalities to MoonCoin such as transfer, mint, burn and standard events (register, deposit, withdraw).

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_coin.py section_1
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_coin/src/lib.rs section_1
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_coin.ts section_1
```

  </TabItem>
</Tabs>

## Step 3) Register a recipient account to receive MoonCoin

In other networks, since tokens/coins are just balance numbers in a contract, anyone can "send" anyone else a random coin, even if the recipient doesn't want it. In Aptos, a user needs to explicitly register to receive a ```Coin<RandomCoin>``` before it can be sent to them.

To register, the recipient just needs to call ```0x1::Coin::register<CoinType>```:

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_coin.py section_2
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_coin/src/lib.rs section_2
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_coin.ts section_2
```

  </TabItem>
</Tabs>

## Step 4) Mint MoonCoin to the recipient as the owner of the MoonCoin

When initializing a new Coin (Step 2), the owning account receives capabilities to mint/burn the new coin. The owner account can mint MoonCoin by calling 0x1::ManagedCoin::mint.

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_coin.py section_3
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_coin/src/lib.rs section_3
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_coin.ts section_3
```

  </TabItem>
</Tabs>

## Step 5) Check Bob's balance of MoonCoin

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_coin.py section_4
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_coin/src/lib.rs section_4
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_coin.ts section_4
```

  </TabItem>
</Tabs>

The data can be verified by visiting either a REST interface or the explorer:
* Alice's account via the [REST interface][alice_account_rest]
* Bob's account on the [explorer][bob_account_explorer]

[account_basics]: /basics/basics-accounts
[alice_account_rest]: https://fullnode.devnet.aptoslabs.com/accounts/a52671f10dc3479b09d0a11ce47694c0/
[bob_account_explorer]: https://explorer.devnet.aptos.dev/account/ec6ec14e4abe10aaa6ad53b0b63a1806
[rest_spec]: https://fullnode.devnet.aptoslabs.com/spec.html
