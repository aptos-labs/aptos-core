---
title: "Your first NFT"
slug: "your-first-nft"
sidebar_position: 3
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Note: The following tutorial is a work in progress. Furthermore, the Aptos' (Non-Fungible) Token specification has not been formalized.

# Tokens and NFTs in Aptos

An [NFT](https://en.wikipedia.org/wiki/Non-fungible_token) is a non-fungible token or data stored on a blockchain that uniquely defines ownership of an asset. NFTs were first defined in [EIP-721](https://eips.ethereum.org/EIPS/eip-721) and later expanded upon in [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155). NFTs typically comprise of the following aspects:

* A name, the name of the asset, which must be unique within a collection
* A description, the description of the asset
* A URL, a non-descript pointer off-chain to more information about the asset could be media such as an image or video or more metadata
* A supply, the total number of units of this NFT, many NFTs have only a single supply while those that have more than one are referred to as editions

Additionally, most NFTs are part of a collection or a set of NFTs with a common attribute, e.g., theme, creator, or minimally contract. Each collection has a similar set of attributes:

* A name, the name of the collection, which must be unique within the creator's account
* A description, the description of the asset
* A URL, a non-descript pointer off-chain to more information about the asset could be media such as an image or video or more metadata

The Aptos implementation for core NFTs or Tokens can be found in [Token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/Token.move).

## Aptos Token Definitions

### The Token

The Aptos Token is defined as:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `id` | `GUID:ID` | A globally unique identifier for this token also useful for identifying the creator |
| `name` | `ASCII::String` | The name of this token, must be unique within the collection |
| `collection` | `GUID:ID` | A globally unique identifier for the collection that contains this token |
| `balance` | `u64` | The current stored amount of this token in relation to the supply, `1 <= balance <= supply` |

The Aptos TokenData is defined as:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `id` | `GUID:ID` | A globally unique identifier for this token also useful for identifying the creator |
| `description` | `ASCII::String` | Describes this token |
| `name` | `ASCII::String` | The name of this token, must be unique within the collection |
| `supply` | `u64` | Total number of editions of this Token |
| `uri` | `ASCII::String` | URL for additional information / media |
| `metadata` | `TokenType` | A generic, a optional user defined struct to contain additional information about this token on-chain |

Tokens are defined with the move attributes `store`, which means that they can be saved to global storage. Tokens cannot be implicitly dropped and must be burned to ensure that the total balance is equal to supply. Tokens cannot be copied. That is the total balance or supply cannot be changed by anyone but the creator due to the lack of a copy operator. Note, the current APIs do not expose the ability for post creation mints. A token can be uniquely identified by either its `id` or by the tuple of `TokenType, collection name, and token name`.

TokenData has the `copy` attribute to support simplicity of token balance splitting. A token split can happen whenever an individual with balance greater than `1` offers another individual a portion of their balance less than their total balance. Users trading Tokens are advised that two Tokens can share the same TokenData as the Aptos standard does not attempt to identify if a token copies attributes from another. Repeating what was said earlier, the token can be uniquely identified by either its `id` or `TokenType`, collection name, and token name. A creator could change any of the values in the set of `TokenType`, collection name, or token name to create similar but not identtcal tokens.

### The Token Collection

Aptos defines a set of collections grouped together by their unique `id`:

```rust
struct Collections<TokenType: copy + drop + store> has key {
    collections: Table<ASCII::String, Collection>,
}

struct TokenMetadata<TokenType: store> has key {
    metadata: Table<ID, TokenType>,
}
```

As the `Collections` has the attribute `key`, it is stored directly to the creators account. It is important to note, that if there were no notion of `Collections` and instead `Collection` had the `key` attribute, an Aptos account could only have a single collection, which often is not the case. A collection can be looked up in the set of Collections by name, hence enforcing a unique collection name.

The Token and TokenData structs are fixed in their content. The resource `TokenMetadata` enables creators to store additional token data. The Data in the table is stored as the `Token`'s unique `ID`. The use of this is optional and requires API specialization due to the limitation that script functions cannot support structs or generics.

Each collection has the following fields:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `tokens` | `Table<ASCII::String, TokenMetadata<TokenType>>` | Keeps track of all Tokens associated with this collection |
| `claimed_tokens` | `Table<ASCII::String, address>` | Keeps track of where tokens wth `supply == 1` are stored |
| `description` | `ASCII::String` | Describes this collection |
| `name` | `ASCII::String` | The name of this collection, must be unique within the creators account for the specified `TokenType`. |
| `uri` | `ASCII::String` | URL for additional information / media |
| `count` | `u64` | Total number of distinct Tokens tracked by this collection |
| `maximum` | `Option<u64>` | Optional, maximum amount of tokens that can be minted within this collection |

A collection is not a store for accumulating Tokens, so instead of containing a `Token`, it contains a `TokenData`:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `id` | `GUID:ID` | A globally unique identifier for this token also useful for identifying the creator |
| `data` | `TokenData` | Additional data about this token, this is set of supply of the token `> 1` |

### Storing Tokens

In order to acquire and store tokens, a user must have a `Gallery` of `TokenType`:

```rust
struct Gallery has key {
    gallery: Table<ID, Token>,
}
```

Like the `Collections`, this is stored as a resource on an Aptos account.

## Introducing Tokens

As part of our core framework, Aptos provides a basic [Token](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/Token.move) interface with no additional data, or explicitly one in which the `TokenMetadata` resource has no entry for that token. The motivations for doing so include:

* Creating a new token requires writing Move code
* The Script function for creating a new token must be specialized as Move does not support template types or structs as input arguments
* Template types on script functions add extra friction to writing script functions

This tutorial will walk you through the process of
* creating your own Token collection,
* a Token of our favorite cat,
* and giving that token to someone else.

This tutorial builds on [Your first transaction](/tutorials/your-first-transaction) as a library for this example. The following tutorial contains example code that can be downloaded in its entirety below:

<Tabs>
  <TabItem value="python" label="Python" default>

For this tutorial, will be focusing on `first_nft.py` and re-using the `first_transaction.py` library from the previous tutorial.

You can find the python project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python)

  </TabItem>
  <TabItem value="rust" label="Rust" default>

