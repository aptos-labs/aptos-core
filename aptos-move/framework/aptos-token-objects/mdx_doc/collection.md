
<a id="0x4_collection"></a>

# Module `0x4::collection`

This defines an object&#45;based Collection. A collection acts as a set organizer for a group of
tokens. This includes aspects such as a general description, project URI, name, and may contain
other useful generalizations across this set of tokens.

Being built upon objects enables collections to be relatively flexible. As core primitives it
supports:
&#42; Common fields: name, uri, description, creator
&#42; MutatorRef leaving mutability configuration to a higher level component
&#42; Addressed by a global identifier of creator&apos;s address and collection name, thus collections
cannot be deleted as a restriction of the object model.
&#42; Optional support for collection&#45;wide royalties
&#42; Optional support for tracking of supply with events on mint or burn

TODO:
&#42; Consider supporting changing the name of the collection with the MutatorRef. This would
require adding the field original_name.
&#42; Consider supporting changing the aspects of supply with the MutatorRef.
&#42; Add aggregator support when added to framework


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


<pre><code><b>use</b> <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;<br /></code></pre>



<a id="0x4_collection_Collection"></a>

## Resource `Collection`

Represents the common fields for a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="collection.md#0x4_collection_Collection">Collection</a> <b>has</b> key<br /></code></pre>



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
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off&#45;chain
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

<a id="0x4_collection_MutatorRef"></a>

## Struct `MutatorRef`

This enables mutating description and URI by higher level services.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x4_collection_MutationEvent"></a>

## Struct `MutationEvent`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x4_collection_Mutation"></a>

## Struct `Mutation`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="collection.md#0x4_collection_Mutation">Mutation</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_FixedSupply"></a>

## Resource `FixedSupply`

Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.
and adding events and supply tracking to a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> <b>has</b> key<br /></code></pre>



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

<a id="0x4_collection_UnlimitedSupply"></a>

## Resource `UnlimitedSupply`

Unlimited supply tracker, this is useful for adding events and supply tracking to a collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> <b>has</b> key<br /></code></pre>



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

<a id="0x4_collection_ConcurrentSupply"></a>

## Resource `ConcurrentSupply`

Supply tracker, useful for tracking amount of issued tokens.
If max_value is not set to U64_MAX, this ensures that a limited number of tokens are minted.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;</code>
</dt>
<dd>
 Total minted &#45; total burned
</dd>
<dt>
<code>total_minted: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_BurnEvent"></a>

## Struct `BurnEvent`



<pre><code><b>struct</b> <a href="collection.md#0x4_collection_BurnEvent">BurnEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x4_collection_MintEvent"></a>

## Struct `MintEvent`



<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MintEvent">MintEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x4_collection_Burn"></a>

## Struct `Burn`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="collection.md#0x4_collection_Burn">Burn</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="collection.md#0x4_collection">collection</a>: <b>address</b></code>
</dt>
<dd>

</dd>
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
<dt>
<code>previous_owner: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_collection_Mint"></a>

## Struct `Mint`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="collection.md#0x4_collection_Mint">Mint</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="collection.md#0x4_collection">collection</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u64&gt;</code>
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

<a id="0x4_collection_ConcurrentBurnEvent"></a>

## Struct `ConcurrentBurnEvent`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br />&#35;[deprecated]<br /><b>struct</b> <a href="collection.md#0x4_collection_ConcurrentBurnEvent">ConcurrentBurnEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_addr: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x4_collection_ConcurrentMintEvent"></a>

## Struct `ConcurrentMintEvent`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br />&#35;[deprecated]<br /><b>struct</b> <a href="collection.md#0x4_collection_ConcurrentMintEvent">ConcurrentMintEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u64&gt;</code>
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

<a id="0x4_collection_SetMaxSupply"></a>

## Struct `SetMaxSupply`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="collection.md#0x4_collection_SetMaxSupply">SetMaxSupply</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;</code>
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



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_U64">MAX_U64</a>: u64 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x4_collection_EURI_TOO_LONG"></a>

The URI is over the maximum length


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x4_collection_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 &#61; 512;<br /></code></pre>



