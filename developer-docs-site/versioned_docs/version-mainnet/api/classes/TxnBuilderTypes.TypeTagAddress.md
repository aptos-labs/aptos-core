---
id: "TxnBuilderTypes.TypeTagAddress"
title: "Class: TypeTagAddress"
sidebar_label: "TypeTagAddress"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TypeTagAddress

## Hierarchy

- [`TypeTag`](TxnBuilderTypes.TypeTag.md)

  ↳ **`TypeTagAddress`**

## Constructors

### constructor

• **new TypeTagAddress**()

#### Inherited from

[TypeTag](TxnBuilderTypes.TypeTag.md).[constructor](TxnBuilderTypes.TypeTag.md#constructor)

## Methods

### serialize

▸ **serialize**(`serializer`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `serializer` | [`Serializer`](BCS.Serializer.md) |

#### Returns

`void`

#### Overrides

[TypeTag](TxnBuilderTypes.TypeTag.md).[serialize](TxnBuilderTypes.TypeTag.md#serialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:76](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L76)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TypeTag`](TxnBuilderTypes.TypeTag.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TypeTag`](TxnBuilderTypes.TypeTag.md)

#### Inherited from

[TypeTag](TxnBuilderTypes.TypeTag.md).[deserialize](TxnBuilderTypes.TypeTag.md#deserialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L10)

___

### load

▸ `Static` **load**(`_deserializer`): [`TypeTagAddress`](TxnBuilderTypes.TypeTagAddress.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `_deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TypeTagAddress`](TxnBuilderTypes.TypeTagAddress.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:80](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L80)
