---
id: "TxnBuilderTypes.TypeTag"
title: "Class: TypeTag"
sidebar_label: "TypeTag"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TypeTag

## Hierarchy

- **`TypeTag`**

  ↳ [`TypeTagBool`](TxnBuilderTypes.TypeTagBool.md)

  ↳ [`TypeTagU8`](TxnBuilderTypes.TypeTagU8.md)

  ↳ [`TypeTagU64`](TxnBuilderTypes.TypeTagU64.md)

  ↳ [`TypeTagU128`](TxnBuilderTypes.TypeTagU128.md)

  ↳ [`TypeTagAddress`](TxnBuilderTypes.TypeTagAddress.md)

  ↳ [`TypeTagSigner`](TxnBuilderTypes.TypeTagSigner.md)

  ↳ [`TypeTagVector`](TxnBuilderTypes.TypeTagVector.md)

  ↳ [`TypeTagStruct`](TxnBuilderTypes.TypeTagStruct.md)

## Constructors

### constructor

• **new TypeTag**()

## Methods

### serialize

▸ `Abstract` **serialize**(`serializer`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `serializer` | [`Serializer`](BCS.Serializer.md) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:8](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L8)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TypeTag`](TxnBuilderTypes.TypeTag.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TypeTag`](TxnBuilderTypes.TypeTag.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L10)
