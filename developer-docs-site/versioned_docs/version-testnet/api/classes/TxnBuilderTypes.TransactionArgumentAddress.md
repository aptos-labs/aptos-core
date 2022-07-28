---
id: "TxnBuilderTypes.TransactionArgumentAddress"
title: "Class: TransactionArgumentAddress"
sidebar_label: "TransactionArgumentAddress"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionArgumentAddress

## Hierarchy

- [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

  ↳ **`TransactionArgumentAddress`**

## Constructors

### constructor

• **new TransactionArgumentAddress**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`AccountAddress`](TxnBuilderTypes.AccountAddress.md) |

#### Overrides

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[constructor](TxnBuilderTypes.TransactionArgument.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:473](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L473)

## Properties

### value

• `Readonly` **value**: [`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:477](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L477)

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

▸ `Static` **load**(`deserializer`): [`TransactionArgumentAddress`](TxnBuilderTypes.TransactionArgumentAddress.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgumentAddress`](TxnBuilderTypes.TransactionArgumentAddress.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:482](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L482)