<a id="0x4_collection_EALREADY_CONCURRENT"></a>

Tried upgrading collection to concurrent, but collection is already concurrent


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EALREADY_CONCURRENT">EALREADY_CONCURRENT</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x4_collection_ECOLLECTION_DOES_NOT_EXIST"></a>

The collection does not exist


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x4_collection_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is over the maximum length


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED"></a>

The collection has reached its supply and no more tokens can be minted, unless some are burned


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED">ECOLLECTION_SUPPLY_EXCEEDED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x4_collection_ECONCURRENT_NOT_ENABLED"></a>

Concurrent feature flag is not yet enabled, so the function cannot be performed


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECONCURRENT_NOT_ENABLED">ECONCURRENT_NOT_ENABLED</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x4_collection_EDESCRIPTION_TOO_LONG"></a>

The description is over the maximum length


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x4_collection_EINVALID_MAX_SUPPLY"></a>

The new max supply cannot be less than the current supply


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EINVALID_MAX_SUPPLY">EINVALID_MAX_SUPPLY</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO"></a>

The max supply must be positive


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO">EMAX_SUPPLY_CANNOT_BE_ZERO</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x4_collection_ENO_MAX_SUPPLY_IN_COLLECTION"></a>

The collection does not have a max supply


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ENO_MAX_SUPPLY_IN_COLLECTION">ENO_MAX_SUPPLY_IN_COLLECTION</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x4_collection_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x4_collection_MAX_DESCRIPTION_LENGTH"></a>



<pre><code><b>const</b> <a href="collection.md#0x4_collection_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>: u64 &#61; 2048;<br /></code></pre>



<a id="0x4_collection_create_fixed_collection"></a>

## Function `create_fixed_collection`

