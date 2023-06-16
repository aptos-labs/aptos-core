
<a name="0x4_collection"></a>

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
-  [Resource `FixedSupply`](#0x4_collection_FixedSupply)
-  [Resource `UnlimitedSupply`](#0x4_collection_UnlimitedSupply)
-  [Struct `BurnEvent`](#0x4_collection_BurnEvent)
-  [Struct `MintEvent`](#0x4_collection_MintEvent)
-  [Constants](#@Constants_0)
-  [Function `create_fixed_collection`](#0x4_collection_create_fixed_collection)
-  [Function `create_unlimited_collection`](#0x4_collection_create_unlimited_collection)
-  [Function `create_untracked_collection`](#0x4_collection_create_untracked_collection)
-  [Function `create_collection_internal`](#0x4_collection_create_collection_internal)
-  [Function `create_collection_address`](#0x4_collection_create_collection_address)
-  [Function `create_collection_seed`](#0x4_collection_create_collection_seed)
-  [Function `increment_supply`](#0x4_collection_increment_supply)
-  [Function `decrement_supply`](#0x4_collection_decrement_supply)
-  [Function `generate_mutator_ref`](#0x4_collection_generate_mutator_ref)
-  [Function `check_collection_exists`](#0x4_collection_check_collection_exists)
-  [Function `borrow`](#0x4_collection_borrow)
-  [Function `count`](#0x4_collection_count)
-  [Function `creator`](#0x4_collection_creator)
-  [Function `description`](#0x4_collection_description)
-  [Function `name`](#0x4_collection_name)
-  [Function `uri`](#0x4_collection_uri)
-  [Function `borrow_mut`](#0x4_collection_borrow_mut)
-  [Function `set_description`](#0x4_collection_set_description)
-  [Function `set_uri`](#0x4_collection_set_uri)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;
</code></pre>



<a name="0x4_collection_Collection"></a>

## Resource `Collection`

Represents the common fields for a collection.


<pre><code>#[resource_group_member(#[group = <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="collection.md#0x4_collection_Collection">Collection</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>
 The creator of this collection.
</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 A brief description of the collection.
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 An optional categorization of similar token.
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
 storage; the URL length will likely need a maximum any suggestions?
</dd>
<dt>
<code>mutation_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="collection.md#0x4_collection_MutationEvent">collection::MutationEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon any mutation of the collection.
</dd>
</dl>


</details>

<a name="0x4_collection_MutatorRef"></a>

## Struct `MutatorRef`

This enables mutating description and URI by higher level services.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_collection_MutationEvent"></a>

## Struct `MutationEvent`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_collection_FixedSupply"></a>

## Resource `FixedSupply`

Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.
and adding events and supply tracking to a collection.


<pre><code>#[resource_group_member(#[group = <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> <b>has</b> key
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
<code>burn_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="collection.md#0x4_collection_BurnEvent">collection::BurnEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon burning a Token.
</dd>
<dt>
<code>mint_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="collection.md#0x4_collection_MintEvent">collection::MintEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon minting an Token.
</dd>
</dl>


</details>

<a name="0x4_collection_UnlimitedSupply"></a>

## Resource `UnlimitedSupply`

Unlimited supply tracker, this is useful for adding events and supply tracking to a collection.


<pre><code>#[resource_group_member(#[group = <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> <b>has</b> key
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
<code>burn_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="collection.md#0x4_collection_BurnEvent">collection::BurnEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon burning a Token.
</dd>
<dt>
<code>mint_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="collection.md#0x4_collection_MintEvent">collection::MintEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon minting an Token.
</dd>
</dl>


</details>

<a name="0x4_collection_BurnEvent"></a>

## Struct `BurnEvent`



<pre><code><b>struct</b> <a href="collection.md#0x4_collection_BurnEvent">BurnEvent</a> <b>has</b> drop, store
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
<code><a href="token.md#0x4_token">token</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_collection_MintEvent"></a>

## Struct `MintEvent`



<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MintEvent">MintEvent</a> <b>has</b> drop, store
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
<code><a href="token.md#0x4_token">token</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x4_collection_EURI_TOO_LONG"></a>

The URI is over the maximum length


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 = 4;
</code></pre>



<a name="0x4_collection_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 = 512;
</code></pre>



<a name="0x4_collection_ECOLLECTION_DOES_NOT_EXIST"></a>

The collection does not exist


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a name="0x4_collection_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is over the maximum length


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>: u64 = 3;
</code></pre>



<a name="0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED"></a>

The collection has reached its supply and no more tokens can be minted, unless some are burned


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED">ECOLLECTION_SUPPLY_EXCEEDED</a>: u64 = 2;
</code></pre>



<a name="0x4_collection_EDESCRIPTION_TOO_LONG"></a>

The description is over the maximum length


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>: u64 = 5;
</code></pre>



<a name="0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO"></a>

The max supply must be positive


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO">EMAX_SUPPLY_CANNOT_BE_ZERO</a>: u64 = 6;
</code></pre>



<a name="0x4_collection_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>: u64 = 128;
</code></pre>



<a name="0x4_collection_MAX_DESCRIPTION_LENGTH"></a>



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>: u64 = 2048;
</code></pre>



<a name="0x4_collection_create_fixed_collection"></a>

## Function `create_fixed_collection`

Creates a fixed-sized collection, or a collection that supports a fixed amount of tokens.
This is useful to create a guaranteed, limited supply on-chain digital asset. For example,
a collection 1111 vicious vipers. Note, creating restrictions such as upward limits results
in data structures that prevent Aptos from parallelizing mints of this collection type.
Beyond that, it adds supply tracking with events.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_fixed_collection">create_fixed_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_fixed_collection">create_fixed_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    max_supply: u64,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef {
    <b>assert</b>!(max_supply != 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="collection.md#0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO">EMAX_SUPPLY_CANNOT_BE_ZERO</a>));
    <b>let</b> collection_seed = <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(&name);
    <b>let</b> constructor_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, collection_seed);
    <b>let</b> object_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);

    <b>let</b> supply = <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> {
        current_supply: 0,
        max_supply,
        total_minted: 0,
        burn_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&object_signer),
        mint_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&object_signer),
    };

    <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>(
        creator,
        constructor_ref,
        description,
        name,
        <a href="royalty.md#0x4_royalty">royalty</a>,
        uri,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply),
    )
}
</code></pre>



</details>

<a name="0x4_collection_create_unlimited_collection"></a>

## Function `create_unlimited_collection`

Creates an unlimited collection. This has support for supply tracking but does not limit
the supply of tokens.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_unlimited_collection">create_unlimited_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_unlimited_collection">create_unlimited_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef {
    <b>let</b> collection_seed = <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(&name);
    <b>let</b> constructor_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, collection_seed);
    <b>let</b> object_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);

    <b>let</b> supply = <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> {
        current_supply: 0,
        total_minted: 0,
        burn_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&object_signer),
        mint_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&object_signer),
    };

    <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>(
        creator,
        constructor_ref,
        description,
        name,
        <a href="royalty.md#0x4_royalty">royalty</a>,
        uri,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply),
    )
}
</code></pre>



</details>

<a name="0x4_collection_create_untracked_collection"></a>

## Function `create_untracked_collection`

Creates an untracked collection, or a collection that supports an arbitrary amount of
tokens. This is useful for mass airdrops that fully leverage Aptos parallelization.
TODO: Hide this until we bring back meaningful way to enforce burns


<pre><code><b>fun</b> <a href="collection.md#0x4_collection_create_untracked_collection">create_untracked_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="collection.md#0x4_collection_create_untracked_collection">create_untracked_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef {
    <b>let</b> collection_seed = <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(&name);
    <b>let</b> constructor_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, collection_seed);

    <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(
        creator,
        constructor_ref,
        description,
        name,
        <a href="royalty.md#0x4_royalty">royalty</a>,
        uri,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    )
}
</code></pre>



</details>

<a name="0x4_collection_create_collection_internal"></a>

## Function `create_collection_internal`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>&lt;Supply: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, constructor_ref: <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, supply: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;Supply&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>&lt;Supply: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    constructor_ref: ConstructorRef,
    description: String,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
    supply: Option&lt;Supply&gt;,
): ConstructorRef {
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&name) &lt;= <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&uri) &lt;= <a href="collection.md#0x4_collection_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&description) &lt;= <a href="collection.md#0x4_collection_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>));

    <b>let</b> object_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);

    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="collection.md#0x4_collection_Collection">Collection</a> {
        creator: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),
        description,
        name,
        uri,
        mutation_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&object_signer),
    };
    <b>move_to</b>(&object_signer, <a href="collection.md#0x4_collection">collection</a>);

    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&supply)) {
        <b>move_to</b>(&object_signer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(supply))
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(supply)
    };

    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="royalty.md#0x4_royalty">royalty</a>)) {
        <a href="royalty.md#0x4_royalty_init">royalty::init</a>(&constructor_ref, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> <a href="royalty.md#0x4_royalty">royalty</a>))
    };

    <b>let</b> transfer_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&constructor_ref);
    <a href="../../aptos-framework/doc/object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(&transfer_ref);

    constructor_ref
}
</code></pre>



