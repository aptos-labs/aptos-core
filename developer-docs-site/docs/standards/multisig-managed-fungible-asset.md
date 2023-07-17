---
title: "Manage Fungible Assets with Aptos Multisig Account"
slug: "multisig-managed-fungible-assets"
---

# Manage Fungible Assets with Aptos Framework Multisig Account

This tutorial introduces a practical use case that combines Aptos framework multisig account with fungible asset standard to enhance the security margin of the management of fungible assets. Make sure you have understood module publishing and Aptos framework multisig account before moving on to the tutorial. If not, it is highly recommended to try out the following tutorials first:

* [Your First Move Module](../tutorials/first-move-module.md)

## Step 1: Pick an SDK

This tutorial was created for the [TypeScript SDK](../sdks/ts-sdk/index.md).

Other developers are invited to add support for the [Python SDK](../sdks/python-sdk.md), [Rust SDK](../sdks/rust-sdk.md), and [Unity SDK](../sdks/unity-sdk.md)!

## Step 2: Publish the module

To create a fungible asset controlled by an Aptos framework multisig account with all the administrative operations (mint, transfer, burn, freeze/unfreeze), a well-designed smart contract based on fungible asset standard is a prerequisite. The Aptos team provides an example code in `aptos-core` repo.

Clone the `aptos-core` repo:

```bash
git clone git@github.com:aptos-labs/aptos-core.git ~/aptos-core
```

Navigate to the `managed_fungible_asset` directory and then publish this package onto your `default` account using CLI:

```bash
cd ~/aptos-core/aptos-move/move-examples/fungible_asset/managed_fungible_asset
aptos move publish --named-addresses example_addr=default
```

Navigate to the `multisig_managed_coin` directory and then publish this package onto your `default` account using CLI too:

```bash
cd ~/aptos-core/aptos-move/move-examples/fungible_asset/multisig_managed_coin
aptos move publish --named-addresses example_addr=default
```

For this tutorial, `multisig_managed_coin` need to call functions defined in `managed_fungible_asset` on the same address. So both modules have to be published.

:::tip
Do not forget to fund the account with faucet before publishing modules.
:::

## Step 3: Start The example

```bash
cd ~/aptos-core/ecosystem/typescript/sdk/examples/typescript
```

Run the `multisig_managed_coin` example:

```bash
MODULE_ADDR=${DEFAULT_ACCOUNT_ADDRESS} pnpm run multisig_managed_coin
```

:::tip
This example uses the Aptos devnet, which has historically been reset each Thursday.
Make sure devnet is live when you try running the example!
if you are running local-testnet with faucet, you can run the following command instead:

```bash
APTOS_NODE_URL=http://0.0.0.0:8080 APTOS_FAUCET_URL=http://0.0.0.0:8081 MODULE_ADDR=${DEFAULT_ACCOUNT_ADDRESS}  pnpm run multisig_managed_coin
```

:::

The example script should execute successfully without any errors. Then you are able to see what it did by searching the `owner1` and `owner2` addresses printed to the console on Aptos explorer.

Let's follow the script to understand what it does:

### Generate single signer accounts

First, we will generate three single signer accounts, owner1, owner2 and owner3 who will co-own an Aptos framework multisig account.

```typescript title="Generate 3 single signers"
:!: static/sdks/typescript/examples/typescript/multisig_managed_coin.ts section_1
```

### Create an Aptos framework multisig account with a managed fungible asset

Next, let owner1 call the `initialize()` function defined in `multisig_managed_coin.move`, which first create an Aptos framework multisig account owned by owner1 and add both owner2 and owner3 as owners. Also, it creates a fungible asset called "meme coin" with customized settings denoted in the argument list and make the multisig account the admin of the fungible asset.
Also, each proposal needs at least 2 approvals to execute.

```typescript title="Query the multisig account and then call the initialize function"
:!: static/sdks/typescript/examples/typescript/multisig_managed_coin.ts section_2
```

### Mint

Then we mint 1000 and 2000 meme coin to owner2 and owner3, respectively. The proposed transaction is submitted by owner2 and gets an additional approval from owner3.

```typescript title="Mint 1000 to owner2 and 2000 to owner3"
:!: static/sdks/typescript/examples/typescript/multisig_managed_coin.ts section_3
```

### Freeze

After minting, the example shows how to freeze account owner1. The proposed transaction is again submitted by owner2 and approved by owner3 in addition.

```typescript title="Freeze owner1"
:!: static/sdks/typescript/examples/typescript/multisig_managed_coin.ts section_4
```

:::tip
Unfreeze is similar that just replace the last argument of `set_primary_stores_frozen_status` function to `false`.
:::

### Force transfer

When owner1 is frozen, normal transfer cannot withdraw from or deposit to that account. But as the admin of "meme coin", the multisig account has the capability to do that.
Next, Owner2 proposed a transaction to force transfer 1000 meme coins from owner3 to owner1. This time, owner1 approves it.

```typescript title="Force transfer 1000 meme coins from owner3 to owner1"
:!: static/sdks/typescript/examples/typescript/multisig_managed_coin.ts section_5
```

### Burn

Finally, all the three owners have 1000 meme coins. Let's burn all the coins! Owner2 makes the proposal and owner1 approves it.

```typescript title="Burn 1000 meme coins from all the three owners' accounts"
:!: static/sdks/typescript/examples/typescript/multisig_managed_coin.ts section_6
```

## Conclusion

This tutorial shows an e2e flow of using Aptos framework multisig account to administrate fungible asset. Similarly, you can create your own module and leverage our powerful SDK to create the administration schema that fits your needs.