Creates a fixed&#45;sized collection, or a collection that supports a fixed amount of tokens.
This is useful to create a guaranteed, limited supply on&#45;chain digital asset. For example,
a collection 1111 vicious vipers. Note, creating restrictions such as upward limits results
in data structures that prevent Aptos from parallelizing mints of this collection type.
Beyond that, it adds supply tracking with events.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_fixed_collection">create_fixed_collection</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_fixed_collection">create_fixed_collection</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    description: String,<br />    max_supply: u64,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>assert</b>!(max_supply !&#61; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="collection.md#0x4_collection_EMAX_SUPPLY_CANNOT_BE_ZERO">EMAX_SUPPLY_CANNOT_BE_ZERO</a>));<br />    <b>let</b> collection_seed &#61; <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(&amp;name);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, collection_seed);<br /><br />    <b>let</b> supply &#61; <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />        current_supply: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(max_supply),<br />        total_minted: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),<br />    &#125;;<br /><br />    <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>(<br />        creator,<br />        constructor_ref,<br />        description,<br />        name,<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply),<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_create_unlimited_collection"></a>

## Function `create_unlimited_collection`

Creates an unlimited collection. This has support for supply tracking but does not limit
the supply of tokens.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_unlimited_collection">create_unlimited_collection</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_unlimited_collection">create_unlimited_collection</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> collection_seed &#61; <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(&amp;name);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, collection_seed);<br /><br />    <b>let</b> supply &#61; <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />        current_supply: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),<br />        total_minted: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),<br />    &#125;;<br /><br />    <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>(<br />        creator,<br />        constructor_ref,<br />        description,<br />        name,<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply),<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_create_untracked_collection"></a>

## Function `create_untracked_collection`

Creates an untracked collection, or a collection that supports an arbitrary amount of
tokens. This is useful for mass airdrops that fully leverage Aptos parallelization.
TODO: Hide this until we bring back meaningful way to enforce burns


<pre><code><b>fun</b> <a href="collection.md#0x4_collection_create_untracked_collection">create_untracked_collection</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="collection.md#0x4_collection_create_untracked_collection">create_untracked_collection</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> collection_seed &#61; <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(&amp;name);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, collection_seed);<br /><br />    <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(<br />        creator,<br />        constructor_ref,<br />        description,<br />        name,<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_create_collection_internal"></a>

## Function `create_collection_internal`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>&lt;Supply: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, constructor_ref: <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, supply: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;Supply&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_create_collection_internal">create_collection_internal</a>&lt;Supply: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    constructor_ref: ConstructorRef,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />    supply: Option&lt;Supply&gt;,<br />): ConstructorRef &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="collection.md#0x4_collection_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;description) &lt;&#61; <a href="collection.md#0x4_collection_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>));<br /><br />    <b>let</b> object_signer &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&amp;constructor_ref);<br /><br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />        creator: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />        description,<br />        name,<br />        uri,<br />        mutation_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&amp;object_signer),<br />    &#125;;<br />    <b>move_to</b>(&amp;object_signer, <a href="collection.md#0x4_collection">collection</a>);<br /><br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;supply)) &#123;<br />        <b>move_to</b>(&amp;object_signer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(supply))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(supply)<br />    &#125;;<br /><br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="royalty.md#0x4_royalty">royalty</a>)) &#123;<br />        <a href="royalty.md#0x4_royalty_init">royalty::init</a>(&amp;constructor_ref, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> <a href="royalty.md#0x4_royalty">royalty</a>))<br />    &#125;;<br /><br />    <b>let</b> transfer_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&amp;constructor_ref);<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(&amp;transfer_ref);<br /><br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_create_collection_address"></a>

## Function `create_collection_address`

Generates the collections address based upon the creators address and the collection&apos;s name


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_address">create_collection_address</a>(creator: &amp;<b>address</b>, name: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_address">create_collection_address</a>(creator: &amp;<b>address</b>, name: &amp;String): <b>address</b> &#123;<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(creator, <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name))<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_create_collection_seed"></a>

## Function `create_collection_seed`

Named objects are derived from a seed, the collection&apos;s seed is its name.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name: &amp;String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(name) &lt;&#61; <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));<br />    &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(name)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_increment_supply"></a>

## Function `increment_supply`

Called by token on mint to increment supply if there&apos;s an appropriate Supply struct.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_increment_supply">increment_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, <a href="token.md#0x4_token">token</a>: <b>address</b>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u64&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_increment_supply">increment_supply</a>(<br />    <a href="collection.md#0x4_collection">collection</a>: &amp;Object&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;,<br />    <a href="token.md#0x4_token">token</a>: <b>address</b>,<br />): Option&lt;AggregatorSnapshot&lt;u64&gt;&gt; <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>, <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> collection_addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);<br />    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_addr)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_addr);<br />        <b>assert</b>!(<br />            <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_try_add">aggregator_v2::try_add</a>(&amp;<b>mut</b> supply.current_supply, 1),<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED">ECOLLECTION_SUPPLY_EXCEEDED</a>),<br />        );<br />        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&amp;<b>mut</b> supply.total_minted, 1);<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="collection.md#0x4_collection_Mint">Mint</a> &#123;<br />                <a href="collection.md#0x4_collection">collection</a>: collection_addr,<br />                index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_snapshot">aggregator_v2::snapshot</a>(&amp;supply.total_minted),<br />                <a href="token.md#0x4_token">token</a>,<br />            &#125;,<br />        );<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_snapshot">aggregator_v2::snapshot</a>(&amp;supply.total_minted))<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr);<br />        supply.current_supply &#61; supply.current_supply &#43; 1;<br />        supply.total_minted &#61; supply.total_minted &#43; 1;<br />        <b>assert</b>!(<br />            supply.current_supply &lt;&#61; supply.max_supply,<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_SUPPLY_EXCEEDED">ECOLLECTION_SUPPLY_EXCEEDED</a>),<br />        );<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />                <a href="collection.md#0x4_collection_Mint">Mint</a> &#123;<br />                    <a href="collection.md#0x4_collection">collection</a>: collection_addr,<br />                    index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_snapshot">aggregator_v2::create_snapshot</a>(supply.total_minted),<br />                    <a href="token.md#0x4_token">token</a>,<br />                &#125;,<br />            );<br />        &#125;;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(&amp;<b>mut</b> supply.mint_events,<br />            <a href="collection.md#0x4_collection_MintEvent">MintEvent</a> &#123;<br />                index: supply.total_minted,<br />                <a href="token.md#0x4_token">token</a>,<br />            &#125;,<br />        );<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_snapshot">aggregator_v2::create_snapshot</a>&lt;u64&gt;(supply.total_minted))<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr);<br />        supply.current_supply &#61; supply.current_supply &#43; 1;<br />        supply.total_minted &#61; supply.total_minted &#43; 1;<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />                <a href="collection.md#0x4_collection_Mint">Mint</a> &#123;<br />                    <a href="collection.md#0x4_collection">collection</a>: collection_addr,<br />                    index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_snapshot">aggregator_v2::create_snapshot</a>(supply.total_minted),<br />                    <a href="token.md#0x4_token">token</a>,<br />                &#125;,<br />            );<br />        &#125;;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />            &amp;<b>mut</b> supply.mint_events,<br />            <a href="collection.md#0x4_collection_MintEvent">MintEvent</a> &#123;<br />                index: supply.total_minted,<br />                <a href="token.md#0x4_token">token</a>,<br />            &#125;,<br />        );<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_snapshot">aggregator_v2::create_snapshot</a>&lt;u64&gt;(supply.total_minted))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_decrement_supply"></a>

## Function `decrement_supply`

Called by token on burn to decrement supply if there&apos;s an appropriate Supply struct.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_decrement_supply">decrement_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, <a href="token.md#0x4_token">token</a>: <b>address</b>, index: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, previous_owner: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_decrement_supply">decrement_supply</a>(<br />    <a href="collection.md#0x4_collection">collection</a>: &amp;Object&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;,<br />    <a href="token.md#0x4_token">token</a>: <b>address</b>,<br />    index: Option&lt;u64&gt;,<br />    previous_owner: <b>address</b>,<br />) <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>, <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> collection_addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);<br />    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_addr)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_addr);<br />        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_sub">aggregator_v2::sub</a>(&amp;<b>mut</b> supply.current_supply, 1);<br /><br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="collection.md#0x4_collection_Burn">Burn</a> &#123;<br />                <a href="collection.md#0x4_collection">collection</a>: collection_addr,<br />                index: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;index),<br />                <a href="token.md#0x4_token">token</a>,<br />                previous_owner,<br />            &#125;,<br />        );<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr);<br />        supply.current_supply &#61; supply.current_supply &#45; 1;<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />                <a href="collection.md#0x4_collection_Burn">Burn</a> &#123;<br />                    <a href="collection.md#0x4_collection">collection</a>: collection_addr,<br />                    index: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;index),<br />                    <a href="token.md#0x4_token">token</a>,<br />                    previous_owner,<br />                &#125;,<br />            );<br />        &#125;;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />            &amp;<b>mut</b> supply.burn_events,<br />            <a href="collection.md#0x4_collection_BurnEvent">BurnEvent</a> &#123;<br />                index: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;index),<br />                <a href="token.md#0x4_token">token</a>,<br />            &#125;,<br />        );<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_addr);<br />        supply.current_supply &#61; supply.current_supply &#45; 1;<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />                <a href="collection.md#0x4_collection_Burn">Burn</a> &#123;<br />                    <a href="collection.md#0x4_collection">collection</a>: collection_addr,<br />                    index: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;index),<br />                    <a href="token.md#0x4_token">token</a>,<br />                    previous_owner,<br />                &#125;,<br />            );<br />        &#125;;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />            &amp;<b>mut</b> supply.burn_events,<br />            <a href="collection.md#0x4_collection_BurnEvent">BurnEvent</a> &#123;<br />                index: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;index),<br />                <a href="token.md#0x4_token">token</a>,<br />            &#125;,<br />        );<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_generate_mutator_ref">generate_mutator_ref</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_generate_mutator_ref">generate_mutator_ref</a>(ref: &amp;ConstructorRef): <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> &#123;<br />    <b>let</b> <a href="../../aptos-framework/doc/object.md#0x1_object">object</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(ref);<br />    <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> &#123; self: <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_upgrade_to_concurrent"></a>

## Function `upgrade_to_concurrent`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_upgrade_to_concurrent">upgrade_to_concurrent</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_upgrade_to_concurrent">upgrade_to_concurrent</a>(<br />    ref: &amp;ExtendRef,<br />) <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> &#123;<br />    <b>let</b> metadata_object_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(ref);<br />    <b>let</b> metadata_object_signer &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(ref);<br /><br />    <b>let</b> (supply, current_supply, total_minted, burn_events, mint_events) &#61; <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(<br />        metadata_object_address<br />    )) &#123;<br />        <b>let</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> &#123;<br />            current_supply,<br />            max_supply,<br />            total_minted,<br />            burn_events,<br />            mint_events,<br />        &#125; &#61; <b>move_from</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(metadata_object_address);<br /><br />        <b>let</b> supply &#61; <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />            current_supply: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(max_supply),<br />            total_minted: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),<br />        &#125;;<br />        (supply, current_supply, total_minted, burn_events, mint_events)<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(metadata_object_address)) &#123;<br />        <b>let</b> <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a> &#123;<br />            current_supply,<br />            total_minted,<br />            burn_events,<br />            mint_events,<br />        &#125; &#61; <b>move_from</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(metadata_object_address);<br /><br />        <b>let</b> supply &#61; <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />            current_supply: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),<br />            total_minted: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">aggregator_v2::create_unbounded_aggregator</a>(),<br />        &#125;;<br />        (supply, current_supply, total_minted, burn_events, mint_events)<br />    &#125; <b>else</b> &#123;<br />        // untracked <a href="collection.md#0x4_collection">collection</a> is already concurrent, and other variants too.<br />        <b>abort</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="collection.md#0x4_collection_EALREADY_CONCURRENT">EALREADY_CONCURRENT</a>)<br />    &#125;;<br /><br />    // <b>update</b> current state:<br />    <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&amp;<b>mut</b> supply.current_supply, current_supply);<br />    <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&amp;<b>mut</b> supply.total_minted, total_minted);<br />    <b>move_to</b>(&amp;metadata_object_signer, supply);<br /><br />    <a href="../../aptos-framework/doc/event.md#0x1_event_destroy_handle">event::destroy_handle</a>(burn_events);<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_destroy_handle">event::destroy_handle</a>(mint_events);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(addr: <b>address</b>) &#123;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(addr),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="collection.md#0x4_collection_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_borrow"></a>

## Function `borrow`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_borrow">borrow</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="collection.md#0x4_collection_Collection">collection::Collection</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_borrow">borrow</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &amp;Object&lt;T&gt;): &amp;<a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <b>let</b> collection_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);<br />    <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(collection_address);<br />    <b>borrow_global</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(collection_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_count"></a>

## Function `count`

Provides the count of the current selection if supply tracking is used

Note: Calling this method from transaction that also mints/burns, prevents
it from being parallelized.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_count">count</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_count">count</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;<br />): Option&lt;u64&gt; <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>, <a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>, <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a> &#123;<br />    <b>let</b> collection_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="collection.md#0x4_collection">collection</a>);<br />    <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(collection_address);<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_address);<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&amp;supply.current_supply))<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address);<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current_supply)<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_UnlimitedSupply">UnlimitedSupply</a>&gt;(collection_address);<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current_supply)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_creator"></a>

