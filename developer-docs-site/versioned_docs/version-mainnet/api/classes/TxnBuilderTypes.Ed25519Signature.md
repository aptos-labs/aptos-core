---
id: "TxnBuilderTypes.Ed25519Signature"
title: "Class: Ed25519Signature"
sidebar_label: "Ed25519Signature"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).Ed25519Signature

## Constructors

### constructor

• **new Ed25519Signature**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `Uint8Array` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:28](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L28)

## Properties

### value

• `Readonly` **value**: `Uint8Array`

___

### LENGTH

▪ `Static` `Readonly` **LENGTH**: ``64``

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:26](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L26)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:34](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L34)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:38](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L38)
