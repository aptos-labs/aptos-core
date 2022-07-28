---
id: "TxnBuilderTypes.Ed25519PublicKey"
title: "Class: Ed25519PublicKey"
sidebar_label: "Ed25519PublicKey"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).Ed25519PublicKey

## Constructors

### constructor

• **new Ed25519PublicKey**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `Uint8Array` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:8](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L8)

## Properties

### value

• `Readonly` **value**: `Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:6](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L6)

___

### LENGTH

▪ `Static` `Readonly` **LENGTH**: `number` = `32`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:4](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L4)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:15](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L15)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts:19](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/ed25519.ts#L19)
