---
id: "TxnBuilderTypes.AuthenticationKey"
title: "Class: AuthenticationKey"
sidebar_label: "AuthenticationKey"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).AuthenticationKey

Each account stores an authentication key. Authentication key enables account owners to rotate
their private key(s) associated with the account without changing the address that hosts their account.

**`see`** {@link * https://aptos.dev/basics/basics-accounts | Account Basics}

Account addresses can be derived from AuthenticationKey

## Constructors

### constructor

• **new AuthenticationKey**(`bytes`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `bytes` | `Uint8Array` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts:20](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts#L20)

## Properties

### bytes

• `Readonly` **bytes**: `Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts:18](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts#L18)

___

### LENGTH

▪ `Static` `Readonly` **LENGTH**: `number` = `32`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts:14](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts#L14)

___

### MULTI\_ED25519\_SCHEME

▪ `Static` `Readonly` **MULTI\_ED25519\_SCHEME**: `number` = `1`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts:16](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts#L16)

## Methods

### derivedAddress

▸ **derivedAddress**(): [`HexString`](HexString.md)

Derives an account address from AuthenticationKey. Since current AccountAddress is 32 bytes,
AuthenticationKey bytes are directly translated to AccountAddress.

#### Returns

[`HexString`](HexString.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts:44](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts#L44)

___

### fromMultiEd25519PublicKey

▸ `Static` **fromMultiEd25519PublicKey**(`publicKey`): [`AuthenticationKey`](TxnBuilderTypes.AuthenticationKey.md)

Converts a K-of-N MultiEd25519PublicKey to AuthenticationKey with:
`auth_key = sha3-256(p_1 | … | p_n | K | 0x01)`. `K` represents the K-of-N required for
authenticating the transaction. `0x01` is the 1-byte scheme for multisig.

#### Parameters

| Name | Type |
| :------ | :------ |
| `publicKey` | [`MultiEd25519PublicKey`](TxnBuilderTypes.MultiEd25519PublicKey.md) |

#### Returns

[`AuthenticationKey`](TxnBuilderTypes.AuthenticationKey.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts:32](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/authentication_key.ts#L32)
