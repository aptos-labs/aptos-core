---
id: "TxnBuilderTypes.Script"
title: "Class: Script"
sidebar_label: "Script"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).Script

## Constructors

### constructor

• **new Script**(`code`, `ty_args`, `args`)

Scripts contain the Move bytecodes payload that can be submitted to Aptos chain for execution.

**`example`**
A coin transfer function has one type argument "CoinType".
```
public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
```

**`example`**
A coin transfer function has three arugments "from", "to" and "amount".
```
public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
```

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `code` | `Uint8Array` | Move bytecode |
| `ty_args` | [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\> | Type arguments that bytecode requires. |
| `args` | [`Seq`](../namespaces/BCS.md#seq)<[`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)\> | Arugments to bytecode function. |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:97](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L97)

## Properties

### args

• `Readonly` **args**: [`Seq`](../namespaces/BCS.md#seq)<[`TransactionArgument`](TxnBuilderTypes.TransactionArgument.md)\>

___

### code

• `Readonly` **code**: `Uint8Array`

___

### ty\_args

• `Readonly` **ty\_args**: [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\>

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:103](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L103)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`Script`](TxnBuilderTypes.Script.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`Script`](TxnBuilderTypes.Script.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:109](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L109)
