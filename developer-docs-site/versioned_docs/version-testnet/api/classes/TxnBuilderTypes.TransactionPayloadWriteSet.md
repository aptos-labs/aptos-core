---
id: "TxnBuilderTypes.TransactionPayloadWriteSet"
title: "Class: TransactionPayloadWriteSet"
sidebar_label: "TransactionPayloadWriteSet"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionPayloadWriteSet

## Hierarchy

- [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

  ↳ **`TransactionPayloadWriteSet`**

## Constructors

### constructor

• **new TransactionPayloadWriteSet**()

#### Inherited from

[TransactionPayload](TxnBuilderTypes.TransactionPayload.md).[constructor](TxnBuilderTypes.TransactionPayload.md#constructor)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:330](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L330)

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

▸ `Static` **load**(`deserializer`): [`TransactionPayloadWriteSet`](TxnBuilderTypes.TransactionPayloadWriteSet.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionPayloadWriteSet`](TxnBuilderTypes.TransactionPayloadWriteSet.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:334](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L334)
