---
title: "Creating a Signed Transaction"
slug: "creating-a-signed-transaction"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Creating a Signed Transaction

All transactions executed on the Aptos blockchain must be signed. This requirement is enforced by the chain for security reasons.

## Generating the signing message

The first step in signing a transaction is to generate the signing message from the transaction. To generate such a signing message, you can use:

- The Aptos node's [REST API](https://fullnode.devnet.aptoslabs.com/v1/spec#/). The Aptos node will generate the signing message, the transaction signature and will submit the signed transaction to the Aptos blockchain. However, this approach is not secure.
  - Also see the tutorial [Your First Transaction](../tutorials/first-transaction.md) that explains this approach.
- However, you may prefer instead that your client application, for example, a hardware security module (HSM), be responsible for generating the signed transaction. In this approach, before submitting transactions, a client must:
  - Serialize the transactions into bytes, and
  - Sign the bytes with the account private key. See [Accounts][account] for how account and private key works.

This guide will introduce the concepts behind constructing a transaction, generating the appropriate message to sign using the BCS-serialization, and various methods for signing within Aptos.

:::tip

We strongly recommend that you use the BCS format for submitting transactions to the Aptos blockchain.

:::

## Overview

Creating a transaction that is ready to be executed on the Aptos blockchain requires the following four steps:

1. Create a raw transaction, `RawTransaction`, also called unsigned transaction.
2. Generate the signing message containing the appropriate salt (`prefix_bytes`), and generate the signature of the raw transaction by using the client's private key.
3. Create the `Authenticator` and the signed transaction, and
4. BCS-serialize the signed transaction (not shown in the diagram in [Overview](#overview) section).

:::info
Code examples in this section are in Typescript.
:::

See the below high-level flow diagram showing how a raw transaction becomes a signed transaction:

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/creating-signed-transaction-light.svg'),
    dark: useBaseUrl('/img/docs/creating-signed-transaction-dark.svg'),
  }}
/>

Unsigned transactions are known as `RawTransaction`s. They contain all the information about how to execute an operation on an account within Aptos. But they lack the appropriate authorization with a signature or `Authenticator`.

In Aptos blockchain, all the data is encoded as [BCS][bcs] (Binary Canonical Serialization).

Aptos supports many different approaches to signing a transaction but defaults to a single signer using [Ed25519][ed25519].

The `Authenticator` produced during the signing of the transaction gives authorization to the Aptos blockchain to execute the transaction on behalf of the account owner.

## Key concepts

### Raw transaction

A raw transaction consists of the following fields:

- **sender** (Address): Account address of the sender.
- **sequence_number** (uint64): Sequence number of this transaction. This must match the sequence number stored in the sender's account at the time the transaction executes.
- **payload**: Instructions for the Aptos blockchain, including publishing a module, execute a script function or execute a script payload.
- **max_gas_amount** (uint64): Maximum total gas to spend for this transaction. The account must have more than this gas or the transaction will be discarded during validation.
- **gas_unit_price** (uint64): Price to be paid per gas unit. During execution the `total_gas_amount`, calculated as: `total_gas_amount = total_gas_units_consumed * gas_unit_price`, must not exceed `max_gas_amount` or the transaction will abort during the execution. `total_gas_units_consumed` represents the total units of gas consumed when executing the transaction.
- **expiration_timestamp_secs** (uint64): The blockchain timestamp at which the blockchain would discard this transaction.
- **chain_id** (uint8): The chain ID of the blockchain that this transaction is intended to be run on.

### BCS

Binary Canonical Serialization (BCS) is a serialization format applied to the raw (unsigned) transaction. See [BCS][bcs] for a description of the design goals of BCS.

BCS is not a self-describing format. As such, in order to deserialize a message, one must know the message type and layout ahead of time.

An example of how BCS serializes a string is shown below:

```typescript
// A string is serialized as: byte length + byte representation of the string. The byte length is required for deserialization. Without it, no way the deserializer knows how many bytes to deserialize.
const bytes: Unint8Array = bcs_serialize_string("aptos");
assert(bytes == [5, 0x61, 0x70, 0x74, 0x6f, 0x73]);
```

### Signing message

The bytes of a BCS-serialized raw transaction are referred as a **signing message**.

In addition, in Aptos, any content that is signed or hashed is salted with a unique prefix to distinguish it from other types of messages. This is done to ensure that the content can only be used in the intended scenarios. The signing message of a RawTransaction is prefixed with `prefix_bytes`, which is `sha3_256("APTOS::RawTransaction")`. Therefore:

`signing_message = prefix_bytes | bcs_bytes_of_raw_transaction.`. `|` denotes concatenation.

:::note

