---
id: "TxnBuilderTypes.AccountAuthenticatorMultiEd25519"
title: "Class: AccountAuthenticatorMultiEd25519"
sidebar_label: "AccountAuthenticatorMultiEd25519"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).AccountAuthenticatorMultiEd25519

## Hierarchy

- [`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

  ↳ **`AccountAuthenticatorMultiEd25519`**

## Constructors

### constructor

• **new AccountAuthenticatorMultiEd25519**(`public_key`, `signature`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `public_key` | [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md) |
| `signature` | [`MultiEd25519Signature`](TxnBuilderTypes.MultiEd25519Signature.md) |

#### Overrides

[AccountAuthenticator](TxnBuilderTypes.AccountAuthenticator.md).[constructor](TxnBuilderTypes.AccountAuthenticator.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:135](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L135)

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

[AccountAuthenticator](TxnBuilderTypes.AccountAuthenticator.md).[serialize](TxnBuilderTypes.AccountAuthenticator.md#serialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:139](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L139)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

#### Inherited from

[AccountAuthenticator](TxnBuilderTypes.AccountAuthenticator.md).[deserialize](TxnBuilderTypes.AccountAuthenticator.md#deserialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:103](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L103)

___

### load

▸ `Static` **load**(`deserializer`): [`AccountAuthenticatorMultiEd25519`](TxnBuilderTypes.AccountAuthenticatorMultiEd25519.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`AccountAuthenticatorMultiEd25519`](TxnBuilderTypes.AccountAuthenticatorMultiEd25519.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:145](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L145)
