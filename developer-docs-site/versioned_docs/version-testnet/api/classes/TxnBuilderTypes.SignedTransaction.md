---
id: "TxnBuilderTypes.SignedTransaction"
title: "Class: SignedTransaction"
sidebar_label: "SignedTransaction"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).SignedTransaction

## Constructors

### constructor

• **new SignedTransaction**(`raw_txn`, `authenticator`)

A SignedTransaction consists of a raw transaction and an authenticator. The authenticator
contains a client's public key and the signature of the raw transaction.

**`see`** [Creating a Signed Transaction](https://aptos.dev/guides/creating-a-signed-transaction/)

**`see`** authenticator.ts for details.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `raw_txn` | [`RawTransaction`](TxnBuilderTypes.RawTransaction.md) |  |
| `authenticator` | [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md) | Contains a client's public key and the signature of the raw transaction.   Authenticator has 3 flavors: single signature, multi-signature and multi-agent. |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:295](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L295)

## Properties

### authenticator

• `Readonly` **authenticator**: [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

___

### raw\_txn

• `Readonly` **raw\_txn**: [`RawTransaction`](TxnBuilderTypes.RawTransaction.md)

## Methods

### serialize

▸ **serialize**(`serializer`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `serializer` | [`Serializer`](BCS.Serializer.md) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:297](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L297)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`SignedTransaction`](TxnBuilderTypes.SignedTransaction.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`SignedTransaction`](TxnBuilderTypes.SignedTransaction.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:302](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L302)