</details>

<a name="0x4_collection_create_collection_address"></a>

## Function `create_collection_address`

Generates the collections address based upon the creators address and the collection's name


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_address">create_collection_address</a>(creator: &<b>address</b>, name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_address">create_collection_address</a>(creator: &<b>address</b>, name: &String): <b>address</b> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(creator, <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name))
}
</code></pre>



</details>

<a name="0x4_collection_create_collection_seed"></a>

## Function `create_collection_seed`

Named objects are derived from a seed, the collection's seed is its name.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name: &String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(name) &lt;= <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));
    *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(name)
}
</code></pre>



</details>

<a name="0x4_collection_increment_supply"></a>

## Function `increment_supply`

Called by token on mint to increment supply if there's an appropriate Supply struct.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_increment_supply">increment_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, <a href="token.md#0x4_token">token</a>: <b>address</b>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_increment_supply">increment_supply</a>(
    <a href="collection.md#0x4_collection">collection</a>: &Object&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;,
    <a href="token.md#0x4_token">token</a>: <b>address</b>,
): Option&lt;u64&gt; <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> {
    <b>let</b> collection_addr = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);
    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr);
        supply.current_supply = supply.current_supply + 1;
        supply.total_minted = supply.total_minted + 1;
        <b>assert</b>!(
            supply.current_supply &lt;= supply.max_supply,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED">ECOLLECTION_SUPPLY_EXCEEDED</a>),
        );
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> supply.mint_events,
            <a href="collection.md#0x4_collection_MintEvent">MintEvent</a> {
                index: supply.total_minted,
                <a href="token.md#0x4_token">token</a>,
            },
        );
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.total_minted)
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr);
        supply.current_supply = supply.current_supply + 1;
        supply.total_minted = supply.total_minted + 1;
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> supply.mint_events,
            <a href="collection.md#0x4_collection_MintEvent">MintEvent</a> {
                index: supply.total_minted,
                <a href="token.md#0x4_token">token</a>,
            },
        );
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.total_minted)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x4_collection_decrement_supply"></a>

