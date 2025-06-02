
<a id="0x4_aptos_token"></a>

# Module `0x4::aptos_token`

This defines a minimally viable token for no-code solutions akin to the original token at
0x3::token module.
The key features are:
* Base token and collection features
* Creator definable mutability for tokens
* Creator-based freezing of tokens
* Standard object-based transfer and events
* Metadata property type


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


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="collection.md#0x4_collection">0x4::collection</a>;
<b>use</b> <a href="property_map.md#0x4_property_map">0x4::property_map</a>;
<b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;
<b>use</b> <a href="token.md#0x4_token">0x4::token</a>;
</code></pre>



<a id="0x4_aptos_token_AptosCollection"></a>

## Resource `AptosCollection`

Storage state for managing the no-code Collection.


<pre><code>#[resource_group_member(#[group = <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> <b>has</b> key
</code></pre>



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
 Determines if the creator can mutate the collection's description
</dd>
<dt>
<code>mutable_uri: bool</code>
</dt>
<dd>
 Determines if the creator can mutate the collection's uri
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

Storage state for managing the no-code Token.


<pre><code>#[resource_group_member(#[group = <a href="../../aptos-framework/doc/object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> <b>has</b> key
</code></pre>



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


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a id="0x4_aptos_token_EFIELD_NOT_MUTABLE"></a>

The field being changed is not mutable


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>: u64 = 4;
</code></pre>



<a id="0x4_aptos_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ENOT_CREATOR">ENOT_CREATOR</a>: u64 = 3;
</code></pre>



<a id="0x4_aptos_token_ETOKEN_DOES_NOT_EXIST"></a>

The token does not exist


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a id="0x4_aptos_token_EPROPERTIES_NOT_MUTABLE"></a>

The property map being mutated is not mutable


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>: u64 = 6;
</code></pre>



<a id="0x4_aptos_token_ETOKEN_NOT_BURNABLE"></a>

The token being burned is not burnable


<pre><code><b>const</b> <a href="aptos_token.md#0x4_aptos_token_ETOKEN_NOT_BURNABLE">ETOKEN_NOT_BURNABLE</a>: u64 = 5;
</code></pre>



<a id="0x4_aptos_token_create_collection"></a>

## Function `create_collection`

Create a new collection


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection">create_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection">create_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    max_supply: u64,
    name: String,
    uri: String,
    mutable_description: bool,
    mutable_royalty: bool,
    mutable_uri: bool,
    mutable_token_description: bool,
    mutable_token_name: bool,
    mutable_token_properties: bool,
    mutable_token_uri: bool,
    tokens_burnable_by_creator: bool,
    tokens_freezable_by_creator: bool,
    royalty_numerator: u64,
    royalty_denominator: u64,
) {
    <a href="aptos_token.md#0x4_aptos_token_create_collection_object">create_collection_object</a>(
        creator,
        description,
        max_supply,
        name,
        uri,
        mutable_description,
        mutable_royalty,
        mutable_uri,
        mutable_token_description,
        mutable_token_name,
        mutable_token_properties,
        mutable_token_uri,
        tokens_burnable_by_creator,
        tokens_freezable_by_creator,
        royalty_numerator,
        royalty_denominator
    );
}
</code></pre>



</details>

<a id="0x4_aptos_token_create_collection_object"></a>

## Function `create_collection_object`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection_object">create_collection_object</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_create_collection_object">create_collection_object</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    max_supply: u64,
    name: String,
    uri: String,
    mutable_description: bool,
    mutable_royalty: bool,
    mutable_uri: bool,
    mutable_token_description: bool,
    mutable_token_name: bool,
    mutable_token_properties: bool,
    mutable_token_uri: bool,
    tokens_burnable_by_creator: bool,
    tokens_freezable_by_creator: bool,
    royalty_numerator: u64,
    royalty_denominator: u64,
): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt; {
    <b>let</b> creator_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>let</b> <a href="royalty.md#0x4_royalty">royalty</a> = <a href="royalty.md#0x4_royalty_create">royalty::create</a>(royalty_numerator, royalty_denominator, creator_addr);
    <b>let</b> constructor_ref = <a href="collection.md#0x4_collection_create_fixed_collection">collection::create_fixed_collection</a>(
        creator,
        description,
        max_supply,
        name,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="royalty.md#0x4_royalty">royalty</a>),
        uri,
    );

    <b>let</b> object_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);
    <b>let</b> mutator_ref = <b>if</b> (mutable_description || mutable_uri) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="collection.md#0x4_collection_generate_mutator_ref">collection::generate_mutator_ref</a>(&constructor_ref))
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> royalty_mutator_ref = <b>if</b> (mutable_royalty) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="royalty.md#0x4_royalty_generate_mutator_ref">royalty::generate_mutator_ref</a>(<a href="../../aptos-framework/doc/object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(&constructor_ref)))
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> aptos_collection = <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
        mutator_ref,
        royalty_mutator_ref,
        mutable_description,
        mutable_uri,
        mutable_token_description,
        mutable_token_name,
        mutable_token_properties,
        mutable_token_uri,
        tokens_burnable_by_creator,
        tokens_freezable_by_creator,
    };
    <b>move_to</b>(&object_signer, aptos_collection);
    <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&constructor_ref)
}
</code></pre>