## Function `creator`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_creator">creator</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_creator">creator</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <a href="collection.md#0x4_collection_borrow">borrow</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).creator<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_description"></a>

## Function `description`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_description">description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_description">description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <a href="collection.md#0x4_collection_borrow">borrow</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).description<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_name"></a>

## Function `name`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_name">name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_name">name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <a href="collection.md#0x4_collection_borrow">borrow</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).name<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_uri"></a>

## Function `uri`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_uri">uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_uri">uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <a href="collection.md#0x4_collection_borrow">borrow</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).uri<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>fun</b> <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>): &amp;<b>mut</b> <a href="collection.md#0x4_collection_Collection">collection::Collection</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>): &amp;<b>mut</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <a href="collection.md#0x4_collection_check_collection_exists">check_collection_exists</a>(mutator_ref.self);<br />    <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(mutator_ref.self)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_set_name"></a>

## Function `set_name`

Callers of this function must be aware that changing the name will change the calculated
collection&apos;s address when calling <code>create_collection_address</code>.
Once the collection has been created, the collection address should be saved for reference and
<code>create_collection_address</code> should not be used to derive the collection&apos;s address.

After changing the collection&apos;s name, to create tokens &#45; only call functions that accept the collection object as an argument.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_name">set_name</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_name">set_name</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, name: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="collection.md#0x4_collection_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref);<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="collection.md#0x4_collection_Mutation">Mutation</a> &#123;<br />        mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;name&quot;) ,<br />        <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>(mutator_ref.self),<br />        old_value: <a href="collection.md#0x4_collection">collection</a>.name,<br />        new_value: name,<br />    &#125;);<br />    <a href="collection.md#0x4_collection">collection</a>.name &#61; name;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_description">set_description</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_description">set_description</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, description: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;description) &lt;&#61; <a href="collection.md#0x4_collection_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>));<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="collection.md#0x4_collection_Mutation">Mutation</a> &#123;<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;description&quot;),<br />            <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>(mutator_ref.self),<br />            old_value: <a href="collection.md#0x4_collection">collection</a>.description,<br />            new_value: description,<br />        &#125;);<br />    &#125;;<br />    <a href="collection.md#0x4_collection">collection</a>.description &#61; description;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> <a href="collection.md#0x4_collection">collection</a>.mutation_events,<br />        <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> &#123; mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;description&quot;) &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_uri">set_uri</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_uri">set_uri</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, uri: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="collection.md#0x4_collection_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="collection.md#0x4_collection_borrow_mut">borrow_mut</a>(mutator_ref);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="collection.md#0x4_collection_Mutation">Mutation</a> &#123;<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;uri&quot;),<br />            <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>(mutator_ref.self),<br />            old_value: <a href="collection.md#0x4_collection">collection</a>.uri,<br />            new_value: uri,<br />        &#125;);<br />    &#125;;<br />    <a href="collection.md#0x4_collection">collection</a>.uri &#61; uri;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> <a href="collection.md#0x4_collection">collection</a>.mutation_events,<br />        <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> &#123; mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;uri&quot;) &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_collection_set_max_supply"></a>

