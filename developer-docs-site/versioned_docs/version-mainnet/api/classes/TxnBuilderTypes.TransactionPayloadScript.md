---
id: "TxnBuilderTypes.TransactionPayloadScript"
title: "Class: TransactionPayloadScript"
sidebar_label: "TransactionPayloadScript"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionPayloadScript

## Hierarchy

- [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

  ↳ **`TransactionPayloadScript`**

## Constructors

### constructor

• **new TransactionPayloadScript**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`Script`](TxnBuilderTypes.Script.md) |

#### Overrides

[TransactionPayload](TxnBuilderTypes.TransactionPayload.md).[constructor](TxnBuilderTypes.TransactionPayload.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:340](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L340)

## Properties

### value

• `Readonly` **value**: [`Script`](TxnBuilderTypes.Script.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:344](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L344)

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

▸ `Static` **load**(`deserializer`): [`TransactionPayloadScript`](TxnBuilderTypes.TransactionPayloadScript.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionPayloadScript`](TxnBuilderTypes.TransactionPayloadScript.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:349](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L349)
