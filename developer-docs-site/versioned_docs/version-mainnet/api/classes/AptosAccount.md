---
id: "AptosAccount"
title: "Class: AptosAccount"
sidebar_label: "AptosAccount"
sidebar_position: 0
custom_edit_url: null
---

Class for creating and managing Aptos account

## Constructors

### constructor

• **new AptosAccount**(`privateKeyBytes?`, `address?`)

Creates new account instance. Constructor allows passing in an address,
to handle account key rotation, where auth_key != public_key

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `privateKeyBytes?` | `Uint8Array` | Private key from which account key pair will be generated. If not specified, new key pair is going to be created. |
| `address?` | [`MaybeHexString`](../modules.md#maybehexstring) | Account address (e.g. 0xe8012714cd17606cee7188a2a365eef3fe760be598750678c8c5954eb548a591). If not specified, a new one will be generated from public key |

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:41](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L41)

## Properties

### accountAddress

• `Private` `Readonly` **accountAddress**: [`HexString`](HexString.md)

Address associated with the given account

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:25](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L25)

___

### authKeyCached

• `Private` `Optional` **authKeyCached**: [`HexString`](HexString.md)

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:27](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L27)

___

### signingKey

• `Readonly` **signingKey**: `SignKeyPair`

A private key and public key, associated with the given account

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:20](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L20)

## Methods

### address

▸ **address**(): [`HexString`](HexString.md)

This is the key by which Aptos account is referenced.
It is the 32-byte of the SHA-3 256 cryptographic hash
of the public key(s) concatenated with a signature scheme identifier byte

#### Returns

[`HexString`](HexString.md)

Address associated with the given account

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:56](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L56)

___

### authKey

▸ **authKey**(): [`HexString`](HexString.md)

This key enables account owners to rotate their private key(s)
associated with the account without changing the address that hosts their account.
See here for more info: [https://aptos.dev/basics/basics-accounts#single-signer-authentication](https://aptos.dev/basics/basics-accounts#single-signer-authentication)

#### Returns

[`HexString`](HexString.md)

Authentication key for the associated account

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:66](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L66)

___

### pubKey

▸ **pubKey**(): [`HexString`](HexString.md)

This key is generated with Ed25519 scheme.
Public key is used to check a signature of transaction, signed by given account

#### Returns

[`HexString`](HexString.md)

The public key for the associated account

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:81](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L81)

___

### signBuffer

▸ **signBuffer**(`buffer`): [`HexString`](HexString.md)

Signs specified `buffer` with account's private key

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `buffer` | `Buffer` | A buffer to sign |

#### Returns

[`HexString`](HexString.md)

A signature HexString

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:90](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L90)

___

### signHexString

▸ **signHexString**(`hexString`): [`HexString`](HexString.md)

Signs specified `hexString` with account's private key

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `hexString` | [`MaybeHexString`](../modules.md#maybehexstring) | A regular string or HexString to sign |

#### Returns

[`HexString`](HexString.md)

A signature HexString

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:100](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L100)

___

### toPrivateKeyObject

▸ **toPrivateKeyObject**(): [`AptosAccountObject`](../interfaces/AptosAccountObject.md)

Derives account address, public key and private key

**`example`** An example of the returned AptosAccountObject object
```
{
   address: "0xe8012714cd17606cee7188a2a365eef3fe760be598750678c8c5954eb548a591",
   publicKeyHex: "0xf56d8524faf79fbc0f48c13aeed3b0ce5dd376b4db93b8130a107c0a5e04ba04",
   privateKeyHex: `0x009c9f7c992a06cfafe916f125d8adb7a395fca243e264a8e56a4b3e6accf940
     d2b11e9ece3049ce60e3c7b4a1c58aebfa9298e29a30a58a67f1998646135204`
}
```

#### Returns

[`AptosAccountObject`](../interfaces/AptosAccountObject.md)

AptosAccountObject instance.

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:118](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L118)

___

### fromAptosAccountObject

▸ `Static` **fromAptosAccountObject**(`obj`): [`AptosAccount`](AptosAccount.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `obj` | [`AptosAccountObject`](../interfaces/AptosAccountObject.md) |

#### Returns

[`AptosAccount`](AptosAccount.md)

#### Defined in

[ecosystem/typescript/sdk/src/aptos_account.ts:29](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_account.ts#L29)