</details>

<a id="0x4_aptos_token_mint"></a>

## Function `mint`

With an existing collection, directly mint a viable token into the creators account.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint">mint</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint">mint</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: String,
    description: String,
    name: String,
    uri: String,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <a href="aptos_token.md#0x4_aptos_token_mint_token_object">mint_token_object</a>(creator, <a href="collection.md#0x4_collection">collection</a>, description, name, uri, property_keys, property_types, property_values);
}
</code></pre>



</details>

<a id="0x4_aptos_token_mint_token_object"></a>

## Function `mint_token_object`

Mint a token into an existing collection, and retrieve the object / address of the token.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_token_object">mint_token_object</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_token_object">mint_token_object</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: String,
    description: String,
    name: String,
    uri: String,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt; <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> constructor_ref = <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(
        creator,
        <a href="collection.md#0x4_collection">collection</a>,
        description,
        name,
        uri,
        property_keys,
        property_types,
        property_values,
    );

    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator, &<a href="collection.md#0x4_collection">collection</a>);

    // If tokens are freezable, add a transfer ref <b>to</b> be able <b>to</b> <b>freeze</b> transfers
    <b>let</b> freezable_by_creator = <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="collection.md#0x4_collection">collection</a>);
    <b>if</b> (freezable_by_creator) {
        <b>let</b> aptos_token_addr = <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(&constructor_ref);
        <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = &<b>mut</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>[aptos_token_addr];
        <b>let</b> transfer_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&constructor_ref);
        <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref.fill(transfer_ref);
    };

    <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&constructor_ref)
}
</code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound"></a>

## Function `mint_soul_bound`

With an existing collection, directly mint a soul bound token into the recipient's account.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound">mint_soul_bound</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, soul_bound_to: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound">mint_soul_bound</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: String,
    description: String,
    name: String,
    uri: String,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    soul_bound_to: <b>address</b>,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound_token_object">mint_soul_bound_token_object</a>(
        creator,
        <a href="collection.md#0x4_collection">collection</a>,
        description,
        name,
        uri,
        property_keys,
        property_types,
        property_values,
        soul_bound_to
    );
}
</code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound_token_object"></a>

## Function `mint_soul_bound_token_object`

With an existing collection, directly mint a soul bound token into the recipient's account.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound_token_object">mint_soul_bound_token_object</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, soul_bound_to: <b>address</b>): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_soul_bound_token_object">mint_soul_bound_token_object</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: String,
    description: String,
    name: String,
    uri: String,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    soul_bound_to: <b>address</b>,
): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt; <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> constructor_ref = <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(
        creator,
        <a href="collection.md#0x4_collection">collection</a>,
        description,
        name,
        uri,
        property_keys,
        property_types,
        property_values,
    );

    <b>let</b> transfer_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(&constructor_ref);
    <b>let</b> linear_transfer_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_linear_transfer_ref">object::generate_linear_transfer_ref</a>(&transfer_ref);
    <a href="../../aptos-framework/doc/object.md#0x1_object_transfer_with_ref">object::transfer_with_ref</a>(linear_transfer_ref, soul_bound_to);
    <a href="../../aptos-framework/doc/object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(&transfer_ref);

    <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&constructor_ref)
}
</code></pre>



</details>

<a id="0x4_aptos_token_mint_internal"></a>

