---
title: "Creating a Multisig and Multiagent Transaction"
slug: "multisig-multiagent-transaction"
---

The Aptos blockchain supports three types of transactions: 

1. Single signature transaction.
2. Multi signature transaction (multisig), and 
3. Multiagent transaction. 

:::tip 📌 
This document describes the multisig and multiagent transactions. For single signature transaction, see the document [Creating a Signed Transaction](/guides/sign-a-transaction.md).
:::

## Multisig transaction

A multisig transaction is used to send transactions involving a single K-of-N multisig account address. A K-of-N multisig account has a single account address that is derived from a multisig authentication key. See [Multisigner authentication](/concepts/basics-accounts.md#multisigner-authentication). 

For a transaction to be executed with a K-of-N multisig account, at least K out of the N authorized signers must have signed the transaction. For example, Alice, Bob, and Charlie created a multisig account on the Aptos blockchain with a threshold of 2, i.e., setting the value of K to 2. This joint account is now a 2-of-3 multisig account. If Alice wants to move coins out of this joint account, she needs, in addition to her own signature, at least one more signature, of Bob or Charlie’s, to reach the threshold of 2 for her transaction to be executed on the Aptos blockchain. 

## Multiagent transaction

A multiagent transaction requires multiple signers. A multiagent transaction must include the sender’s signature, along with a list of secondary signers’ signatures. For example, Alice minted some NFT tokens and she would like to transfer one of the NFTs to Bob. Token transfers require both the sender and receiver to sign the transaction. In this case, Alice prepares a multiagent transaction with her own signature as the sender signature, and she also needs Bob’s signature as the secondary signature for her transaction to be executed.

The rest of this document describes the detailed steps of creating a multisig and a multiagent transaction.

## Creating a multisig transaction

1. Create a K-of-N multisig account.
2. Create a raw transaction.
3. Create a signing message.
4. Create a multisig `Authenticator` and a `SignedTransaction`.
5. Finally, serialize and submit the `SignedTransaction`.

:::tip 

The example code in this document is pseudo-code only, and is written in Typescript. The data types used are defined in [Creating a Signed Transaction](/guides/sign-a-transaction.md). 

:::

### Creating a K-of-N multisig account

1. Generate a K-of-N multisig authentication key. Use `0x01` as the 1-byte multisig scheme identifier and your preferred threshold value for `K`. See [Signature scheme identifiers](/concepts/basics-accounts.md#signature-scheme-identifiers).
2. Derive the account address from the authentication key.

In the below example code, the `new Uint8Array([2, 1])` contains the threshold K, set to 2 and the 1-byte multisig scheme identifier, set to `1` .

```typescript
import * as SHA3 from 'js-sha3';
const alicePubKey: Uint8Array = <KEY_BYTES>;
const bobPubKey: Uint8Array = <KEY_BYTES>;
const charliePubKey: Uint8Array = <KEY_BYTES>;
// Concatenates the public keys, sets the threshold value K to 2 and the signature scheme identifier to 1
const bytes = [...alicePubKey, ...bobPubKey, ...charliePubKey, ...new Uint8Array([2, 1])];
const hash = SHA3.sha3_256.create();
hash.update(Buffer.from(bytes));
const authKey = new Uint8Array(hash.arrayBuffer());
const multisigAccount = new AccountAddress(authKey);
```

### Creating a raw transaction

```typescript
function createRawTransaction(): RawTransaction {
  const payload: ScriptFunction = {
    module: {
      address: hexToAccountAddress("0x01"),
      name: "TestCoin"
    },
    function: "transfer",
    ty_args: [],
    args: [
      BCS.serialize(hexToAccountAddress("0x02")), // recipient of the transfer
      BCS.serialize_uint64(2), // amount to transfer
    ]
  }

  // Assume the multisigAccount has test coins
  return {
    "sender": multisigAccount,
    "sequence_number": 1n,
    "max_gas_amount": 2000n,
    "gas_unit_price": 1n,
    "expiration_timestamp_secs": Math.floor(Date.now() / 1000) + 600,
    "payload": payload,
    "chain_id": 3
  };
}
```

### Creating a signing message

The below example code creates the signing message, following the same steps described in [Creating a Signed Transaction](guides/sign-a-transaction.md#step-2-creating-the-signing-message-and-signing-it).

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
const signingMessage = serializeRawTransaction(rawTxn);
```

### Creating a multisig Authenticator and a SignedTransaction

A multisig transaction requires at least K (threshold) signers to sign the same signing message. See the below code example. 

```typescript
interface  MultiEd25519PublicKey {
  // A list of public keys
  public_keys: Uint8Array[],
  // At least `threshold` signatures must be valid
  threshold: Uint8,
}

interface MultiEd25519Signature {
    // A list of signatures
    signatures: Uint8Array[],
    // 4 bytes, at most 32 signatures are supported.
    // If Nth bit value is `1`, the Nth signature should be provided in `signatures`. Bits are read from left to right.
    bitmap: Uint8Array,
}

interface MultisigAuthenticator {
  public_key: MultiEd25519PublicKey,
  signature: MultiEd25519Signature
}

const sig0 = aliceAccount.signBuffer(signingMessage);
const sig2 = charlieAccount.signBuffer(signingMessage);
// Marks the 1st and 3rd signer has signed. Left-most bit in first element is sig0 in this example.
const bitmap = new Uint8Array([0b10100000, 0b00000000, 0b00000000, 0b00000000]);

const muliEd25519Sig: MultiEd25519Signature = {
  signatures: [
    sig0.toUint8Array(),
    sig2.toUint8Array(),
  ],
  bitmap
};

const multisigAuthenticator: MultisigAuthenticator = {
  public_key: {
    public_keys: [alicePubKey, bobPubKey, charliePubKey],
    threshold: 2
  },
  signature: muliEd25519Sig
}

const signedTransaction: SignedTransaction = {
  raw_txn: rawTxn,
  authenticator: multisigAuthenticator
};
```

### Serializing and submitting the SignedTransaction

The `SignedTransaction` is serialized and submitted with the same method described in [Creating a Signed Transaction](/guides/sign-a-transaction.md#step-4-serializing-signedtransaction). 

## Creating a multiagent transaction

A multiagent transaction requires multiple signers. The signer can be either a multisig signer or a single-sig signer.

### Creating a raw transaction

Multiagent transaction signers do not sign a `RawTransaction` directly as they do in the single signature or multisig transactions. Instead, they sign a `RawTransactionWithData`. The `RawTransactionWithData` contains a `RawTransaction` plus a list of secondary signers’ account addresses. See the below code example.

```typescript
// Below transaction transfers 1 token from account 0x01 to account 0x02. 
// Both accounts need to sign the transaction.
function createRawTransaction(): RawTransaction {
  const payload: ScriptFunction = {
    module: {
      address: hexToAccountAddress("0x01"),
      name: "Token"
    },
    function: "direct_transfer_script",
    ty_args: [],
    args: [
      BCS.serialize(hexToAccountAddress("0x02")), // receipient of the transfer
      BCS.bcsSerializeStr("Fancy NFT collection"), // NFT collection
      BCS.bcsSerializeStr("Fancy Token"), // token name
      BCS.bcsSerializeUint64(1), // amount
    ]
  }

  // Assume the multisigAccount has test coins
  return {
    "sender": hexToAccountAddress("0x01"),
    "sequence_number": 1n,
    "max_gas_amount": 2000n,
    "gas_unit_price": 1n,
    "expiration_timestamp_secs": Math.floor(Date.now() / 1000) + 600,
    "payload": payload,
    "chain_id": 3
  };
}

interface RawTransactionWithData {
	raw_txn: RawTransaction,
	secondary_signer_addresses: AccountAddress[]
}

const rawTxn = createRawTransaction();

const txnWithData: RawTransactionWithData = {
  raw_txn: rawTxn,
  secondary_signer_addresses: [hexToAccountAddress("0x02")]
}
```

### Creating a signing message

The signing message is created by following the same steps in [Creating a Signed Transaction](/guides/sign-a-transaction.md#step-2-creating-the-signing-message-and-signing-it).  However, instead of `APTOS::RawTransaction`, the prefix `APTOS::RawTransactionWithData` is used. Also, the payload for creating the signing message is `RawTransactionWithData` instead of `RawTransaction`. See the below code example.

```typescript
import * as Nacl from "tweetnacl";

function hashPrefix(): Buffer {
  let hash = SHA3.sha3_256.create();
  // Not APTOS::RawTransaction
  hash.update(`APTOS::RawTransactionWithData`);
  return Buffer.from(hash.arrayBuffer());
}

function bcsSerializeRawTransactionWithData(txnWithData: RawTransactionWithData): Buffer {
  ...
}

// This will serialize a raw transaction into bytes
function serializeRawTransactionWithData(txnWithData: RawTransactionWithData): Buffer {
  // Generate a hash prefix
  const prefix = hashPrefix();

  // Serialize txn with BCS
  const bcsSerializedTxn = bcsSerializeRawTransactionWithData(txnWithData);

  return Buffer.concat([prefix, bcsSerializedTxn]);
}

const signingMessage = serializeRawTransactionWithData(txnWithData);
```

### Creating a multiagent Authenticator and a SignedTransaction

```typescript
import * as Nacl from "tweetnacl";

type AuthenticatorType = Authenticator | MultisigAuthenticator;
interface MultiAgentAuthenticator {
	sender: AuthenticatorType,
  secondary_signer_addresses: AccountAddress[],
	secondary_signers: AuthenticatorType[]
}

const multiagentAuthenticator: MultiAgentAuthenticator {
  sender: {
    public_key: ACCOUNT_01_PUBLIC_KEY,
    signature: Nacl.sign(signingMessage, ACCOUNT_01_PRIVATE_KEY)
  },
  secondary_signer_addresses: [hexToAccountAddress("0x02")],
  secondary_signers: [
    {
      public_key: ACCOUNT_02_PUBLIC_KEY,
      signature: Nacl.sign(signingMessage, ACCOUNT_02_PRIVATE_KEY)
    }
  ]
}

const signedTransaction: SignedTransaction = {
  raw_txn: rawTxn,
  authenticator: multiagentAuthenticator
};
```

`SignedTransaction` can be serialized and submitted with the same method described in [Creating a Signed Transaction](/guides/sign-a-transaction.md#step-4-serializing-signedtransaction).