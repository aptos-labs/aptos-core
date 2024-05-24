
<a id="0x4_token"></a>

# Module `0x4::token`

This defines an object&#45;based Token. The key differentiating features from the Aptos standard
token are:
&#42; Decoupled token ownership from token data.
&#42; Explicit data model for token metadata via adjacent resources
&#42; Extensible framework for tokens


-  [Resource `Token`](#0x4_token_Token)
-  [Resource `TokenIdentifiers`](#0x4_token_TokenIdentifiers)
-  [Resource `ConcurrentTokenIdentifiers`](#0x4_token_ConcurrentTokenIdentifiers)
-  [Struct `BurnRef`](#0x4_token_BurnRef)
-  [Struct `MutatorRef`](#0x4_token_MutatorRef)
-  [Struct `MutationEvent`](#0x4_token_MutationEvent)
-  [Struct `Mutation`](#0x4_token_Mutation)
-  [Constants](#@Constants_0)
-  [Function `create_common`](#0x4_token_create_common)
-  [Function `create_common_with_collection`](#0x4_token_create_common_with_collection)
-  [Function `create_token`](#0x4_token_create_token)
-  [Function `create`](#0x4_token_create)
-  [Function `create_numbered_token_object`](#0x4_token_create_numbered_token_object)
-  [Function `create_numbered_token`](#0x4_token_create_numbered_token)
-  [Function `create_named_token_object`](#0x4_token_create_named_token_object)
-  [Function `create_named_token`](#0x4_token_create_named_token)
-  [Function `create_named_token_from_seed`](#0x4_token_create_named_token_from_seed)
-  [Function `create_from_account`](#0x4_token_create_from_account)
-  [Function `create_token_address`](#0x4_token_create_token_address)
-  [Function `create_token_address_with_seed`](#0x4_token_create_token_address_with_seed)
-  [Function `create_token_seed`](#0x4_token_create_token_seed)
-  [Function `create_token_name_with_seed`](#0x4_token_create_token_name_with_seed)
-  [Function `generate_mutator_ref`](#0x4_token_generate_mutator_ref)
-  [Function `generate_burn_ref`](#0x4_token_generate_burn_ref)
-  [Function `address_from_burn_ref`](#0x4_token_address_from_burn_ref)
-  [Function `borrow`](#0x4_token_borrow)
-  [Function `creator`](#0x4_token_creator)
-  [Function `collection_name`](#0x4_token_collection_name)
-  [Function `collection_object`](#0x4_token_collection_object)
-  [Function `description`](#0x4_token_description)
-  [Function `name`](#0x4_token_name)
-  [Function `uri`](#0x4_token_uri)
-  [Function `royalty`](#0x4_token_royalty)
-  [Function `index`](#0x4_token_index)
-  [Function `borrow_mut`](#0x4_token_borrow_mut)
-  [Function `burn`](#0x4_token_burn)
-  [Function `set_description`](#0x4_token_set_description)
-  [Function `set_name`](#0x4_token_set_name)
-  [Function `set_uri`](#0x4_token_set_uri)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /><b>use</b> <a href="collection.md#0x4_collection">0x4::collection</a>;<br /><b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;<br /></code></pre>



<a id="0x4_token_Token"></a>

## Resource `Token`

Represents the common fields to all tokens.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="token.md#0x4_token_Token">Token</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;</code>
</dt>
<dd>
 The collection from which this token resides.
</dd>
<dt>
<code>index: u64</code>
</dt>
<dd>
 Deprecated in favor of <code>index</code> inside TokenIdentifiers.
 Will be populated until concurrent_token_v2_enabled feature flag is enabled.

 Unique identifier within the collection, optional, 0 means unassigned
</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 A brief description of the token.
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Deprecated in favor of <code>name</code> inside TokenIdentifiers.
 Will be populated until concurrent_token_v2_enabled feature flag is enabled.

 The name of the token, which should be unique within the collection; the length of name
 should be smaller than 128, characters, eg: &quot;Aptos Animal #1234&quot;
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off&#45;chain
 storage; the URL length will likely need a maximum any suggestions?
</dd>
<dt>
<code>mutation_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x4_token_MutationEvent">token::MutationEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon any mutation of the token.
</dd>
</dl>


</details>

<a id="0x4_token_TokenIdentifiers"></a>

## Resource `TokenIdentifiers`

Represents first addition to the common fields for all tokens
Starts being populated once aggregator_v2_api_enabled is enabled.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u64&gt;</code>
</dt>
<dd>
 Unique identifier within the collection, optional, 0 means unassigned
</dd>
<dt>
<code>name: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_DerivedStringSnapshot">aggregator_v2::DerivedStringSnapshot</a></code>
</dt>
<dd>
 The name of the token, which should be unique within the collection; the length of name
 should be smaller than 128, characters, eg: &quot;Aptos Animal #1234&quot;
</dd>
</dl>


</details>

<a id="0x4_token_ConcurrentTokenIdentifiers"></a>

## Resource `ConcurrentTokenIdentifiers`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br />&#35;[deprecated]<br /><b>struct</b> <a href="token.md#0x4_token_ConcurrentTokenIdentifiers">ConcurrentTokenIdentifiers</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_token_BurnRef"></a>

## Struct `BurnRef`

This enables burning an NFT, if possible, it will also delete the object. Note, the data
in inner and self occupies 32&#45;bytes each, rather than have both, this data structure makes
a small optimization to support either and take a fixed amount of 34&#45;bytes.


<pre><code><b>struct</b> <a href="token.md#0x4_token_BurnRef">BurnRef</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/doc/object.md#0x1_object_DeleteRef">object::DeleteRef</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>self: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_token_MutatorRef"></a>

## Struct `MutatorRef`

This enables mutating description and URI by higher level services.


<pre><code><b>struct</b> <a href="token.md#0x4_token_MutatorRef">MutatorRef</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x4_token_MutationEvent"></a>

## Struct `MutationEvent`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code><b>struct</b> <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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

<a id="0x4_token_Mutation"></a>

## Struct `Mutation`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x4_token_Mutation">Mutation</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x4_token_EURI_TOO_LONG"></a>

The URI is over the maximum length


<pre><code><b>const</b> <a href="token.md#0x4_token_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x4_token_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x4_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 &#61; 512;<br /></code></pre>



<a id="0x4_token_EDESCRIPTION_TOO_LONG"></a>

The description is over the maximum length


<pre><code><b>const</b> <a href="token.md#0x4_token_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x4_token_MAX_DESCRIPTION_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x4_token_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>: u64 &#61; 2048;<br /></code></pre>



<a id="0x4_token_EFIELD_NOT_MUTABLE"></a>

The field being changed is not mutable


<pre><code><b>const</b> <a href="token.md#0x4_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x4_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code><b>const</b> <a href="token.md#0x4_token_ENOT_CREATOR">ENOT_CREATOR</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x4_token_ESEED_TOO_LONG"></a>

The seed is over the maximum length


<pre><code><b>const</b> <a href="token.md#0x4_token_ESEED_TOO_LONG">ESEED_TOO_LONG</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x4_token_ETOKEN_DOES_NOT_EXIST"></a>

The token does not exist


<pre><code><b>const</b> <a href="token.md#0x4_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x4_token_ETOKEN_NAME_TOO_LONG"></a>

The token name is over the maximum length


<pre><code><b>const</b> <a href="token.md#0x4_token_ETOKEN_NAME_TOO_LONG">ETOKEN_NAME_TOO_LONG</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x4_token_MAX_TOKEN_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x4_token_MAX_TOKEN_NAME_LENGTH">MAX_TOKEN_NAME_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x4_token_MAX_TOKEN_SEED_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x4_token_MAX_TOKEN_SEED_LENGTH">MAX_TOKEN_SEED_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x4_token_create_common"></a>

## Function `create_common`



<pre><code><b>fun</b> <a href="token.md#0x4_token_create_common">create_common</a>(constructor_ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_prefix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_with_index_suffix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="token.md#0x4_token_create_common">create_common</a>(<br />    constructor_ref: &amp;ConstructorRef,<br />    creator_address: <b>address</b>,<br />    collection_name: String,<br />    description: String,<br />    name_prefix: String,<br />    // If <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>, numbered <a href="token.md#0x4_token">token</a> is created &#45; i.e. index is appended <b>to</b> the name.<br />    // If <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>, name_prefix is the full name of the <a href="token.md#0x4_token">token</a>.<br />    name_with_index_suffix: Option&lt;String&gt;,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />) &#123;<br />    <b>let</b> collection_addr &#61; <a href="collection.md#0x4_collection_create_collection_address">collection::create_collection_address</a>(&amp;creator_address, &amp;collection_name);<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;Collection&gt;(collection_addr);<br /><br />    <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(<br />        constructor_ref,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name_prefix,<br />        name_with_index_suffix,<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_common_with_collection"></a>

## Function `create_common_with_collection`



<pre><code><b>fun</b> <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(constructor_ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_prefix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_with_index_suffix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(<br />    constructor_ref: &amp;ConstructorRef,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;Collection&gt;,<br />    description: String,<br />    name_prefix: String,<br />    // If <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>, numbered <a href="token.md#0x4_token">token</a> is created &#45; i.e. index is appended <b>to</b> the name.<br />    // If <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>, name_prefix is the full name of the <a href="token.md#0x4_token">token</a>.<br />    name_with_index_suffix: Option&lt;String&gt;,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />) &#123;<br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;name_with_index_suffix)) &#123;<br />        // Be conservative, <b>as</b> we don&apos;t know what length the index will be, and <b>assume</b> worst case (20 chars in MAX_U64)<br />        <b>assert</b>!(<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name_prefix) &#43; 20 &#43; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(<br />                <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;name_with_index_suffix)<br />            ) &lt;&#61; <a href="token.md#0x4_token_MAX_TOKEN_NAME_LENGTH">MAX_TOKEN_NAME_LENGTH</a>,<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_ETOKEN_NAME_TOO_LONG">ETOKEN_NAME_TOO_LONG</a>)<br />        );<br />    &#125; <b>else</b> &#123;<br />        <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name_prefix) &lt;&#61; <a href="token.md#0x4_token_MAX_TOKEN_NAME_LENGTH">MAX_TOKEN_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_ETOKEN_NAME_TOO_LONG">ETOKEN_NAME_TOO_LONG</a>));<br />    &#125;;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;description) &lt;&#61; <a href="token.md#0x4_token_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="token.md#0x4_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br /><br />    <b>let</b> object_signer &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);<br /><br />    // TODO[agg_v2](cleanup) once this flag is enabled, cleanup <a href="../../aptos-framework/doc/code.md#0x1_code">code</a> for aggregator_api_enabled &#61; <b>false</b>.<br />    // Flag which controls whether <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a> functions from <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2">aggregator_v2</a> <b>module</b> can be called.<br />    <b>let</b> aggregator_api_enabled &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_aggregator_v2_api_enabled">features::aggregator_v2_api_enabled</a>();<br />    // Flag which controls whether we are going <b>to</b> still <b>continue</b> writing <b>to</b> deprecated fields.<br />    <b>let</b> concurrent_token_v2_enabled &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_concurrent_token_v2_enabled">features::concurrent_token_v2_enabled</a>();<br /><br />    <b>let</b> (deprecated_index, deprecated_name) &#61; <b>if</b> (aggregator_api_enabled) &#123;<br />        <b>let</b> index &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_with_default">option::destroy_with_default</a>(<br />            <a href="collection.md#0x4_collection_increment_concurrent_supply">collection::increment_concurrent_supply</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;object_signer)),<br />            <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_snapshot">aggregator_v2::create_snapshot</a>&lt;u64&gt;(0)<br />        );<br /><br />        // If create_numbered_token called us, add index <b>to</b> the name.<br />        <b>let</b> name &#61; <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;name_with_index_suffix)) &#123;<br />            <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_derive_string_concat">aggregator_v2::derive_string_concat</a>(name_prefix, &amp;index, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> name_with_index_suffix))<br />        &#125; <b>else</b> &#123;<br />            <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_derived_string">aggregator_v2::create_derived_string</a>(name_prefix)<br />        &#125;;<br /><br />        // Until concurrent_token_v2_enabled is enabled, we still need <b>to</b> write <b>to</b> deprecated fields.<br />        // Otherwise we put empty values there.<br />        // (we need <b>to</b> do these calls before creating token_concurrent, <b>to</b> avoid copying objects)<br />        <b>let</b> deprecated_index &#61; <b>if</b> (concurrent_token_v2_enabled) &#123;<br />            0<br />        &#125; <b>else</b> &#123;<br />            <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read_snapshot">aggregator_v2::read_snapshot</a>(&amp;index)<br />        &#125;;<br />        <b>let</b> deprecated_name &#61; <b>if</b> (concurrent_token_v2_enabled) &#123;<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;&quot;)<br />        &#125; <b>else</b> &#123;<br />            <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read_derived_string">aggregator_v2::read_derived_string</a>(&amp;name)<br />        &#125;;<br /><br />        // If aggregator_api_enabled, we always populate newly added fields<br />        <b>let</b> token_concurrent &#61; <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> &#123;<br />            index,<br />            name,<br />        &#125;;<br />        <b>move_to</b>(&amp;object_signer, token_concurrent);<br /><br />        (deprecated_index, deprecated_name)<br />    &#125; <b>else</b> &#123;<br />        // If aggregator_api_enabled is disabled, we cannot <b>use</b> increment_concurrent_supply or<br />        // create <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>, so we fallback <b>to</b> the <b>old</b> behavior.<br />        <b>let</b> id &#61; <a href="collection.md#0x4_collection_increment_supply">collection::increment_supply</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;object_signer));<br />        <b>let</b> index &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_get_with_default">option::get_with_default</a>(&amp;<b>mut</b> id, 0);<br /><br />        // If create_numbered_token called us, add index <b>to</b> the name.<br />        <b>let</b> name &#61; <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;name_with_index_suffix)) &#123;<br />            <b>let</b> name &#61; name_prefix;<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_append">string::append</a>(&amp;<b>mut</b> name, to_string&lt;u64&gt;(&amp;index));<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_append">string::append</a>(&amp;<b>mut</b> name, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> name_with_index_suffix));<br />            name<br />        &#125; <b>else</b> &#123;<br />            name_prefix<br />        &#125;;<br /><br />        (index, name)<br />    &#125;;<br /><br />    <b>let</b> <a href="token.md#0x4_token">token</a> &#61; <a href="token.md#0x4_token_Token">Token</a> &#123;<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        index: deprecated_index,<br />        description,<br />        name: deprecated_name,<br />        uri,<br />        mutation_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&amp;object_signer),<br />    &#125;;<br />    <b>move_to</b>(&amp;object_signer, <a href="token.md#0x4_token">token</a>);<br /><br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="royalty.md#0x4_royalty">royalty</a>)) &#123;<br />        <a href="royalty.md#0x4_royalty_init">royalty::init</a>(constructor_ref, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> <a href="royalty.md#0x4_royalty">royalty</a>))<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_token"></a>

## Function `create_token`

Creates a new token object with a unique address and returns the ConstructorRef
for additional specialization.
This takes in the collection object instead of the collection name.
This function must be called if the collection name has been previously changed.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token">create_token</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token">create_token</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;Collection&gt;,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_object">object::create_object</a>(creator_address);<br />    <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(<br />        &amp;constructor_ref,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create"></a>

## Function `create`

Creates a new token object with a unique address and returns the ConstructorRef
for additional specialization.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create">create</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create">create</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection_name: String,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_object">object::create_object</a>(creator_address);<br />    <a href="token.md#0x4_token_create_common">create_common</a>(<br />        &amp;constructor_ref,<br />        creator_address,<br />        collection_name,<br />        description,<br />        name,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_numbered_token_object"></a>

## Function `create_numbered_token_object`

Creates a new token object with a unique address and returns the ConstructorRef
for additional specialization.
The name is created by concatenating the (name_prefix, index, name_suffix).
After flag concurrent_token_v2_enabled is enabled, this function will allow
creating tokens in parallel, from the same collection, while providing sequential names.

This takes in the collection object instead of the collection name.
This function must be called if the collection name has been previously changed.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_numbered_token_object">create_numbered_token_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_with_index_prefix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_with_index_suffix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_numbered_token_object">create_numbered_token_object</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;Collection&gt;,<br />    description: String,<br />    name_with_index_prefix: String,<br />    name_with_index_suffix: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_object">object::create_object</a>(creator_address);<br />    <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(<br />        &amp;constructor_ref,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name_with_index_prefix,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(name_with_index_suffix),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_numbered_token"></a>

## Function `create_numbered_token`

Creates a new token object with a unique address and returns the ConstructorRef
for additional specialization.
The name is created by concatenating the (name_prefix, index, name_suffix).
After flag concurrent_token_v2_enabled is enabled, this function will allow
creating tokens in parallel, from the same collection, while providing sequential names.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_numbered_token">create_numbered_token</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_with_index_prefix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name_with_index_suffix: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_numbered_token">create_numbered_token</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection_name: String,<br />    description: String,<br />    name_with_index_prefix: String,<br />    name_with_index_suffix: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_object">object::create_object</a>(creator_address);<br />    <a href="token.md#0x4_token_create_common">create_common</a>(<br />        &amp;constructor_ref,<br />        creator_address,<br />        collection_name,<br />        description,<br />        name_with_index_prefix,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(name_with_index_suffix),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_named_token_object"></a>

## Function `create_named_token_object`

Creates a new token object from a token name and returns the ConstructorRef for
additional specialization.
This function must be called if the collection name has been previously changed.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_named_token_object">create_named_token_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_named_token_object">create_named_token_object</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;Collection&gt;,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> seed &#61; <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(&amp;<a href="collection.md#0x4_collection_name">collection::name</a>(<a href="collection.md#0x4_collection">collection</a>), &amp;name);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, seed);<br />    <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(<br />        &amp;constructor_ref,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_named_token"></a>

## Function `create_named_token`

Creates a new token object from a token name and returns the ConstructorRef for
additional specialization.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_named_token">create_named_token</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_named_token">create_named_token</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection_name: String,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> seed &#61; <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(&amp;collection_name, &amp;name);<br /><br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, seed);<br />    <a href="token.md#0x4_token_create_common">create_common</a>(<br />        &amp;constructor_ref,<br />        creator_address,<br />        collection_name,<br />        description,<br />        name,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_named_token_from_seed"></a>

## Function `create_named_token_from_seed`

Creates a new token object from a token name and seed.
Returns the ConstructorRef for additional specialization.
This function must be called if the collection name has been previously changed.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_named_token_from_seed">create_named_token_from_seed</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, seed: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_named_token_from_seed">create_named_token_from_seed</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;Collection&gt;,<br />    description: String,<br />    name: String,<br />    seed: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> seed &#61; <a href="token.md#0x4_token_create_token_name_with_seed">create_token_name_with_seed</a>(&amp;<a href="collection.md#0x4_collection_name">collection::name</a>(<a href="collection.md#0x4_collection">collection</a>), &amp;name, &amp;seed);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, seed);<br />    <a href="token.md#0x4_token_create_common_with_collection">create_common_with_collection</a>(&amp;constructor_ref, <a href="collection.md#0x4_collection">collection</a>, description, name, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="royalty.md#0x4_royalty">royalty</a>, uri);<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_from_account"></a>

## Function `create_from_account`

DEPRECATED: Use <code>create</code> instead for identical behavior.

Creates a new token object from an account GUID and returns the ConstructorRef for
additional specialization.


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_from_account">create_from_account</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_from_account">create_from_account</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection_name: String,<br />    description: String,<br />    name: String,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,<br />    uri: String,<br />): ConstructorRef &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> constructor_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_from_account">object::create_object_from_account</a>(creator);<br />    <a href="token.md#0x4_token_create_common">create_common</a>(<br />        &amp;constructor_ref,<br />        creator_address,<br />        collection_name,<br />        description,<br />        name,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        <a href="royalty.md#0x4_royalty">royalty</a>,<br />        uri<br />    );<br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_token_address"></a>

## Function `create_token_address`

Generates the token&apos;s address based upon the creator&apos;s address, the collection&apos;s name and the token&apos;s name.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_address">create_token_address</a>(creator: &amp;<b>address</b>, <a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_address">create_token_address</a>(creator: &amp;<b>address</b>, <a href="collection.md#0x4_collection">collection</a>: &amp;String, name: &amp;String): <b>address</b> &#123;<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(creator, <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>, name))<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_token_address_with_seed"></a>

## Function `create_token_address_with_seed`

Generates the token&apos;s address based upon the creator&apos;s address, the collection object and the token&apos;s name and seed.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_address_with_seed">create_token_address_with_seed</a>(creator: <b>address</b>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, seed: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_address_with_seed">create_token_address_with_seed</a>(creator: <b>address</b>, <a href="collection.md#0x4_collection">collection</a>: String, name: String, seed: String): <b>address</b> &#123;<br />    <b>let</b> seed &#61; <a href="token.md#0x4_token_create_token_name_with_seed">create_token_name_with_seed</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, &amp;name, &amp;seed);<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(&amp;creator, seed)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_token_seed"></a>

## Function `create_token_seed`

Named objects are derived from a seed, the token&apos;s seed is its name appended to the collection&apos;s name.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>: &amp;String, name: &amp;String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(name) &lt;&#61; <a href="token.md#0x4_token_MAX_TOKEN_NAME_LENGTH">MAX_TOKEN_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_ETOKEN_NAME_TOO_LONG">ETOKEN_NAME_TOO_LONG</a>));<br />    <b>let</b> seed &#61; &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="collection.md#0x4_collection">collection</a>);<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, b&quot;::&quot;);<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seed, &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(name));<br />    seed<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_create_token_name_with_seed"></a>

## Function `create_token_name_with_seed`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_name_with_seed">create_token_name_with_seed</a>(<a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, seed: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_name_with_seed">create_token_name_with_seed</a>(<a href="collection.md#0x4_collection">collection</a>: &amp;String, name: &amp;String, seed: &amp;String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(seed) &lt;&#61; <a href="token.md#0x4_token_MAX_TOKEN_SEED_LENGTH">MAX_TOKEN_SEED_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_ESEED_TOO_LONG">ESEED_TOO_LONG</a>));<br />    <b>let</b> seeds &#61; <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>, name);<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> seeds, &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(seed));<br />    seeds<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_mutator_ref">generate_mutator_ref</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_mutator_ref">generate_mutator_ref</a>(ref: &amp;ConstructorRef): <a href="token.md#0x4_token_MutatorRef">MutatorRef</a> &#123;<br />    <b>let</b> <a href="../../aptos-framework/doc/object.md#0x1_object">object</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(ref);<br />    <a href="token.md#0x4_token_MutatorRef">MutatorRef</a> &#123; self: <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a BurnRef, which gates the ability to burn the given token.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_burn_ref">generate_burn_ref</a>(ref: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="token.md#0x4_token_BurnRef">token::BurnRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_burn_ref">generate_burn_ref</a>(ref: &amp;ConstructorRef): <a href="token.md#0x4_token_BurnRef">BurnRef</a> &#123;<br />    <b>let</b> (inner, self) &#61; <b>if</b> (<a href="../../aptos-framework/doc/object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(ref)) &#123;<br />        <b>let</b> delete_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(ref);<br />        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(delete_ref), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(ref);<br />        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(addr))<br />    &#125;;<br />    <a href="token.md#0x4_token_BurnRef">BurnRef</a> &#123; self, inner &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_address_from_burn_ref"></a>

## Function `address_from_burn_ref`

Extracts the tokens address from a BurnRef.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_address_from_burn_ref">address_from_burn_ref</a>(ref: &amp;<a href="token.md#0x4_token_BurnRef">token::BurnRef</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_address_from_burn_ref">address_from_burn_ref</a>(ref: &amp;<a href="token.md#0x4_token_BurnRef">BurnRef</a>): <b>address</b> &#123;<br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;ref.inner)) &#123;<br />        <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_delete_ref">object::address_from_delete_ref</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;ref.inner))<br />    &#125; <b>else</b> &#123;<br />        &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;ref.self)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_borrow"></a>

## Function `borrow`



<pre><code><b>fun</b> <a href="token.md#0x4_token_borrow">borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="token.md#0x4_token_Token">token::Token</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="token.md#0x4_token_borrow">borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;Object&lt;T&gt;): &amp;<a href="token.md#0x4_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <b>let</b> token_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(token_address),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x4_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>),<br />    );<br />    <b>borrow_global</b>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(token_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_creator"></a>

## Function `creator`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_creator">creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_creator">creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <a href="collection.md#0x4_collection_creator">collection::creator</a>(<a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).<a href="collection.md#0x4_collection">collection</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_collection_name"></a>

## Function `collection_name`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_collection_name">collection_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_collection_name">collection_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <a href="collection.md#0x4_collection_name">collection::name</a>(<a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).<a href="collection.md#0x4_collection">collection</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_collection_object"></a>

## Function `collection_object`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_collection_object">collection_object</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_collection_object">collection_object</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): Object&lt;Collection&gt; <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).<a href="collection.md#0x4_collection">collection</a><br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_description"></a>

## Function `description`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_description">description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_description">description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).description<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_name"></a>

## Function `name`

Avoid this method in the same transaction as the token is minted
as that would prohibit transactions to be executed in parallel.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_name">name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_name">name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a>, <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> &#123;<br />    <b>let</b> token_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="token.md#0x4_token">token</a>);<br />    <b>if</b> (<b>exists</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(token_address)) &#123;<br />        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read_derived_string">aggregator_v2::read_derived_string</a>(&amp;<b>borrow_global</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(token_address).name)<br />    &#125; <b>else</b> &#123;<br />        <a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).name<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_uri"></a>

## Function `uri`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_uri">uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_uri">uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).uri<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_royalty"></a>

## Function `royalty`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty">royalty</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty">royalty</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): Option&lt;Royalty&gt; <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>);<br />    <b>let</b> <a href="royalty.md#0x4_royalty">royalty</a> &#61; <a href="royalty.md#0x4_royalty_get">royalty::get</a>(<a href="token.md#0x4_token">token</a>);<br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="royalty.md#0x4_royalty">royalty</a>)) &#123;<br />        <a href="royalty.md#0x4_royalty">royalty</a><br />    &#125; <b>else</b> &#123;<br />        <b>let</b> creator &#61; <a href="token.md#0x4_token_creator">creator</a>(<a href="token.md#0x4_token">token</a>);<br />        <b>let</b> collection_name &#61; <a href="token.md#0x4_token_collection_name">collection_name</a>(<a href="token.md#0x4_token">token</a>);<br />        <b>let</b> collection_address &#61; <a href="collection.md#0x4_collection_create_collection_address">collection::create_collection_address</a>(&amp;creator, &amp;collection_name);<br />        <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;(collection_address);<br />        <a href="royalty.md#0x4_royalty_get">royalty::get</a>(<a href="collection.md#0x4_collection">collection</a>)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_index"></a>

## Function `index`

Avoid this method in the same transaction as the token is minted
as that would prohibit transactions to be executed in parallel.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x4_token_index">index</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_index">index</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): u64 <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a>, <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> &#123;<br />    <b>let</b> token_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="token.md#0x4_token">token</a>);<br />    <b>if</b> (<b>exists</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(token_address)) &#123;<br />        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read_snapshot">aggregator_v2::read_snapshot</a>(&amp;<b>borrow_global</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(token_address).index)<br />    &#125; <b>else</b> &#123;<br />        <a href="token.md#0x4_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).index<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>fun</b> <a href="token.md#0x4_token_borrow_mut">borrow_mut</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>): &amp;<b>mut</b> <a href="token.md#0x4_token_Token">token::Token</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="token.md#0x4_token_borrow_mut">borrow_mut</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>): &amp;<b>mut</b> <a href="token.md#0x4_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(mutator_ref.self),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x4_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>),<br />    );<br />    <b>borrow_global_mut</b>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(mutator_ref.self)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_burn"></a>

## Function `burn`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_burn">burn</a>(burn_ref: <a href="token.md#0x4_token_BurnRef">token::BurnRef</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_burn">burn</a>(burn_ref: <a href="token.md#0x4_token_BurnRef">BurnRef</a>) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a>, <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> &#123;<br />    <b>let</b> (addr, previous_owner) &#61; <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;burn_ref.inner)) &#123;<br />        <b>let</b> delete_ref &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> burn_ref.inner);<br />        <b>let</b> addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_delete_ref">object::address_from_delete_ref</a>(&amp;delete_ref);<br />        <b>let</b> previous_owner &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_owner">object::owner</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(addr));<br />        <a href="../../aptos-framework/doc/object.md#0x1_object_delete">object::delete</a>(delete_ref);<br />        (addr, previous_owner)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> burn_ref.self);<br />        <b>let</b> previous_owner &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_owner">object::owner</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(addr));<br />        (addr, previous_owner)<br />    &#125;;<br /><br />    <b>if</b> (<a href="royalty.md#0x4_royalty_exists_at">royalty::exists_at</a>(addr)) &#123;<br />        <a href="royalty.md#0x4_royalty_delete">royalty::delete</a>(addr)<br />    &#125;;<br /><br />    <b>let</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        index: deprecated_index,<br />        description: _,<br />        name: _,<br />        uri: _,<br />        mutation_events,<br />    &#125; &#61; <b>move_from</b>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(addr);<br /><br />    <b>let</b> index &#61; <b>if</b> (<b>exists</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(addr)) &#123;<br />        <b>let</b> <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> &#123;<br />            index,<br />            name: _,<br />        &#125; &#61; <b>move_from</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(addr);<br />        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read_snapshot">aggregator_v2::read_snapshot</a>(&amp;index)<br />    &#125; <b>else</b> &#123;<br />        deprecated_index<br />    &#125;;<br /><br />    <a href="../../aptos-framework/doc/event.md#0x1_event_destroy_handle">event::destroy_handle</a>(mutation_events);<br />    <a href="collection.md#0x4_collection_decrement_supply">collection::decrement_supply</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, addr, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(index), previous_owner);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_description">set_description</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_description">set_description</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>, description: String) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;description) &lt;&#61; <a href="token.md#0x4_token_MAX_DESCRIPTION_LENGTH">MAX_DESCRIPTION_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_EDESCRIPTION_TOO_LONG">EDESCRIPTION_TOO_LONG</a>));<br />    <b>let</b> <a href="token.md#0x4_token">token</a> &#61; <a href="token.md#0x4_token_borrow_mut">borrow_mut</a>(mutator_ref);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x4_token_Mutation">Mutation</a> &#123;<br />            token_address: mutator_ref.self,<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;description&quot;),<br />            old_value: <a href="token.md#0x4_token">token</a>.description,<br />            new_value: description<br />        &#125;)<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> <a href="token.md#0x4_token">token</a>.mutation_events,<br />        <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> &#123;<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;description&quot;),<br />            old_value: <a href="token.md#0x4_token">token</a>.description,<br />            new_value: description<br />        &#125;,<br />    );<br />    <a href="token.md#0x4_token">token</a>.description &#61; description;<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_set_name"></a>

## Function `set_name`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_name">set_name</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_name">set_name</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>, name: String) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a>, <a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="token.md#0x4_token_MAX_TOKEN_NAME_LENGTH">MAX_TOKEN_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_ETOKEN_NAME_TOO_LONG">ETOKEN_NAME_TOO_LONG</a>));<br /><br />    <b>let</b> <a href="token.md#0x4_token">token</a> &#61; <a href="token.md#0x4_token_borrow_mut">borrow_mut</a>(mutator_ref);<br /><br />    <b>let</b> old_name &#61; <b>if</b> (<b>exists</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(mutator_ref.self)) &#123;<br />        <b>let</b> token_concurrent &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x4_token_TokenIdentifiers">TokenIdentifiers</a>&gt;(mutator_ref.self);<br />        <b>let</b> old_name &#61; <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read_derived_string">aggregator_v2::read_derived_string</a>(&amp;token_concurrent.name);<br />        token_concurrent.name &#61; <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_derived_string">aggregator_v2::create_derived_string</a>(name);<br />        old_name<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> old_name &#61; <a href="token.md#0x4_token">token</a>.name;<br />        <a href="token.md#0x4_token">token</a>.name &#61; name;<br />        old_name<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x4_token_Mutation">Mutation</a> &#123;<br />            token_address: mutator_ref.self,<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;name&quot;),<br />            old_value: old_name,<br />            new_value: name<br />        &#125;)<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> <a href="token.md#0x4_token">token</a>.mutation_events,<br />        <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> &#123;<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;name&quot;),<br />            old_value: old_name,<br />            new_value: name<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_token_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_uri">set_uri</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_uri">set_uri</a>(mutator_ref: &amp;<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>, uri: String) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="token.md#0x4_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="token.md#0x4_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>let</b> <a href="token.md#0x4_token">token</a> &#61; <a href="token.md#0x4_token_borrow_mut">borrow_mut</a>(mutator_ref);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x4_token_Mutation">Mutation</a> &#123;<br />            token_address: mutator_ref.self,<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;uri&quot;),<br />            old_value: <a href="token.md#0x4_token">token</a>.uri,<br />            new_value: uri,<br />        &#125;)<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> <a href="token.md#0x4_token">token</a>.mutation_events,<br />        <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> &#123;<br />            mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;uri&quot;),<br />            old_value: <a href="token.md#0x4_token">token</a>.uri,<br />            new_value: uri,<br />        &#125;,<br />    );<br />    <a href="token.md#0x4_token">token</a>.uri &#61; uri;<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