## Function `mint_internal`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_mint_internal">mint_internal</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: String,
    description: String,
    name: String,
    uri: String,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): ConstructorRef <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> constructor_ref = <a href="token.md#0x4_token_create">token::create</a>(creator, <a href="collection.md#0x4_collection">collection</a>, description, name, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), uri);

    <b>let</b> object_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);

    <b>let</b> collection_obj = <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator, &<a href="collection.md#0x4_collection">collection</a>);
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&collection_obj);

    <b>let</b> mutator_ref = <b>if</b> (
        <a href="collection.md#0x4_collection">collection</a>.mutable_token_description
            || <a href="collection.md#0x4_collection">collection</a>.mutable_token_name
            || <a href="collection.md#0x4_collection">collection</a>.mutable_token_uri
    ) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="token.md#0x4_token_generate_mutator_ref">token::generate_mutator_ref</a>(&constructor_ref))
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> burn_ref = <b>if</b> (<a href="collection.md#0x4_collection">collection</a>.tokens_burnable_by_creator) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="token.md#0x4_token_generate_burn_ref">token::generate_burn_ref</a>(&constructor_ref))
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
        burn_ref,
        transfer_ref: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        mutator_ref,
        property_mutator_ref: <a href="property_map.md#0x4_property_map_generate_mutator_ref">property_map::generate_mutator_ref</a>(&constructor_ref),
    };
    <b>move_to</b>(&object_signer, <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>);

    <b>let</b> properties = <a href="property_map.md#0x4_property_map_prepare_input">property_map::prepare_input</a>(property_keys, property_types, property_values);
    <a href="property_map.md#0x4_property_map_init">property_map::init</a>(&constructor_ref, properties);

    constructor_ref
}
</code></pre>



</details>

<a id="0x4_aptos_token_borrow"></a>

## Function `borrow`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &Object&lt;T&gt;): &<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> token_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(token_address),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>),
    );
    &<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>[token_address]
}
</code></pre>



</details>

<a id="0x4_aptos_token_are_properties_mutable"></a>

## Function `are_properties_mutable`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>);
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_token_properties
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_burnable"></a>

## Function `is_burnable`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_burnable">is_burnable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_burnable">is_burnable</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(&<a href="token.md#0x4_token">token</a>).burn_ref.is_some()
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_freezable_by_creator"></a>

## Function `is_freezable_by_creator`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_freezable_by_creator">is_freezable_by_creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_freezable_by_creator">is_freezable_by_creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_description"></a>

## Function `is_mutable_description`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_description">is_mutable_description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_description">is_mutable_description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_description">is_mutable_collection_token_description</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_name"></a>

## Function `is_mutable_name`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_name">is_mutable_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_name">is_mutable_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_name">is_mutable_collection_token_name</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_uri"></a>

## Function `is_mutable_uri`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_uri">is_mutable_uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_uri">is_mutable_uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_uri">is_mutable_collection_token_uri</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))
}
</code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow"></a>

## Function `authorized_borrow`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &<a href="aptos_token.md#0x4_aptos_token_AptosToken">aptos_token::AptosToken</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &Object&lt;T&gt;, creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> token_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(token_address),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>),
    );

    <b>assert</b>!(
        <a href="token.md#0x4_token_creator">token::creator</a>(*<a href="token.md#0x4_token">token</a>) == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_ENOT_CREATOR">ENOT_CREATOR</a>),
    );
    &<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>[token_address]
}
</code></pre>



</details>

<a id="0x4_aptos_token_burn"></a>

## Function `burn`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_burn">burn</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_burn">burn</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.burn_ref.is_some(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_ETOKEN_NOT_BURNABLE">ETOKEN_NOT_BURNABLE</a>),
    );
    <b>move</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>;
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <b>move_from</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a>&gt;(<a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&<a href="token.md#0x4_token">token</a>));
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
        burn_ref,
        transfer_ref: _,
        mutator_ref: _,
        property_mutator_ref,
    } = <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>;
    <a href="property_map.md#0x4_property_map_burn">property_map::burn</a>(property_mutator_ref);
    <a href="token.md#0x4_token_burn">token::burn</a>(burn_ref.extract());
}
</code></pre>



</details>

<a id="0x4_aptos_token_freeze_transfer"></a>

## Function `freeze_transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_freeze_transfer">freeze_transfer</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_freeze_transfer">freeze_transfer</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))
            && <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref.is_some(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <a href="../../aptos-framework/doc/object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>());
}
</code></pre>



</details>

<a id="0x4_aptos_token_unfreeze_transfer"></a>

