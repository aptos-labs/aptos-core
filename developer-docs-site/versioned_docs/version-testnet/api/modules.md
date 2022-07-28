---
id: "modules"
title: "aptos"
sidebar_label: "Exports"
sidebar_position: 0.5
custom_edit_url: null
---

## Namespaces

- [BCS](namespaces/BCS.md)
- [TxnBuilderTypes](namespaces/TxnBuilderTypes.md)
- [Types](namespaces/Types.md)

## Classes

- [AptosAccount](classes/AptosAccount.md)
- [AptosClient](classes/AptosClient.md)
- [FaucetClient](classes/FaucetClient.md)
- [HexString](classes/HexString.md)
- [RequestError](classes/RequestError.md)
- [TokenClient](classes/TokenClient.md)
- [TransactionBuilderEd25519](classes/TransactionBuilderEd25519.md)
- [TransactionBuilderMultiEd25519](classes/TransactionBuilderMultiEd25519.md)

## Interfaces

- [AptosAccountObject](interfaces/AptosAccountObject.md)

## Type Aliases

### AptosClientConfig

Ƭ **AptosClientConfig**: `Omit`<`AxiosRequestConfig`, ``"data"`` \| ``"cancelToken"`` \| ``"method"``\>

#### Defined in

[ecosystem/typescript/sdk/src/aptos_client.ts:29](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_client.ts#L29)

___

### MaybeHexString

Ƭ **MaybeHexString**: [`HexString`](classes/HexString.md) \| `string` \| [`HexEncodedBytes`](namespaces/Types.md#hexencodedbytes)

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:5](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L5)

___

### SigningFn

Ƭ **SigningFn**: (`txn`: [`SigningMessage`](namespaces/TxnBuilderTypes.md#signingmessage)) => [`Ed25519Signature`](classes/TxnBuilderTypes.Ed25519Signature.md) \| [`MultiEd25519Signature`](classes/TxnBuilderTypes.MultiEd25519Signature.md)

#### Type declaration

▸ (`txn`): [`Ed25519Signature`](classes/TxnBuilderTypes.Ed25519Signature.md) \| [`MultiEd25519Signature`](classes/TxnBuilderTypes.MultiEd25519Signature.md)

Function that takes in a Signing Message (serialized raw transaction)
 and returns a signature

##### Parameters

| Name | Type |
| :------ | :------ |
| `txn` | [`SigningMessage`](namespaces/TxnBuilderTypes.md#signingmessage) |

##### Returns

[`Ed25519Signature`](classes/TxnBuilderTypes.Ed25519Signature.md) \| [`MultiEd25519Signature`](classes/TxnBuilderTypes.MultiEd25519Signature.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/builder.ts:22](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/builder.ts#L22)

## Functions

### raiseForStatus

▸ **raiseForStatus**<`T`\>(`expectedStatus`, `response`, `requestContent?`): `void`

#### Type parameters

| Name |
| :------ |
| `T` |

#### Parameters

| Name | Type |
| :------ | :------ |
| `expectedStatus` | `number` |
| `response` | `AxiosResponse`<`T`, [`AptosError`](interfaces/Types.AptosError.md)\> |
| `requestContent?` | `any` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/aptos_client.ts:31](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_client.ts#L31)
