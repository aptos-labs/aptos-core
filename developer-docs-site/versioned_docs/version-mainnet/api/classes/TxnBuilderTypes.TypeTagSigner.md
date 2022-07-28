---
id: "TxnBuilderTypes.TypeTagSigner"
title: "Class: TypeTagSigner"
sidebar_label: "TypeTagSigner"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TypeTagSigner

## Hierarchy

- [`TypeTag`](TxnBuilderTypes.TypeTag.md)

  ↳ **`TypeTagSigner`**

## Constructors

### constructor

• **new TypeTagSigner**()

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:86](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L86)

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

▸ `Static` **load**(`_deserializer`): [`TypeTagSigner`](TxnBuilderTypes.TypeTagSigner.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `_deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TypeTagSigner`](TxnBuilderTypes.TypeTagSigner.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:90](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L90)
