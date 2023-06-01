---
title: "Aptos Token V2"
id: "aptos-token-v2"
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
was developed with the following as an improvement on the Aptos Token standard. It has these ideas in mind:
* **Flexibility** - NFTs are flexible and can be customized to accommodate any creative designs.
* **Composability** - Multiple NFTs can be easily composed together, such that the final object is greater than the sum of its parts
* **Scalability** - Greater parallelism between tokens

The base token only provides minimal functionalities and is meant to build upon to add more functionalities. All of its
functions are non-entry and thus not callable directly from off chain. Creators need to write their own modules that use
these fuctionalities or use "no code" solutions also provided in the framework. One such solution is [aptos_token](#aptos-token)
which provides functionalities such as custom metadata (via PropertyMap) and soul bound.

## Comparison to Token V1

Token V2 uses Aptos [objects](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/object.move)
rather than account resources traditionally used in Move. This allows for storing data outside the account and adding
flexibility in this way. 
* Tokens can be easily extended with custom data and functionalities without requiring any changes in the framework
* Transfers are simply a reference update
* Direct transfer is allowed without an opt in
* NFTs can own other NFTs adding easy composability
* Soul bound tokens can be easily supported

## Collections and tokens as objects
In this Token standard, both collections and tokens will be separate [objects](./aptos-object.md). They have their own
distinct addresses and can be referenced both on and off chain by address. Each object can contain multiple resources
so collections and tokens are extensible by default, allowing the creator to add custom data and functionalities without
having to modify the framework.

On chain, another struct can include a reference to the collection or token objects like below:
```rust
struct ReferenceExample has key {
    my_collection: Object<Collection>,
    my_token: Object<Token>,
}
```
where both `my_collection` and `my_token` are addresses (with `Object<>` wrapper).

Off-chain, the address of the object can be passed along to replace object arguments in entry functions called by transaction creation.
as arguments. For example:
```rust
public entry fun my_function(my_collection: Object<Collection>) {
    // Do something with the collection
}
```

Collection and token addresses will also be used to query data such as fetching all resources via fullnode API or against
an indexing service.

### Royalties
Following the object extensibility pattern, royalties are added to collections or tokens as a resource with associated
functionality provided by [the royalty module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/royalty.move)
Royalty can be updated as long as a `MutatorRef`, a storable struct that grants  permissions, is generated at creation
time and stored.

See [Aptos Token](#aptos-token) for examples on how Royalty's `MutatorRef` can be stored and used.
Royalty can also be set directly on a token if it has a different royalty config than the collection's.

## Token lifecycle
All token v2 modules are deployed at `0x4`.

### Collection creation
Every token belongs to a collection. The developer first needs to create a collection with:
1. A fixed maximum supply. Current supply is tracked and cannot exceed the maximum set.
```rust
use aptos_token_objects::collection;

public entry fun create_collection(creator: &signer) {
    let max_supply = 1000;
    collection::create_fixed_collection(
        creator,
        "My Collection Description",
        max_supply,
        "My Collection",
        royalty,
        "https://mycollection.com",
    );
}
```
2. Unlimited supply. Current supply is still tracked but there's no maximum enforced.
```rust
public entry fun create_collection(creator: &signer) {
    collection::create_unlimited_collection(
        creator,
        "My Collection Description",
        "My Collection",
        royalty,
        "https://mycollection.com",
    );
}
```
Note that both track the current supply. Maximum supply cannot be changed after the collection is created, and a
collection cannot be converted from unlimited to fixed supply or vice versa.

A collection has the following attributes:
* Collection name - unique within each account. This means a single creator account cannot create more than one
collection with the same name.
* Description - modifiable with a `MutatorRef` and smaller than 2048 characters
* URI length - modifiable with a `MutatorRef` and smaller than 512 characters
* Royalty - specifies how many % of the sale price goes to the creator of the collection. This can be changed with a
`MutatorRef` generated by the Royalty module.

A `MutatorRef`, a storable struct that grants permissions to mutate, can be generated only during creation of the collection.
If created, the holder of the `MutatorRef` can change the `description` and the `URI length` of the collection.
```rust
public entry fun create_collection(creator: &signer) {
    let collection_constructor_ref = &collection::create_unlimited_collection(
        creator,
        "My Collection Description",
        "My Collection",
        royalty,
        "https://mycollection.com",
    );
    let mutator_ref = collection::get_mutator_ref(collection_constructor_ref);
    // Store the mutator ref somewhere safe
}
```

### Collection customization
A collection can be customized by adding more data (as resources) or functionalities. For example, a collection can track
when it was created in order to limit when tokens can be minted.
```rust
struct MyCollectionMetadata has key {
    creation_timestamp_secs: u64,
}

public entry fun create_collection(creator: &signer) {
    // Constructor ref is a non-storable struct returned when creating a new object.
    // It can generate an object signer to add resources to the collection object.
    let collection_constructor_ref = &collection::create_unlimited_collection(
        creator,
        "My Collection Description",
        "My Collection",
        royalty,
        "https://mycollection.com",
    );
    // Constructor ref can be exchanged for signer to add resources to the collection object.
    let collection_signer = &object::generate_signer(collection_constructor_ref);
    move_to(collection_signer, MyCollectionMetadata { creation_timestamp_secs: timestamp::now_seconds() } })
}
```

### Token creation
Creators can mint tokens, which are separate objects from the collection. This allows for greater customization.
Tokens can be created in two ways:
1. Named tokens. These tokens have deterministic addresses that are sha256 hash of the creator address, collection name,
and token name, concatenated. This allows for predictable addresses and easier querying of tokens. However,
named tokens are fully deletable and thus burning them will only delete the token data and not fully delete the underlying
object
```rust
use aptos_token_objects::token;

public entry fun mint_token(creator: &signer) {
    token::create_named_token(
        creator,
        "My Collection",
        "My named Token description",
        "My named token",
        royalty,
        "https://mycollection.com/my-named-token.jpeg",
    );
}
```
2. (Unnamed) tokens based on the creator account's guid. These tokens have addresses are generated based on the creator
account's incrementing guid. The addresses of unnamed tokens are not deterministic as the account's guid can change outside
minting. Thus, querying for unnamed tokens is more difficult and requires indexing.
```rust
use aptos_token_objects::token;

public entry fun mint_token(creator: &signer) {
    token::create_from_account(
        creator,
        "My Collection",
        "My named Token description",
        "My named token",
        royalty,
        "https://mycollection.com/my-named-token.jpeg",
    );
}
```

Creators should cautiously consider whether they should use `create_named_token` or `create_from_account` when building
their custom collection/token. In general `create_from_account` is recommended as it allows for clean deletion if the
tokens are burnt and generally, deterministic addresses for tokens are not always necessary thanks to indexing services.
One example that would prefer deterministic addresses and thus `create_named_token` is a collection of soul bound tokens
where each token's address is created from the holder's name.

### Token properties
Tokens by default have the following properties:
* Token name - unique within each collection. A collection cannot have more than one token with the same name.
* Token description - modifiable with a `MutatorRef` and smaller than 2048 characters
* Token URI length - modifiable with a `MutatorRef` and smaller than 512 characters
* Royalty - It's less common to have royalty setting on the token instead of collection. But this allows a token to have
a different royalty setting than the collection's.

A `MutatorRef` can be generated only during creation of the token.
```rust
public entry fun mint_token(creator: &signer) {
    // Constructor ref is a non-storable struct returned when creating a new object.
    // It can be exchanged for signer to add resources to the token object.
    let token_constructor_ref = &token::create_from_account(
        creator,
        "My Collection",
        "My named Token description",
        "My named token",
        royalty,
        "https://mycollection.com/my-named-token.jpeg",
    );
    let mutator_ref = token::generate_mutator_ref(token_constructor_ref);
    // Store the mutator ref somewhere safe
}
```

### Token customization
More data can be added to the token as resources, similar to [for collections](#collection-customization).

### Token burn
Tokens can be burned by the creator if they generated and stored a `BurnRef` during the creation of the token.
```rust
public entry fun mint_token(creator: &signer) {
    let token_constructor_ref = &token::create_from_account(
        creator,
        "My Collection",
        "My named Token description",
        "My named token",
        royalty,
        "https://mycollection.com/my-named-token.jpeg",
    );
    let burn_ref = token::generate_burn_ref(token_constructor_ref);
    // Store the burn ref somewhere safe
}

public entry fun burn_token(token: Object<Token>) {
    // Remove all custom data from the token object.
    let token_address = object::object_address(&token);
    let CustomData { ... } = move_from<CustomData>(token_address);

    // Retrieve the burn ref from storage    
    let burn_ref = ...;
    token::burn(burn_ref);
}
```
Note that if any custom data was added to the token objects, the `burn_token` function needs to first remove those data.
token::burn only deletes the object if it was created as an unnamed token. Named token will have all token data removed,
but the object will stay, thus creating a "burned" defunct object.

### Token transfer
Tokens can be simply transferred as objects to any user via `object::transfer`

## Aptos Token
[Aptos Token](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/aptos_token.move)
is a "no code" solution that builds on top of the base token v2 standard and provides a more complete solution that
allows creators to mint NFTs without writing any code. It provides the following main features:
* Soul bound tokens which are non-transferable by holders
* Custom defined properties stored in a [PropertyMap](#property-map), a simple map data structure of attribute name (string) -> values (bytes).
* [Freezing and unfreezing transfers of non-soul bound tokens](#creator-management)
* [Creator management functionalities - modify a collection or token's metadata](#creator-management)

### Property Map
Similar to in Token Standard v1, Aptos Token provides an extensible [PropertyMap](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/property_map.move)
that provides type safe, but generic properties for a given NFT. Creators can set pass initial properties when minting a
token and can freely add or remove properties later.

Tokens can be minted using the provided `aptos_token::mint`. This is an entry function and can be called via a transaction
directly.
```rust
public entry fun mint(
    creator: &signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector<String>,
    property_types: vector<String>,
    property_values: vector<vector<u8>>,
    ) acquires AptosCollection, AptosToken
```

To mint a soul bound token, the creator can call `aptos_token::mint_soul_bound` instead. This will create a token that
the holder cannot transfer.
```rust
public entry fun mint_soul_bound(
    creator: &signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector<String>,
    property_types: vector<String>,
    property_values: vector<vector<u8>>,
    soul_bound_to: address,
) acquires AptosCollection
```

### Creator management
By default, the creator can do the following:
* Mint and burn tokens, including soul bound tokens
* Disallow transferring a token (freeze) and allow transferring a token (unfreeze)
* Update the collection's description and uri
* Add/Remove metadata properties from a token's property map
* Update a collection's royalty setting
* Update a token's name, description and uri

### Further customization
Aptos Token is provided as a "no code" convenient solution, but it's not extensible. This is evident as most functions
are entry functions and do not return any ref (constructor, mutator, etc.). The `aptos_token` module stores and manages
the refs obtained from creating the collection and token objects and do not expose raw access to them.

If a creator wants more custom functionalities such as being able to forcefully transfer a soul bound token, they would
need to write their own custom module that builds on top of the base token v2 standard. They can of course borrow inspiration
and code from the Aptos Token module.

## Fungible Token
Similar to [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155), the Token v2 standard also supports fungible tokens
(also known as semi-fungible tokens). An example of this would be armor tokens in a game. Each armor token represents a
type of armor and is a token in a collection with metadata (e.g. durability, defense, etc.) and can be minted and burned.
However, there are multiple instances of the same armor type. For example, a player can have 3 wooden armors, where wooden armor
is a token in the Armor collection.

This can be easily built by combining Token v2 and Fungible Assets. After the creator creates the Armor collection and the
Wooden Armor token, they can make the Wooden Armor token "fungible":

```rust
use aptos_framework::primary_fungible_store;

public entry fun create_armor_collection(creator: &signer) {
    collection::create_unlimited_collection(
        creator,
        "Collection containing different types of armors. Each armor type is a separate token",
        "Armor",
        royalty,
        "https://myarmor.com",
    );
}

public entry fun create_armor_type(creator: &signer, armor_type: String) {
    let new_armor_type_constructor_ref = &token::create_from_account(
        creator,
        "Armor",
        "Armor description",
        armor_type,
        royalty,
        "https://myarmor.com/my-named-token.jpeg",
    );
    // Make this armor token fungible so there can multiple instances of it.
    primary_fungible_store::create_primary_store_enabled_fungible_asset(
        new_armor_type_constructor_ref,
        maximum_number_of_armors,
        armor_type,
        "ARMOR",
        0, // Armor cannot be divided so decimals is 0,
        "https://mycollection.com/armor-icon.jpeg",
        "https://myarmor.com",
    );

    // Add properties such as durability, defence, etc. to this armor token
}
```

Now the creator can mint multiple instances of the same armor type and transfer them to players. The players can freely
transfer the armor tokens to each other the same way they would transfer a fungible asset.
