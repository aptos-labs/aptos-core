---
id: "TxnBuilderTypes.TransactionArgumentU128"
title: "Class: TransactionArgumentU128"
sidebar_label: "TransactionArgumentU128"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionArgumentU128

## Hierarchy

- [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

  ↳ **`TransactionArgumentU128`**

## Constructors

### constructor

• **new TransactionArgumentU128**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `bigint` |

#### Overrides

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[constructor](TxnBuilderTypes.TransactionArgument.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:457](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L457)

## Properties

### value

• `Readonly` **value**: `bigint`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:461](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L461)

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

▸ `Static` **load**(`deserializer`): [`TransactionArgumentU128`](TxnBuilderTypes.TransactionArgumentU128.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgumentU128`](TxnBuilderTypes.TransactionArgumentU128.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:466](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L466)
