---
id: "TxnBuilderTypes.TransactionArgumentBool"
title: "Class: TransactionArgumentBool"
sidebar_label: "TransactionArgumentBool"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionArgumentBool

## Hierarchy

- [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

  ↳ **`TransactionArgumentBool`**

## Constructors

### constructor

• **new TransactionArgumentBool**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `boolean` |

#### Overrides

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[constructor](TxnBuilderTypes.TransactionArgument.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:505](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L505)

## Properties

### value

• `Readonly` **value**: `boolean`

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

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[serialize](TxnBuilderTypes.TransactionArgument.md#serialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:509](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L509)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

#### Inherited from

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[deserialize](TxnBuilderTypes.TransactionArgument.md#deserialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:403](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L403)

___

### load

▸ `Static` **load**(`deserializer`): [`TransactionArgumentBool`](TxnBuilderTypes.TransactionArgumentBool.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgumentBool`](TxnBuilderTypes.TransactionArgumentBool.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:514](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L514)
