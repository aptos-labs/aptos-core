---
title: "TypeScript Index"
slug: "index"
hidden: false
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';

# Aptos TypeScript SDK

:::note
This documentation is for TypeScript SDK V1, aka `aptos`. For a more robust and better SDK support, we recommend upgrading to the new TypeScript SDK [@aptos-labs/ts-sdk](https://github.com/aptos-labs/aptos-ts-sdk). Take a look at the [documentation](../ts-sdk-v2/index.md) and the [migration guide](../ts-sdk-v2/migration-guide.md)
:::

Aptos provides a fully supported TypeScript SDK with the source code in the [Aptos-core GitHub](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) repository. Much of the functionality of the TypeScript SDK can be found in the [Rust](../rust-sdk.md) and [Python](../python-sdk.md) SDKs. Nevertheless, Aptos strongly encourages you to use the TypeScript SDK for app development whenever possible.

Looking to migrate to `v2`? check out the [migration guide](./migration-guide.md)
:::

## Overview

Aptos provides a fully supported TypeScript SDK with the source code in the [aptos-ts-sdk GitHub](https://github.com/aptos-labs/aptos-ts-sdk) repository.
The Aptos TypeScript SDK provides a convenient way to interact with the Aptos blockchain using TypeScript. It offers a set of utility functions, classes, and types to simplify the integration process and enhance developer productivity.

- **Developer experience** Strongly typed APIs and Interfaces, autocomplete, comprehensive documentation.
- **Stability** Test suite runs against Aptos fullnode and indexer with a local network
- **Transaction Builder** Intuitive and simplified transaction builder flow
- **Serialization/deserialization support** Full nested serialization/deserialization support and Move sub-classes to easily serialize and deserialize Move types

## Installation

<Tabs groupId="install-sdk">
  <TabItem value="pnpm" label="pnpm">

```bash
 pnpm i @aptos-labs/ts-sdk
```

  </TabItem>
  <TabItem value="npm" label="npm">

```bash
 npm i @aptos-labs/ts-sdk
```

  </TabItem>
  <TabItem value="yarn" label="yarn">

```bash
 yarn add @aptos-labs/ts-sdk
```

  </TabItem>
    <TabItem value="bun" label="bun">

```bash
 bun i @aptos-labs/ts-sdk
```

  </TabItem>
</Tabs>

## Quick Start

### Set up Aptos

```ts
const aptos = new Aptos(); // default to devnet

// with custom configuration
const aptosConfig = new AptosConfig({ network: Network.TESTNET });
const aptos = new Aptos(aptosConfig);
```

### Fetch data from chain

```ts
const ledgerInfo = await aptos.getLedgerInfo();
const modules = await aptos.getAccountModules({ accountAddress: "0x123" });
const tokens = await aptos.getAccountOwnedTokens({ accountAddress: "0x123" });
```

### Transfer APT coin transaction

```ts
const transaction = await aptos.transferCoinTransaction({
  sender: alice,
  recipient: bob.accountAddress,
  amount: 100,
});
const pendingTransaction = await aptos.signAndSubmitTransaction({ signer: alice, transaction });
```

### Build and submit transaction

```ts
// generate a new account key pair
const alice: Account = Account.generate();

// create the account on chain
await aptos.fundAccount({ accountAddress: alice.accountAddress, amount: 1000 });

// submit transaction to transfer APT coin from Alice to Bob
const bobAddress = "0xb0b";

const transaction = await aptos.build.transaction({
  sender: alice.accountAddress,
  data: {
    function: "0x1::coin::transfer",
    typeArguments: ["0x1::aptos_coin::AptosCoin"],
    functionArguments: [bobAddress, 100],
  },
});

// using sign and submit separately
const senderAuthenticator = aptos.sign.transaction({ signer: alice, transaction });
const pendingTransaction = await aptos.submit.transaction({ transaction, senderAuthenticator });

// using signAndSubmit combined
const pendingTransaction = await aptos.signAndSubmitTransaction({ signer: alice, transaction });
```
