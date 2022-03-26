---
title: "Your first NFT"
slug: "your-first-nft"
sidebar_position: 3
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Note: The following tutorial is a work in progress.

# Your first NFT

An [NFT](https://en.wikipedia.org/wiki/Non-fungible_token) is a non-fungible token or data stored on a blockchain that uniquely defines ownership of an asset. NFTs were first defined in [EIP-721](https://eips.ethereum.org/EIPS/eip-721) and later expanded upon in [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155). NFTs typically comprise of the following aspects:

* A name, the name of the asset
* A description, the description of the asset
* A URL, a non-descript pointer off-chain to more information about the asset could be media such as an image or video or more metadata
* A supply, the total number of units of this NFT, many NFTs have only a single supply while those that have more than one are referred to as editions

Additionally, most NFTs are part of a collection or a set of NFTs with a common attribute, e.g., theme, creator, or minimally contract.

The Aptos implementation for core NFTs or Tokens can be found in [Token.move](https://github.com/aptos-labs/aptos-core/blob/nft/aptos-move/framework/aptos-framework/sources/Token.move).

## Aptos Token Definitions

### The Token

The Aptos Token is defined as:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `id` | `GUID:ID` | A globally unique identifier for this token also useful for identifying the creator |
| `collection` | `GUID:ID` | A globally unique identifier for the collection that contains this token |
| `balance` | `u64` | The current stored amount of this token in relation to the supply, `1 <= balance <= supply` |
| `data` | `TokenData` | Additional data about this token, this is set of supply of the token is 1 |

The Aptos TokenData is defined as:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `description` | `ASCII::String` | Describes this token |
| `metadata` | `TokenType` | A generic, a optional user defined struct to contain additional information about this token on-chain |
| `name` | `ASCII::String` | The name of this token |
| `supply` | `u64` | Total number of editions of this Token |
| `uri` | `ASCII::String` | URL for additional information / media |

Tokens are defined with the move attributes `drop` and `store`, which means that they can be burned and saved to global storage. Tokens, however, cannot be copied the total number in supply cannot be changed, e.g., the balance of one Token copied to a new instance of that Token. Hence because Tokens lack the `copy` attribute, they are finite. The `id` and `balance` of a token provide its globally unique identity.

TokenData has the `copy` attribute because none of the fields on that define part of its globally unique identity. Users trading Tokens are advised that two Tokens can share the same TokenData as the Aptos standard does not attempt to identify if a token copies attributes from another. It is worth repeating that it is the `id` of a Token and its `balance` that define a token and the rest of the fields are there to assist in data retrieval and understanding of the token.

TokenType provides for optional data extensibility of the Aptos Token. This data, too, is expected to be copy.

### The Token Collection

Aptos defines a set of collections grouped together by their unique `id`:

```rust
struct Collections<TokenType: copy + drop + store> has key {
    collections: Table<ID, Collection<TokenType>>,
}
```

As the `Collections` has the attribute `key`, it is stored directly to the creators account. It is important to note, that if there were no notion of `Collections` and instead `Collection` had the `key` attribute, an Aptos account could only have a single collection of `TokenType`, which often is not the case.

Each collection has the following fields:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `tokens` | `Table<GUID::ID, TokenMetadata<TokenType>>` | Keeps track of all Tokens associated with this collection |
| `claimed_tokens` | `Table<GUID::ID, address>` | Keeps track of where tokens wth `supply == 1` are stored |
| `id` | `GUID::ID` | Unique identifier for this collection |
| `description` | `ASCII::String` | Describes this collection |
| `name` | `ASCII::String` | The name of this collection |
| `uri` | `ASCII::String` | URL for additional information / media |
| `count` | `u64` | Total number of distinct Tokens tracked by this collection |
| `maximum` | `Option<u64>` | Optional, maximum amount of tokens that can be minted within this collection |

A collection is not a store for accumulating Tokens, so instead of containing a `Token`, it contains a `TokenMetadata`:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `id` | `GUID:ID` | A globally unique identifier for this token also useful for identifying the creator |
| `data` | `TokenData` | Additional data about this token, this is set of supply of the token `> 1` |

### Storing Tokens

In order to acquire and store tokens, a user must have a `Gallery` of `TokenType`:

```rust
struct Gallery<TokenType: copy + drop + store> has key {
    gallery: Table<ID, Token<TokenType>>,
}
```

Like the `Collections`, this is stored as a resource on an Aptos account.

## Create Tokens

## Sharing Tokens