## Function `set_max_supply`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_max_supply">set_max_supply</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, max_supply: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_max_supply">set_max_supply</a>(mutator_ref: &amp;<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, max_supply: u64) <b>acquires</b> <a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>, <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> &#123;<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(mutator_ref.self);<br />    <b>let</b> collection_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="collection.md#0x4_collection">collection</a>);<br />    <b>let</b> old_max_supply;<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_ConcurrentSupply">ConcurrentSupply</a>&gt;(collection_address);<br />        <b>let</b> current_supply &#61; <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&amp;supply.current_supply);<br />        <b>assert</b>!(<br />            max_supply &gt;&#61; current_supply,<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EINVALID_MAX_SUPPLY">EINVALID_MAX_SUPPLY</a>),<br />        );<br />        old_max_supply &#61; <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_max_value">aggregator_v2::max_value</a>(&amp;supply.current_supply);<br />        supply.current_supply &#61; <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(max_supply);<br />        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_add">aggregator_v2::add</a>(&amp;<b>mut</b> supply.current_supply, current_supply);<br />    &#125; <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address)) &#123;<br />        <b>let</b> supply &#61; <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address);<br />        <b>assert</b>!(<br />            max_supply &gt;&#61; supply.current_supply,<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EINVALID_MAX_SUPPLY">EINVALID_MAX_SUPPLY</a>),<br />        );<br />        old_max_supply &#61; supply.max_supply;<br />        supply.max_supply &#61; max_supply;<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="collection.md#0x4_collection_ENO_MAX_SUPPLY_IN_COLLECTION">ENO_MAX_SUPPLY_IN_COLLECTION</a>)<br />    &#125;;<br /><br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="collection.md#0x4_collection_SetMaxSupply">SetMaxSupply</a> &#123; <a href="collection.md#0x4_collection">collection</a>, old_max_supply, new_max_supply: max_supply &#125;);<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
