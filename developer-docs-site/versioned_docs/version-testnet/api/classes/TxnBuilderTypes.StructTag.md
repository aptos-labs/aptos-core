---
id: "TxnBuilderTypes.StructTag"
title: "Class: StructTag"
sidebar_label: "StructTag"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).StructTag

## Constructors

### constructor

• **new StructTag**(`address`, `module_name`, `name`, `type_args`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `address` | [`AccountAddress`](TxnBuilderTypes.AccountAddress.md) |
| `module_name` | [`Identifier`](TxnBuilderTypes.Identifier.md) |
| `name` | [`Identifier`](TxnBuilderTypes.Identifier.md) |
| `type_args` | [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\> |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:128](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L128)

## Properties

### address

• `Readonly` **address**: [`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

___

### module\_name

• `Readonly` **module\_name**: [`Identifier`](TxnBuilderTypes.Identifier.md)

___

### name

• `Readonly` **name**: [`Identifier`](TxnBuilderTypes.Identifier.md)

___

### type\_args

• `Readonly` **type\_args**: [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\>

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:155](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L155)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`StructTag`](TxnBuilderTypes.StructTag.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`StructTag`](TxnBuilderTypes.StructTag.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:162](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L162)

___

### fromString

▸ `Static` **fromString**(`structTag`): [`StructTag`](TxnBuilderTypes.StructTag.md)

Converts a string literal to a StructTag

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `structTag` | `string` | String literal in format "AcountAddress::ModuleName::ResourceName",   e.g. "0x01::TestCoin::TestCoin" |

#### Returns

[`StructTag`](TxnBuilderTypes.StructTag.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts:141](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/type_tag.ts#L141)
