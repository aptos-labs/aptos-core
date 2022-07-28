---
id: "TxnBuilderTypes.TransactionAuthenticatorEd25519"
title: "Class: TransactionAuthenticatorEd25519"
sidebar_label: "TransactionAuthenticatorEd25519"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionAuthenticatorEd25519

## Hierarchy

- [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

  ↳ **`TransactionAuthenticatorEd25519`**

## Constructors

### constructor

• **new TransactionAuthenticatorEd25519**(`public_key`, `signature`)

An authenticator for single signature.

**`see`** [Creating a Signed Transaction](https://aptos.dev/guides/creating-a-signed-transaction/)
for details about generating a signature.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `public_key` | [`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md) | Client's public key. |
| `signature` | [`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md) | Signature of a raw transaction. |

#### Overrides

[TransactionAuthenticator](TxnBuilderTypes.TransactionAuthenticator.md).[constructor](TxnBuilderTypes.TransactionAuthenticator.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:34](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L34)

## Properties

### public\_key

• `Readonly` **public\_key**: [`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md)

___

### signature

• `Readonly` **signature**: [`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:38](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L38)

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

▸ `Static` **load**(`deserializer`): [`TransactionAuthenticatorEd25519`](TxnBuilderTypes.TransactionAuthenticatorEd25519.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionAuthenticatorEd25519`](TxnBuilderTypes.TransactionAuthenticatorEd25519.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:44](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L44)
