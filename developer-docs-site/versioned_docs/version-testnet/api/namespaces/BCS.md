---
id: "BCS"
title: "Namespace: BCS"
sidebar_label: "BCS"
sidebar_position: 0
custom_edit_url: null
---

## Classes

- [Deserializer](../classes/BCS.Deserializer.md)
- [Serializer](../classes/BCS.Serializer.md)

## Type Aliases

### AnyNumber

Ƭ **AnyNumber**: `bigint` \| `number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:8](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L8)

___

### Bytes

Ƭ **Bytes**: `Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:9](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L9)

___

### Seq

Ƭ **Seq**<`T`\>: `T`[]

#### Type parameters

| Name |
| :------ |
| `T` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:1](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L1)

___

### Uint128

Ƭ **Uint128**: `bigint`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:7](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L7)

___

### Uint16

Ƭ **Uint16**: `number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:4](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L4)

___

### Uint32

Ƭ **Uint32**: `number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:5](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L5)

___

### Uint64

Ƭ **Uint64**: `bigint`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:6](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L6)

___

### Uint8

Ƭ **Uint8**: `number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts:3](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/types.ts#L3)

## Functions

### bcsSerializeUint64

▸ **bcsSerializeUint64**(`value`): [`Bytes`](BCS.md#bytes)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`AnyNumber`](BCS.md#anynumber) |

#### Returns

[`Bytes`](BCS.md#bytes)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts:37](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts#L37)

___

### bcsToBytes

▸ **bcsToBytes**<`T`\>(`value`): [`Bytes`](BCS.md#bytes)

#### Type parameters

| Name | Type |
| :------ | :------ |
| `T` | extends `Serializable` |

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `T` |

#### Returns

[`Bytes`](BCS.md#bytes)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts:31](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts#L31)

___

### deserializeVector

▸ **deserializeVector**(`deserializer`, `cls`): `any`[]

Deserializes a vector of values.

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](../classes/BCS.Deserializer.md) |
| `cls` | `any` |

#### Returns

`any`[]

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts:22](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts#L22)

___

### serializeVector

▸ **serializeVector**<`T`\>(`value`, `serializer`): `void`

Serializes a vector values that are "Serializable".

#### Type parameters

| Name | Type |
| :------ | :------ |
| `T` | extends `Serializable` |

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`Seq`](BCS.md#seq)<`T`\> |
| `serializer` | [`Serializer`](../classes/BCS.Serializer.md) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts:12](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/helper.ts#L12)
