
<a id="0x4_collection"></a>

# Module `0x4::collection`

This defines an object-based Collection. A collection acts as a set organizer for a group of
tokens. This includes aspects such as a general description, project URI, name, and may contain
other useful generalizations across this set of tokens.

Being built upon objects enables collections to be relatively flexible. As core primitives it
supports:
* Common fields: name, uri, description, creator
* MutatorRef leaving mutability configuration to a higher level component
* Addressed by a global identifier of creator's address and collection name, thus collections
cannot be deleted as a restriction of the object model.
* Optional support for collection-wide royalties
* Optional support for tracking of supply with events on mint or burn

TODO:
* Consider supporting changing the name of the collection with the MutatorRef. This would
require adding the field original_name.
* Consider supporting changing the aspects of supply with the MutatorRef.
* Add aggregator support when added to framework


-  [Resource `Collection`](#0x4_collection_Collection)
-  [Struct `MutatorRef`](#0x4_collection_MutatorRef)
-  [Struct `MutationEvent`](#0x4_collection_MutationEvent)
-  [Struct `Mutation`](#0x4_collection_Mutation)
-  [Resource `FixedSupply`](#0x4_collection_FixedSupply)
-  [Resource `UnlimitedSupply`](#0x4_collection_UnlimitedSupply)
-  [Resource `ConcurrentSupply`](#0x4_collection_ConcurrentSupply)
-  [Struct `BurnEvent`](#0x4_collection_BurnEvent)
-  [Struct `MintEvent`](#0x4_collection_MintEvent)
-  [Struct `Burn`](#0x4_collection_Burn)
-  [Struct `Mint`](#0x4_collection_Mint)
-  [Struct `ConcurrentBurnEvent`](#0x4_collection_ConcurrentBurnEvent)
-  [Struct `ConcurrentMintEvent`](#0x4_collection_ConcurrentMintEvent)
-  [Struct `SetMaxSupply`](#0x4_collection_SetMaxSupply)
-  [Constants](#@Constants_0)
-  [Function `create_fixed_collection`](#0x4_collection_create_fixed_collection)
-  [Function `create_unlimited_collection`](#0x4_collection_create_unlimited_collection)
-  [Function `create_untracked_collection`](#0x4_collection_create_untracked_collection)
-  [Function `create_collection_internal`](#0x4_collection_create_collection_internal)
-  [Function `create_collection_address`](#0x4_collection_create_collection_address)
-  [Function `create_collection_seed`](#0x4_collection_create_collection_seed)
-  [Function `increment_supply`](#0x4_collection_increment_supply)
-  [Function `increment_concurrent_supply`](#0x4_collection_increment_concurrent_supply)
-  [Function `decrement_supply`](#0x4_collection_decrement_supply)
-  [Function `generate_mutator_ref`](#0x4_collection_generate_mutator_ref)
-  [Function `upgrade_to_concurrent`](#0x4_collection_upgrade_to_concurrent)
-  [Function `check_collection_exists`](#0x4_collection_check_collection_exists)
-  [Function `borrow`](#0x4_collection_borrow)
-  [Function `count`](#0x4_collection_count)
-  [Function `creator`](#0x4_collection_creator)
-  [Function `description`](#0x4_collection_description)
-  [Function `name`](#0x4_collection_name)
-  [Function `uri`](#0x4_collection_uri)
-  [Function `borrow_mut`](#0x4_collection_borrow_mut)
-  [Function `set_name`](#0x4_collection_set_name)
-  [Function `set_description`](#0x4_collection_set_description)
-  [Function `set_uri`](#0x4_collection_set_uri)
-  [Function `set_max_supply`](#0x4_collection_set_max_supply)


<pre><code>use 0x1::aggregator_v2;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::object;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
use 0x4::royalty;
</code></pre>



<a id="0x4_collection_Collection"></a>

## Resource `Collection`

Represents the common fields for a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct Collection has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>
 The creator of this collection.
</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>
 A brief description of the collection.
</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 An optional categorization of similar token.
</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
 storage; the URL length will likely need a maximum any suggestions?
</dd>
<dt>
<code>mutation_events: event::EventHandle&lt;collection::MutationEvent&gt;</code>
</dt>
<dd>
 Emitted upon any mutation of the collection.
</dd>
</dl>


</details>

<a id="0x4_collection_MutatorRef"></a>

## Struct `MutatorRef`

This enables mutating description and URI by higher level services.


<pre><code>struct MutatorRef has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_MutationEvent"></a>

## Struct `MutationEvent`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code>struct MutationEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutated_field_name: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_Mutation"></a>

## Struct `Mutation`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code>&#35;[event]
struct Mutation has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutated_field_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: object::Object&lt;collection::Collection&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_value: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_value: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_FixedSupply"></a>

## Resource `FixedSupply`

Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.
and adding events and supply tracking to a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct FixedSupply has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: u64</code>
</dt>
<dd>
 Total minted - total burned
</dd>
<dt>
<code>max_supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_minted: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_events: event::EventHandle&lt;collection::BurnEvent&gt;</code>
</dt>
<dd>
 Emitted upon burning a Token.
</dd>
<dt>
<code>mint_events: event::EventHandle&lt;collection::MintEvent&gt;</code>
</dt>
<dd>
 Emitted upon minting an Token.
</dd>
</dl>


</details>

<a id="0x4_collection_UnlimitedSupply"></a>

## Resource `UnlimitedSupply`

Unlimited supply tracker, this is useful for adding events and supply tracking to a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct UnlimitedSupply has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_minted: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_events: event::EventHandle&lt;collection::BurnEvent&gt;</code>
</dt>
<dd>
 Emitted upon burning a Token.
</dd>
<dt>
<code>mint_events: event::EventHandle&lt;collection::MintEvent&gt;</code>
</dt>
<dd>
 Emitted upon minting an Token.
</dd>
</dl>


</details>

<a id="0x4_collection_ConcurrentSupply"></a>

## Resource `ConcurrentSupply`

Supply tracker, useful for tracking amount of issued tokens.
If max_value is not set to U64_MAX, this ensures that a limited number of tokens are minted.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct ConcurrentSupply has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: aggregator_v2::Aggregator&lt;u64&gt;</code>
</dt>
<dd>
 Total minted - total burned
</dd>
<dt>
<code>total_minted: aggregator_v2::Aggregator&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_BurnEvent"></a>

## Struct `BurnEvent`



<pre><code>struct BurnEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>token: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_MintEvent"></a>

## Struct `MintEvent`



<pre><code>struct MintEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>token: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_Burn"></a>

## Struct `Burn`



<pre><code>&#35;[event]
struct Burn has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection: address</code>
</dt>
<dd>

</dd>
<dt>
<code>index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>token: address</code>
</dt>
<dd>

</dd>
<dt>
<code>previous_owner: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_Mint"></a>

## Struct `Mint`



<pre><code>&#35;[event]
struct Mint has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection: address</code>
</dt>
<dd>

</dd>
<dt>
<code>index: aggregator_v2::AggregatorSnapshot&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>token: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_ConcurrentBurnEvent"></a>

## Struct `ConcurrentBurnEvent`



<pre><code>&#35;[event]
&#35;[deprecated]
struct ConcurrentBurnEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>token: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_ConcurrentMintEvent"></a>

## Struct `ConcurrentMintEvent`



<pre><code>&#35;[event]
&#35;[deprecated]
struct ConcurrentMintEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>index: aggregator_v2::AggregatorSnapshot&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>token: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_SetMaxSupply"></a>

## Struct `SetMaxSupply`



<pre><code>&#35;[event]
struct SetMaxSupply has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection: object::Object&lt;collection::Collection&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_max_supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_max_supply: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x4_collection_MAX_U64"></a>



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;
</code></pre>



<a id="0x4_collection_EURI_TOO_LONG"></a>

The URI is over the maximum length


<pre><code>const EURI_TOO_LONG: u64 &#61; 4;
</code></pre>



<a id="0x4_collection_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;
</code></pre>



<a id="0x4_collection_EALREADY_CONCURRENT"></a>

Tried upgrading collection to concurrent, but collection is already concurrent


<pre><code>const EALREADY_CONCURRENT: u64 &#61; 8;
</code></pre>



<a id="0x4_collection_ECOLLECTION_DOES_NOT_EXIST"></a>

The collection does not exist


<pre><code>const ECOLLECTION_DOES_NOT_EXIST: u64 &#61; 1;
</code></pre>



<a id="0x4_collection_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is over the maximum length


<pre><code>const ECOLLECTION_NAME_TOO_LONG: u64 &#61; 3;
</code></pre>



<a id="0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED"></a>

The collection has reached its supply and no more tokens can be minted, unless some are burned


<pre><code>const ECOLLECTION_SUPPLY_EXCEEDED: u64 &#61; 2;
</code></pre>



<a id="0x4_collection_ECONCURRENT_NOT_ENABLED"></a>

Concurrent feature flag is not yet enabled, so the function cannot be performed


<pre><code>const ECONCURRENT_NOT_ENABLED: u64 &#61; 7;
</code></pre>



<a id="0x4_collection_EDESCRIPTION_TOO_LONG"></a>

The description is over the maximum length


<pre><code>const EDESCRIPTION_TOO_LONG: u64 &#61; 5;
</code></pre>



<a id="0x4_collection_EINVALID_MAX_SUPPLY"></a>

The new max supply cannot be less than the current supply


<pre><code>const EINVALID_MAX_SUPPLY: u64 &#61; 9;
</code></pre>



<a id="0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO"></a>

The max supply must be positive


<pre><code>const EMAX_SUPPLY_CANNOT_BE_ZERO: u64 &#61; 6;
</code></pre>



<a id="0x4_collection_ENO_MAX_SUPPLY_IN_COLLECTION"></a>

The collection does not have a max supply


<pre><code>const ENO_MAX_SUPPLY_IN_COLLECTION: u64 &#61; 10;
</code></pre>



<a id="0x4_collection_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code>const MAX_COLLECTION_NAME_LENGTH: u64 &#61; 128;
</code></pre>



<a id="0x4_collection_MAX_DESCRIPTION_LENGTH"></a>



<pre><code>const MAX_DESCRIPTION_LENGTH: u64 &#61; 2048;
</code></pre>



<a id="0x4_collection_create_fixed_collection"></a>

## Function `create_fixed_collection`

Creates a fixed-sized collection, or a collection that supports a fixed amount of tokens.
This is useful to create a guaranteed, limited supply on-chain digital asset. For example,
a collection 1111 vicious vipers. Note, creating restrictions such as upward limits results
in data structures that prevent Aptos from parallelizing mints of this collection type.
Beyond that, it adds supply tracking with events.


<pre><code>public fun create_fixed_collection(creator: &amp;signer, description: string::String, max_supply: u64, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_fixed_collection(
    creator: &amp;signer,
    description: String,
    max_supply: u64,
    name: String,
    royalty: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef &#123;
    assert!(max_supply !&#61; 0, error::invalid_argument(EMAX_SUPPLY_CANNOT_BE_ZERO));
    let collection_seed &#61; create_collection_seed(&amp;name);
    let constructor_ref &#61; object::create_named_object(creator, collection_seed);
    let object_signer &#61; object::generate_signer(&amp;constructor_ref);
    if (features::concurrent_token_v2_enabled()) &#123;
        let supply &#61; ConcurrentSupply &#123;
            current_supply: aggregator_v2::create_aggregator(max_supply),
            total_minted: aggregator_v2::create_unbounded_aggregator(),
        &#125;;

        create_collection_internal(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::some(supply),
        )
    &#125; else &#123;
        let supply &#61; FixedSupply &#123;
            current_supply: 0,
            max_supply,
            total_minted: 0,
            burn_events: object::new_event_handle(&amp;object_signer),
            mint_events: object::new_event_handle(&amp;object_signer),
        &#125;;

        create_collection_internal(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::some(supply),
        )
    &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_create_unlimited_collection"></a>

## Function `create_unlimited_collection`

Creates an unlimited collection. This has support for supply tracking but does not limit
the supply of tokens.


<pre><code>public fun create_unlimited_collection(creator: &amp;signer, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_unlimited_collection(
    creator: &amp;signer,
    description: String,
    name: String,
    royalty: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef &#123;
    let collection_seed &#61; create_collection_seed(&amp;name);
    let constructor_ref &#61; object::create_named_object(creator, collection_seed);
    let object_signer &#61; object::generate_signer(&amp;constructor_ref);

    if (features::concurrent_token_v2_enabled()) &#123;
        let supply &#61; ConcurrentSupply &#123;
            current_supply: aggregator_v2::create_unbounded_aggregator(),
            total_minted: aggregator_v2::create_unbounded_aggregator(),
        &#125;;

        create_collection_internal(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::some(supply),
        )
    &#125; else &#123;
        let supply &#61; UnlimitedSupply &#123;
            current_supply: 0,
            total_minted: 0,
            burn_events: object::new_event_handle(&amp;object_signer),
            mint_events: object::new_event_handle(&amp;object_signer),
        &#125;;

        create_collection_internal(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::some(supply),
        )
    &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_create_untracked_collection"></a>

## Function `create_untracked_collection`

Creates an untracked collection, or a collection that supports an arbitrary amount of
tokens. This is useful for mass airdrops that fully leverage Aptos parallelization.
TODO: Hide this until we bring back meaningful way to enforce burns


<pre><code>fun create_untracked_collection(creator: &amp;signer, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_untracked_collection(
    creator: &amp;signer,
    description: String,
    name: String,
    royalty: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef &#123;
    let collection_seed &#61; create_collection_seed(&amp;name);
    let constructor_ref &#61; object::create_named_object(creator, collection_seed);

    create_collection_internal&lt;FixedSupply&gt;(
        creator,
        constructor_ref,
        description,
        name,
        royalty,
        uri,
        option::none(),
    )
&#125;
</code></pre>



</details>

<a id="0x4_collection_create_collection_internal"></a>

## Function `create_collection_internal`



<pre><code>fun create_collection_internal&lt;Supply: key&gt;(creator: &amp;signer, constructor_ref: object::ConstructorRef, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String, supply: option::Option&lt;Supply&gt;): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun create_collection_internal&lt;Supply: key&gt;(
    creator: &amp;signer,
    constructor_ref: ConstructorRef,
    description: String,
    name: String,
    royalty: Option&lt;Royalty&gt;,
    uri: String,
    supply: Option&lt;Supply&gt;,
): ConstructorRef &#123;
    assert!(string::length(&amp;name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));
    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
    assert!(string::length(&amp;description) &lt;&#61; MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));

    let object_signer &#61; object::generate_signer(&amp;constructor_ref);

    let collection &#61; Collection &#123;
        creator: signer::address_of(creator),
        description,
        name,
        uri,
        mutation_events: object::new_event_handle(&amp;object_signer),
    &#125;;
    move_to(&amp;object_signer, collection);

    if (option::is_some(&amp;supply)) &#123;
        move_to(&amp;object_signer, option::destroy_some(supply))
    &#125; else &#123;
        option::destroy_none(supply)
    &#125;;

    if (option::is_some(&amp;royalty)) &#123;
        royalty::init(&amp;constructor_ref, option::extract(&amp;mut royalty))
    &#125;;

    let transfer_ref &#61; object::generate_transfer_ref(&amp;constructor_ref);
    object::disable_ungated_transfer(&amp;transfer_ref);

    constructor_ref
&#125;
</code></pre>



</details>

<a id="0x4_collection_create_collection_address"></a>

## Function `create_collection_address`

Generates the collections address based upon the creators address and the collection's name


<pre><code>public fun create_collection_address(creator: &amp;address, name: &amp;string::String): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_address(creator: &amp;address, name: &amp;String): address &#123;
    object::create_object_address(creator, create_collection_seed(name))
&#125;
</code></pre>



</details>

<a id="0x4_collection_create_collection_seed"></a>

## Function `create_collection_seed`

Named objects are derived from a seed, the collection's seed is its name.


<pre><code>public fun create_collection_seed(name: &amp;string::String): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_seed(name: &amp;String): vector&lt;u8&gt; &#123;
    assert!(string::length(name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));
    &#42;string::bytes(name)
&#125;
</code></pre>



</details>

<a id="0x4_collection_increment_supply"></a>

## Function `increment_supply`

Called by token on mint to increment supply if there's an appropriate Supply struct.
TODO[agg_v2](cleanup): remove in a future release. We need to have both functions, as
increment_concurrent_supply cannot be used until AGGREGATOR_API_V2 is enabled.


<pre><code>public(friend) fun increment_supply(collection: &amp;object::Object&lt;collection::Collection&gt;, token: address): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun increment_supply(
    collection: &amp;Object&lt;Collection&gt;,
    token: address,
): Option&lt;u64&gt; acquires FixedSupply, UnlimitedSupply &#123;
    let collection_addr &#61; object::object_address(collection);
    if (exists&lt;FixedSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_addr);
        supply.current_supply &#61; supply.current_supply &#43; 1;
        supply.total_minted &#61; supply.total_minted &#43; 1;
        assert!(
            supply.current_supply &lt;&#61; supply.max_supply,
            error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),
        );

        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Mint &#123;
                    collection: collection_addr,
                    index: aggregator_v2::create_snapshot(supply.total_minted),
                    token,
                &#125;,
            );
        &#125;;
        event::emit_event(&amp;mut supply.mint_events,
            MintEvent &#123;
                index: supply.total_minted,
                token,
            &#125;,
        );
        option::some(supply.total_minted)
    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_addr);
        supply.current_supply &#61; supply.current_supply &#43; 1;
        supply.total_minted &#61; supply.total_minted &#43; 1;
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Mint &#123;
                    collection: collection_addr,
                    index: aggregator_v2::create_snapshot(supply.total_minted),
                    token,
                &#125;,
            );
        &#125;;
        event::emit_event(
            &amp;mut supply.mint_events,
            MintEvent &#123;
                index: supply.total_minted,
                token,
            &#125;,
        );
        option::some(supply.total_minted)
    &#125; else if (exists&lt;ConcurrentSupply&gt;(collection_addr)) &#123;
        abort error::invalid_argument(ECONCURRENT_NOT_ENABLED)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_increment_concurrent_supply"></a>

## Function `increment_concurrent_supply`

Called by token on mint to increment supply if there's an appropriate Supply struct.


<pre><code>public(friend) fun increment_concurrent_supply(collection: &amp;object::Object&lt;collection::Collection&gt;, token: address): option::Option&lt;aggregator_v2::AggregatorSnapshot&lt;u64&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun increment_concurrent_supply(
    collection: &amp;Object&lt;Collection&gt;,
    token: address,
): Option&lt;AggregatorSnapshot&lt;u64&gt;&gt; acquires FixedSupply, UnlimitedSupply, ConcurrentSupply &#123;
    let collection_addr &#61; object::object_address(collection);
    if (exists&lt;ConcurrentSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_addr);
        assert!(
            aggregator_v2::try_add(&amp;mut supply.current_supply, 1),
            error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),
        );
        aggregator_v2::add(&amp;mut supply.total_minted, 1);
        event::emit(
            Mint &#123;
                collection: collection_addr,
                index: aggregator_v2::snapshot(&amp;supply.total_minted),
                token,
            &#125;,
        );
        option::some(aggregator_v2::snapshot(&amp;supply.total_minted))
    &#125; else if (exists&lt;FixedSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_addr);
        supply.current_supply &#61; supply.current_supply &#43; 1;
        supply.total_minted &#61; supply.total_minted &#43; 1;
        assert!(
            supply.current_supply &lt;&#61; supply.max_supply,
            error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),
        );
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Mint &#123;
                    collection: collection_addr,
                    index: aggregator_v2::create_snapshot(supply.total_minted),
                    token,
                &#125;,
            );
        &#125;;
        event::emit_event(&amp;mut supply.mint_events,
            MintEvent &#123;
                index: supply.total_minted,
                token,
            &#125;,
        );
        option::some(aggregator_v2::create_snapshot&lt;u64&gt;(supply.total_minted))
    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_addr);
        supply.current_supply &#61; supply.current_supply &#43; 1;
        supply.total_minted &#61; supply.total_minted &#43; 1;
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Mint &#123;
                    collection: collection_addr,
                    index: aggregator_v2::create_snapshot(supply.total_minted),
                    token,
                &#125;,
            );
        &#125;;
        event::emit_event(
            &amp;mut supply.mint_events,
            MintEvent &#123;
                index: supply.total_minted,
                token,
            &#125;,
        );
        option::some(aggregator_v2::create_snapshot&lt;u64&gt;(supply.total_minted))
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_decrement_supply"></a>

## Function `decrement_supply`

Called by token on burn to decrement supply if there's an appropriate Supply struct.


<pre><code>public(friend) fun decrement_supply(collection: &amp;object::Object&lt;collection::Collection&gt;, token: address, index: option::Option&lt;u64&gt;, previous_owner: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun decrement_supply(
    collection: &amp;Object&lt;Collection&gt;,
    token: address,
    index: Option&lt;u64&gt;,
    previous_owner: address,
) acquires FixedSupply, UnlimitedSupply, ConcurrentSupply &#123;
    let collection_addr &#61; object::object_address(collection);
    if (exists&lt;ConcurrentSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_addr);
        aggregator_v2::sub(&amp;mut supply.current_supply, 1);

        event::emit(
            Burn &#123;
                collection: collection_addr,
                index: &#42;option::borrow(&amp;index),
                token,
                previous_owner,
            &#125;,
        );
    &#125; else if (exists&lt;FixedSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_addr);
        supply.current_supply &#61; supply.current_supply &#45; 1;
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Burn &#123;
                    collection: collection_addr,
                    index: &#42;option::borrow(&amp;index),
                    token,
                    previous_owner,
                &#125;,
            );
        &#125;;
        event::emit_event(
            &amp;mut supply.burn_events,
            BurnEvent &#123;
                index: &#42;option::borrow(&amp;index),
                token,
            &#125;,
        );
    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_addr)) &#123;
        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_addr);
        supply.current_supply &#61; supply.current_supply &#45; 1;
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(
                Burn &#123;
                    collection: collection_addr,
                    index: &#42;option::borrow(&amp;index),
                    token,
                    previous_owner,
                &#125;,
            );
        &#125;;
        event::emit_event(
            &amp;mut supply.burn_events,
            BurnEvent &#123;
                index: &#42;option::borrow(&amp;index),
                token,
            &#125;,
        );
    &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code>public fun generate_mutator_ref(ref: &amp;object::ConstructorRef): collection::MutatorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mutator_ref(ref: &amp;ConstructorRef): MutatorRef &#123;
    let object &#61; object::object_from_constructor_ref&lt;Collection&gt;(ref);
    MutatorRef &#123; self: object::object_address(&amp;object) &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code>public fun upgrade_to_concurrent(ref: &amp;object::ExtendRef)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_to_concurrent(
    ref: &amp;ExtendRef,
) acquires FixedSupply, UnlimitedSupply &#123;
    let metadata_object_address &#61; object::address_from_extend_ref(ref);
    let metadata_object_signer &#61; object::generate_signer_for_extending(ref);
    assert!(features::concurrent_token_v2_enabled(), error::invalid_argument(ECONCURRENT_NOT_ENABLED));

    let (supply, current_supply, total_minted, burn_events, mint_events) &#61; if (exists&lt;FixedSupply&gt;(
        metadata_object_address
    )) &#123;
        let FixedSupply &#123;
            current_supply,
            max_supply,
            total_minted,
            burn_events,
            mint_events,
        &#125; &#61; move_from&lt;FixedSupply&gt;(metadata_object_address);

        let supply &#61; ConcurrentSupply &#123;
            current_supply: aggregator_v2::create_aggregator(max_supply),
            total_minted: aggregator_v2::create_unbounded_aggregator(),
        &#125;;
        (supply, current_supply, total_minted, burn_events, mint_events)
    &#125; else if (exists&lt;UnlimitedSupply&gt;(metadata_object_address)) &#123;
        let UnlimitedSupply &#123;
            current_supply,
            total_minted,
            burn_events,
            mint_events,
        &#125; &#61; move_from&lt;UnlimitedSupply&gt;(metadata_object_address);

        let supply &#61; ConcurrentSupply &#123;
            current_supply: aggregator_v2::create_unbounded_aggregator(),
            total_minted: aggregator_v2::create_unbounded_aggregator(),
        &#125;;
        (supply, current_supply, total_minted, burn_events, mint_events)
    &#125; else &#123;
        // untracked collection is already concurrent, and other variants too.
        abort error::invalid_argument(EALREADY_CONCURRENT)
    &#125;;

    // update current state:
    aggregator_v2::add(&amp;mut supply.current_supply, current_supply);
    aggregator_v2::add(&amp;mut supply.total_minted, total_minted);
    move_to(&amp;metadata_object_signer, supply);

    event::destroy_handle(burn_events);
    event::destroy_handle(mint_events);
&#125;
</code></pre>



</details>

<a id="0x4_collection_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code>fun check_collection_exists(addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun check_collection_exists(addr: address) &#123;
    assert!(
        exists&lt;Collection&gt;(addr),
        error::not_found(ECOLLECTION_DOES_NOT_EXIST),
    );
&#125;
</code></pre>



</details>

<a id="0x4_collection_borrow"></a>

## Function `borrow`



<pre><code>fun borrow&lt;T: key&gt;(collection: &amp;object::Object&lt;T&gt;): &amp;collection::Collection
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow&lt;T: key&gt;(collection: &amp;Object&lt;T&gt;): &amp;Collection &#123;
    let collection_address &#61; object::object_address(collection);
    check_collection_exists(collection_address);
    borrow_global&lt;Collection&gt;(collection_address)
&#125;
</code></pre>



</details>

<a id="0x4_collection_count"></a>

## Function `count`

Provides the count of the current selection if supply tracking is used

Note: Calling this method from transaction that also mints/burns, prevents
it from being parallelized.


<pre><code>&#35;[view]
public fun count&lt;T: key&gt;(collection: object::Object&lt;T&gt;): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun count&lt;T: key&gt;(
    collection: Object&lt;T&gt;
): Option&lt;u64&gt; acquires FixedSupply, UnlimitedSupply, ConcurrentSupply &#123;
    let collection_address &#61; object::object_address(&amp;collection);
    check_collection_exists(collection_address);

    if (exists&lt;ConcurrentSupply&gt;(collection_address)) &#123;
        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_address);
        option::some(aggregator_v2::read(&amp;supply.current_supply))
    &#125; else if (exists&lt;FixedSupply&gt;(collection_address)) &#123;
        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_address);
        option::some(supply.current_supply)
    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_address)) &#123;
        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_address);
        option::some(supply.current_supply)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x4_collection_creator"></a>

## Function `creator`



<pre><code>&#35;[view]
public fun creator&lt;T: key&gt;(collection: object::Object&lt;T&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun creator&lt;T: key&gt;(collection: Object&lt;T&gt;): address acquires Collection &#123;
    borrow(&amp;collection).creator
&#125;
</code></pre>



</details>

<a id="0x4_collection_description"></a>

## Function `description`



<pre><code>&#35;[view]
public fun description&lt;T: key&gt;(collection: object::Object&lt;T&gt;): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun description&lt;T: key&gt;(collection: Object&lt;T&gt;): String acquires Collection &#123;
    borrow(&amp;collection).description
&#125;
</code></pre>



</details>

<a id="0x4_collection_name"></a>

## Function `name`



<pre><code>&#35;[view]
public fun name&lt;T: key&gt;(collection: object::Object&lt;T&gt;): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;T: key&gt;(collection: Object&lt;T&gt;): String acquires Collection &#123;
    borrow(&amp;collection).name
&#125;
</code></pre>



</details>

<a id="0x4_collection_uri"></a>

## Function `uri`



<pre><code>&#35;[view]
public fun uri&lt;T: key&gt;(collection: object::Object&lt;T&gt;): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun uri&lt;T: key&gt;(collection: Object&lt;T&gt;): String acquires Collection &#123;
    borrow(&amp;collection).uri
&#125;
</code></pre>



</details>

<a id="0x4_collection_borrow_mut"></a>

## Function `borrow_mut`



<pre><code>fun borrow_mut(mutator_ref: &amp;collection::MutatorRef): &amp;mut collection::Collection
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut(mutator_ref: &amp;MutatorRef): &amp;mut Collection &#123;
    check_collection_exists(mutator_ref.self);
    borrow_global_mut&lt;Collection&gt;(mutator_ref.self)
&#125;
</code></pre>



</details>

<a id="0x4_collection_set_name"></a>

## Function `set_name`

Callers of this function must be aware that changing the name will change the calculated
collection's address when calling <code>create_collection_address</code>.
Once the collection has been created, the collection address should be saved for reference and
<code>create_collection_address</code> should not be used to derive the collection's address.

After changing the collection's name, to create tokens - only call functions that accept the collection object as an argument.


<pre><code>public fun set_name(mutator_ref: &amp;collection::MutatorRef, name: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_name(mutator_ref: &amp;MutatorRef, name: String) acquires Collection &#123;
    assert!(string::length(&amp;name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));
    let collection &#61; borrow_mut(mutator_ref);
    event::emit(Mutation &#123;
        mutated_field_name: string::utf8(b&quot;name&quot;) ,
        collection: object::address_to_object(mutator_ref.self),
        old_value: collection.name,
        new_value: name,
    &#125;);
    collection.name &#61; name;
&#125;
</code></pre>



</details>

<a id="0x4_collection_set_description"></a>

## Function `set_description`



<pre><code>public fun set_description(mutator_ref: &amp;collection::MutatorRef, description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_description(mutator_ref: &amp;MutatorRef, description: String) acquires Collection &#123;
    assert!(string::length(&amp;description) &lt;&#61; MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));
    let collection &#61; borrow_mut(mutator_ref);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(Mutation &#123;
            mutated_field_name: string::utf8(b&quot;description&quot;),
            collection: object::address_to_object(mutator_ref.self),
            old_value: collection.description,
            new_value: description,
        &#125;);
    &#125;;
    collection.description &#61; description;
    event::emit_event(
        &amp;mut collection.mutation_events,
        MutationEvent &#123; mutated_field_name: string::utf8(b&quot;description&quot;) &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x4_collection_set_uri"></a>

## Function `set_uri`



<pre><code>public fun set_uri(mutator_ref: &amp;collection::MutatorRef, uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_uri(mutator_ref: &amp;MutatorRef, uri: String) acquires Collection &#123;
    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
    let collection &#61; borrow_mut(mutator_ref);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(Mutation &#123;
            mutated_field_name: string::utf8(b&quot;uri&quot;),
            collection: object::address_to_object(mutator_ref.self),
            old_value: collection.uri,
            new_value: uri,
        &#125;);
    &#125;;
    collection.uri &#61; uri;
    event::emit_event(
        &amp;mut collection.mutation_events,
        MutationEvent &#123; mutated_field_name: string::utf8(b&quot;uri&quot;) &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x4_collection_set_max_supply"></a>

## Function `set_max_supply`



<pre><code>public fun set_max_supply(mutator_ref: &amp;collection::MutatorRef, max_supply: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_max_supply(mutator_ref: &amp;MutatorRef, max_supply: u64) acquires ConcurrentSupply, FixedSupply &#123;
    let collection &#61; object::address_to_object&lt;Collection&gt;(mutator_ref.self);
    let collection_address &#61; object::object_address(&amp;collection);
    let old_max_supply;

    if (exists&lt;ConcurrentSupply&gt;(collection_address)) &#123;
        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_address);
        let current_supply &#61; aggregator_v2::read(&amp;supply.current_supply);
        assert!(
            max_supply &gt;&#61; current_supply,
            error::out_of_range(EINVALID_MAX_SUPPLY),
        );
        old_max_supply &#61; aggregator_v2::max_value(&amp;supply.current_supply);
        supply.current_supply &#61; aggregator_v2::create_aggregator(max_supply);
        aggregator_v2::add(&amp;mut supply.current_supply, current_supply);
    &#125; else if (exists&lt;FixedSupply&gt;(collection_address)) &#123;
        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_address);
        assert!(
            max_supply &gt;&#61; supply.current_supply,
            error::out_of_range(EINVALID_MAX_SUPPLY),
        );
        old_max_supply &#61; supply.max_supply;
        supply.max_supply &#61; max_supply;
    &#125; else &#123;
        abort error::invalid_argument(ENO_MAX_SUPPLY_IN_COLLECTION)
    &#125;;

    event::emit(SetMaxSupply &#123; collection, old_max_supply, new_max_supply: max_supply &#125;);
&#125;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
