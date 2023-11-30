---
title: "Transaction Builder"
---

The SDK provides a simplified and meaningful transaction builder flow to handles the transaction creation lifecycle.

The transaction builder is separated to different namespaces for each transaction step in the transaction submission flow.
Each namespace/step can be accessed by initiating the [Aptos class](./sdk-configuration.md)

- **build** - Build a raw transaction that can be signed and then submitted to chain
- **simulate** - Simulate a transaction before signing and submitting to chain
- **sign** - Sign a raw transaction to later submit to chain
- **submit** - Submit a transaction to chain

Each step provides supports to all the different transaction types Aptos supports -

- **simple transaction** - Single signer
- **complex transaction** - Sponsor and multi agent

## Submit transaction

### Simple transaction

```ts
// build a transaction
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
const committedTransaction = await aptos.submit.transaction({ transaction, senderAuthenticator });

// using signAndSubmit combined
const committedTransaction = await aptos.signAndSubmitTransaction({ signer: alice, transaction });
```

### Complex transaction - Multi agent

```ts
// build a transaction
const transaction = await aptos.build.multiAgentTransaction({
  sender: alice.accountAddress,
  secondarySignerAddresses: [secondarySignerAccount.accountAddress],
  data: {
    function: "0x1::coin::transfer",
    typeArguments: ["0x1::aptos_coin::AptosCoin"],
    functionArguments: [bobAddress, 100],
  },
});

// sign transaction
const senderAuthenticator = aptos.sign.transaction({ signer: alice, transaction });
const secondarySignerAuthenticator = aptos.sign.transaction({ signer: secondarySignerAccount, transaction });
// submit transaction
const committedTransaction = await aptos.submit.multiAgentTransaction({
  transaction,
  senderAuthenticator,
  additionalSignersAuthenticators: [secondarySignerAuthenticator],
});
```

### Complex transaction - Simple transaction with Sponsor transaction

```ts
// build a transaction
const transaction = await aptos.build.transaction({
  sender: alice.accountAddress,
  withFeePayer: true,
  data: {
    function: "0x1::coin::transfer",
    typeArguments: ["0x1::aptos_coin::AptosCoin"],
    functionArguments: [bobAddress, 100],
  },
});

// sign transaction
const senderAuthenticator = aptos.sign.transaction({ signer: alice, transaction });
const feePayerSignerAuthenticator = aptos.sign.transactionAsFeePayer({
  signer: feePayerAccount,
  transaction,
});
// submit transaction
const committedTransaction = await aptos.submit.transaction({
  transaction,
  senderAuthenticator,
  feePayerAuthenticator: feePayerSignerAuthenticator,
});
```

### Complex transaction - Multi agent with Sponsor transaction

```ts
// build a transaction
const transaction = await aptos.build.multiAgentTransaction({
  sender: alice.accountAddress,
  secondarySignerAddresses: [secondarySignerAccount.accountAddress],
  withFeePayer: true,
  data: {
    function: "0x1::coin::transfer",
    typeArguments: ["0x1::aptos_coin::AptosCoin"],
    functionArguments: [bobAddress, 100],
  },
});

// sign transaction
const senderAuthenticator = aptos.sign.transaction({ signer: alice, transaction });
const secondarySignerAuthenticator = aptos.sign.transaction({ signer: secondarySignerAccount, transaction });
const feePayerSignerAuthenticator = aptos.sign.transactionAsFeePayer({
  signer: feePayerAccount,
  transaction,
});
// submit transaction
const committedTransaction = await aptos.submit.multiAgentTransaction({
  transaction,
  senderAuthenticator,
  additionalSignersAuthenticators: [secondarySignerAuthenticator],
  feePayerAuthenticator: feePayerSignerAuthenticator,
});
```

## Simulate transaction

### Simple transaction

```ts
const transaction = await aptos.build.transaction({
  sender: alice.accountAddress,
  data: {
    function: "0x1::coin::transfer",
    functionArguments: [bobAddress, 100],
  },
});
const [userTransactionResponse] = await aptos.simulate.transaction({
  signerPublicKey: alice.publicKey,
  transaction,
});
```

### Complex transaction - Multi agent

```ts
const transaction = await aptos.build.multiAgentTransaction({
  sender: alice.accountAddress,
  secondarySignerAddresses: [secondarySignerAccount.accountAddress],
  data: {
    function: "0x1::coin::transfer",
    functionArguments: [bobAddress, 100],
  },
});
const [userTransactionResponse] = await aptos.simulate.multiAgentTransaction({
  signerPublicKey: alice.publicKey,
  transaction,
  secondarySignersPublicKeys: [secondarySignerAccount.publicKey],
});
```

### Complex transaction - Simple transaction with Sponsor transaction

```ts
const transaction = await aptos.build.transaction({
  sender: alice.accountAddress,
  withFeePayer: true,
  data: {
    function: "0x1::coin::transfer",
    functionArguments: [bobAddress, 100],
  },
});
const [userTransactionResponse] = await aptos.simulate.transaction({
  signerPublicKey: alice.publicKey,
  transaction,
  feePayerPublicKey: feePayerAccount.publicKey,
});
```

### Complex transaction - Multi agent with Sponsor transaction

```ts
const transaction = await aptos.build.multiAgentTransaction({
  sender: alice.accountAddress,
  secondarySignerAddresses: [secondarySignerAccount.accountAddress],
  withFeePayer: true,
  data: {
    function: "0x1::coin::transfer",
    functionArguments: [bobAddress, 100],
  },
});
const [userTransactionResponse] = await aptos.simulate.multiAgentTransaction({
  signerPublicKey: alice.publicKey,
  transaction,
  secondarySignersPublicKeys: [secondarySignerAccount.publicKey],
  feePayerPublicKey: feePayerAccount.publicKey,
});
```

## Transaction Management

The TypeScript SDK provides a transaction management layer to submit as many transaction for a single account as possible while respecting a high throughput.

Read more about it [here](https://aptos.dev/guides/transaction-management)

In the SDK, the transaction management layer implements 2 components

- `AccountSequenceNumber` that handles and manages an account sequence number.
- `TransactionWorker` that provides a simple framework for receiving payloads to be processed

To use and leverage the transaction management layer, we provide an array of payloads to the batch function that in turns pass it into the worker to process and generate transactions and submit it to chain.

```ts
const aptos = new Aptos();
const sender = Account.generate();
await aptos.fundAccount({ accountAddress: sender.accountAddress, amount: 10000000000 })
// recipients is an array of accounts
const recipients = [Account.generate(),Account.generate(),Account.generate()]

// create payloads
const payloads: InputGenerateTransactionPayloadData[] = [];

for (let i = 0; i < recipients.length; i += 1) {
  const txn: InputGenerateTransactionPayloadData = {
    function: "0x1::aptos_account::transfer",
    functionArguments: [recipients[i].accountAddress, 10],
  };
  payloads.push(txn);
}

await aptos.batchTransactionsForSingleAccount({ sender, data: payloads }));
```
