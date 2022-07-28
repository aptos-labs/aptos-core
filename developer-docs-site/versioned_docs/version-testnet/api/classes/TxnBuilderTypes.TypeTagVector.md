---
id: "TxnBuilderTypes.TypeTagVector"
title: "Class: TypeTagVector"
sidebar_label: "TypeTagVector"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TypeTagVector

## Hierarchy

- [`TypeTag`](TxnBuilderTypes.TypeTag.md)

  ↳ **`TypeTagVector`**

## Constructors

### constructor

• **new TypeTagVector**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`TypeTag`](TxnBuilderTypes.TypeTag.md) |

#### Overrides

[TypeTag](TxnBuilderTypes.TypeTag.md).[constructor](TxnBuilderTypes.TypeTag.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:96](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L96)

## Properties

### value

• `Readonly` **value**: [`TypeTag`](TxnBuilderTypes.TypeTag.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:100](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L100)

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

▸ `Static` **load**(`deserializer`): [`TypeTagVector`](TxnBuilderTypes.TypeTagVector.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TypeTagVector`](TxnBuilderTypes.TypeTagVector.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:105](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L105)
