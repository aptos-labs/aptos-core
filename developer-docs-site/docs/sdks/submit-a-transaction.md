---
title: "Submitting a transaction"
slug: "submit-a-transaction"
---

# Submitting Transactions through TS SDK

This tutorial shows the steps of creating, signing and submitting a transaction through Typescript SDK. As tutorial **[Your First Transaction](https://aptos.dev/tutorials/your-first-transaction)** points out, transactions in JSON format can be submitted through Aptos REST APIs. Typescript SDK provides wrappers to significantly reduce the amount of manual work needed to prepare and submit transactions in JSON format. Typescript SDK also supports signing and submitting transactions in BCS format. See [Creating a Signed Transaction](https://aptos.dev/guides/creating-a-signed-transaction) for more info. In this tutorial you will submit a transaction in BCS format.

# Prerequisites

In order to follow along the tutorial, you need to install the latest TS SDK. Go to your project root directory and run

`npm install aptos` or `yarn add aptos`

:::note
Although Typescript is used in this tutorial, Aptos TS SDK also works in Javascript projects.
:::

The source code of this tutorial can be found here: TODO add link

# Step 1: Create accounts

Let’s assume user Alice wants to send 717 test coins to user Bob. We need to create two user accounts first.

```ts
import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes } from "aptos";

// devnet is used here for testing
const NODE_URL = "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = "https://faucet.devnet.aptoslabs.com";

const client = new AptosClient(NODE_URL);
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

// Generates key pair for Alice
const alice = new AptosAccount();
// Creates Alice's account and mint 5000 test coins
await faucetClient.fundAccount(alice.address(), 5000);

let resources = await client.getAccountResources(alice.address());
let accountResource = resources.find((r) => r.type === "0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>");
console.log(`Alice coins: ${(accountResource?.data as any).coin.value}. Should be 5000!`);

// Generates key pair for Bob
const bob = new AptosAccount();
// Creates Bob's account and mint 0 test coins
await faucetClient.fundAccount(bob.address(), 0);

resources = await client.getAccountResources(bob.address());
accountResource = resources.find((r) => r.type === "0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>");
console.log(`Bob coins: ${(accountResource?.data as any).coin.value}. Should be 0!`);
```

We created two accounts on Aptos’s devenet, and minted 5000 test coins for Alice’s account and 0 test coin for Bob’s account.

# Step 2: Prepare transaction payload

Typescript SDK supports 3 types of transaction payloads: `ScriptFunction`, `Script` and `ModuleBundle`. See [https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.TransactionPayload.html](https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.TransactionPayload.html) for the details.

ScriptFunction payload is used to invoke an on-chain Move script function. Within ScriptFunction payload, you are able to specify the function name, arguments, etc. Script payload contains the bytecodes for Aptos VM to execute. Within Script payload, you are able to provide script code in bytes and the arguments to the script. ModuleBundle payload is used to publish multiple modules at once. Within ModuleBundle payload, you are able to provide the module bytecodes.

To transfer coins from Alice’s account to Bob’s account, we need to prepare a ScriptFunction payload with a `transfer` function.

```ts
// We need to pass a token type to the `transfer` function.
const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::TestCoin::TestCoin"));

const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
  TxnBuilderTypes.ScriptFunction.natual(
    // Fully qualified module name, `AccountAddress::ModuleName`
    "0x1::Coin",
    // Module function
    "transfer",
    // The coin type to transfer
    [token],
    // Arguments for function `transfer`: receiver account address and amount to transfer
    [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(bob.address())), BCS.bcsSerializeUint64(717)],
  ),
);
```

The Move function `transfer` requires a coin type as type argument. Function `transfer` is defined here [https://github.com/aptos-labs/aptos-core/blob/faf4f94260d4716c8a774b3c17f579d203cc4013/aptos-move/framework/aptos-framework/sources/Coin.move#L311](https://github.com/aptos-labs/aptos-core/blob/faf4f94260d4716c8a774b3c17f579d203cc4013/aptos-move/framework/aptos-framework/sources/Coin.move#L311). In above code snippet, we want to transfer the `TestCoin` that is defined under account `0x1` and module `TestCoin`. The fully qualified name for the TestCoin is therefore “0x1::TestCoin::TestCoin”.

All arguments in ScriptFunction payload need to be BCS serialized. In above snippet, we serialized Bob’s account address and the amount number to transfer.

# Step 3: Sign and submit the transaction

After assembling a transaction payload, we are ready to create a RawTransaction instance that wraps the payload we just created. RawTransaction can then be signed and submitted.

```ts
// Sequence number is a security measure to prevent re-play attack.
const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
  client.getAccount(alice.address()),
  client.getChainId(),
]);

// See class definiton here
// https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.RawTransaction.html#constructor.
const rawTxn = new TxnBuilderTypes.RawTransaction(
  // Transaction sender account address (Alice's)
  TxnBuilderTypes.AccountAddress.fromHex(alice.address()),
  // Account sequnece number
  BigInt(sequnceNumber),
  // Payload we assembled from the previous step
  scriptFunctionPayload,
  // Max gas unit to spend
  1000n,
  // Gas price per unit
  1n,
  // Expiration timestamp. Transaction is discarded if it is not executed within 10 seconds from now.
  BigInt(Math.floor(Date.now() / 1000) + 10),
  // The chain id that this transaction is targeting
  new TxnBuilderTypes.ChainId(chainId),
);

// Sign the raw transaction with account1's private key
const bcsTxn = AptosClient.generateBCSTransaction(alice, rawTxn);
// Submit the transaction
const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

// Wait for the transaction to finish
await client.waitForTransaction(transactionRes.hash);

resources = await client.getAccountResources(bob.address());
accountResource = resources.find((r) => r.type === "0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>");
console.log(`Bob coins: ${(accountResource?.data as any).coin.value}. Should be 717!`);
```

# Output

The output after executing:

```tsx
Alice coins: 5000. Should be 5000!
Bob coins: 0. Should be 0!
Bob coins: 717. Should be 717!
```

# Security of BCS transaction

Submitting transactions in BCS format is more secure than submitting transaction in JSON format. Code that submits transactions in JSON format delegates the signing messages creation to REST API server. This renders a risk that a user signs an unintended transaction faked by a malicious API server. The malicious API server could potentially transfer all the balance under a user’s account to another account.