The prefixing step is not shown in the diagram in the [Overview](#overview) section.

:::

### Signature

A signature is the result of hashing the signing message with the client's private key. By default Aptos uses the [Ed25519][ed25519] scheme to generate the signature of the raw transaction.

- By signing a signing message with the private key, clients prove to the Aptos blockchain that they have authorized the transaction be executed.
- Aptos blockchain will validate the signature with client account's public key to ensure that the transaction submitted is indeed signed by the client.

### Signed transaction

A signed transaction consists of:

- A raw transaction, and
- An authenticator. The authenticator contains a client's public key and the signature of the raw transaction.

This signed transaction is further BCS-serialized (not shown in the diagram in [Overview](#overview) section), after which the signed transaction is ready for submission to the [Aptos REST interface](https://fullnode.devnet.aptoslabs.com/v1/spec#/).

### Multisignature transactions

The Aptos blockchain supports several signing methods for transactions, including the single signature, the K-of-N multisig, and more.

A K-of-N multisig transaction means that for such a transaction to be executed, at least K out of the N authorized signers have signed the transaction and passed the check conducted by the chain.

Transaction signatures are wrapped in `Authenticator`. The Aptos blockchain validates the transactions submitted by clients by using the Authenticator data. See a few examples below:

In Typescript, this is how a single signer authenticator looks like:

```typescript
interface Authenticator {
  public_key: Uint8Array;
  signature: Uint8Array;
}
```

A multisig authenticator example is shown below:

```typescript
interface MultiEd25519PublicKey {
  // A list of public keys
  public_keys: Uint8Array[];
  // At least `threshold` signatures must be valid
  threshold: Uint8;
}

interface MultiEd25519Signature {
  // A list of signatures
  signatures: Uint8Array[];
  // 4 bytes, at most 32 signatures are supported.
  // If Nth bit value is `1`, the Nth signature should be provided in `signatures`. Bits are read from left to right
  bitmap: Uint8Array;
}

interface MultisigAuthenticator {
  public_key: MultiEd25519PublicKey;
  signature: MultiEd25519Signature;
}
```

## Detailed steps

1. Creating a RawTransaction.
2. Preparing the message to be signed and signing it.
3. Creating an Authenticator and a SignedTransaction.
4. Serializing SignedTransaction.

### Step 1. Creating a RawTransaction

The below example assumes the transaction has a script function payload.

```typescript
interface AccountAddress {
  // 32-byte array
  address: Uint8Array;
}

interface ModuleId {
  address: AccountAddress;
  name: string;
}

interface ScriptFunction {
  module: ModuleId;
  function: string;
  ty_args: string[];
  args: Uint8Array[];
}

interface RawTransaction {
  sender: AccountAddress;
  sequence_number: number;
  payload: ScriptFunction;
  max_gas_amount: number;
  gas_unit_price: number;
  expiration_timestamp_secs: number;
  chain_id: number;
}

function createRawTransaction(): RawTransaction {
  const payload: ScriptFunction = {
    module: {
      address: hexToAccountAddress("0x01"),
      name: "AptosCoin",
    },
    function: "transfer",
    ty_args: [],
    args: [
      BCS.serialize(hexToAccountAddress("0x02")), // receipient of the transfer
      BCS.serialize_uint64(2), // amount to transfer
    ],
  };

  return {
    sender: hexToAccountAddress("0x01"),
    sequence_number: 1n,
    max_gas_amount: 2000n,
    gas_unit_price: 1n,
    // Unix timestamp, in seconds + 10 minutes
    expiration_timestamp_secs: Math.floor(Date.now() / 1000) + 600,
    payload: payload,
    chain_id: 3,
  };
}
```

:::note

The `BCS` serialization shown in the code above is not the same as the BCS Serialization operation shown in the [Overview](#overview) section.

:::

### Step 2. Creating the signing message and signing it

1. Generate prefix (`prefix_bytes`) with [SHA3_256][sha3] hash bytes of string `APTOS::RawTransaction`.
2. Bytes of BCS serialized RawTransaction.
3. Concat the prefix and BCS bytes.
4. Signing the bytes with account private key.

```typescript
import * as Nacl from "tweetnacl";

function hashPrefix(): Buffer {
  let hash = SHA3.sha3_256.create();
  hash.update(`APTOS::RawTransaction`);
  return Buffer.from(hash.arrayBuffer());
}

function bcsSerializeRawTransaction(txn: RawTransaction): Buffer {
  ...
}

// This will serialize a raw transaction into bytes
function serializeRawTransaction(txn: RawTransaction): Buffer {
  // Generate a hash prefix
  const prefix = hashPrefix();

  // Serialize txn with BCS
  const bcsSerializedTxn = bcsSerializeRawTransaction(txn);

  return Buffer.concat([prefix, bcsSerializedTxn]);
}

const rawTxn = createRawTransaction();
const signature = Nacl.sign(hashRawTransaction(rawTxn), ACCOUNT_PRIVATE_KEY);
```

### Step 3. Creating an Authenticator and a SignedTransaction

```typescript
interface Authenticator {
  public_key: Uint8Array;
  signature: Uint8Array;
}

interface SignedTransaction {
  raw_txn: RawTransaction;
  authenticator: Authenticator;
}

const authenticator = {
  public_key: PUBLIC_KEY,
  signature: signature,
};

const signedTransaction: SignedTransaction = {
  raw_txn: rawTxn,
  authenticator: authenticator,
};
```

### Step 4. Serializing SignedTransaction

:::note
This step is not shown in the flow diagram in the [Overiew](#overview) section.
:::

Serializing SignedTransaction into bytes with BCS.

```typescript
const signedTransactionPayload = bcsSerializeSignedTransaction(signedTransaction);
```

## Submitting the signed transaction

Finally, submitting the transaction with the required header "Content-Type".

To submit a signed transaction in the BCS format, the client must pass in a specific header, as shown in the below example:

```
curl -X POST -H "Content-Type: application/x.aptos.signed_transaction+bcs" --data-binary "@path/to/file_contains_bcs_bytes_of_signed_txn" https://some_domain/transactions
```

[first_transaction]: /tutorials/first-transaction
[account]: /concepts/basics-accounts
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/
[bcs]: https://docs.rs/bcs/latest/bcs/
[sha3]: https://en.wikipedia.org/wiki/SHA-3
[ed25519]: https://ed25519.cr.yp.to/