TODO

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

For this tutorial, will be focusing on `first_nft.ts` and re-using the `first_transaction.ts` library from the previous tutorial.

You can find the typescript project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript)

  </TabItem>
</Tabs>

### Creating a Collection

The Aptos Token enables creators to create finite or unlimited collections. Many NFTs are of a limited nature, where the creator only intends on creating a certain amount forever, this enforces scarcity. Whereas other tokens may have an unlimited nature, for example, a collection used for utility may see new tokens appear over time. SimpleToken collections can be instantiated with either behavior by using the appropriate script function:

Finite, that is no more than a `maximum` number of tokens can ever be minted:
```rust
public(script) fun create_finite_collection_script(
    account: signer,
    description: vector<u8>,
    name: vector<u8>,
    uri: vector<u8>,
    maximum: u64,
)
```

Unlimited, that is there is no limit to the number of tokens that can be added to the collection:
```rust
public(script) fun create_unlimited_collection_script(
    account: signer,
    description: vector<u8>,
    name: vector<u8>,
    uri: vector<u8>,
)
```

These script functions can be called via the REST API. The following demonstrates how to call the as demonstrated in the following:

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_nft.py section_1
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>

TODO
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_1
```

  </TabItem>
</Tabs>

### Creating a Token

Tokens can be created after collection creation. To do so, the token must specify the same `collection_name` as specified as the name of a previously created collection `name`. The Move script function to create a `SimpleToken` is:

```rust
public(script) fun create_token_script(
    account: signer,
    collection_name: vector<u8>,
    description: vector<u8>,
    name: vector<u8>,
    supply: u64,
    uri: vector<u8>,
)
```

These script functions can be called via the REST API. The following demonstrates how to call the as demonstrated in the following:

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_nft.py section_2
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>

TODO
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_2
```
  </TabItem>
</Tabs>

### Giving Away a Token

In Aptos and Move, each token occupies space and has ownership. Because of this, token transfers are not unilateral and require two phase process similar to a bulletin board. The sender must first register that a token is available for the recipient to claim, the recipient must then claim this token. This has been implemented in a proof of concept move module called [`TokenTransfer`](https://github.com/aptos-labs/aptos-core/blob/nft/aptos-move/framework/aptos-framework/sources/TokenTransfers.move). `SimpleToken` provides a few wrapper functions to support transferring to another account, claiming that transfer, or stopping that transfer.

#### Obtaining the Token ID

In order to transfer the token, the sender must first identify the token id based upon knowing the creator's account, the collection name, and the token name. This can be obtained by querying REST:

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_nft.py section_3
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>

TODO
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_3
```
  </TabItem>
</Tabs>

#### Offering the Token

The following Move script function in `Token` supports transferring a token to another account, effectively registering that the other account can claim the token:

```rust
public(script) fun offer_script(
    sender: signer,
    receiver: address,
    creator: address,
    token_creation_num: u64,
    amount: u64,
)
```

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_nft.py section_4
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>

TODO
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_4
```
  </TabItem>
</Tabs>

#### Claiming the Token

The following Move script function in `SimpleToken` supports receiving a token provided by the previous function, effectively claiming a token:

```rust
public(script) fun claim_script(
    sender: signer,
    receiver: address,
    creator: address,
    token_creation_num: u64,
    amount: u64,
)
```

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_nft.py section_5
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
TODO
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_5
```
  </TabItem>
</Tabs>

## Todos for Tokens

* Add ability for additional mints
* Ensure that at least a single token is produced at time of mint
* Add events -- needs feedback on what events
* Provide mutable APIs for tokens
* Write a smoketest for generics and simple token directly
* Enable burning in a safe way
