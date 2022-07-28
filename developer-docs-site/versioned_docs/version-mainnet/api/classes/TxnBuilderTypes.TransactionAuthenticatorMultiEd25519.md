---
id: "TxnBuilderTypes.TransactionAuthenticatorMultiEd25519"
title: "Class: TransactionAuthenticatorMultiEd25519"
sidebar_label: "TransactionAuthenticatorMultiEd25519"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionAuthenticatorMultiEd25519

## Hierarchy

- [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

  ↳ **`TransactionAuthenticatorMultiEd25519`**

## Constructors

### constructor

• **new TransactionAuthenticatorMultiEd25519**(`public_key`, `signature`)

An authenticator for multiple signatures.

#### Parameters

| Name | Type |
| :------ | :------ |
| `public_key` | [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md) |
| `signature` | [`MultiEd25519Signature`](TxnBuilderTypes.MultiEd25519Signature.md) |

#### Overrides

[TransactionAuthenticator](TxnBuilderTypes.TransactionAuthenticator.md).[constructor](TxnBuilderTypes.TransactionAuthenticator.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:59](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L59)

## Properties

### public\_key

• `Readonly` **public\_key**: [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md)

___

### signature

• `Readonly` **signature**: [`MultiEd25519Signature`](TxnBuilderTypes.MultiEd25519Signature.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:63](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L63)

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

▸ `Static` **load**(`deserializer`): [`TransactionAuthenticatorMultiEd25519`](TxnBuilderTypes.TransactionAuthenticatorMultiEd25519.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionAuthenticatorMultiEd25519`](TxnBuilderTypes.TransactionAuthenticatorMultiEd25519.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:69](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L69)
