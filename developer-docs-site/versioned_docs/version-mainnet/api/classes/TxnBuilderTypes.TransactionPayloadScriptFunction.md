---
id: "TxnBuilderTypes.TransactionPayloadScriptFunction"
title: "Class: TransactionPayloadScriptFunction"
sidebar_label: "TransactionPayloadScriptFunction"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionPayloadScriptFunction

## Hierarchy

- [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

  ↳ **`TransactionPayloadScriptFunction`**

## Constructors

### constructor

• **new TransactionPayloadScriptFunction**(`value`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`ScriptFunction`](TxnBuilderTypes.ScriptFunction.md) |

#### Overrides

[TransactionPayload](TxnBuilderTypes.TransactionPayload.md).[constructor](TxnBuilderTypes.TransactionPayload.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:372](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L372)

## Properties

### value

• `Readonly` **value**: [`ScriptFunction`](TxnBuilderTypes.ScriptFunction.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:376](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L376)

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

▸ `Static` **load**(`deserializer`): [`TransactionPayloadScriptFunction`](TxnBuilderTypes.TransactionPayloadScriptFunction.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionPayloadScriptFunction`](TxnBuilderTypes.TransactionPayloadScriptFunction.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:381](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L381)
