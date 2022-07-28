---
id: "TxnBuilderTypes.AccountAuthenticator"
title: "Class: AccountAuthenticator"
sidebar_label: "AccountAuthenticator"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).AccountAuthenticator

## Hierarchy

- **`AccountAuthenticator`**

  ↳ [`AccountAuthenticatorEd25519`](TxnBuilderTypes.AccountAuthenticatorEd25519.md)

  ↳ [`AccountAuthenticatorMultiEd25519`](TxnBuilderTypes.AccountAuthenticatorMultiEd25519.md)

## Constructors

### constructor

• **new AccountAuthenticator**()

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:101](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L101)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`AccountAuthenticator`](TxnBuilderTypes.AccountAuthenticator.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:103](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L103)
