
<a id="0x4_aptos_token"></a>

# Module `0x4::aptos_token`

This defines a minimally viable token for no&#45;code solutions akin to the original token at
0x3::token module.
The key features are:
&#42; Base token and collection features
&#42; Creator definable mutability for tokens
&#42; Creator&#45;based freezing of tokens
&#42; Standard object&#45;based transfer and events
&#42; Metadata property type


-  [Resource `AptosCollection`](#0x4_aptos_token_AptosCollection)
-  [Resource `AptosToken`](#0x4_aptos_token_AptosToken)
-  [Constants](#@Constants_0)
-  [Function `create_collection`](#0x4_aptos_token_create_collection)
-  [Function `create_collection_object`](#0x4_aptos_token_create_collection_object)
-  [Function `mint`](#0x4_aptos_token_mint)
-  [Function `mint_token_object`](#0x4_aptos_token_mint_token_object)
-  [Function `mint_soul_bound`](#0x4_aptos_token_mint_soul_bound)
-  [Function `mint_soul_bound_token_object`](#0x4_aptos_token_mint_soul_bound_token_object)
-  [Function `mint_internal`](#0x4_aptos_token_mint_internal)
-  [Function `borrow`](#0x4_aptos_token_borrow)
-  [Function `are_properties_mutable`](#0x4_aptos_token_are_properties_mutable)
-  [Function `is_burnable`](#0x4_aptos_token_is_burnable)
-  [Function `is_freezable_by_creator`](#0x4_aptos_token_is_freezable_by_creator)
-  [Function `is_mutable_description`](#0x4_aptos_token_is_mutable_description)
-  [Function `is_mutable_name`](#0x4_aptos_token_is_mutable_name)
-  [Function `is_mutable_uri`](#0x4_aptos_token_is_mutable_uri)
-  [Function `authorized_borrow`](#0x4_aptos_token_authorized_borrow)
-  [Function `burn`](#0x4_aptos_token_burn)
-  [Function `freeze_transfer`](#0x4_aptos_token_freeze_transfer)
-  [Function `unfreeze_transfer`](#0x4_aptos_token_unfreeze_transfer)
-  [Function `set_description`](#0x4_aptos_token_set_description)
-  [Function `set_name`](#0x4_aptos_token_set_name)
-  [Function `set_uri`](#0x4_aptos_token_set_uri)
-  [Function `add_property`](#0x4_aptos_token_add_property)
-  [Function `add_typed_property`](#0x4_aptos_token_add_typed_property)
-  [Function `remove_property`](#0x4_aptos_token_remove_property)
-  [Function `update_property`](#0x4_aptos_token_update_property)
-  [Function `update_typed_property`](#0x4_aptos_token_update_typed_property)
-  [Function `collection_object`](#0x4_aptos_token_collection_object)
-  [Function `borrow_collection`](#0x4_aptos_token_borrow_collection)
-  [Function `is_mutable_collection_description`](#0x4_aptos_token_is_mutable_collection_description)
-  [Function `is_mutable_collection_royalty`](#0x4_aptos_token_is_mutable_collection_royalty)
-  [Function `is_mutable_collection_uri`](#0x4_aptos_token_is_mutable_collection_uri)
-  [Function `is_mutable_collection_token_description`](#0x4_aptos_token_is_mutable_collection_token_description)
-  [Function `is_mutable_collection_token_name`](#0x4_aptos_token_is_mutable_collection_token_name)
-  [Function `is_mutable_collection_token_uri`](#0x4_aptos_token_is_mutable_collection_token_uri)
-  [Function `is_mutable_collection_token_properties`](#0x4_aptos_token_is_mutable_collection_token_properties)
-  [Function `are_collection_tokens_burnable`](#0x4_aptos_token_are_collection_tokens_burnable)
-  [Function `are_collection_tokens_freezable`](#0x4_aptos_token_are_collection_tokens_freezable)
-  [Function `authorized_borrow_collection`](#0x4_aptos_token_authorized_borrow_collection)
-  [Function `set_collection_description`](#0x4_aptos_token_set_collection_description)
-  [Function `set_collection_royalties`](#0x4_aptos_token_set_collection_royalties)
-  [Function `set_collection_royalties_call`](#0x4_aptos_token_set_collection_royalties_call)
-  [Function `set_collection_uri`](#0x4_aptos_token_set_collection_uri)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="collection.md#0x4_collection">0x4::collection</a>;<br /><b>use</b> <a href="property_map.md#0x4_property_map">0x4::property_map</a>;<br /><b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;<br /><b>use</b> <a href="token.md#0x4_token">0x4::token</a>;<br /></code></pre>



<a id="0x4_aptos_token_AptosCollection"></a>

## Resource `AptosCollection`

Storage state for managing the no&#45;code Collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutator_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>&gt;</code>
</dt>
<dd>
 Used to mutate collection fields
</dd>
<dt>
<code>royalty_mutator_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_MutatorRef">royalty::MutatorRef</a>&gt;</code>
</dt>
<dd>
 Used to mutate royalties
</dd>
<dt>
<code>mutable_description: bool</code>
</dt>
<dd>
 Determines if the creator can mutate the collection&apos;s description
</dd>
<dt>
<code>mutable_uri: bool</code>
</dt>
<dd>
 Determines if the creator can mutate the collection&apos;s uri
</dd>
<dt>
<code>mutable_token_description: bool</code>
</dt>
<dd>
 Determines if the creator can mutate token descriptions
</dd>
<dt>
<code>mutable_token_name: bool</code>
</dt>
<dd>
 Determines if the creator can mutate token names
</dd>
<dt>
<code>mutable_token_properties: bool</code>
</dt>
<dd>
 Determines if the creator can mutate token properties
</dd>
<dt>
<code>mutable_token_uri: bool</code>
</dt>
<dd>
 Determines if the creator can mutate token uris
</dd>
<dt>
<code>tokens_burnable_by_creator: bool</code>
</dt>
<dd>
 Determines if the creator can burn tokens
</dd>
<dt>
<code>tokens_freezable_by_creator: bool</code>
</dt>
<dd>
 Determines if the creator can freeze tokens
</dd>
</dl>


</details>

<a id="0x4_aptos_token_AptosToken"></a>

## Resource `AptosToken`

Storage state for managing the no&#45;code Token.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]<br /><b>struct</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token.md#0x4_token_BurnRef">token::BurnRef</a>&gt;</code>
</dt>
<dd>
 Used to burn.
</dd>
<dt>
<code>transfer_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/doc/object.md#0x1_object_TransferRef">object::TransferRef</a>&gt;</code>
</dt>
<dd>
 Used to control freeze.
</dd>
<dt>
<code>mutator_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>&gt;</code>
</dt>
<dd>
 Used to mutate fields
</dd>
<dt>
<code>property_mutator_ref: <a href="property_map.md#0x4_property_map_MutatorRef">property_map::MutatorRef</a></code>
</dt>
<dd>
 Used to mutate properties
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST"></a>

The collection does not exist


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x4_aptos_token_EFIELD_NOT_MUTABLE"></a>

The field being changed is not mutable


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x4_aptos_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ENOT_CREATOR">ENOT_CREATOR</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x4_aptos_token_ETOKEN_DOES_NOT_EXIST"></a>

The token does not exist


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x4_aptos_token_EPROPERTIES_NOT_MUTABLE"></a>

The property map being mutated is not mutable


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x4_aptos_token_ETOKEN_NOT_BURNABLE"></a>

The token being burned is not burnable


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ETOKEN_NOT_BURNABLE">ETOKEN_NOT_BURNABLE</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x4_aptos_token_create_collection"></a>

## Function `create_collection`

Create a new collection


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection">create_collection</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection">create_collection</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    description: String,<br />    max_supply: u64,<br />    name: String,<br />    uri: String,<br />    mutable_description: bool,<br />    mutable_royalty: bool,<br />    mutable_uri: bool,<br />    mutable_token_description: bool,<br />    mutable_token_name: bool,<br />    mutable_token_properties: bool,<br />    mutable_token_uri: bool,<br />    tokens_burnable_by_creator: bool,<br />    tokens_freezable_by_creator: bool,<br />    royalty_numerator: u64,<br />    royalty_denominator: u64,<br />) &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_create_collection_object">create_collection_object</a>(<br />        creator,<br />        description,<br />        max_supply,<br />        name,<br />        uri,<br />        mutable_description,<br />        mutable_royalty,<br />        mutable_uri,<br />        mutable_token_description,<br />        mutable_token_name,<br />        mutable_token_properties,<br />        mutable_token_uri,<br />        tokens_burnable_by_creator,<br />        tokens_freezable_by_creator,<br />        royalty_numerator,<br />        royalty_denominator<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_create_collection_object"></a>

## Function `create_collection_object`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection_object">create_collection_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection_object">create_collection_object</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    description: String,<br />    max_supply: u64,<br />    name: String,<br />    uri: String,<br />    mutable_description: bool,<br />    mutable_royalty: bool,<br />    mutable_uri: bool,<br />    mutable_token_description: bool,<br />    mutable_token_name: bool,<br />    mutable_token_properties: bool,<br />    mutable_token_uri: bool,<br />    tokens_burnable_by_creator: bool,<br />    tokens_freezable_by_creator: bool,<br />    royalty_numerator: u64,<br />    royalty_denominator: u64,<br />): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt; &#123;<br />    <b>let</b> creator_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> <a href="royalty.md#0x4_royalty">royalty</a> &#61; <a href="royalty.md#0x4_royalty_create">royalty::create</a>(royalty_numerator, royalty_denominator, creator_addr);<br />    <b>let</b> constructor_ref &#61; <a href="collection.md#0x4_collection_create_fixed_collection">collection::create_fixed_collection</a>(<br />        creator,<br />        description,<br />        max_supply,<br />        name,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="royalty.md#0x4_royalty">royalty</a>),<br />        uri,<br />    );<br /><br />    <b>let</b> object_signer &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&amp;constructor_ref);<br />    <b>let</b> mutator_ref &#61; <b>if</b> (mutable_description &#124;&#124; mutable_uri) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="collection.md#0x4_collection_generate_mutator_ref">collection::generate_mutator_ref</a>(&amp;constructor_ref))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    <b>let</b> royalty_mutator_ref &#61; <b>if</b> (mutable_royalty) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="royalty.md#0x4_royalty_generate_mutator_ref">royalty::generate_mutator_ref</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(&amp;constructor_ref)))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    <b>let</b> aptos_collection &#61; <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />        mutator_ref,<br />        royalty_mutator_ref,<br />        mutable_description,<br />        mutable_uri,<br />        mutable_token_description,<br />        mutable_token_name,<br />        mutable_token_properties,<br />        mutable_token_uri,<br />        tokens_burnable_by_creator,<br />        tokens_freezable_by_creator,<br />    &#125;;<br />    <b>move_to</b>(&amp;object_signer, aptos_collection);<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&amp;constructor_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_mint"></a>

## Function `mint`

With an existing collection, directly mint a viable token into the creators account.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint">mint</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint">mint</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: String,<br />    description: String,<br />    name: String,<br />    uri: String,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_mint_token_object">mint_token_object</a>(creator, <a href="collection.md#0x4_collection">collection</a>, description, name, uri, property_keys, property_types, property_values);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_mint_token_object"></a>

## Function `mint_token_object`

Mint a token into an existing collection, and retrieve the object / address of the token.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_token_object">mint_token_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_token_object">mint_token_object</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: String,<br />    description: String,<br />    name: String,<br />    uri: String,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt; <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> constructor_ref &#61; <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(<br />        creator,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name,<br />        uri,<br />        property_keys,<br />        property_types,<br />        property_values,<br />    );<br /><br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator, &amp;<a href="collection.md#0x4_collection">collection</a>);<br /><br />    // If tokens are freezable, add a transfer ref <b>to</b> be able <b>to</b> <b>freeze</b> transfers<br />    <b>let</b> freezable_by_creator &#61; <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="collection.md#0x4_collection">collection</a>);<br />    <b>if</b> (freezable_by_creator) &#123;<br />        <b>let</b> aptos_token_addr &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(&amp;constructor_ref);<br />        <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(aptos_token_addr);<br />        <b>let</b> transfer_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&amp;constructor_ref);<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(&amp;<b>mut</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref, transfer_ref);<br />    &#125;;<br /><br />    <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&amp;constructor_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound"></a>

## Function `mint_soul_bound`

With an existing collection, directly mint a soul bound token into the recipient&apos;s account.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound">mint_soul_bound</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, soul_bound_to: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound">mint_soul_bound</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: String,<br />    description: String,<br />    name: String,<br />    uri: String,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    soul_bound_to: <b>address</b>,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound_token_object">mint_soul_bound_token_object</a>(<br />        creator,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name,<br />        uri,<br />        property_keys,<br />        property_types,<br />        property_values,<br />        soul_bound_to<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound_token_object"></a>

## Function `mint_soul_bound_token_object`

With an existing collection, directly mint a soul bound token into the recipient&apos;s account.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound_token_object">mint_soul_bound_token_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, soul_bound_to: <b>address</b>): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound_token_object">mint_soul_bound_token_object</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: String,<br />    description: String,<br />    name: String,<br />    uri: String,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    soul_bound_to: <b>address</b>,<br />): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt; <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> constructor_ref &#61; <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(<br />        creator,<br />        <a href="collection.md#0x4_collection">collection</a>,<br />        description,<br />        name,<br />        uri,<br />        property_keys,<br />        property_types,<br />        property_values,<br />    );<br /><br />    <b>let</b> transfer_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&amp;constructor_ref);<br />    <b>let</b> linear_transfer_ref &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_linear_transfer_ref">object::generate_linear_transfer_ref</a>(&amp;transfer_ref);<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_transfer_with_ref">object::transfer_with_ref</a>(linear_transfer_ref, soul_bound_to);<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(&amp;transfer_ref);<br /><br />    <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&amp;constructor_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_mint_internal"></a>

## Function `mint_internal`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: String,<br />    description: String,<br />    name: String,<br />    uri: String,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />): ConstructorRef <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> constructor_ref &#61; <a href="token.md#0x4_token_create">token::create</a>(creator, <a href="collection.md#0x4_collection">collection</a>, description, name, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), uri);<br /><br />    <b>let</b> object_signer &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&amp;constructor_ref);<br /><br />    <b>let</b> collection_obj &#61; <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator, &amp;<a href="collection.md#0x4_collection">collection</a>);<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;collection_obj);<br /><br />    <b>let</b> mutator_ref &#61; <b>if</b> (<br />        <a href="collection.md#0x4_collection">collection</a>.mutable_token_description<br />            &#124;&#124; <a href="collection.md#0x4_collection">collection</a>.mutable_token_name<br />            &#124;&#124; <a href="collection.md#0x4_collection">collection</a>.mutable_token_uri<br />    ) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="token.md#0x4_token_generate_mutator_ref">token::generate_mutator_ref</a>(&amp;constructor_ref))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    <b>let</b> burn_ref &#61; <b>if</b> (<a href="collection.md#0x4_collection">collection</a>.tokens_burnable_by_creator) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="token.md#0x4_token_generate_burn_ref">token::generate_burn_ref</a>(&amp;constructor_ref))<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;;<br /><br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />        burn_ref,<br />        transfer_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        mutator_ref,<br />        property_mutator_ref: <a href="property_map.md#0x4_property_map_generate_mutator_ref">property_map::generate_mutator_ref</a>(&amp;constructor_ref),<br />    &#125;;<br />    <b>move_to</b>(&amp;object_signer, <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>);<br /><br />    <b>let</b> properties &#61; <a href="property_map.md#0x4_property_map_prepare_input">property_map::prepare_input</a>(property_keys, property_types, property_values);<br />    <a href="property_map.md#0x4_property_map_init">property_map::init</a>(&amp;constructor_ref, properties);<br /><br />    constructor_ref<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_borrow"></a>

## Function `borrow`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;Object&lt;T&gt;): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> token_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(token_address),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>),<br />    );<br />    <b>borrow_global</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(token_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_are_properties_mutable"></a>

## Function `are_properties_mutable`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> <a href="collection.md#0x4_collection">collection</a> &#61; <a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>);<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_token_properties<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_burnable"></a>

## Function `is_burnable`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_burnable">is_burnable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_burnable">is_burnable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(&amp;<a href="token.md#0x4_token">token</a>).burn_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_freezable_by_creator"></a>

## Function `is_freezable_by_creator`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_freezable_by_creator">is_freezable_by_creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_freezable_by_creator">is_freezable_by_creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_description"></a>

## Function `is_mutable_description`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_description">is_mutable_description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_description">is_mutable_description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_description">is_mutable_collection_token_description</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_name"></a>

## Function `is_mutable_name`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_name">is_mutable_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_name">is_mutable_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_name">is_mutable_collection_token_name</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_uri"></a>

## Function `is_mutable_uri`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_uri">is_mutable_uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_uri">is_mutable_uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_uri">is_mutable_collection_token_uri</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow"></a>

## Function `authorized_borrow`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;Object&lt;T&gt;, creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> token_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(token_address),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>),<br />    );<br /><br />    <b>assert</b>!(<br />        <a href="token.md#0x4_token_creator">token::creator</a>(&#42;<a href="token.md#0x4_token">token</a>) &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_ENOT_CREATOR">ENOT_CREATOR</a>),<br />    );<br />    <b>borrow_global</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(token_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_burn"></a>

## Function `burn`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_burn">burn</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_burn">burn</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.burn_ref),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_ETOKEN_NOT_BURNABLE">ETOKEN_NOT_BURNABLE</a>),<br />    );<br />    <b>move</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <b>move_from</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&amp;<a href="token.md#0x4_token">token</a>));<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />        burn_ref,<br />        transfer_ref: _,<br />        mutator_ref: _,<br />        property_mutator_ref,<br />    &#125; &#61; <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>;<br />    <a href="property_map.md#0x4_property_map_burn">property_map::burn</a>(property_mutator_ref);<br />    <a href="token.md#0x4_token_burn">token::burn</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> burn_ref));<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_freeze_transfer"></a>

## Function `freeze_transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_freeze_transfer">freeze_transfer</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_freeze_transfer">freeze_transfer</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))<br />            &amp;&amp; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref));<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_unfreeze_transfer"></a>

## Function `unfreeze_transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_unfreeze_transfer">unfreeze_transfer</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_unfreeze_transfer">unfreeze_transfer</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))<br />            &amp;&amp; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_enable_ungated_transfer">object::enable_ungated_transfer</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref));<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_description">set_description</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_description">set_description</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    description: String,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_is_mutable_description">is_mutable_description</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <a href="token.md#0x4_token_set_description">token::set_description</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.mutator_ref), description);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_name"></a>

## Function `set_name`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_name">set_name</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_name">set_name</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    name: String,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_is_mutable_name">is_mutable_name</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <a href="token.md#0x4_token_set_name">token::set_name</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.mutator_ref), name);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_uri">set_uri</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_uri">set_uri</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    uri: String,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_is_mutable_uri">is_mutable_uri</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <a href="token.md#0x4_token_set_uri">token::set_uri</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.mutator_ref), uri);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_add_property"></a>

## Function `add_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_property">add_property</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_property">add_property</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    key: String,<br />    type: String,<br />    value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),<br />    );<br /><br />    <a href="property_map.md#0x4_property_map_add">property_map::add</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, key, type, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_add_typed_property"></a>

## Function `add_typed_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_typed_property">add_typed_property</a>&lt;T: key, V: drop&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: V)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_typed_property">add_typed_property</a>&lt;T: key, V: drop&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    key: String,<br />    value: V,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),<br />    );<br /><br />    <a href="property_map.md#0x4_property_map_add_typed">property_map::add_typed</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, key, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_remove_property"></a>

## Function `remove_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_remove_property">remove_property</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_remove_property">remove_property</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    key: String,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),<br />    );<br /><br />    <a href="property_map.md#0x4_property_map_remove">property_map::remove</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, &amp;key);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_update_property"></a>

## Function `update_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_property">update_property</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_property">update_property</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    key: String,<br />    type: String,<br />    value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),<br />    );<br /><br />    <a href="property_map.md#0x4_property_map_update">property_map::update</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, &amp;key, type, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_update_typed_property"></a>

## Function `update_typed_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_typed_property">update_typed_property</a>&lt;T: key, V: drop&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: V)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_typed_property">update_typed_property</a>&lt;T: key, V: drop&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,<br />    key: String,<br />    value: V,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> &#123;<br />    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&amp;<a href="token.md#0x4_token">token</a>, creator);<br />    <b>assert</b>!(<br />        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),<br />    );<br /><br />    <a href="property_map.md#0x4_property_map_update_typed">property_map::update_typed</a>(&amp;<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, &amp;key, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_collection_object"></a>

## Function `collection_object`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: &amp;String): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt; &#123;<br />    <b>let</b> collection_addr &#61; <a href="collection.md#0x4_collection_create_collection_address">collection::create_collection_address</a>(&amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator), name);<br />    <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_borrow_collection"></a>

## Function `borrow_collection`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &amp;Object&lt;T&gt;): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> collection_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_address),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),<br />    );<br />    <b>borrow_global</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_description"></a>

## Function `is_mutable_collection_description`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_description">is_mutable_collection_description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_description">is_mutable_collection_description</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_description<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_royalty"></a>

## Function `is_mutable_collection_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_royalty">is_mutable_collection_royalty</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_royalty">is_mutable_collection_royalty</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).royalty_mutator_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_uri"></a>

## Function `is_mutable_collection_uri`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_uri">is_mutable_collection_uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_uri">is_mutable_collection_uri</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_uri<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_description"></a>

## Function `is_mutable_collection_token_description`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_description">is_mutable_collection_token_description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_description">is_mutable_collection_token_description</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_token_description<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_name"></a>

## Function `is_mutable_collection_token_name`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_name">is_mutable_collection_token_name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_name">is_mutable_collection_token_name</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_token_name<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_uri"></a>

## Function `is_mutable_collection_token_uri`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_uri">is_mutable_collection_token_uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_uri">is_mutable_collection_token_uri</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_token_uri<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_properties"></a>

## Function `is_mutable_collection_token_properties`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_properties">is_mutable_collection_token_properties</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_properties">is_mutable_collection_token_properties</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).mutable_token_properties<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_burnable"></a>

## Function `are_collection_tokens_burnable`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_burnable">are_collection_tokens_burnable</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_burnable">are_collection_tokens_burnable</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).tokens_burnable_by_creator<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_freezable"></a>

## Function `are_collection_tokens_freezable`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>&lt;T: key&gt;(<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>).tokens_freezable_by_creator<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow_collection"></a>

## Function `authorized_borrow_collection`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &amp;<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &amp;Object&lt;T&gt;, creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &amp;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> collection_address &#61; <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_address),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),<br />    );<br />    <b>assert</b>!(<br />        <a href="collection.md#0x4_collection_creator">collection::creator</a>(&#42;<a href="collection.md#0x4_collection">collection</a>) &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_ENOT_CREATOR">ENOT_CREATOR</a>),<br />    );<br />    <b>borrow_global</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_description"></a>

## Function `set_collection_description`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_description">set_collection_description</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_description">set_collection_description</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />    description: String,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> aptos_collection &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, creator);<br />    <b>assert</b>!(<br />        aptos_collection.mutable_description,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <a href="collection.md#0x4_collection_set_description">collection::set_description</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;aptos_collection.mutator_ref), description);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties"></a>

## Function `set_collection_royalties`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties">set_collection_royalties</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties">set_collection_royalties</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />    <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> aptos_collection &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, creator);<br />    <b>assert</b>!(<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;aptos_collection.royalty_mutator_ref),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <a href="royalty.md#0x4_royalty_update">royalty::update</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;aptos_collection.royalty_mutator_ref), <a href="royalty.md#0x4_royalty">royalty</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties_call"></a>

## Function `set_collection_royalties_call`



<pre><code>entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties_call">set_collection_royalties_call</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, royalty_numerator: u64, royalty_denominator: u64, payee_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties_call">set_collection_royalties_call</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />    royalty_numerator: u64,<br />    royalty_denominator: u64,<br />    payee_address: <b>address</b>,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> <a href="royalty.md#0x4_royalty">royalty</a> &#61; <a href="royalty.md#0x4_royalty_create">royalty::create</a>(royalty_numerator, royalty_denominator, payee_address);<br />    <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties">set_collection_royalties</a>(creator, <a href="collection.md#0x4_collection">collection</a>, <a href="royalty.md#0x4_royalty">royalty</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_uri"></a>

## Function `set_collection_uri`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_uri">set_collection_uri</a>&lt;T: key&gt;(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_uri">set_collection_uri</a>&lt;T: key&gt;(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,<br />    uri: String,<br />) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> &#123;<br />    <b>let</b> aptos_collection &#61; <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>(&amp;<a href="collection.md#0x4_collection">collection</a>, creator);<br />    <b>assert</b>!(<br />        aptos_collection.mutable_uri,<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),<br />    );<br />    <a href="collection.md#0x4_collection_set_uri">collection::set_uri</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;aptos_collection.mutator_ref), uri);<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
