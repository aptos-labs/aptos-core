---
id: "TxnBuilderTypes.RawTransaction"
title: "Class: RawTransaction"
sidebar_label: "RawTransaction"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).RawTransaction

## Constructors

### constructor

• **new RawTransaction**(`sender`, `sequence_number`, `payload`, `max_gas_amount`, `gas_unit_price`, `expiration_timestamp_secs`, `chain_id`)

RawTransactions contain the metadata and payloads that can be submitted to Aptos chain for execution.
RawTransactions must be signed before Aptos chain can execute them.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `sender` | [`AccountAddress`](TxnBuilderTypes.AccountAddress.md) | Account address of the sender. |
| `sequence_number` | `bigint` | Sequence number of this transaction. This must match the sequence number stored in   the sender's account at the time the transaction executes. |
| `payload` | [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md) | Instructions for the Aptos Blockchain, including publishing a module,   execute a script function or execute a script payload. |
| `max_gas_amount` | `bigint` | Maximum total gas to spend for this transaction. The account must have more   than this gas or the transaction will be discarded during validation. |
| `gas_unit_price` | `bigint` | Price to be paid per gas unit. |
| `expiration_timestamp_secs` | `bigint` | The blockchain timestamp at which the blockchain would discard this transaction. |
| `chain_id` | [`ChainId`](TxnBuilderTypes.ChainId.md) | The chain ID of the blockchain that this transaction is intended to be run on. |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:38](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L38)

## Properties

### chain\_id

• `Readonly` **chain\_id**: [`ChainId`](TxnBuilderTypes.ChainId.md)

___

### expiration\_timestamp\_secs

• `Readonly` **expiration\_timestamp\_secs**: `bigint`

___

### gas\_unit\_price

• `Readonly` **gas\_unit\_price**: `bigint`

___

### max\_gas\_amount

• `Readonly` **max\_gas\_amount**: `bigint`

___

### payload

• `Readonly` **payload**: [`TransactionPayload`](TxnBuilderTypes.TransactionPayload.md)

___

### sender

• `Readonly` **sender**: [`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

___

### sequence\_number

• `Readonly` **sequence\_number**: `bigint`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:48](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L48)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`RawTransaction`](TxnBuilderTypes.RawTransaction.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`RawTransaction`](TxnBuilderTypes.RawTransaction.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:58](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L58)
