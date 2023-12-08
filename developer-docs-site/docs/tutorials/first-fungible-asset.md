---
title: "Your First Fungible Asset"
slug: "your-first-fungible-asset"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Fungible Asset

This tutorial introduces how you can compile, deploy, and mint your own fungible asset (FA), named [FACoin](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/fungible_asset/fa_coin).
Make sure you have understood FA before moving on to the tutorial. If not, it is highly recommended to read it first.

* [Fungible Asset](../standards/fungible-asset.md)

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [TypeScript SDK](../sdks/new-ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)
* [Rust SDK](../sdks/rust-sdk.md)

---

## Step 2: Install the CLI

[Install the precompiled binary for the Aptos CLI](../tools/aptos-cli/install-cli/index.md).

---

## Step 3: Run the example

<Tabs groupId="examples">
  <TabItem value="typescript" label="Typescript">

Clone the `aptos-ts-sdk` repo:

```bash
git clone https://github.com/aptos-labs/aptos-ts-sdk.git
```

Navigate to the TypeScript SDK directory:

```bash
cd ~/aptos-ts-sdk/examples/typescript
```

Install the necessary dependencies:

```bash
pnpm install
```

Run the TypeScript [`your_fungible_asset`](https://github.com/aptos-labs/aptos-ts-sdk/blob/main/examples/typescript/your_fungible_asset.ts) example:

```bash
pnpm run your_fungible_asset
```

  </TabItem>
</Tabs>

---


The application will complete, printing:

```bash
===Publishing FAcoin package===

Transaction hash: 0x90fbe811171dde1ffad9157314ce0c4f6070fd5c1095a0e18a0b5634d10d7f48
metadata address: 0x77503715cb75fcc276b4e7236210fb0bcac7b510f6428233b65468b1cd3d708b
All the balances in this exmaple refer to balance in primary fungible stores of each account.
Alice's initial FACoin balance: 0.
Bob's initial FACoin balance: 0.
Charlie's initial balance: 0.
Alice mints Charlie 100 coins.
Charlie's updated FACoin primary fungible store balance: 100.
Alice freezes Bob's account.
Alice as the admin forcefully transfers the newly minted coins of Charlie to Bob ignoring that Bob's account is frozen.
Bob's updated FACoin balance: 100.
Alice unfreezes Bob's account.
Alice burns 50 coins from Bob.
Bob's updated FACoin balance: 50.
Bob transfers 10 coins to Alice as the owner.
Alice's updated FACoin balance: 10.
Bob's updated FACoin balance: 40.
```

---

## Step 4: FACoin in depth

### Step 4.1: Building and publishing the FACoin package

Move contracts are effectively a set of Move modules known as a package. When deploying or upgrading a new package, the compiler must be invoked with `--save-metadata` to publish the package. In the case of FACoin, the following output files are critical:

- `build/Examples/package-metadata.bcs`: Contains the metadata associated with the package.
- `build/Examples/bytecode_modules/fa_coin.mv`: Contains the bytecode for the `fa_coin.move` module.

These are read by the example and published to the Aptos blockchain:

<Tabs groupId="examples">
  <TabItem value="typescript" label="Typescript">

In the TypeScript example, we use `aptos move build-publish-payload` command to compile and build the module. 
That command builds the `build` folder that contains the `package-metadata.bcs` and the bytecode for the `moon_coin.mv` module. The command also builds a publication transaction payload and stores it in a JSON output file that we can later read from to get the `metadataBytes` and `byteCode` to publish the contract to chain with.

Compile the package:
```ts
export function compilePackage(
  packageDir: string,
  outputFile: string,
  namedAddresses: Array<{ name: string; address: AccountAddress }>,
) {
  const addressArg = namedAddresses.map(({ name, address }) => `${name}=${address}`).join(" ");
  // Assume-yes automatically overwrites the previous compiled version, only do this if you are sure you want to overwrite the previous version.
  const compileCommand = `aptos move build-publish-payload --json-output-file ${outputFile} --package-dir ${packageDir} --named-addresses ${addressArg} --assume-yes`;
  execSync(compileCommand);
}

compilePackage("move/facoin", "move/facoin/facoin.json", [{ name: "FACoin", address: alice.accountAddress }]);
```

Publish the package to chain:
```ts
export function getPackageBytesToPublish(filePath: string) {
  // current working directory - the root folder of this repo
  const cwd = process.cwd();
  // target directory - current working directory + filePath (filePath json file is generated with the prevoius, compilePackage, cli command)
  const modulePath = path.join(cwd, filePath);

  const jsonData = JSON.parse(fs.readFileSync(modulePath, "utf8"));

  const metadataBytes = jsonData.args[0].value;
  const byteCode = jsonData.args[1].value;

  return { metadataBytes, byteCode };
}

const { metadataBytes, byteCode } = getPackageBytesToPublish("move/facoin/facoin.json");

// Publish FACoin package to chain
const transaction = await aptos.publishPackageTransaction({
  account: alice.accountAddress,
  metadataBytes,
  moduleBytecode: byteCode,
});

const pendingTransaction = await aptos.signAndSubmitTransaction({
  signer: alice,
  transaction,
});

await aptos.waitForTransaction({ transactionHash: pendingTransaction.hash });
```

  </TabItem>
</Tabs>

---

### Step 4.2: Understanding the FACoin module

The FACoin module contains a function called `init_module` in which it creates a named metadata object that defines a type of FA called "FACoin" with a bunch of properties. The `init_module` function is called when the module is published. In this case, FACoin initializes the `FACoin` metadata object, owned by the owner of the account. According to the module code, the owner will be the admin of "FACoin" so that they are entitled to manage "FACoin" under the fungible asset framework.

:::tip Managed Fungible Asset framework
[`Managed Fungible Asset`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/fungible_asset/managed_fungible_asset/sources/managed_fungible_asset.move) is a full-fledged FA management framework for FAs directly managed by users. It provides convenience wrappers around different `refs` and both primary and secondary fungible stores. This example is a simplified version that only deal with primary stores.
:::

---

### Step 4.3: Understanding the management primitives of FACoin

The creator of FACoin have several managing primitives:

- **Minting**: Creating new coins.
- **Burning**: Deleting coins.
- **Freezing/Unfreezing**: Disabling/Enabling the owner of an account to withdraw from or deposit to their primary fungible store of FACoin.
- **Withdraw**: Withdrawing FACoin from the primary store of any account regardless of being frozen or not.
- **Deposit**: Deposit FACoin from the primary store of any account regardless of being frozen or not.
- **Transfer**: Withdraw from one account and deposit to another regardless of either being frozen or not.

:::tip
The entity that creates FACoin gains the capabilities for minting, burning, freezing/unfreezing, and forceful transferring between any fungible stores no matter they are frozen or not.
So `Withdraw`, `Deposit`, and `Transfer` in the management module have different semantics than those described in fungible asset framework that limited by frozen status.
:::

---

#### Step 4.3.1: Initializing "FACoin" metadata object

After publishing the module to the Aptos blockchain, the entity that published that coin type should initialize a metadata object describing the information about this FA:

```rust title="fa_coin.move snippet"
:!: static/move-examples/fungible_asset/fa_coin/sources/FACoin.move initialize
```

This ensures that this FA type has never been initialized before using a named object. Notice the first line create a named object with a static seed, if the metadata object has been created it will abort.
Then we call `primary_fungible_store::create_primary_store_enabled_fungible_asset` to create a FA metadata resource inside the newly created object, most of the time you call this function to initialize
the metadata object. After this call, we generate all the `Refs` that are necessary for the management api and store them in a customized wrapper resource.

:::tip
FACoin does all the initialization automatically upon package publishing via `init_module(&signer)`.
:::

Different from coin module, FA doesn't require users to register to use it because primary store will be automatically created if necessary.

---

#### Step 4.3.3: Managing a coin

Minting coins requires `MintRef` that was produced during initialization. the function `mint` (see below) takes in the creator and an amount, and returns a `FungibleAsset` struct containing that amount of FA. If the FA tracks supply, it will be updated.

```rust title="fa_coin.move snippet"
:!: static/move-examples/fungible_asset/fa_coin/sources/FACoin.move mint
```

`FACoin` makes this easier by providing an entry function `fa_coin::mint` that accesses the required `MintRef` for the creator.

Similarly, the module provides `burn`, `set_frozen_flag`, `transfer`, `Withdraw` and `Deposit` functions to manage FACoin following the same pattern with different refs.

---

#### Step 4.3.4: API of Transferring FAs

Aptos provides several APIs to support FA flows with same names in different modules:

- `fungible_asset::{transfer/withdraw/deposit}`: Move FA between different unfrozen fungible stores objects.
- `fungible_asset::{transfer/withdraw/deposit}_with_ref`: Move FA between different fungible stores objects with the corresponding `TransferRef` regardless of their frozen status.
- `primary_fungible_store::{transfer/withdraw/deposit}`: Move FA between unfrozen primary stores of different accounts.

:::tip important
There are two separate withdraw and deposit events instead of a single transfer event.
:::

## Supporting documentation

* [Aptos CLI](../tools/aptos-cli/use-cli/use-aptos-cli.md)
* [Fungible Asset](../standards/fungible-asset.md)
* [TypeScript SDK](../sdks/new-ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)
* [Rust SDK](../sdks/rust-sdk.md)
* [REST API specification](https://aptos.dev/nodes/aptos-api-spec#/)
