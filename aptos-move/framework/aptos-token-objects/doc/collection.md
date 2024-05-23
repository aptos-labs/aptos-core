
<a id="0x4_collection"></a>

# Module `0x4::collection`

This defines an object&#45;based Collection. A collection acts as a set organizer for a group of<br/> tokens. This includes aspects such as a general description, project URI, name, and may contain<br/> other useful generalizations across this set of tokens.<br/><br/> Being built upon objects enables collections to be relatively flexible. As core primitives it<br/> supports:<br/> &#42; Common fields: name, uri, description, creator<br/> &#42; MutatorRef leaving mutability configuration to a higher level component<br/> &#42; Addressed by a global identifier of creator&apos;s address and collection name, thus collections<br/>   cannot be deleted as a restriction of the object model.<br/> &#42; Optional support for collection&#45;wide royalties<br/> &#42; Optional support for tracking of supply with events on mint or burn<br/><br/> TODO:<br/> &#42; Consider supporting changing the name of the collection with the MutatorRef. This would<br/>   require adding the field original_name.<br/> &#42; Consider supporting changing the aspects of supply with the MutatorRef.<br/> &#42; Add aggregator support when added to framework


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


<pre><code>use 0x1::aggregator_v2;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x4::royalty;<br/></code></pre>



<a id="0x4_collection_Collection"></a>

## Resource `Collection`

Represents the common fields for a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Collection has key<br/></code></pre>



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
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off&#45;chain<br/> storage; the URL length will likely need a maximum any suggestions?
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


<pre><code>struct MutatorRef has drop, store<br/></code></pre>



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

Contains the mutated fields name. This makes the life of indexers easier, so that they can<br/> directly understand the behavior in a writeset.


<pre><code>struct MutationEvent has drop, store<br/></code></pre>



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

Contains the mutated fields name. This makes the life of indexers easier, so that they can<br/> directly understand the behavior in a writeset.


<pre><code>&#35;[event]<br/>struct Mutation has drop, store<br/></code></pre>



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

Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.<br/> and adding events and supply tracking to a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct FixedSupply has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: u64</code>
</dt>
<dd>
 Total minted &#45; total burned
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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct UnlimitedSupply has key<br/></code></pre>



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

Supply tracker, useful for tracking amount of issued tokens.<br/> If max_value is not set to U64_MAX, this ensures that a limited number of tokens are minted.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct ConcurrentSupply has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: aggregator_v2::Aggregator&lt;u64&gt;</code>
</dt>
<dd>
 Total minted &#45; total burned
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



<pre><code>struct BurnEvent has drop, store<br/></code></pre>



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



<pre><code>struct MintEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct Burn has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct Mint has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>&#35;[deprecated]<br/>struct ConcurrentBurnEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>&#35;[deprecated]<br/>struct ConcurrentMintEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct SetMaxSupply has drop, store<br/></code></pre>



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



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x4_collection_EURI_TOO_LONG"></a>

The URI is over the maximum length


<pre><code>const EURI_TOO_LONG: u64 &#61; 4;<br/></code></pre>



<a id="0x4_collection_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;<br/></code></pre>



<a id="0x4_collection_EALREADY_CONCURRENT"></a>

Tried upgrading collection to concurrent, but collection is already concurrent


<pre><code>const EALREADY_CONCURRENT: u64 &#61; 8;<br/></code></pre>



<a id="0x4_collection_ECOLLECTION_DOES_NOT_EXIST"></a>

The collection does not exist


<pre><code>const ECOLLECTION_DOES_NOT_EXIST: u64 &#61; 1;<br/></code></pre>



<a id="0x4_collection_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is over the maximum length


<pre><code>const ECOLLECTION_NAME_TOO_LONG: u64 &#61; 3;<br/></code></pre>



<a id="0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED"></a>

The collection has reached its supply and no more tokens can be minted, unless some are burned


<pre><code>const ECOLLECTION_SUPPLY_EXCEEDED: u64 &#61; 2;<br/></code></pre>



<a id="0x4_collection_ECONCURRENT_NOT_ENABLED"></a>

Concurrent feature flag is not yet enabled, so the function cannot be performed


<pre><code>const ECONCURRENT_NOT_ENABLED: u64 &#61; 7;<br/></code></pre>



<a id="0x4_collection_EDESCRIPTION_TOO_LONG"></a>

