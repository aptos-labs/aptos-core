---
title: "Aptos Token"
id: "aptos-token"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos Token

:::tip Aptos token tutorial
Also see the tutorial [Your First NFT](/tutorials/your-first-nft.md).
:::


## Overview of NFT

An [NFT](https://en.wikipedia.org/wiki/Non-fungible_token) is a non-fungible [token](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) or data stored on a blockchain that uniquely defines ownership of an asset. NFTs were first defined in [EIP-721](https://eips.ethereum.org/EIPS/eip-721) and later expanded upon in [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155). NFTs are typically defined using the following properties:

- `name`: The name of the asset. It must be unique within a collection.
- `description`: The description of the asset.
- `uri`: A URL pointer to off-chain for more information about the asset. The asset could be media such as an image or video or more metadata.
- `supply`: The total number of units of this NFT. Many NFTs have only a single supply, while those that have more than one are referred to as editions.

Additionally, most NFTs are part of a collection or a set of NFTs with a common attribute, for example, a theme, creator, or minimally contract. Each collection has a similar set of attributes:

- `name`: The name of the collection. The name must be unique within the creator's account.
- `description`: The description of the collection.
- `uri`: A URL pointer to off-chain for more information about the asset. The asset could be media such as an image or video or more metadata.
- `supply`: The total number of NFTs in this collection. 
- `maximum`: The maximum number of NFTs that this collection can have. If `maximum` is set to 0, then supply is untracked. 

## Design principles

The [Aptos token standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) is developed with the following principles:

- **Interoperability**: Provide a standard implementation to improve interoperability across the ecosystem projects. Moreover, Move being a static language without dynamic dispatch makes this principle even more imperative.
- **Liquidity**: Achieve maximal liquidity by defining the NFT, fungible (non-decimal) and semi-fungible tokens in one contract. These different types of tokens can be easily stored, transferred and transacted in the same way. As a consequence, it becomes easier to achieve maximal interoperability across the marketplaces, exchanges, and other methods of exchange.
- **Rich on-chain token properties**: Enable the customization of on-chain token properties. Users can define their own properties and store them on-chain. This can potentially eliminate the need for the off-chain metadata.
- **Reduced overhead**: Reduce the cost of creating large amounts of NFTs from fungible tokens. This can lead to, for example, reduced overhead for similar tokens by the reuse of on-chain metadata for certain fungible tokens.

:::tip Fungible token → NFT
The Aptos token standard supports [mutation of a fungible token to an NFT](#evolving-from-fungible-token-to-nft).
:::

### Storing customized token properties on-chain

The Aptos token standard provides an easy way to store properties on-chain. You can define your own properties based on your application. Each token can also have its own property values. These properties can be directly read and written by smart contracts.

#### Default properties

You can add your properties to [`default_properties`](https://github.com/aptos-labs/aptos-core/blob/e62fd09cb1c916d857fa655b3f174991ef8698b3/aptos-move/framework/aptos-token/sources/token.move#L98) in the TokenData. The properties defined here are shared by all tokens by default.

The `default_properties` field is a key-value store with type information. It leverages the PropertyMap module which contain functions for serializing and deserializing different primitive types to the property value.

#### Token properties

You can also use the `token_properties` defined in the token itself for customization on-chain. You can create customized values for a property of this  specific token, thereby allowing a token to have a different property value from its default.

### Evolving from fungible token to NFT

Fungible tokens share the same default property values. However, these property values can evolve over time and become different from each other. To support such evolution of token properties, the Aptos token standard provides the `property_version` field. Here is how it works:

- During the token creation (minting), all tokens initially have `property_version` set to `0` and these tokens can be stacked together as fungible token. 
- When the creators mutate the default properties of a token, the mutated token will be assigned a unique `property_version` to create a new [`token_id`](https://github.com/aptos-labs/aptos-core/blob/bba1690d7268759bd86ccd7459d7967172f1da24/aptos-move/framework/aptos-token/sources/token.move#L288) to differentiate it from other fungible tokens. This unique `token_id` allows the token to have its own property values, and all further mutation of this token does **not** change the `property_version` again. This token essentially becomes an NFT now. 

#### Configuring mutability

To make mutability explicit for both the creator and owner, the Aptos token standard provides [`mutability_config`](https://github.com/aptos-labs/aptos-core/blob/bba1690d7268759bd86ccd7459d7967172f1da24/aptos-move/framework/aptos-token/sources/token.move#L100) at both the collection level and the token level to control which fields are mutable. Configurable here means the creator can configure this field to be mutable or immutable during creation.

### Storing metadata off-chain

Follow the standard below to ensure your NFT can be correctly displayed by various wallets.

You should store the metadata in a JSON file located in an off-chain storage solution such as [arweave](https://www.arweave.org/) and provide the URL to the JSON file in the `uri` field of the token or the collection. We recommend the developers follow the [ERC-1155 off-chain data](https://eips.ethereum.org/EIPS/eip-1155) schema to format their JSON files.
```json
{
  "image": "https://www.arweave.net/abcd5678?ext=png",
  "animation_url": "https://www.arweave.net/efgh1234?ext=mp4",
  "external_url": "https://petra.app/",
  "attributes": [
    {
      "trait_type": "web",
      "value": "yes"
    },
    {
      "trait_type": "mobile",
      "value": "yes"
    },
    {
      "trait_type": "extension",
      "value": "yes"
    }
  ],
  "properties": {
    "files": [
      {
        "uri": "https://www.arweave.net/abcd5678?ext=png",
        "type": "image/png"
      },
      {
        "uri": "https://watch.videodelivery.net/9876jkl",
        "type": "unknown",
        "cdn": true
      },
      {
        "uri": "https://www.arweave.net/efgh1234?ext=mp4",
        "type": "video/mp4"
      }
    ],
    "category": "video",
  }
}
```
* `image`: URL to the image asset. You may use the `?ext={file_extension}` query to provide information on the file type.
* `animation_url`: URL to the multimedia attachment of the asset. You may use the same `file_extension` query to provide the file type.
* `external_url`: URL to an external website where the user can also view the image.
* `attributes` - Object array, where an object should contain `trait_type` and `value` fields. `value` can be a string or a number.
* `properties.files`: Object array, where an object should contain the URI and type of the file that is part of the asset. The type should match the file extension. The array should also include files specified in `image` and `animation_url` fields, as well as any other files associated with the asset. You may use the `?ext={file_extension}` query to provide information on the file type.
* `properties.category`: Has supported categories:
  * `image` - PNG, GIF, JPG
  * `video` - MP4, MOV 
  * `audio` - MP3, FLAC, WAV
  * `vr` - 3D models; GLB, GLTF 
  * `html` - HTML pages; scripts and relative paths within the HTML page are also supported

You can also host your files on CDN to provide faster loading time by using the `cdn` flag in the file object. 
When the file exists, this should be the primary location to read the media file (`video`, `audio`, `vr`) by wallet. 
If the file is no longer available, the wallet can fall back to use the `animation_url` to load the file. 
```json
"properties": {
  "files": [
    ...
    {
      "uri": "https://watch.videodelivery.net/52a52c4a261c88f19d267931426c9be6",
      "type": "unknown",
      "cdn": true
    },
    ...
  ]
}
```

## Token data model

The [following diagram](/img/docs/aptos-token-standard-flow.svg) depicts the flow of token data through Aptos.

<ThemedImage
alt="Signed Transaction Flow"
sources={{
light: useBaseUrl('/img/docs/aptos-token-standard-flow.svg'),
dark: useBaseUrl('/img/docs/aptos-token-standard-flow-dark.svg'),
}}
/>

## Token resources

As shown in the diagram above, token-related data are stored at both the creator’s account and the owner’s account.

### Struct-level resources

The following tables describe fields at the struct level. For the definitive list, see the [Move reference documentation](../../guides/move-guides/index.md#aptos-move-documentation) for the [Aptos Token Framework](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/overview.md).

#### Resource stored at the creator’s address

| Field | Description |
| --- | --- |
| [`Collections`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#resource-collections) | Maintains a table called `collection_data`, which maps the collection name to the `CollectionData`. It also stores all the `TokenData` that this creator creates. |
| [`CollectionData`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#struct-collectiondata) | Stores the collection metadata. The supply is the number of tokens created for the current collection. The maxium is the upper bound of tokens in this collection. |
| [`CollectionMutabilityConfig`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_CollectionMutabilityConfig) | Specifies which field is mutable. |
| [`TokenData`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_TokenData) | Acts as the main struct for holding the token metadata. Properties is a where users can add their own properties that are not defined in the token data. Users can mint more tokens based on the `TokenData`, and those tokens share the same `TokenData`. |
| [`TokenMutabilityConfig`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_TokenMutabilityConfig) | Controls which fields are mutable. |
| [`TokenDataId`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_TokenDataId) | An ID used for representing and querying `TokenData` on-chain. This ID mainly contains three fields including creator address, collection name and token name. |
| [`Royalty`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_Royalty) | Specifies the denominator and numerator for calculating the royalty fee. It also has the payee account address for depositing the royalty. |
| `PropertyValue` | Contains both value of a property and type of property. |

#### Resource stored at the owner’s address

| Field | Description |
| --- | --- |
| [`TokenStore`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_TokenStore) | The main struct for storing the token owned by this address. It maps `TokenId` to the actual token. |
| [`Token`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_Token) | The amount is the number of tokens. |
| [`TokenId`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/token.md#0x3_token_TokenId) | `TokenDataId` points to the metadata of this token. The `property_version` represents a token with mutated `PropertyMap` from `default_properties` in the `TokenData`. |

### Coin and token field descriptions

The following tables contain descriptions and examples on key token fields. Once again, see the [Move reference documentation](../../guides/move-guides/index.md#aptos-move-documentation) for the [Aptos Token Framework](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/overview.md).

#### Token module key struct

##### [`Token`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move#L144)

| Field | Type | Description |
| --- | --- | --- |
| `id` | TokenId | Unique identification of the token |
| `amount` | u64 | The number of tokens stored |
| `token_properties` | PropertyMap | The properties of this token; PropertyMap is a key-value map used for storing different types in one map; the value of PropertyMap contains both the value and type information |

##### `TokenId`

| Field | Type | Description |
| --- | --- | --- |
| `token_data_id` | TokenDataId | Identification for the metadata of the token |
| `property_version` | u64 | The version of the property map; when a fungible token is mutated, a new property version is created and assigned to the token to make it an NFT |

##### `TokenDataId`

| Field | Type | Description |
| --- | --- | --- |
| `creator` | address | The address of the creator, eg: 0xcafe |
| `collection` | String | The name of collection; this is unique under the same account, eg: “Aptos Animal Collection” |
| `name` | String | The name of the token; this is the same as the name field of TokenData |

##### `TokenData`

| Field | Type | Description |
| --- | --- | --- |
| `maximum` | u64 | The maximal number of tokens that can be minted under this TokenData; if the maximum is 0, there is no limit |
| `largest_property_version` | u64 | The current largest property version of all tokens with this TokenData |
| `supply` | String | The number of tokens with this TokenData |
| `uri` | String | The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain storage; the URL length should be less than 512 characters, eg: https://arweave.net/Fmmn4ul-7Mv6vzm7JwE69O-I-vd6Bz2QriJO1niwCh4 |
| `royalty` | Royalty | The denominator and numerator for calculating the royalty fee; it also contains payee account address for depositing the Royalty |
| `name` | String | The name of the token, which should be unique within the collection; the length of name should be smaller than 128, characters, eg: “Aptos Animal #1234” |
| `description` | String | The description of the token |
| `default_properties` | PropertyMap | The properties are stored in the TokenData that are shared by all tokens  |
| `mutability_config` | TokenMutabilityConfig | Specifies which fields are mutable  |

##### `CollectionData`

| Field | Type | Description |
| --- | --- | --- |
| `description` | String | A description for the token collection Eg: “Aptos Toad Overload” |
| `name` | String | The collection name, which should be unique among all collections by the creator; the name should also be smaller than 128 characters, eg: “Animal Collection” |
| `uri` | String | The URI for the collection; its length should be smaller than 512 characters |
| `supply` | u64 | The number of different TokenData entries in this collection |
| `maximum` | u64 | If maximal is a non-zero value, the number of created TokenData entries should be smaller or equal to this maximum If maximal is 0, Aptos doen’t track the supply of this collection, and there is no limit |
| `mutability_config` | CollectionMutabilityConfig | Specifies which fields are mutable  |

## Token lifecycle

### Token creation

Every Aptos token belongs to a collection. The developer first needs to create a collection through `create_collection_script` and then create the token belonging to the collection `create_token_script`.
To achieve parallel `TokenData` and `Token` creation, a developer can create unlimited collection and `TokenData` where the `maximum` of the collection and `TokenData` are set as 0. With this setting, the token contract won’t track the supply of types of token (`TokenData` count) and supply of token within each token type. As the result, the `TokenData` and token can be created in parallel.

Aptos also enforces simple validation of the input size and prevents duplication:
* Token name - unique within each collection
* Collection name - unique within each account
* Token and collection name length - smaller than 128 characters
* URI length - smaller than 512 characters
* Property map - can hold at most 1000 properties, and each key should be smaller than 128 characters

### Token mutation

Our standard supports mutation with a principle that the mutable fields are specified during the token creation. This allows the token owner to be informed which fields are mutable when they get the token from the creator.
Our contract uses `CollectionMutabilityConfig` to check if a field is mutable. Our contract uses `TokenMutabilityConfig` to check if a `TokenData` field is mutable.

For mutation of properties, we have both
* `default_properties` stored in `TokenData` shared by all tokens belonging to the `TokenData`
* `token_properties` stored in the token itself

To mutate `default_properties`, developers can use `mutate_tokendata_property` to mutate the properties when `TokenMutabilityConfig` is set to `true`.

> **CAUTION: Set the `TokenMutabilityConfig` field to `false` unless it is absolutely necessary. Allowing `default_properties` to be mutable provides creators too much power; creators can change the burnable config to provide themselves the authority to burn tokens after token creation.**

To mutate `token_properties` stored in the token, our standard uses the `TOKEN_PROPERTY_MUTABLE` property stored in `default_properties`. When the creator creates the `TokenData` with the `TOKEN_PROPERTY_MUTABLE` property set to `true`, the creator can mutate `token_properties`. Note that if the `mutate_tokendata_property` is set to `true`, creators can mutate the `token_properties` anyway since they can overwrite the setting in `default_properties`.

### Token burn

We provide `burn` and `burn_by_creator` functions for token owners and token creators to burn (or destroy) tokens. However, these two functions are also guarded by configs that are specified during the token creation so that both creator and owner are clear on who can burn the token.
Burn is allowed only when the `BURNABLE_BY_OWNER` property is set to `true` in `default_properties`. Burn by creator is allowed when `BURNABLE_BY_CREATOR` is `true` in `default_properties`.
Once all the tokens belonging to a `TokenData` are burned, the `TokenData` will be removed from the creator’s account. Similarly, if all `TokenData` belonging to a collection are removed, the `CollectionData` will be removed from the creator’s account.

## Token transfer

To protect a user from receiving undesired NFTs, an Aptos user must be first offered an NFT, followed by the user claiming the offered NFTs. Then only these NFTs will be deposited in the user's token store. This is the default token transfer behavior. For example:

1. If Alice wants to send Bob an NFT, she must first offer Bob this NFT. This NFT is still stored under Alice’s account. 
1. Only when Bob claims the NFT, this NFT will be removed from Alice’s account and stored in Bob’s token store. 

:::tip Token transfer module
The token transfer is implemented in the [`token_transfers`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token_transfers.move) module. 
:::

### Direct transfer

On the other hand, if a user wants to receive direct transfer of the NFT, skipping the initial steps of offer and claim, then the user can call [`opt_in_direct_transfer`](https://github.com/aptos-labs/aptos-core/blob/283348c6ea4ce198fb27eb3ef1c1e471739aa1aa/aptos-move/framework/aptos-token/sources/token.move#L297) to allow other people to directly transfer the NFTs into the user's token store.  

Note that in both the default token transfer and the direct transfer method, the user will receive the NFT into the user's token store. 

:::tip Turning off direct transfer
The user can also turn off this direct transfer behavior by calling the same `opt_in_direct_transfer` function to reset the behavior to the default behavior. 
:::

