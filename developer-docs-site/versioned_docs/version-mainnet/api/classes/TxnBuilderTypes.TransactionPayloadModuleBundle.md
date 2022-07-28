---
id: "TxnBuilderTypes.TransactionPayloadModuleBundle"
title: "Class: TransactionPayloadModuleBundle"
sidebar_label: "TransactionPayloadModuleBundle"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionPayloadModuleBundle

## Hierarchy

- [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

  ↳ **`TransactionPayloadModuleBundle`**

## Constructors

### constructor

• **new TransactionPayloadModuleBundle**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`ModuleBundle`](TxnBuilderTypes.ModuleBundle.md) |

#### Overrides

[TransactionPayload](TxnBuilderTypes.TransactionPayload.md).[constructor](TxnBuilderTypes.TransactionPayload.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:356](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L356)

## Properties

### value

• `Readonly` **value**: [`ModuleBundle`](TxnBuilderTypes.ModuleBundle.md)

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

[TransactionPayload](TxnBuilderTypes.TransactionPayload.md).[serialize](TxnBuilderTypes.TransactionPayload.md#serialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:360](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L360)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

#### Inherited from

[TransactionPayload](TxnBuilderTypes.TransactionPayload.md).[deserialize](TxnBuilderTypes.TransactionPayload.md#deserialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:312](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L312)

___

### load

▸ `Static` **load**(`deserializer`): [`TransactionPayloadModuleBundle`](TxnBuilderTypes.TransactionPayloadModuleBundle.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionPayloadModuleBundle`](TxnBuilderTypes.TransactionPayloadModuleBundle.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:365](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L365)