## Function `unfreeze_transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_unfreeze_transfer">unfreeze_transfer</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_unfreeze_transfer">unfreeze_transfer</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>(<a href="token.md#0x4_token_collection_object">token::collection_object</a>(<a href="token.md#0x4_token">token</a>))
            && <a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref.is_some(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <a href="../../aptos-framework/doc/object.md#0x1_object_enable_ungated_transfer">object::enable_ungated_transfer</a>(<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.transfer_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>());
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_description">set_description</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_description">set_description</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    description: String,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_is_mutable_description">is_mutable_description</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <a href="token.md#0x4_token_set_description">token::set_description</a>(<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.mutator_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(), description);
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_name"></a>

## Function `set_name`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_name">set_name</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_name">set_name</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    name: String,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_is_mutable_name">is_mutable_name</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <a href="token.md#0x4_token_set_name">token::set_name</a>(<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.mutator_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(), name);
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_uri">set_uri</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_uri">set_uri</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    uri: String,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_is_mutable_uri">is_mutable_uri</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <a href="token.md#0x4_token_set_uri">token::set_uri</a>(<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.mutator_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(), uri);
}
</code></pre>



</details>

<a id="0x4_aptos_token_add_property"></a>

## Function `add_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_property">add_property</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_property">add_property</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    key: String,
    type: String,
    value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),
    );

    <a href="property_map.md#0x4_property_map_add">property_map::add</a>(&<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, key, type, value);
}
</code></pre>



</details>

<a id="0x4_aptos_token_add_typed_property"></a>

## Function `add_typed_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_typed_property">add_typed_property</a>&lt;T: key, V: drop&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_add_typed_property">add_typed_property</a>&lt;T: key, V: drop&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    key: String,
    value: V,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),
    );

    <a href="property_map.md#0x4_property_map_add_typed">property_map::add_typed</a>(&<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, key, value);
}
</code></pre>



</details>

<a id="0x4_aptos_token_remove_property"></a>

## Function `remove_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_remove_property">remove_property</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_remove_property">remove_property</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    key: String,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),
    );

    <a href="property_map.md#0x4_property_map_remove">property_map::remove</a>(&<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, &key);
}
</code></pre>



</details>

<a id="0x4_aptos_token_update_property"></a>

## Function `update_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_property">update_property</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, type: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_property">update_property</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    key: String,
    type: String,
    value: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),
    );

    <a href="property_map.md#0x4_property_map_update">property_map::update</a>(&<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, &key, type, value);
}
</code></pre>



</details>

<a id="0x4_aptos_token_update_typed_property"></a>

## Function `update_typed_property`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_typed_property">update_typed_property</a>&lt;T: key, V: drop&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, key: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_update_typed_property">update_typed_property</a>&lt;T: key, V: drop&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="token.md#0x4_token">token</a>: Object&lt;T&gt;,
    key: String,
    value: V,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>, <a href="aptos_token.md#0x4_aptos_token_AptosToken">AptosToken</a> {
    <b>let</b> <a href="aptos_token.md#0x4_aptos_token">aptos_token</a> = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow">authorized_borrow</a>(&<a href="token.md#0x4_token">token</a>, creator);
    <b>assert</b>!(
        <a href="aptos_token.md#0x4_aptos_token_are_properties_mutable">are_properties_mutable</a>(<a href="token.md#0x4_token">token</a>),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EPROPERTIES_NOT_MUTABLE">EPROPERTIES_NOT_MUTABLE</a>),
    );

    <a href="property_map.md#0x4_property_map_update_typed">property_map::update_typed</a>(&<a href="aptos_token.md#0x4_aptos_token">aptos_token</a>.property_mutator_ref, &key, value);
}
</code></pre>



</details>

<a id="0x4_aptos_token_collection_object"></a>

## Function `collection_object`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_collection_object">collection_object</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: &String): Object&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt; {
    <b>let</b> collection_addr = <a href="collection.md#0x4_collection_create_collection_address">collection::create_collection_address</a>(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator), name);
    <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_addr)
}
</code></pre>



</details>

<a id="0x4_aptos_token_borrow_collection"></a>

## Function `borrow_collection`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): &<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: &Object&lt;T&gt;): &<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> collection_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="token.md#0x4_token">token</a>);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_address),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),
    );
    &<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>[collection_address]
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_description"></a>

## Function `is_mutable_collection_description`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_description">is_mutable_collection_description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_description">is_mutable_collection_description</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_description
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_royalty"></a>

## Function `is_mutable_collection_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_royalty">is_mutable_collection_royalty</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_royalty">is_mutable_collection_royalty</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).royalty_mutator_ref.is_some()
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_uri"></a>

## Function `is_mutable_collection_uri`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_uri">is_mutable_collection_uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_uri">is_mutable_collection_uri</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_uri
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_description"></a>

