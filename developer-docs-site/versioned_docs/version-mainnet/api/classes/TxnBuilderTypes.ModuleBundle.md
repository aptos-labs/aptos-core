---
id: "TxnBuilderTypes.ModuleBundle"
title: "Class: ModuleBundle"
sidebar_label: "ModuleBundle"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).ModuleBundle

## Constructors

### constructor

• **new ModuleBundle**(`codes`)

Contains a list of Modules that can be published together.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `codes` | [`Seq`](../namespaces/BCS.md#seq)<[`Module`](TxnBuilderTypes.Module.md)\> | List of modules. |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:217](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L217)

## Properties

### codes

• `Readonly` **codes**: [`Seq`](../namespaces/BCS.md#seq)<[`Module`](TxnBuilderTypes.Module.md)\>

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:219](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L219)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`ModuleBundle`](TxnBuilderTypes.ModuleBundle.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`ModuleBundle`](TxnBuilderTypes.ModuleBundle.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:223](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L223)
