---
id: "TxnBuilderTypes.TransactionPayload"
title: "Class: TransactionPayload"
sidebar_label: "TransactionPayload"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionPayload

## Hierarchy

- **`TransactionPayload`**

  ↳ [`TransactionPayloadWriteSet`](TxnBuilderTypes.TransactionPayloadWriteSet.md)

  ↳ [`TransactionPayloadScript`](TxnBuilderTypes.TransactionPayloadScript.md)

  ↳ [`TransactionPayloadModuleBundle`](TxnBuilderTypes.TransactionPayloadModuleBundle.md)

  ↳ [`TransactionPayloadScriptFunction`](TxnBuilderTypes.TransactionPayloadScriptFunction.md)

## Constructors

### constructor

• **new TransactionPayload**()

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:310](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L310)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:312](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L312)