## Function `is_mutable_collection_token_description`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_description">is_mutable_collection_token_description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_description">is_mutable_collection_token_description</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_token_description
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_name"></a>

## Function `is_mutable_collection_token_name`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_name">is_mutable_collection_token_name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_name">is_mutable_collection_token_name</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_token_name
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_uri"></a>

## Function `is_mutable_collection_token_uri`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_uri">is_mutable_collection_token_uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_uri">is_mutable_collection_token_uri</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_token_uri
}
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_properties"></a>

## Function `is_mutable_collection_token_properties`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_properties">is_mutable_collection_token_properties</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_is_mutable_collection_token_properties">is_mutable_collection_token_properties</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).mutable_token_properties
}
</code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_burnable"></a>

## Function `are_collection_tokens_burnable`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_burnable">are_collection_tokens_burnable</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_burnable">are_collection_tokens_burnable</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).tokens_burnable_by_creator
}
</code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_freezable"></a>

## Function `are_collection_tokens_freezable`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_are_collection_tokens_freezable">are_collection_tokens_freezable</a>&lt;T: key&gt;(
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
): bool <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <a href="aptos_token.md#0x4_aptos_token_borrow_collection">borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>).tokens_freezable_by_creator
}
</code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow_collection"></a>

## Function `authorized_borrow_collection`



<pre><code><b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &<a href="aptos_token.md#0x4_aptos_token_AptosCollection">aptos_token::AptosCollection</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: &Object&lt;T&gt;, creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): &<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> collection_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>&gt;(collection_address),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_token.md#0x4_aptos_token_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),
    );
    <b>assert</b>!(
        <a href="collection.md#0x4_collection_creator">collection::creator</a>(*<a href="collection.md#0x4_collection">collection</a>) == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_ENOT_CREATOR">ENOT_CREATOR</a>),
    );
    &<a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a>[collection_address]
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_description"></a>

## Function `set_collection_description`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_description">set_collection_description</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_description">set_collection_description</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
    description: String,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> aptos_collection = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>, creator);
    <b>assert</b>!(
        aptos_collection.mutable_description,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <a href="collection.md#0x4_collection_set_description">collection::set_description</a>(aptos_collection.mutator_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(), description);
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties"></a>

## Function `set_collection_royalties`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties">set_collection_royalties</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties">set_collection_royalties</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
    <a href="royalty.md#0x4_royalty">royalty</a>: <a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> aptos_collection = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>, creator);
    <b>assert</b>!(
        aptos_collection.royalty_mutator_ref.is_some(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <a href="royalty.md#0x4_royalty_update">royalty::update</a>(aptos_collection.royalty_mutator_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(), <a href="royalty.md#0x4_royalty">royalty</a>);
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties_call"></a>

## Function `set_collection_royalties_call`



<pre><code>entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties_call">set_collection_royalties_call</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, royalty_numerator: u64, royalty_denominator: u64, payee_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties_call">set_collection_royalties_call</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
    royalty_numerator: u64,
    royalty_denominator: u64,
    payee_address: <b>address</b>,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> <a href="royalty.md#0x4_royalty">royalty</a> = <a href="royalty.md#0x4_royalty_create">royalty::create</a>(royalty_numerator, royalty_denominator, payee_address);
    <a href="aptos_token.md#0x4_aptos_token_set_collection_royalties">set_collection_royalties</a>(creator, <a href="collection.md#0x4_collection">collection</a>, <a href="royalty.md#0x4_royalty">royalty</a>);
}
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_uri"></a>

## Function `set_collection_uri`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_uri">set_collection_uri</a>&lt;T: key&gt;(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_token.md#0x4_aptos_token_set_collection_uri">set_collection_uri</a>&lt;T: key&gt;(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;,
    uri: String,
) <b>acquires</b> <a href="aptos_token.md#0x4_aptos_token_AptosCollection">AptosCollection</a> {
    <b>let</b> aptos_collection = <a href="aptos_token.md#0x4_aptos_token_authorized_borrow_collection">authorized_borrow_collection</a>(&<a href="collection.md#0x4_collection">collection</a>, creator);
    <b>assert</b>!(
        aptos_collection.mutable_uri,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_token.md#0x4_aptos_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>),
    );
    <a href="collection.md#0x4_collection_set_uri">collection::set_uri</a>(aptos_collection.mutator_ref.<a href="aptos_token.md#0x4_aptos_token_borrow">borrow</a>(), uri);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
