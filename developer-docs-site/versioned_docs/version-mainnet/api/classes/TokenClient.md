---
id: "TokenClient"
title: "Class: TokenClient"
sidebar_label: "TokenClient"
sidebar_position: 0
custom_edit_url: null
---

Class for creating, minting and managing minting NFT collections and tokens

## Constructors

### constructor

• **new TokenClient**(`aptosClient`)

Creates new TokenClient instance

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `aptosClient` | [`AptosClient`](AptosClient.md) | AptosClient instance |

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:16](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L16)

## Properties

### aptosClient

• **aptosClient**: [`AptosClient`](AptosClient.md)

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L10)

## Methods

### cancelTokenOffer

▸ **cancelTokenOffer**(`account`, `receiver`, `creator`, `collectionName`, `name`): `Promise`<`string`\>

Removes a token from pending claims list

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `account` | [`AptosAccount`](AptosAccount.md) | AptosAccount which will remove token from pending list |
| `receiver` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which had to claim token |
| `creator` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which created a token |
| `collectionName` | `string` | Name of collection where token is strored |
| `name` | `string` | Token name |

#### Returns

`Promise`<`string`\>

A hash of transaction

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:168](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L168)

___

### claimToken

▸ **claimToken**(`account`, `sender`, `creator`, `collectionName`, `name`): `Promise`<`string`\>

Claims a token on specified account

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `account` | [`AptosAccount`](AptosAccount.md) | AptosAccount which will claim token |
| `sender` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which holds a token |
| `creator` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which created a token |
| `collectionName` | `string` | Name of collection where token is stored |
| `name` | `string` | Token name |

#### Returns

`Promise`<`string`\>

A hash of transaction

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:142](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L142)

___

### createCollection

▸ **createCollection**(`account`, `name`, `description`, `uri`): `Promise`<`string`\>

Creates a new NFT collection within the specified account

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `account` | [`AptosAccount`](AptosAccount.md) | AptosAccount where collection will be created |
| `name` | `string` | Collection name |
| `description` | `string` | Collection description |
| `uri` | `string` | URL to additional info about collection |

#### Returns

`Promise`<`string`\>

A hash of transaction

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:44](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L44)

___

### createToken

▸ **createToken**(`account`, `collectionName`, `name`, `description`, `supply`, `uri`): `Promise`<`string`\>

Creates a new NFT within the specified account

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `account` | [`AptosAccount`](AptosAccount.md) | AptosAccount where token will be created |
| `collectionName` | `string` | Name of collection, that token belongs to |
| `name` | `string` | Token name |
| `description` | `string` | Token description |
| `supply` | `number` | Token supply |
| `uri` | `string` | URL to additional info about token |

#### Returns

`Promise`<`string`\>

A hash of transaction

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:74](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L74)

___

### getCollectionData

▸ **getCollectionData**(`creator`, `collectionName`): `Promise`<`any`\>

Queries collection data

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `creator` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which created a collection |
| `collectionName` | `string` | Collection name |

#### Returns

`Promise`<`any`\>

Collection data in below format
```
 Collection {
   // Describes the collection
   description: string,
   // Unique name within this creators account for this collection
   name: string,
   // URL for additional information/media
   uri: string,
   // Total number of distinct Tokens tracked by the collection
   count: number,
   // Optional maximum number of tokens allowed within this collections
   maximum: number
 }
```

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:205](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L205)

___

### getTokenBalance

▸ **getTokenBalance**(`creator`, `collectionName`, `tokenName`): `Promise`<[`Token`](../interfaces/Types.Token.md)\>

Queries specific token from account's TokenStore

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `creator` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which created a token |
| `collectionName` | `string` | Name of collection, which holds a token |
| `tokenName` | `string` | Token name |

#### Returns

`Promise`<[`Token`](../interfaces/Types.Token.md)\>

Token object in below format
```
Token {
  id: TokenId;
  value: number;
}
```

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:277](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L277)

___

### getTokenData

▸ **getTokenData**(`creator`, `collectionName`, `tokenName`): `Promise`<[`TokenData`](../interfaces/Types.TokenData.md)\>

Queries token data from collection

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `creator` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address which created a token |
| `collectionName` | `string` | Name of collection, which holds a token |
| `tokenName` | `string` | Token name |

#### Returns

`Promise`<[`TokenData`](../interfaces/Types.TokenData.md)\>

Token data in below format
```
TokenData {
    // Unique name within this creators account for this Token's collection
    collection: string;
    // Describes this Token
    description: string;
    // The name of this Token
    name: string;
    // Optional maximum number of this type of Token.
    maximum: number;
    // Total number of this type of Token
    supply: number;
    /// URL for additional information / media
    uri: string;
  }
```

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:242](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L242)

___

### offerToken

▸ **offerToken**(`account`, `receiver`, `creator`, `collectionName`, `name`, `amount`): `Promise`<`string`\>

Transfers specified amount of tokens from account to receiver

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `account` | [`AptosAccount`](AptosAccount.md) | AptosAccount where token from which tokens will be transfered |
| `receiver` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address to which tokens will be transfered |
| `creator` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex-encoded 16 bytes Aptos account address to which created tokens |
| `collectionName` | `string` | Name of collection where token is stored |
| `name` | `string` | Token name |
| `amount` | `number` | Amount of tokens which will be transfered |

#### Returns

`Promise`<`string`\>

A hash of transaction

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:109](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L109)

___

### submitTransactionHelper

▸ **submitTransactionHelper**(`account`, `payload`): `Promise`<`string`\>

Brings together methods for generating, signing and submitting transaction

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `account` | [`AptosAccount`](AptosAccount.md) | AptosAccount which will sign a transaction |
| `payload` | [`TransactionPayload`](../namespaces/Types.md#transactionpayload) | Transaction payload. It depends on transaction type you want to send |

#### Returns

`Promise`<`string`\>

Promise that resolves to transaction hash

#### Defined in

[ecosystem/typescript/sdk/src/token_client.ts:26](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/token_client.ts#L26)