## Function `decrement_supply`

Called by token on burn to decrement supply if there's an appropriate Supply struct.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_decrement_supply">decrement_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, <a href="token.md#0x4_token">token</a>: <b>address</b>, index: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_decrement_supply">decrement_supply</a>(
    <a href="collection.md#0x4_collection">collection</a>: &Object&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;,
    <a href="token.md#0x4_token">token</a>: <b>address</b>,
    index: Option&lt;u64&gt;,
) <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> {
    <b>let</b> collection_addr = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);
    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr);
        supply.current_supply = supply.current_supply - 1;
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> supply.burn_events,
            <a href="collection.md#0x4_collection_BurnEvent">BurnEvent</a> {
                index: *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&index),
                <a href="token.md#0x4_token">token</a>,
            },
        );
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr);
        supply.current_supply = supply.current_supply - 1;
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
            &<b>mut</b> supply.burn_events,
            <a href="collection.md#0x4_collection_BurnEvent">BurnEvent</a> {
                index: *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&index),
                <a href="token.md#0x4_token">token</a>,
            },
        );
    }
}
</code></pre>



</details>

<a name="0x4_collection_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_generate_mutator_ref">generate_mutator_ref</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_generate_mutator_ref">generate_mutator_ref</a>(ref: &ConstructorRef): <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> {
    <b>let</b> <a href="../../aptos-framework/doc/object.md#0x1_object">object</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(ref);
    <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> { self: <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>) }
}
</code></pre>



