---
id: "TxnBuilderTypes.Module"
title: "Class: Module"
sidebar_label: "Module"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).Module

## Constructors

### constructor

• **new Module**(`code`)

Contains the bytecode of a Move module that can be published to the Aptos chain.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `code` | `Uint8Array` | Move bytecode of a module. |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:200](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L200)

## Properties

### code

• `Readonly` **code**: `Uint8Array`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:202](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L202)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`Module`](TxnBuilderTypes.Module.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`Module`](TxnBuilderTypes.Module.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:206](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L206)
