---
id: "TxnBuilderTypes.TransactionArgument"
title: "Class: TransactionArgument"
sidebar_label: "TransactionArgument"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionArgument

## Hierarchy

- **`TransactionArgument`**

  ↳ [`TransactionArgumentU8`](TxnBuilderTypes.TransactionArgumentU8.md)

  ↳ [`TransactionArgumentU64`](TxnBuilderTypes.TransactionArgumentU64.md)

  ↳ [`TransactionArgumentU128`](TxnBuilderTypes.TransactionArgumentU128.md)

  ↳ [`TransactionArgumentAddress`](TxnBuilderTypes.TransactionArgumentAddress.md)

  ↳ [`TransactionArgumentU8Vector`](TxnBuilderTypes.TransactionArgumentU8Vector.md)

  ↳ [`TransactionArgumentBool`](TxnBuilderTypes.TransactionArgumentBool.md)

## Constructors

### constructor

• **new TransactionArgument**()

## Methods

### serialize

▸ `Abstract` **serialize**(`serializer`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `serializer` | [`Serializer`](BCS.Serializer.md) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:401](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L401)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:403](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L403)
