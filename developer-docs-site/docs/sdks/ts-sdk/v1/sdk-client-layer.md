---
title: "API Client Layer"
slug: "typescript-sdk-client-layer"
---

The API client layer in the SDK provides a robust and reliable communication channel between the client-side application and the blockchain server. It is a component of the SDK that enables developers to interact with the network through the use of application programming interfaces (APIs). The client layer is responsible for making API calls to the network, sending and receiving data to and from the network, and handling any errors or exceptions that may occur during the process.

The client layer is used to communicate with the Aptos REST API the Aptos Indexer API and handling of errors and exceptions.
In addition, the client layer component supports submitting transactions in BCS format, which prepares and signs the raw transactions on the client-side. This method leverages the BCS Library and Transaction Builder for constructing the transaction payloads.

By following the instructions in this documentation, you should be able to configure and use the client layer in your project

## Provider class

The client layer exports a [Provider](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/providers/provider.ts) class that extends both the Aptos REST API and the Aptos Indexer API.
The `Provider` class acts as a mediator between the client-side application and the blockchain server, ensuring reliable and efficient communication.
This class provides a high-level interface for the application to interact with the blockchain server. The class is designed to be easy to use and understand, allowing developers to quickly integrate the SDK into their applications.

The `Provider` class accepts:

- `network` - network enum type `mainnet | testnet | devnet` indicates the network the app interacts with.
- `CustomEndpoints` of type `{fullnodeUrl: string, indexerUrl: string}` - this is to support devs who run their own nodes/indexer or to use local development against a local testnet.
- `Config` - an optional argument the AptosClient accepts.
- `doNotFixNodeUrl` - an optional argument the AptosClient accepts.

## Initializing the Provider class

To initialize the Provider class, you will need to pass in the necessary configuration parameters. Here is an example:

```ts
import { Provider, Network } from "aptos";

const provider = new Provider(Network.TESTNET);
```

## Fetch data from chain

To make an API call, you will need to call the appropriate method on the Provider class. The method name and parameters will depend on the specific API you are using. Here is an example:

```ts
const account = await provider.getAccount("0x123");
```

In this example, we are using the `getAccount()` method to retrieve information about an account with the address `0x123`.

## Submit Transaction to chain

To submit a transaction to the Aptos network we should:

1. Generate a raw transaction
2. Sign the generated raw transaction
3. Submit the signed transaction

### Generate a Raw Transaction

The TypeScript SDK provides 2 efficient ways to `generate a raw transaction` that can be signed and submitted to chain.

#### Transaction Builder

The `generateTransaction()` method, accepts an `entry function payload` type and is available for entry funtion transaction submission. It uses the [TransactionBuilderRemoteABI](https://aptos-labs.github.io/ts-sdk-doc/classes/TransactionBuilderRemoteABI.html) to fetch the ABI from the blockchain, serializes the payload arguments based on the entry function argument types and generates and return a raw transaction that can be signed and submitted to the blockchain.

```ts
const alice = new AptosAccount();

const payload = {
  function: "0x123::todolist::create_task",
  type_arguments: [],
  arguments: ["read aptos.dev"],
};

const rawTxn = await provider.generateTransaction(alice.address(), entryFunctionPayload);
```

`function` – This must be a fully qualified function name and composed of `module address`, `module name` and `function name` separated by `::`.
`type_arguments` – This is for the case a Move function expects a generic type argument.
`arguments` – The arguments the function expects.

:::tip
To submit an entry function payload, using the Transaction Builder would be simpler to use as the developer do not need to deal with BCS serialization.
:::

#### BCS Transaction

The `generateRawTransaction()` method, accept `any transaction payload type (entry, script, multisig)` and exepcts for the arguments passed in to be serialized. It then generates and returns a raw transaction that can be signed and submitted to chain.

```ts
const alice = new AptosAccount();

const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
  TxnBuilderTypes.EntryFunction.natural("0x123::todolist", "create_task", [], [bcsSerializeStr("read aptos.dev")]),
);

const rawTxn = await provider.generateRawTransaction(alice.address(), entryFunctionPayload);
```

For simplicity, the TypeScript SDK provides a method that can submit a BCS transaction in a one call.

```ts
const rawTxn = await provider.generateSignSubmitTransaction(alice, entryFunctionPayload);
```

### Sign a Raw Transaction

Once one has generated a raw transaction, they need to sign this transaction with their private key. The TypeScript SDK provides a method that accepts an `aptos account` and `a raw transaction` and signs it.

```ts
const signedTxn = AptosClient.generateBCSTransaction(alice, rawTxn);
```

### Submit transaction to blockchain

Once a transaction has been signed, it is ready to be submitted to the blockchain. The TypeScript SDK provides a method that accepts the `signed transaction` and submits it to the Aptos network.

```ts
const transactionRes = await provider.submitSignedBCSTransaction(signedTxn);
```

## Learn more

The Provider class extends both [AptosClient](./aptos-client.md) and [IndexerClient](./indexer-client.md) classes and gives the end user the option to simply create a Provider instance and call a method by hiding the underlying implementation. If, for any reason, you want to use AptosClient or IndexerClient directly without the Provider class, you are able to do it.
