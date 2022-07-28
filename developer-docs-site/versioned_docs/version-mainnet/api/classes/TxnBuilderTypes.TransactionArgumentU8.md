---
id: "TxnBuilderTypes.TransactionArgumentU8"
title: "Class: TransactionArgumentU8"
sidebar_label: "TransactionArgumentU8"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionArgumentU8

## Hierarchy

- [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

  ↳ **`TransactionArgumentU8`**

## Constructors

### constructor

• **new TransactionArgumentU8**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `number` |

#### Overrides

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[constructor](TxnBuilderTypes.TransactionArgument.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:425](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L425)

## Properties

### value

• `Readonly` **value**: `number`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:429](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L429)

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

▸ `Static` **load**(`deserializer`): [`TransactionArgumentU8`](TxnBuilderTypes.TransactionArgumentU8.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgumentU8`](TxnBuilderTypes.TransactionArgumentU8.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:434](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L434)
