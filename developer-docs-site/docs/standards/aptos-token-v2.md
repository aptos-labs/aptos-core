---
title: "Aptos Token"
id: "aptos-token"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos Token V2

## Overview of NFTs

An [NFT](https://en.wikipedia.org/wiki/Non-fungible_token) is a non-fungible [token](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move) 
or data stored on a blockchain that uniquely defines ownership of an asset. NFTs were first defined in 
[EIP-721](https://eips.ethereum.org/EIPS/eip-721) and later expanded upon in [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155). 
NFTs are typically defined using the following properties:

- `name`: The name of the asset. It must be unique within a collection.
- `description`: The description of the asset.
- `uri`: A URL pointer to off-chain for more information about the asset. The asset could be media such as an image or video or more metadata.
- `supply`: The total number of units of this NFT. Many NFTs have only a single supply, while those that have more than one are referred to as editions.

Additionally, most NFTs are part of a collection or a set of NFTs with a common attribute, for example, a theme, creator,
or minimally contract. Each collection has a similar set of attributes:

- `name`: The name of the collection. The name must be unique within the creator's account.
- `description`: The description of the collection.
- `uri`: A URL pointer to off-chain for more information about the asset. The asset could be media such as an image or video or more metadata.
- `supply`: The total number of NFTs in this collection. 
- `maximum`: The maximum number of NFTs that this collection can have. If `maximum` is set to 0, then supply is untracked. 

## Design principles

The [Aptos token v2 standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move) 
was developed with the following as an improvement on the Aptos Token standard.  It has these ideas in mind:
* **Flexibility** - Allowing creators the ability to be creative in the space and allow for novel usage of NFTs
* **Composability** - Allowing creators the ability to compose multiple NFTs together, such that the final object is greater than the sum of its parts
* **Scalability** - Allowing for even greater parallelism between tokens

Additionally, it took feedback from the community to add support for:
* Instant transfers
* Soul bound tokens
* and much more

## How Token V2 differs from Token V1

Token V2 uses Aptos [objects](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/object.move)
rather than account resources traditionally used in Move.  This allows for storing data outside the account and adding
flexibility in this way.  
* Transfers are simply a reference update
* Direct transfer is allowed without an opt in
* NFTs can own other NFTs adding easy composability
* Soul bound tokens are supported

## Token data model

TODO: Diagram of the data model and associated links

## Aptos Token -> No code tokens

The [Aptos Token](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/aptos_token.move)
module provides a no code solution for most use cases of NFTs.  It provides these main features:
* Base token and collection features
* Creator definable mutability for tokens
* Creator-based freezing of tokens
* Standard object-based transfer and events
* Metadata property type

### Royalties

Royalties are simply [another module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/royalty.move)
that is attached to a token.  They're allowed to be updated as long as a `MutatorRef` is generated at creation time.

### Property Map

Similar to TokenV1, we provide an extensible [PropertyMap](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/property_map.move)
that provides type safe, but generic properties for a given NFT.

## Token lifecycle

### Collection creation

Every Aptos token belongs to a collection. The developer first needs to create a collection with a [fixed or unlimited supply](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/collection.move#L115-L176).
This choice needs to be made up front for supply tracking purposes.

A collection has the following attributes:
* Collection name - unique within each account
* Description - modifiable with a `MutatorRef` and smaller than 2048 characters
* URI length - modifiable with a `MutatorRef` and smaller than 512 characters

A `MutatorRef` can be [generated only on creation](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/collection.move#L321-L325)
of the collection.  If created, the holder of the `MutatorRef` can change the `description` and the `URI length` of the
collection.

### Token creation

Tokens can be [created on an account](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move#L127-L141).
This creates objects for the token that can be transferred between accounts, with extra specialization on top.

### Token mutation

Our standard supports mutation with a principle that the ability for mutation must be specified at creation time. This 
allows the token owner to be informed if the token is mutable when they get the token from the creator.

Supported mutations are:
* Token name
* Token description
* Token URI

A `MutatorRef` must be [generated at token creation time](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move#L157-L161)
and provides the owner of the `MutatorRef` the ability modify the token.

### Token burn

We provide a [`burn` function](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move#L250-L275) 
for anyone who has the `BurnRef` for the token. 

A `BurnRef` must be [generated at token creation time](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move#L163-L173)
and provides the owner of the `BurnRef` the ability modify the token.

### Token transfer

We provide simple functionality for transferring tokens between the sender and receiver. Tokens are objects in this model.  Therefore, tokens can be simply transferred to any user.  Note that there is a
configuration on every object that allows for preventing transfer of objects.  This provides the functionality for
soul bound tokens to not be transferable.

:::tip Token transfer module
The token transfer is implemented in the [`object`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/object.move#L348-L444) module.
:::
