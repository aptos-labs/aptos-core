---
id: "TransactionBuilderEd25519"
title: "Class: TransactionBuilderEd25519"
sidebar_label: "TransactionBuilderEd25519"
sidebar_position: 0
custom_edit_url: null
---

Provides signing method for signing a raw transaction with single public key.

## Hierarchy

- `TransactionBuilder`<[`SigningFn`](../modules.md#signingfn)\>

  ↳ **`TransactionBuilderEd25519`**

## Constructors

### constructor

• **new TransactionBuilderEd25519**(`signingFunction`, `publicKey`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `signingFunction` | [`SigningFn`](../modules.md#signingfn) |
| `publicKey` | `Uint8Array` |

#### Overrides

TransactionBuilder&lt;SigningFn\&gt;.constructor

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:48](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L48)

## Properties

### publicKey

• `Private` `Readonly` **publicKey**: `Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:46](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L46)

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

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:66](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L66)

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

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:53](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L53)

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
