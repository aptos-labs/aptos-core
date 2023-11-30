---
title: "Migration Guide"
---

If you are coming from an earlier version `1.x.x` of `aptos`, you will need to make the following updates.

:::note
In this guide we only mention the API differences and updates you would need to do and excluding new features implementation
:::

### SDK usage and query the Aptos chain

Removed all `<*>Client` modules (i.e `AptosClient`, `FaucetClient`, `CoinClient`, etc) and replaced with a `Aptos` entry point class

**V1**

```ts
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
const aptosClient = new AptosClient(NODE_URL);
const indexerClient = new IndexerClient(INDEXER_URL);
const tokenClient = new TokenClient(aptosClient);
```

**V2**
:::tip
Read more about it [here](./sdk-configuration.md)
:::

```ts
const aptos = new Aptos();

// make queries
const fund = await aptos.fundAccount({ accountAddress: "0x123", amount: 100 });
const modules = await aptos.getAccountModules({ accountAddress: "0x123" });
const tokens = await aptos.getAccountOwnedTokens({ accountAddress: "0x123" });
```

### Configuration class

Introduce `AptosConfig` class that holds the config information for the SDK. Once define it we can pass and use it with the `Aptos` class

```ts
const aptosConfig = new AptosConfig({ network: Network.TESTNET }); // default to devnet
const aptos = new Aptos(config);
```

### Transaction Builder Flow

Removed all separate transaction functions in favor of a more simplified and friendlier transaction builder flow

**V1**

```ts
const aptosClient = new AptosClient(NODE_URL);

// bcs serialized arguments payload
const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
  TxnBuilderTypes.EntryFunction.natural(
    "0x1::aptos_account",
    "transfer",
    [],
    [bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiver.address()))],
  ),
);
// generate a raw transaction
const transaction = await client.generateRawTransaction(sender.address(), entryFunctionPayload);

// non-serialized arguments payload
const payload: Gen.TransactionPayload = {
  type: "entry_function_payload",
  function: "0x1::aptos_account::transfer",
  type_arguments: [],
  arguments: [account2.address().hex(), 100000],
};
// generate a raw transaction
const transaction = await client.generateTransaction(account1.address(), payload);

// sign transaction
const signedTransaction = AptosClient.generateBCSTransaction(sender, transaction);
// submit transaction
const txn = await client.submitSignedBCSTransaction(signedTransaction);
```

**V2**
:::tip
Read more about it [here](./transaction-builder.md)
:::

```ts
const aptos = new Aptos();

// non-serialized arguments transaction
const transaction = await aptos.build.transaction({
  sender: alice.accountAddress,
  data: {
    function: "0x1::coin::transfer",
    typeArguments: ["0x1::aptos_coin::AptosCoin"],
    functionArguments: [bobAddress, 100],
  },
});

// bcs serialized arguments transaction
const transaction = await aptos.build.transaction({
  sender: alice.accountAddress,
  data: {
    function: "0x1::coin::transfer",
    typeArguments: [parseTypeTag("0x1::aptos_coin::AptosCoin")],
    functionArguments: [bobAddress, new U64(100)],
  },
});
// sign transaction
const senderAuthenticator = aptos.sign.transaction({ signer: alice, transaction });
// submit transaction
const committedTransaction = await aptos.submit.transaction({ transaction, senderAuthenticator });
```

### Key Management

Rename `AptosAccount` to `Account` and use static methods to generate / derive an account

**V1**

```ts
// generate a new account (or key pair) OR derive from private key OR derive from private key and address
const account = new AptosAccount(); // supports only Legacy Ed25519

// derive account from derivation path
const account = AptosAccount.fromDerivePath(..)
```

**V2**

:::tip
Read more about it [here](./key-management.md)
:::

```ts
// generate a new account (or key pair)
const account = Account.generate(); // defaults to Legacy Ed25519
const account = Account.generate({ scheme: SingingSchemeInput.Secp256k1 }); // Single Sender Secp256k1
const account = Account.generate({ scheme: SingingSchemeInput.Ed25519, legacy: false }); // Single Sender Ed25519

// derive account from private key
const account = Account.fromPrivateKey({ privateKey });

// derive account from private key and address
const account = Account.fromPrivateKeyAndAddress({ privateKey, address: accountAddress });

// derive account from derivation path
const acccount = Account.fromDerivationPath({
  path,
  mnemonic,
  scheme: SigningSchemeInput.Ed25519,
});
```