The description is over the maximum length


<pre><code>const EDESCRIPTION_TOO_LONG: u64 &#61; 5;<br/></code></pre>



<a id="0x4_collection_EINVALID_MAX_SUPPLY"></a>

The new max supply cannot be less than the current supply


<pre><code>const EINVALID_MAX_SUPPLY: u64 &#61; 9;<br/></code></pre>



<a id="0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO"></a>

The max supply must be positive


<pre><code>const EMAX_SUPPLY_CANNOT_BE_ZERO: u64 &#61; 6;<br/></code></pre>



<a id="0x4_collection_ENO_MAX_SUPPLY_IN_COLLECTION"></a>

The collection does not have a max supply


<pre><code>const ENO_MAX_SUPPLY_IN_COLLECTION: u64 &#61; 10;<br/></code></pre>



<a id="0x4_collection_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code>const MAX_COLLECTION_NAME_LENGTH: u64 &#61; 128;<br/></code></pre>



<a id="0x4_collection_MAX_DESCRIPTION_LENGTH"></a>



<pre><code>const MAX_DESCRIPTION_LENGTH: u64 &#61; 2048;<br/></code></pre>



<a id="0x4_collection_create_fixed_collection"></a>

## Function `create_fixed_collection`

Creates a fixed&#45;sized collection, or a collection that supports a fixed amount of tokens.<br/> This is useful to create a guaranteed, limited supply on&#45;chain digital asset. For example,<br/> a collection 1111 vicious vipers. Note, creating restrictions such as upward limits results<br/> in data structures that prevent Aptos from parallelizing mints of this collection type.<br/> Beyond that, it adds supply tracking with events.


