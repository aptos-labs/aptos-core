---
id: "TxnBuilderTypes.Identifier"
title: "Class: Identifier"
sidebar_label: "Identifier"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).Identifier

## Constructors

### constructor

• **new Identifier**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `string` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/identifier.ts:4](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/identifier.ts#L4)

## Properties

### value

• **value**: `string`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/identifier.ts:6](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/identifier.ts#L6)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`Identifier`](TxnBuilderTypes.Identifier.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`Identifier`](TxnBuilderTypes.Identifier.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/identifier.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/identifier.ts#L10)
