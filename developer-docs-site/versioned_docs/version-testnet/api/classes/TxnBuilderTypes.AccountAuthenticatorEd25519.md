---
id: "TxnBuilderTypes.AccountAuthenticatorEd25519"
title: "Class: AccountAuthenticatorEd25519"
sidebar_label: "AccountAuthenticatorEd25519"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).AccountAuthenticatorEd25519

## Hierarchy

- [`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

  ↳ **`AccountAuthenticatorEd25519`**

## Constructors

### constructor

• **new AccountAuthenticatorEd25519**(`public_key`, `signature`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `public_key` | [`Ed25519PublicKey`](TxnBuilderTypes.Ed25519PublicKey.md) |
| `signature` | [`Ed25519Signature`](TxnBuilderTypes.Ed25519Signature.md) |

#### Overrides

[AccountAuthenticator](TxnBuilderTypes.AccountAuthenticator.md).[constructor](TxnBuilderTypes.AccountAuthenticator.md#constructor)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:117](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L117)

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

[AccountAuthenticator](TxnBuilderTypes.AccountAuthenticator.md).[serialize](TxnBuilderTypes.AccountAuthenticator.md#serialize)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:121](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L121)

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

▸ `Static` **load**(`deserializer`): [`AccountAuthenticatorEd25519`](TxnBuilderTypes.AccountAuthenticatorEd25519.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`AccountAuthenticatorEd25519`](TxnBuilderTypes.AccountAuthenticatorEd25519.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:127](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L127)
