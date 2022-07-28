---
id: "TxnBuilderTypes.TypeTagStruct"
title: "Class: TypeTagStruct"
sidebar_label: "TypeTagStruct"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TypeTagStruct

## Hierarchy

- [`TypeTag`](TxnBuilderTypes.TypeTag.md)

  ↳ **`TypeTagStruct`**

## Constructors

### constructor

• **new TypeTagStruct**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`StructTag`](TxnBuilderTypes.StructTag.md) |

#### Overrides

[TypeTag](TxnBuilderTypes.TypeTag.md).[constructor](TxnBuilderTypes.TypeTag.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:112](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L112)

## Properties

### value

• `Readonly` **value**: [`StructTag`](TxnBuilderTypes.StructTag.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:116](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L116)

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

▸ `Static` **load**(`deserializer`): [`TypeTagStruct`](TxnBuilderTypes.TypeTagStruct.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TypeTagStruct`](TxnBuilderTypes.TypeTagStruct.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:121](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L121)
