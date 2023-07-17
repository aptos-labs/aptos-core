---
title: Object
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Object
The [Object model](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/object.move) allows Move to represent a complex type as a set of resources stored within a single address and offers a rich capability model that allows for fine-grained resource control and ownership management.

In the object model, an NFT or token can place common token data within a Token resource, object data within an ObjectCore resource, and then specialize into additional resources as necessary. For example, a Player object could define a player within a game and be an NFT at the same time. The ObjectCore itself stores both the address of the current owner and the appropriate data for creating event streams.

## Comparison with the account resources model
The existing Aptos data model emphasizes the use of the store ability within Move. Store allows for a struct to exist within any struct that is stored on-chain. As a result, data can live anywhere within any struct and at any address. While this provides great flexibility it has many limitations:

1. Data is not be guaranteed to be accessible, for example, it can be placed within a user-defined resource that may violate expectations for that data, e.g., a creator attempting to burn an NFT put into a user-defined store. This can be confusing to both the users and creators of this data.
2. Data of differing types can be stored to a single data structure (e.g., map, vector) via `any`, but for complex data types `any` incurs additional costs within Move as each access requires deserialization. It also can lead to confusion if API developers expect that a specific any field changes the type it represents.
3. While resource accounts allow for greater autonomy of data, they do so inefficiently for objects and do not take advantage of resource groups.
4. Data cannot be recursively composable, because Move currently prohibits recursive data structures. Furthermore, experience suggests that true recursive data structures can lead to security vulnerabilities.
5. Existing data cannot be easily referenced from entry functions, for example, supporting string validation requires many lines of code. Attempting to make tables directly becomes impractical as keys can be composed of many types, thus specializing to support within entry functions becomes complex.
6. Events cannot be emitted from data but from an account that may not be associated with the data.
7. Transferring logic is limited to the APIs provided in the respective modules and generally requires loading resources on both the sender and receiver adding unnecessary cost overheads.

:::tip
Object is a core primitive in Aptos Move and created via the object module at 0x1::object
:::

## Structure
An object is stored in the ObjectGroup resource group, which enables other resources within the object to be co-located for data locality and data cost savings. It's important to note that not all resources within an object need to be co-located within the ObjectGroup, and it's up to the developer of an object to determine their data layout.

### Object resource group

Object is a container for resources that are stored within a single address. These resources usually represent related data often accessed together and should be stored within a single address for data locality and cost savings.
When created, an object has a resource group, ObjectGroup, by default:
```rust
#[resource_group(scope = global)]
struct ObjectGroup { }
```

Each object also has the core ObjectCore resource with fundamental properties:
```rust
#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct ObjectCore has key {
    /// Used by guid to guarantee globally unique objects and create event streams
    guid_creation_num: u64,
    /// The address (object or account) that owns this object
    owner: address,
    /// Object transferring is a common operation, this allows for disabling and enabling
    /// transfers. Bypassing the use of a the TransferRef.
    allow_ungated_transfer: bool,
    /// Emitted events upon transferring of ownership.
    transfer_events: event::EventHandle<TransferEvent>,
}
```

After creating an object, creators can extend with additional resources. For example, an exchange can create an object for each of its liquidity pools and add a resource to track the pool's liquidity.
```rust
#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct LiquidityPool has key {
    token_a: Object<FungibleAsset>,
    token_b: Object<FungibleAsset>,
    reserves_a: u128,
    reserves_b: u128
}
```

In the above code, `token_a` and `token_b` are references to other objects. Specifically, `Object<T>` is a reference to an object stored at a given address that contains `T` resource. In this example, they're fungible assets (similar to coins). This will be covered in more detail in the fungible asset standard.
LiquidityPool resource is part of the ObjectGroup resource group. This means that the LiquidityPool resource is stored in the same storage slot as the ObjectCore resource. This is more storage and gas efficient for reading and writing data.

LiquidityPool resource can be added during construction of the object:
```rust
use aptos_framework::object::{Self, Object};
use aptos_framework::fungible_asset::FungibleAsset;

public fun create_liquidity_pool(
    token_a: Object<FungibleAsset>,
    token_b: Object<FungibleAsset>,
    reserves_a: u128,
    reserves_b: u128
): Object<LiquidityPool> {
    let exchange_signer = &get_exchange_signer();
    let liquidity_pool_constructor_ref = &object::create_object_from_account(exchange_signer);
    let liquidity_pool_signer = &object::generate_signer(liquidity_pool_constructor_ref);
    move_to(liquidity_pool_signer, LiquidityPool {
        token_a: token_a,
        token_b: token_b,
        reserves_a: reserves_a,
        reserves_b: reserves_b
    });
    object::object_from_constructor_ref(liquidity_pool_constructor_ref)
}
```

More resources can also be added post-creation if the exchange module stores the ExtendRef. This is covered in more detail in the Capabilities section.

## Object Lifecycle
### Creation
Objects can be created via several different functions provided in the object module:
```rust
/// Create a new named object and return the ConstructorRef. Named objects can be queried globally
/// by knowing the user generated seed used to create them. Named objects cannot be deleted.
public fun create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef;

/// Create a new object from a GUID generated by an account.
public fun create_object_from_account(creator: &signer): ConstructorRef;

/// Create a new object from a GUID generated by an object.
public fun create_object_from_object(creator: &signer): ConstructorRef;
```

These functions generate object addresses in different schemas:
1. `create_named_object` generates an address from the caller-provided seed and creator address. This is a deterministic address that can be queried globally. The formula used is sha3(creator address + seed + 0xFD).
2. `create_object_from_account` generates an address from the caller's address and a GUID generated by the caller's account. The formula used is sha3(creator address + account guid + 0xFD).
3. `create_object_from_object` generates an address from the caller's address and a GUID generated by the caller's object. The formula used is sha3(creator address + object guid + 0xFD).
The domain separation ensures there's no conflict among objects created via these different functions.

