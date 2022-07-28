---
id: "TxnBuilderTypes.MultiEd25519PublicKey"
title: "Class: MultiEd25519PublicKey"
sidebar_label: "MultiEd25519PublicKey"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).MultiEd25519PublicKey

## Constructors

### constructor

• **new MultiEd25519PublicKey**(`public_keys`, `threshold`)

Public key for a K-of-N multisig transaction. A K-of-N multisig transaction means that for such a
transaction to be executed, at least K out of the N authorized signers have signed the transaction
and passed the check conducted by the chain.

**`see`** [Creating a Signed Transaction](https://aptos.dev/guides/creating-a-signed-transaction#multisignature-transactions)

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `public_keys` | [`Seq`](../namespaces/BCS.md#seq)<[`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md)\> | A list of public keys |
| `threshold` | `number` | At least "threshold" signatures must be valid |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:22](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L22)

## Properties

### public\_keys

• `Readonly` **public\_keys**: [`Seq`](../namespaces/BCS.md#seq)<[`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md)\>

___

### threshold

• `Readonly` **threshold**: `number`

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:42](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L42)

___

### toBytes

▸ **toBytes**(): `Uint8Array`

Converts a MultiEd25519PublicKey into bytes with: bytes = p1_bytes | ... | pn_bytes | threshold

#### Returns

`Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:31](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L31)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts:46](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/multi_ed25519.ts#L46)
