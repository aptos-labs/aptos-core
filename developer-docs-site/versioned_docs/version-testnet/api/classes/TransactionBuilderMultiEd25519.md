---
id: "TransactionBuilderMultiEd25519"
title: "Class: TransactionBuilderMultiEd25519"
sidebar_label: "TransactionBuilderMultiEd25519"
sidebar_position: 0
custom_edit_url: null
---

Provides signing method for signing a raw transaction with multisig public key.

## Hierarchy

- `TransactionBuilder`<[`SigningFn`](../modules.md#signingfn)\>

  ↳ **`TransactionBuilderMultiEd25519`**

## Constructors

### constructor

• **new TransactionBuilderMultiEd25519**(`signingFunction`, `publicKey`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `signingFunction` | [`SigningFn`](../modules.md#signingfn) |
| `publicKey` | [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md) |

#### Overrides

TransactionBuilder&lt;SigningFn\&gt;.constructor

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:77](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L77)

## Properties

### publicKey

• `Private` `Readonly` **publicKey**: [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:75](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L75)

___

### signingFunction

• `Protected` `Readonly` **signingFunction**: [`SigningFn`](../modules.md#signingfn)

#### Inherited from

TransactionBuilder.signingFunction

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:25](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L25)

## Methods

### sign

▸ **sign**(`rawTxn`): `Uint8Array`

Signs a raw transaction and returns a bcs serialized transaction.

#### Parameters

| Name | Type |
| :------ | :------ |
| `rawTxn` | [`RawTransaction`](TxnBuilderTypes.RawTransaction.md) |

#### Returns

`Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:92](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L92)

___

### signInternal

▸ `Private` **signInternal**(`rawTxn`): [`SignedTransaction`](TxnBuilderTypes.SignedTransaction.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `rawTxn` | [`RawTransaction`](TxnBuilderTypes.RawTransaction.md) |

#### Returns

[`SignedTransaction`](TxnBuilderTypes.SignedTransaction.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:82](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L82)

___

### getSigningMessage

▸ `Static` **getSigningMessage**(`rawTxn`): `Buffer`

Generates a Signing Message out of a raw transaction.

#### Parameters

| Name | Type |
| :------ | :------ |
| `rawTxn` | [`RawTransaction`](TxnBuilderTypes.RawTransaction.md) |

#### Returns

`Buffer`

#### Inherited from

TransactionBuilder.getSigningMessage

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:32](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L32)