<pre><code>public fun create_fixed_collection(creator: &amp;signer, description: string::String, max_supply: u64, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_fixed_collection(<br/>    creator: &amp;signer,<br/>    description: String,<br/>    max_supply: u64,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    assert!(max_supply !&#61; 0, error::invalid_argument(EMAX_SUPPLY_CANNOT_BE_ZERO));<br/>    let collection_seed &#61; create_collection_seed(&amp;name);<br/>    let constructor_ref &#61; object::create_named_object(creator, collection_seed);<br/>    let object_signer &#61; object::generate_signer(&amp;constructor_ref);<br/>    if (features::concurrent_token_v2_enabled()) &#123;<br/>        let supply &#61; ConcurrentSupply &#123;<br/>            current_supply: aggregator_v2::create_aggregator(max_supply),<br/>            total_minted: aggregator_v2::create_unbounded_aggregator(),<br/>        &#125;;<br/><br/>        create_collection_internal(<br/>            creator,<br/>            constructor_ref,<br/>            description,<br/>            name,<br/>            royalty,<br/>            uri,<br/>            option::some(supply),<br/>        )<br/>    &#125; else &#123;<br/>        let supply &#61; FixedSupply &#123;<br/>            current_supply: 0,<br/>            max_supply,<br/>            total_minted: 0,<br/>            burn_events: object::new_event_handle(&amp;object_signer),<br/>            mint_events: object::new_event_handle(&amp;object_signer),<br/>        &#125;;<br/><br/>        create_collection_internal(<br/>            creator,<br/>            constructor_ref,<br/>            description,<br/>            name,<br/>            royalty,<br/>            uri,<br/>            option::some(supply),<br/>        )<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_create_unlimited_collection"></a>

## Function `create_unlimited_collection`

Creates an unlimited collection. This has support for supply tracking but does not limit<br/> the supply of tokens.


<pre><code>public fun create_unlimited_collection(creator: &amp;signer, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_unlimited_collection(<br/>    creator: &amp;signer,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let collection_seed &#61; create_collection_seed(&amp;name);<br/>    let constructor_ref &#61; object::create_named_object(creator, collection_seed);<br/>    let object_signer &#61; object::generate_signer(&amp;constructor_ref);<br/><br/>    if (features::concurrent_token_v2_enabled()) &#123;<br/>        let supply &#61; ConcurrentSupply &#123;<br/>            current_supply: aggregator_v2::create_unbounded_aggregator(),<br/>            total_minted: aggregator_v2::create_unbounded_aggregator(),<br/>        &#125;;<br/><br/>        create_collection_internal(<br/>            creator,<br/>            constructor_ref,<br/>            description,<br/>            name,<br/>            royalty,<br/>            uri,<br/>            option::some(supply),<br/>        )<br/>    &#125; else &#123;<br/>        let supply &#61; UnlimitedSupply &#123;<br/>            current_supply: 0,<br/>            total_minted: 0,<br/>            burn_events: object::new_event_handle(&amp;object_signer),<br/>            mint_events: object::new_event_handle(&amp;object_signer),<br/>        &#125;;<br/><br/>        create_collection_internal(<br/>            creator,<br/>            constructor_ref,<br/>            description,<br/>            name,<br/>            royalty,<br/>            uri,<br/>            option::some(supply),<br/>        )<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_create_untracked_collection"></a>

## Function `create_untracked_collection`

Creates an untracked collection, or a collection that supports an arbitrary amount of<br/> tokens. This is useful for mass airdrops that fully leverage Aptos parallelization.<br/> TODO: Hide this until we bring back meaningful way to enforce burns


<pre><code>fun create_untracked_collection(creator: &amp;signer, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_untracked_collection(<br/>    creator: &amp;signer,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let collection_seed &#61; create_collection_seed(&amp;name);<br/>    let constructor_ref &#61; object::create_named_object(creator, collection_seed);<br/><br/>    create_collection_internal&lt;FixedSupply&gt;(<br/>        creator,<br/>        constructor_ref,<br/>        description,<br/>        name,<br/>        royalty,<br/>        uri,<br/>        option::none(),<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_create_collection_internal"></a>

## Function `create_collection_internal`



<pre><code>fun create_collection_internal&lt;Supply: key&gt;(creator: &amp;signer, constructor_ref: object::ConstructorRef, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String, supply: option::Option&lt;Supply&gt;): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun create_collection_internal&lt;Supply: key&gt;(<br/>    creator: &amp;signer,<br/>    constructor_ref: ConstructorRef,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>    supply: Option&lt;Supply&gt;,<br/>): ConstructorRef &#123;<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));<br/>    assert!(string::length(&amp;description) &lt;&#61; MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));<br/><br/>    let object_signer &#61; object::generate_signer(&amp;constructor_ref);<br/><br/>    let collection &#61; Collection &#123;<br/>        creator: signer::address_of(creator),<br/>        description,<br/>        name,<br/>        uri,<br/>        mutation_events: object::new_event_handle(&amp;object_signer),<br/>    &#125;;<br/>    move_to(&amp;object_signer, collection);<br/><br/>    if (option::is_some(&amp;supply)) &#123;<br/>        move_to(&amp;object_signer, option::destroy_some(supply))<br/>    &#125; else &#123;<br/>        option::destroy_none(supply)<br/>    &#125;;<br/><br/>    if (option::is_some(&amp;royalty)) &#123;<br/>        royalty::init(&amp;constructor_ref, option::extract(&amp;mut royalty))<br/>    &#125;;<br/><br/>    let transfer_ref &#61; object::generate_transfer_ref(&amp;constructor_ref);<br/>    object::disable_ungated_transfer(&amp;transfer_ref);<br/><br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_create_collection_address"></a>

## Function `create_collection_address`

Generates the collections address based upon the creators address and the collection&apos;s name


<pre><code>public fun create_collection_address(creator: &amp;address, name: &amp;string::String): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_address(creator: &amp;address, name: &amp;String): address &#123;<br/>    object::create_object_address(creator, create_collection_seed(name))<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_create_collection_seed"></a>

## Function `create_collection_seed`

Named objects are derived from a seed, the collection&apos;s seed is its name.


<pre><code>public fun create_collection_seed(name: &amp;string::String): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_seed(name: &amp;String): vector&lt;u8&gt; &#123;<br/>    assert!(string::length(name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));<br/>    &#42;string::bytes(name)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_increment_supply"></a>

## Function `increment_supply`

Called by token on mint to increment supply if there&apos;s an appropriate Supply struct.<br/> TODO[agg_v2](cleanup): remove in a future release. We need to have both functions, as<br/> increment_concurrent_supply cannot be used until AGGREGATOR_API_V2 is enabled.


<pre><code>public(friend) fun increment_supply(collection: &amp;object::Object&lt;collection::Collection&gt;, token: address): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun increment_supply(<br/>    collection: &amp;Object&lt;Collection&gt;,<br/>    token: address,<br/>): Option&lt;u64&gt; acquires FixedSupply, UnlimitedSupply &#123;<br/>    let collection_addr &#61; object::object_address(collection);<br/>    if (exists&lt;FixedSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_addr);<br/>        supply.current_supply &#61; supply.current_supply &#43; 1;<br/>        supply.total_minted &#61; supply.total_minted &#43; 1;<br/>        assert!(<br/>            supply.current_supply &lt;&#61; supply.max_supply,<br/>            error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),<br/>        );<br/><br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Mint &#123;<br/>                    collection: collection_addr,<br/>                    index: aggregator_v2::create_snapshot(supply.total_minted),<br/>                    token,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(&amp;mut supply.mint_events,<br/>            MintEvent &#123;<br/>                index: supply.total_minted,<br/>                token,<br/>            &#125;,<br/>        );<br/>        option::some(supply.total_minted)<br/>    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_addr);<br/>        supply.current_supply &#61; supply.current_supply &#43; 1;<br/>        supply.total_minted &#61; supply.total_minted &#43; 1;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Mint &#123;<br/>                    collection: collection_addr,<br/>                    index: aggregator_v2::create_snapshot(supply.total_minted),<br/>                    token,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(<br/>            &amp;mut supply.mint_events,<br/>            MintEvent &#123;<br/>                index: supply.total_minted,<br/>                token,<br/>            &#125;,<br/>        );<br/>        option::some(supply.total_minted)<br/>    &#125; else if (exists&lt;ConcurrentSupply&gt;(collection_addr)) &#123;<br/>        abort error::invalid_argument(ECONCURRENT_NOT_ENABLED)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_increment_concurrent_supply"></a>

## Function `increment_concurrent_supply`

Called by token on mint to increment supply if there&apos;s an appropriate Supply struct.


<pre><code>public(friend) fun increment_concurrent_supply(collection: &amp;object::Object&lt;collection::Collection&gt;, token: address): option::Option&lt;aggregator_v2::AggregatorSnapshot&lt;u64&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun increment_concurrent_supply(<br/>    collection: &amp;Object&lt;Collection&gt;,<br/>    token: address,<br/>): Option&lt;AggregatorSnapshot&lt;u64&gt;&gt; acquires FixedSupply, UnlimitedSupply, ConcurrentSupply &#123;<br/>    let collection_addr &#61; object::object_address(collection);<br/>    if (exists&lt;ConcurrentSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_addr);<br/>        assert!(<br/>            aggregator_v2::try_add(&amp;mut supply.current_supply, 1),<br/>            error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),<br/>        );<br/>        aggregator_v2::add(&amp;mut supply.total_minted, 1);<br/>        event::emit(<br/>            Mint &#123;<br/>                collection: collection_addr,<br/>                index: aggregator_v2::snapshot(&amp;supply.total_minted),<br/>                token,<br/>            &#125;,<br/>        );<br/>        option::some(aggregator_v2::snapshot(&amp;supply.total_minted))<br/>    &#125; else if (exists&lt;FixedSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_addr);<br/>        supply.current_supply &#61; supply.current_supply &#43; 1;<br/>        supply.total_minted &#61; supply.total_minted &#43; 1;<br/>        assert!(<br/>            supply.current_supply &lt;&#61; supply.max_supply,<br/>            error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),<br/>        );<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Mint &#123;<br/>                    collection: collection_addr,<br/>                    index: aggregator_v2::create_snapshot(supply.total_minted),<br/>                    token,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(&amp;mut supply.mint_events,<br/>            MintEvent &#123;<br/>                index: supply.total_minted,<br/>                token,<br/>            &#125;,<br/>        );<br/>        option::some(aggregator_v2::create_snapshot&lt;u64&gt;(supply.total_minted))<br/>    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_addr);<br/>        supply.current_supply &#61; supply.current_supply &#43; 1;<br/>        supply.total_minted &#61; supply.total_minted &#43; 1;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Mint &#123;<br/>                    collection: collection_addr,<br/>                    index: aggregator_v2::create_snapshot(supply.total_minted),<br/>                    token,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(<br/>            &amp;mut supply.mint_events,<br/>            MintEvent &#123;<br/>                index: supply.total_minted,<br/>                token,<br/>            &#125;,<br/>        );<br/>        option::some(aggregator_v2::create_snapshot&lt;u64&gt;(supply.total_minted))<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_decrement_supply"></a>

## Function `decrement_supply`

Called by token on burn to decrement supply if there&apos;s an appropriate Supply struct.


<pre><code>public(friend) fun decrement_supply(collection: &amp;object::Object&lt;collection::Collection&gt;, token: address, index: option::Option&lt;u64&gt;, previous_owner: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun decrement_supply(<br/>    collection: &amp;Object&lt;Collection&gt;,<br/>    token: address,<br/>    index: Option&lt;u64&gt;,<br/>    previous_owner: address,<br/>) acquires FixedSupply, UnlimitedSupply, ConcurrentSupply &#123;<br/>    let collection_addr &#61; object::object_address(collection);<br/>    if (exists&lt;ConcurrentSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_addr);<br/>        aggregator_v2::sub(&amp;mut supply.current_supply, 1);<br/><br/>        event::emit(<br/>            Burn &#123;<br/>                collection: collection_addr,<br/>                index: &#42;option::borrow(&amp;index),<br/>                token,<br/>                previous_owner,<br/>            &#125;,<br/>        );<br/>    &#125; else if (exists&lt;FixedSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_addr);<br/>        supply.current_supply &#61; supply.current_supply &#45; 1;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Burn &#123;<br/>                    collection: collection_addr,<br/>                    index: &#42;option::borrow(&amp;index),<br/>                    token,<br/>                    previous_owner,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(<br/>            &amp;mut supply.burn_events,<br/>            BurnEvent &#123;<br/>                index: &#42;option::borrow(&amp;index),<br/>                token,<br/>            &#125;,<br/>        );<br/>    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_addr)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_addr);<br/>        supply.current_supply &#61; supply.current_supply &#45; 1;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(<br/>                Burn &#123;<br/>                    collection: collection_addr,<br/>                    index: &#42;option::borrow(&amp;index),<br/>                    token,<br/>                    previous_owner,<br/>                &#125;,<br/>            );<br/>        &#125;;<br/>        event::emit_event(<br/>            &amp;mut supply.burn_events,<br/>            BurnEvent &#123;<br/>                index: &#42;option::borrow(&amp;index),<br/>                token,<br/>            &#125;,<br/>        );<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code>public fun generate_mutator_ref(ref: &amp;object::ConstructorRef): collection::MutatorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mutator_ref(ref: &amp;ConstructorRef): MutatorRef &#123;<br/>    let object &#61; object::object_from_constructor_ref&lt;Collection&gt;(ref);<br/>    MutatorRef &#123; self: object::object_address(&amp;object) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code>public fun upgrade_to_concurrent(ref: &amp;object::ExtendRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_to_concurrent(<br/>    ref: &amp;ExtendRef,<br/>) acquires FixedSupply, UnlimitedSupply &#123;<br/>    let metadata_object_address &#61; object::address_from_extend_ref(ref);<br/>    let metadata_object_signer &#61; object::generate_signer_for_extending(ref);<br/>    assert!(features::concurrent_token_v2_enabled(), error::invalid_argument(ECONCURRENT_NOT_ENABLED));<br/><br/>    let (supply, current_supply, total_minted, burn_events, mint_events) &#61; if (exists&lt;FixedSupply&gt;(<br/>        metadata_object_address<br/>    )) &#123;<br/>        let FixedSupply &#123;<br/>            current_supply,<br/>            max_supply,<br/>            total_minted,<br/>            burn_events,<br/>            mint_events,<br/>        &#125; &#61; move_from&lt;FixedSupply&gt;(metadata_object_address);<br/><br/>        let supply &#61; ConcurrentSupply &#123;<br/>            current_supply: aggregator_v2::create_aggregator(max_supply),<br/>            total_minted: aggregator_v2::create_unbounded_aggregator(),<br/>        &#125;;<br/>        (supply, current_supply, total_minted, burn_events, mint_events)<br/>    &#125; else if (exists&lt;UnlimitedSupply&gt;(metadata_object_address)) &#123;<br/>        let UnlimitedSupply &#123;<br/>            current_supply,<br/>            total_minted,<br/>            burn_events,<br/>            mint_events,<br/>        &#125; &#61; move_from&lt;UnlimitedSupply&gt;(metadata_object_address);<br/><br/>        let supply &#61; ConcurrentSupply &#123;<br/>            current_supply: aggregator_v2::create_unbounded_aggregator(),<br/>            total_minted: aggregator_v2::create_unbounded_aggregator(),<br/>        &#125;;<br/>        (supply, current_supply, total_minted, burn_events, mint_events)<br/>    &#125; else &#123;<br/>        // untracked collection is already concurrent, and other variants too.<br/>        abort error::invalid_argument(EALREADY_CONCURRENT)<br/>    &#125;;<br/><br/>    // update current state:<br/>    aggregator_v2::add(&amp;mut supply.current_supply, current_supply);<br/>    aggregator_v2::add(&amp;mut supply.total_minted, total_minted);<br/>    move_to(&amp;metadata_object_signer, supply);<br/><br/>    event::destroy_handle(burn_events);<br/>    event::destroy_handle(mint_events);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code>fun check_collection_exists(addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun check_collection_exists(addr: address) &#123;<br/>    assert!(<br/>        exists&lt;Collection&gt;(addr),<br/>        error::not_found(ECOLLECTION_DOES_NOT_EXIST),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_borrow"></a>

## Function `borrow`



<pre><code>fun borrow&lt;T: key&gt;(collection: &amp;object::Object&lt;T&gt;): &amp;collection::Collection<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow&lt;T: key&gt;(collection: &amp;Object&lt;T&gt;): &amp;Collection &#123;<br/>    let collection_address &#61; object::object_address(collection);<br/>    check_collection_exists(collection_address);<br/>    borrow_global&lt;Collection&gt;(collection_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_count"></a>

## Function `count`

Provides the count of the current selection if supply tracking is used<br/><br/> Note: Calling this method from transaction that also mints/burns, prevents<br/> it from being parallelized.


<pre><code>&#35;[view]<br/>public fun count&lt;T: key&gt;(collection: object::Object&lt;T&gt;): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun count&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;<br/>): Option&lt;u64&gt; acquires FixedSupply, UnlimitedSupply, ConcurrentSupply &#123;<br/>    let collection_address &#61; object::object_address(&amp;collection);<br/>    check_collection_exists(collection_address);<br/><br/>    if (exists&lt;ConcurrentSupply&gt;(collection_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_address);<br/>        option::some(aggregator_v2::read(&amp;supply.current_supply))<br/>    &#125; else if (exists&lt;FixedSupply&gt;(collection_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_address);<br/>        option::some(supply.current_supply)<br/>    &#125; else if (exists&lt;UnlimitedSupply&gt;(collection_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;UnlimitedSupply&gt;(collection_address);<br/>        option::some(supply.current_supply)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_creator"></a>

## Function `creator`



<pre><code>&#35;[view]<br/>public fun creator&lt;T: key&gt;(collection: object::Object&lt;T&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun creator&lt;T: key&gt;(collection: Object&lt;T&gt;): address acquires Collection &#123;<br/>    borrow(&amp;collection).creator<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_description"></a>

## Function `description`



<pre><code>&#35;[view]<br/>public fun description&lt;T: key&gt;(collection: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun description&lt;T: key&gt;(collection: Object&lt;T&gt;): String acquires Collection &#123;<br/>    borrow(&amp;collection).description<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_name"></a>

## Function `name`



<pre><code>&#35;[view]<br/>public fun name&lt;T: key&gt;(collection: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;T: key&gt;(collection: Object&lt;T&gt;): String acquires Collection &#123;<br/>    borrow(&amp;collection).name<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_uri"></a>

## Function `uri`



<pre><code>&#35;[view]<br/>public fun uri&lt;T: key&gt;(collection: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun uri&lt;T: key&gt;(collection: Object&lt;T&gt;): String acquires Collection &#123;<br/>    borrow(&amp;collection).uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_borrow_mut"></a>

## Function `borrow_mut`



<pre><code>fun borrow_mut(mutator_ref: &amp;collection::MutatorRef): &amp;mut collection::Collection<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut(mutator_ref: &amp;MutatorRef): &amp;mut Collection &#123;<br/>    check_collection_exists(mutator_ref.self);<br/>    borrow_global_mut&lt;Collection&gt;(mutator_ref.self)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_set_name"></a>

## Function `set_name`

Callers of this function must be aware that changing the name will change the calculated<br/> collection&apos;s address when calling <code>create_collection_address</code>.<br/> Once the collection has been created, the collection address should be saved for reference and<br/> <code>create_collection_address</code> should not be used to derive the collection&apos;s address.<br/><br/> After changing the collection&apos;s name, to create tokens &#45; only call functions that accept the collection object as an argument.


<pre><code>public fun set_name(mutator_ref: &amp;collection::MutatorRef, name: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_name(mutator_ref: &amp;MutatorRef, name: String) acquires Collection &#123;<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));<br/>    let collection &#61; borrow_mut(mutator_ref);<br/>    event::emit(Mutation &#123;<br/>        mutated_field_name: string::utf8(b&quot;name&quot;) ,<br/>        collection: object::address_to_object(mutator_ref.self),<br/>        old_value: collection.name,<br/>        new_value: name,<br/>    &#125;);<br/>    collection.name &#61; name;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_set_description"></a>

## Function `set_description`



<pre><code>public fun set_description(mutator_ref: &amp;collection::MutatorRef, description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_description(mutator_ref: &amp;MutatorRef, description: String) acquires Collection &#123;<br/>    assert!(string::length(&amp;description) &lt;&#61; MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));<br/>    let collection &#61; borrow_mut(mutator_ref);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Mutation &#123;<br/>            mutated_field_name: string::utf8(b&quot;description&quot;),<br/>            collection: object::address_to_object(mutator_ref.self),<br/>            old_value: collection.description,<br/>            new_value: description,<br/>        &#125;);<br/>    &#125;;<br/>    collection.description &#61; description;<br/>    event::emit_event(<br/>        &amp;mut collection.mutation_events,<br/>        MutationEvent &#123; mutated_field_name: string::utf8(b&quot;description&quot;) &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_set_uri"></a>

## Function `set_uri`



<pre><code>public fun set_uri(mutator_ref: &amp;collection::MutatorRef, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_uri(mutator_ref: &amp;MutatorRef, uri: String) acquires Collection &#123;<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));<br/>    let collection &#61; borrow_mut(mutator_ref);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Mutation &#123;<br/>            mutated_field_name: string::utf8(b&quot;uri&quot;),<br/>            collection: object::address_to_object(mutator_ref.self),<br/>            old_value: collection.uri,<br/>            new_value: uri,<br/>        &#125;);<br/>    &#125;;<br/>    collection.uri &#61; uri;<br/>    event::emit_event(<br/>        &amp;mut collection.mutation_events,<br/>        MutationEvent &#123; mutated_field_name: string::utf8(b&quot;uri&quot;) &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_collection_set_max_supply"></a>

## Function `set_max_supply`



<pre><code>public fun set_max_supply(mutator_ref: &amp;collection::MutatorRef, max_supply: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_max_supply(mutator_ref: &amp;MutatorRef, max_supply: u64) acquires ConcurrentSupply, FixedSupply &#123;<br/>    let collection &#61; object::address_to_object&lt;Collection&gt;(mutator_ref.self);<br/>    let collection_address &#61; object::object_address(&amp;collection);<br/>    let old_max_supply;<br/><br/>    if (exists&lt;ConcurrentSupply&gt;(collection_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;ConcurrentSupply&gt;(collection_address);<br/>        let current_supply &#61; aggregator_v2::read(&amp;supply.current_supply);<br/>        assert!(<br/>            max_supply &gt;&#61; current_supply,<br/>            error::out_of_range(EINVALID_MAX_SUPPLY),<br/>        );<br/>        old_max_supply &#61; aggregator_v2::max_value(&amp;supply.current_supply);<br/>        supply.current_supply &#61; aggregator_v2::create_aggregator(max_supply);<br/>        aggregator_v2::add(&amp;mut supply.current_supply, current_supply);<br/>    &#125; else if (exists&lt;FixedSupply&gt;(collection_address)) &#123;<br/>        let supply &#61; borrow_global_mut&lt;FixedSupply&gt;(collection_address);<br/>        assert!(<br/>            max_supply &gt;&#61; supply.current_supply,<br/>            error::out_of_range(EINVALID_MAX_SUPPLY),<br/>        );<br/>        old_max_supply &#61; supply.max_supply;<br/>        supply.max_supply &#61; max_supply;<br/>    &#125; else &#123;<br/>        abort error::invalid_argument(ENO_MAX_SUPPLY_IN_COLLECTION)<br/>    &#125;;<br/><br/>    event::emit(SetMaxSupply &#123; collection, old_max_supply, new_max_supply: max_supply &#125;);<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
