---
id: "TxnBuilderTypes.TransactionArgumentU8Vector"
title: "Class: TransactionArgumentU8Vector"
sidebar_label: "TransactionArgumentU8Vector"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionArgumentU8Vector

## Hierarchy

- [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

  ↳ **`TransactionArgumentU8Vector`**

## Constructors

### constructor

• **new TransactionArgumentU8Vector**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `Uint8Array` |

#### Overrides

[TransactionArgument](TxnBuilderTypes.TransactionArgument.md).[constructor](TxnBuilderTypes.TransactionArgument.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:489](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L489)

## Properties

### value

• `Readonly` **value**: `Uint8Array`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:493](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L493)

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

▸ `Static` **load**(`deserializer`): [`TransactionArgumentU8Vector`](TxnBuilderTypes.TransactionArgumentU8Vector.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgumentU8Vector`](TxnBuilderTypes.TransactionArgumentU8Vector.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:498](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L498)