Note that since named objects have deterministic addresses, they cannot be deleted. This is to prevent a malicious user from creating an object with the same seed as a named object and deleting it.

### Object capabilities (refs)
The object creation functions all return a transient ConstructorRef that cannot be stored. ConstructorRef allows adding resources to an object (see example from the previous section).
ConstructorRef can also be used to generate the other capabilities (or "refs") that are used to manage the object:
```rust
/// Generates the DeleteRef, which can be used to remove Object from global storage.
public fun generate_delete_ref(ref: &ConstructorRef): DeleteRef;

/// Generates the ExtendRef, which can be used to add new events and resources to the object.
public fun generate_extend_ref(ref: &ConstructorRef): ExtendRef;

/// Generates the TransferRef, which can be used to manage object transfers.
public fun generate_transfer_ref(ref: &ConstructorRef): TransferRef;

/// Create a signer for the ConstructorRef
public fun generate_signer(ref: &ConstructorRef): signer;
```
These refs can be stored and used to manage the object.

DeleteRef can be used to delete the object:
```rust
use aptos_framework::object::{Object, DeleteRef};

struct DeleteRefStore has key {
    delete_ref: DeleteRef,
}

public fun delete_liquidity_pool(liquidity_pool: Object<LiquidityPool>) {
    let liquidity_pool_address = object::object_address(liquidity_pool);
    // Remove all resources added to the liquidity pool object.
    let LiquidityPool {
        token_a: _,
        token_b: _,
        reserves_a: _,
        reserves_b: _
    } = move_from<LiquidityPool>(liquidity_pool_address);
    let DeleteRefStore { delete_ref } = move_from<DeleteRefStore>(liquidity_pool_address);
    // Delete the object itself.
    object::delete_object(delete_ref);
}
```

ExtendRef can be used to add resources to the object like the LiquidityPool resource in the previous section:
TransferRef can be used to disable owner-transfer when `ungated_transfer_allowed = true` or to forcefully transfer the object without the owner being involved:
```rust
use aptos_framework::object::{Object, TransferRef};

struct TransferRefStore has key {
    transfer_ref: TransferRef,
}

public fun disable_owner_transfer(liquidity_pool: Object<LiquidityPool>) {
    let liquidity_pool_address = object::object_address(liquidity_pool);
    let transfer_ref = &borrow_global_mut<TransferRefStore>(liquidity_pool_address).transfer_ref;
    object::disable_ungated_transfer(transfer_ref);
}

public fun creator_transfer(liquidity_pool: Object<LiquidityPool>, new_owner: address) {
    let liquidity_pool_address = object::object_address(liquidity_pool);
    let transfer_ref = &borrow_global_mut<TransferRefStore>(liquidity_pool_address).transfer_ref;
    object::transfer_with_ref(object::generate_linear_transfer_ref(transfer_ref), new_owner);
}
```

Once the resources have been created on an object, they can be modified by the creator modules without the refs/ Example:
```rust
public entry fun modify_reserves(liquidity_pool: Object<LiquidityPool>) {
    let liquidity_pool = &mut borrow_global_mut<LiquidityPool>(liquidity_pool);
    liquidity_pool.reserves_a = liquidity_pool.reserves_a + 1000;
}
```

### Object reference
A reference to the object can be generated any time and stored in a resource as part of an object or account:
```rust
/// Returns the address of within a ConstructorRef
public fun object_from_constructor_ref<T: key>(ref: &ConstructorRef): Object<T>;
```
`Object<T>` is a reference around the object address with the guarantee that `T` exists when the reference is created. For example, we can create an `Object<LiquidityPool>` for a liquidity pool object.
Creating an object reference with a non-existent `T` will fail at runtime.
Note that after references are created and stored, they do not guarantee that the resource `T` or the entire object itself has not been deleted.

### Events
Objects come with transfer_events by default, which are emitted when the object is transferred. Transfer events are stored in the ObjectCore resource.

Additionally, similar to account resources, events can be added in an object' resources. The object module offers the following functions to create event handles for objects:
```rust
/// Create a guid for the object, typically used for events
public fun create_guid(object: &signer): guid::GUID;

/// Generate a new event handle.
public fun new_event_handle<T: drop + store>(object: &signer): event::EventHandle<T>;
```

These event handles can be stored in the custom resources added to the object. Example:
```rust
struct LiquidityPoolEventStore has key {
    create_events: event::EventHandle<CreateLiquidtyPoolEvent>,
}

struct CreateLiquidtyPoolEvent {
    token_a: address,
    token_b: address,
    reserves_a: u128,
    reserves_b: u128,
}

public entry fun create_liquidity_pool_with_events() {
    let exchange_signer = &get_exchange_signer();
    let liquidity_pool_constructor_ref = &object::create_object_from_account(exchange_signer);
    let liquidity_pool_signer = &object::generate_signer(liquidity_pool_constructor_ref);
    let event_handle = object::new_event_handle<CreateLiquidtyPoolEvent>(liquidity_pool_signer);
    event::emit<CreateLiquidtyPoolEvent>(event_handle, CreateLiquidtyPoolEvent {
        token_a: token_a,
        token_b: token_b,
        reserves_a: reserves_a,
        reserves_b: reserves_b,
    });
    let liquidity_pool = move_to(liquidity_pool_signer, LiquidityPool {
        token_a: token_a,
        token_b: token_b,
        reserves_a: reserves_a,
        reserves_b: reserves_b,
        create_events: event_handle,
    });
}
```
