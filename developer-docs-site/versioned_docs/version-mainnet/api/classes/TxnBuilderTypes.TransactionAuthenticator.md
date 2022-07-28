---
id: "TxnBuilderTypes.TransactionAuthenticator"
title: "Class: TransactionAuthenticator"
sidebar_label: "TransactionAuthenticator"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).TransactionAuthenticator

## Hierarchy

- **`TransactionAuthenticator`**

  ↳ [`TransactionAuthenticatorEd25519`](TxnBuilderTypes.TransactionAuthenticatorEd25519.md)

  ↳ [`TransactionAuthenticatorMultiEd25519`](TxnBuilderTypes.TransactionAuthenticatorMultiEd25519.md)

  ↳ [`TransactionAuthenticatorMultiAgent`](TxnBuilderTypes.TransactionAuthenticatorMultiAgent.md)

## Constructors

### constructor

• **new TransactionAuthenticator**()

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:8](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L8)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`TransactionAuthenticator`](TxnBuilderTypes.TransactionAuthenticator.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authenticator.ts#L10)
