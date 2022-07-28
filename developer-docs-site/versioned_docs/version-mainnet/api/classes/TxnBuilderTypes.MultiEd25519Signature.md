---
id: "TxnBuilderTypes.MultiEd25519Signature"
title: "Class: MultiEd25519Signature"
sidebar_label: "MultiEd25519Signature"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).MultiEd25519Signature

## Constructors

### constructor

• **new MultiEd25519Signature**(`signatures`, `bitmap`)

Signature for a K-of-N multisig transaction.

**`see`** [Creating a Signed Transaction](https://aptos.dev/guides/creating-a-signed-transaction#multisignature-transactions)

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `signatures` | [`Seq`](../namespaces/BCS.md#seq)<[`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md)\> | A list of ed25519 signatures |
| `bitmap` | `Uint8Array` | 4 bytes, at most 32 signatures are supported. If Nth bit value is `1`, the Nth signature should be provided in `signatures`. Bits are read from left to right |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:73](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L73)

## Properties

### bitmap

• `Readonly` **bitmap**: `Uint8Array`

___

### signatures

• `Readonly` **signatures**: [`Seq`](../namespaces/BCS.md#seq)<[`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md)\>

___

### BITMAP\_LEN

▪ `Static` **BITMAP\_LEN**: `number` = `4`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:61](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L61)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:139](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L139)

___

### toBytes

▸ **toBytes**(): `Uint8Array`

Converts a MultiEd25519Signature into bytes with `bytes = s1_bytes | ... | sn_bytes | bitmap`

#### Returns

`Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:82](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L82)

___

### createBitmap

▸ `Static` **createBitmap**(`bits`): `Uint8Array`

Helper method to create a bitmap out of the specified bit positions

**`example`**
Here's an example of valid `bits`
```
[0, 2, 31]
```
`[0, 2, 31]` means the 1st, 3rd and 32nd bits should be set in the bitmap.
The result bitmap should be 0b1010000000000000000000000000001

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `bits` | [`Seq`](../namespaces/BCS.md#seq)<`number`\> | The bitmap positions that should be set. A position starts at index 0. Valid position should range between 0 and 31. |

#### Returns

`Uint8Array`

bitmap that is 32bit long

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:107](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L107)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`MultiEd25519Signature`](TxnBuilderTypes.MultiEd25519Signature.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`MultiEd25519Signature`](TxnBuilderTypes.MultiEd25519Signature.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:143](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L143)