</details>

<a name="0x4_collection_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(addr: <b>address</b>) {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(addr),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="collection.md#0x4_collection_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),
    );
}
</code></pre>



</details>

<a name="0x4_collection_borrow"></a>

## Function `borrow`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_borrow">borrow</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<a href="collection.md#0x4_collection_Collection">collection::Collection</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_borrow">borrow</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &Object&lt;T&gt;): &<a href="collection.md#0x4_collection_Collection">Collection</a> {
    <b>let</b> collection_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);
    <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(collection_address);
    <b>borrow_global</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(collection_address)
}
</code></pre>



</details>

<a name="0x4_collection_count"></a>

## Function `count`

Provides the count of the current selection if supply tracking is used


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_count">count</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_count">count</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): Option&lt;u64&gt; <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> {
    <b>let</b> collection_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&<a href="collection.md#0x4_collection">collection</a>);
    <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(collection_address);

    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address);
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current_supply)
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_address)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_address);
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current_supply)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x4_collection_creator"></a>

## Function `creator`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_creator">creator</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_creator">creator</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <a href="collection.md#0x4_collection_borrow">borrow</a>(&<a href="collection.md#0x4_collection">collection</a>).creator
}
</code></pre>



</details>

<a name="0x4_collection_description"></a>

## Function `description`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_description">description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_description">description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <a href="collection.md#0x4_collection_borrow">borrow</a>(&<a href="collection.md#0x4_collection">collection</a>).description
}
</code></pre>



</details>

<a name="0x4_collection_name"></a>

## Function `name`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_name">name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_name">name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <a href="collection.md#0x4_collection_borrow">borrow</a>(&<a href="collection.md#0x4_collection">collection</a>).name
}
</code></pre>



</details>

<a name="0x4_collection_uri"></a>

## Function `uri`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_uri">uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_uri">uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <a href="collection.md#0x4_collection_borrow">borrow</a>(&<a href="collection.md#0x4_collection">collection</a>).uri
}
</code></pre>



</details>

<a name="0x4_collection_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>): &<b>mut</b> <a href="collection.md#0x4_collection_Collection">collection::Collection</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>): &<b>mut</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(mutator_ref.self);
    <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(mutator_ref.self)
}
</code></pre>



</details>

<a name="0x4_collection_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_description">set_description</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_description">set_description</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, description: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&description) &lt;= <a href="collection.md#0x4_collection_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>));
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref);
    <a href="collection.md#0x4_collection">collection</a>.description = description;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="collection.md#0x4_collection">collection</a>.mutation_events,
        <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"description") },
    );
}
</code></pre>



</details>

<a name="0x4_collection_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_uri">set_uri</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_uri">set_uri</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, uri: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&uri) &lt;= <a href="collection.md#0x4_collection_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref);
    <a href="collection.md#0x4_collection">collection</a>.uri = uri;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="collection.md#0x4_collection">collection</a>.mutation_events,
        <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"uri") },
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
