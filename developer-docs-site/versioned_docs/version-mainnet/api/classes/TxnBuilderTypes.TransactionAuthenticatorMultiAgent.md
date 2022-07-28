---
id: "TxnBuilderTypes.TransactionAuthenticatorMultiAgent"
title: "Class: TransactionAuthenticatorMultiAgent"
sidebar_label: "TransactionAuthenticatorMultiAgent"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionAuthenticatorMultiAgent

## Hierarchy

- [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

  ↳ **`TransactionAuthenticatorMultiAgent`**

## Constructors

### constructor

• **new TransactionAuthenticatorMultiAgent**(`sender`, `secondary_signer_addresses`, `secondary_signers`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `sender` | [`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md) |
| `secondary_signer_addresses` | [`Seq`](../namespaces/BCS.md#seq)<[`AccountAddress`](TxnBuilderTypes.AccountAddress.md)\> |
| `secondary_signers` | [`Seq`](../namespaces/BCS.md#seq)<[`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)\> |

#### Overrides

[TransactionAuthenticator](TxnBuilderTypes.TransactionAuthenticator.md).[constructor](TxnBuilderTypes.TransactionAuthenticator.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:77](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L77)

## Properties

### secondary\_signer\_addresses

• `Readonly` **secondary\_signer\_addresses**: [`Seq`](../namespaces/BCS.md#seq)<[`AccountAddress`](TxnBuilderTypes.AccountAddress.md)\>

___

### secondary\_signers

• `Readonly` **secondary\_signers**: [`Seq`](../namespaces/BCS.md#seq)<[`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)\>

___

### sender

• `Readonly` **sender**: [`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

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

[TransactionAuthenticator](TxnBuilderTypes.TransactionAuthenticator.md).[serialize](TxnBuilderTypes.TransactionAuthenticator.md#serialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:85](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L85)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

#### Inherited from

[TransactionAuthenticator](TxnBuilderTypes.TransactionAuthenticator.md).[deserialize](TxnBuilderTypes.TransactionAuthenticator.md#deserialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L10)

___

### load

▸ `Static` **load**(`deserializer`): [`TransactionAuthenticatorMultiAgent`](TxnBuilderTypes.TransactionAuthenticatorMultiAgent.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionAuthenticatorMultiAgent`](TxnBuilderTypes.TransactionAuthenticatorMultiAgent.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:92](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L92)
