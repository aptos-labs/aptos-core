
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


<pre><code>use 0x1::error;
use 0x1::object;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
use 0x4::collection;
use 0x4::property_map;
use 0x4::royalty;
use 0x4::token;
</code></pre>



<a id="0x4_aptos_token_AptosCollection"></a>

## Resource `AptosCollection`

Storage state for managing the no-code Collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct AptosCollection has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutator_ref: option::Option&lt;collection::MutatorRef&gt;</code>
</dt>
<dd>
 Used to mutate collection fields
</dd>
<dt>
<code>royalty_mutator_ref: option::Option&lt;royalty::MutatorRef&gt;</code>
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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]
struct AptosToken has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: option::Option&lt;token::BurnRef&gt;</code>
</dt>
<dd>
 Used to burn.
</dd>
<dt>
<code>transfer_ref: option::Option&lt;object::TransferRef&gt;</code>
</dt>
<dd>
 Used to control freeze.
</dd>
<dt>
<code>mutator_ref: option::Option&lt;token::MutatorRef&gt;</code>
</dt>
<dd>
 Used to mutate fields
</dd>
<dt>
<code>property_mutator_ref: property_map::MutatorRef</code>
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


<pre><code>const ECOLLECTION_DOES_NOT_EXIST: u64 &#61; 1;
</code></pre>



<a id="0x4_aptos_token_EFIELD_NOT_MUTABLE"></a>

The field being changed is not mutable


<pre><code>const EFIELD_NOT_MUTABLE: u64 &#61; 4;
</code></pre>



<a id="0x4_aptos_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code>const ENOT_CREATOR: u64 &#61; 3;
</code></pre>



<a id="0x4_aptos_token_ETOKEN_DOES_NOT_EXIST"></a>

The token does not exist


<pre><code>const ETOKEN_DOES_NOT_EXIST: u64 &#61; 2;
</code></pre>



<a id="0x4_aptos_token_EPROPERTIES_NOT_MUTABLE"></a>

The property map being mutated is not mutable


<pre><code>const EPROPERTIES_NOT_MUTABLE: u64 &#61; 6;
</code></pre>



<a id="0x4_aptos_token_ETOKEN_NOT_BURNABLE"></a>

The token being burned is not burnable


<pre><code>const ETOKEN_NOT_BURNABLE: u64 &#61; 5;
</code></pre>



<a id="0x4_aptos_token_create_collection"></a>

## Function `create_collection`

Create a new collection


<pre><code>public entry fun create_collection(creator: &amp;signer, description: string::String, max_supply: u64, name: string::String, uri: string::String, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_collection(
    creator: &amp;signer,
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
) &#123;
    create_collection_object(
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
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_create_collection_object"></a>

## Function `create_collection_object`



<pre><code>public fun create_collection_object(creator: &amp;signer, description: string::String, max_supply: u64, name: string::String, uri: string::String, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64): object::Object&lt;aptos_token::AptosCollection&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_object(
    creator: &amp;signer,
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
): Object&lt;AptosCollection&gt; &#123;
    let creator_addr &#61; signer::address_of(creator);
    let royalty &#61; royalty::create(royalty_numerator, royalty_denominator, creator_addr);
    let constructor_ref &#61; collection::create_fixed_collection(
        creator,
        description,
        max_supply,
        name,
        option::some(royalty),
        uri,
    );

    let object_signer &#61; object::generate_signer(&amp;constructor_ref);
    let mutator_ref &#61; if (mutable_description &#124;&#124; mutable_uri) &#123;
        option::some(collection::generate_mutator_ref(&amp;constructor_ref))
    &#125; else &#123;
        option::none()
    &#125;;

    let royalty_mutator_ref &#61; if (mutable_royalty) &#123;
        option::some(royalty::generate_mutator_ref(object::generate_extend_ref(&amp;constructor_ref)))
    &#125; else &#123;
        option::none()
    &#125;;

    let aptos_collection &#61; AptosCollection &#123;
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
    &#125;;
    move_to(&amp;object_signer, aptos_collection);
    object::object_from_constructor_ref(&amp;constructor_ref)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_mint"></a>

## Function `mint`

With an existing collection, directly mint a viable token into the creators account.


<pre><code>public entry fun mint(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint(
    creator: &amp;signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector&lt;String&gt;,
    property_types: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires AptosCollection, AptosToken &#123;
    mint_token_object(creator, collection, description, name, uri, property_keys, property_types, property_values);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_mint_token_object"></a>

## Function `mint_token_object`

Mint a token into an existing collection, and retrieve the object / address of the token.


<pre><code>public fun mint_token_object(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;): object::Object&lt;aptos_token::AptosToken&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_token_object(
    creator: &amp;signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector&lt;String&gt;,
    property_types: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
): Object&lt;AptosToken&gt; acquires AptosCollection, AptosToken &#123;
    let constructor_ref &#61; mint_internal(
        creator,
        collection,
        description,
        name,
        uri,
        property_keys,
        property_types,
        property_values,
    );

    let collection &#61; collection_object(creator, &amp;collection);

    // If tokens are freezable, add a transfer ref to be able to freeze transfers
    let freezable_by_creator &#61; are_collection_tokens_freezable(collection);
    if (freezable_by_creator) &#123;
        let aptos_token_addr &#61; object::address_from_constructor_ref(&amp;constructor_ref);
        let aptos_token &#61; borrow_global_mut&lt;AptosToken&gt;(aptos_token_addr);
        let transfer_ref &#61; object::generate_transfer_ref(&amp;constructor_ref);
        option::fill(&amp;mut aptos_token.transfer_ref, transfer_ref);
    &#125;;

    object::object_from_constructor_ref(&amp;constructor_ref)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound"></a>

## Function `mint_soul_bound`

With an existing collection, directly mint a soul bound token into the recipient's account.


<pre><code>public entry fun mint_soul_bound(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, soul_bound_to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint_soul_bound(
    creator: &amp;signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector&lt;String&gt;,
    property_types: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
    soul_bound_to: address,
) acquires AptosCollection &#123;
    mint_soul_bound_token_object(
        creator,
        collection,
        description,
        name,
        uri,
        property_keys,
        property_types,
        property_values,
        soul_bound_to
    );
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound_token_object"></a>

## Function `mint_soul_bound_token_object`

With an existing collection, directly mint a soul bound token into the recipient's account.


<pre><code>public fun mint_soul_bound_token_object(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, soul_bound_to: address): object::Object&lt;aptos_token::AptosToken&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_soul_bound_token_object(
    creator: &amp;signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector&lt;String&gt;,
    property_types: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
    soul_bound_to: address,
): Object&lt;AptosToken&gt; acquires AptosCollection &#123;
    let constructor_ref &#61; mint_internal(
        creator,
        collection,
        description,
        name,
        uri,
        property_keys,
        property_types,
        property_values,
    );

    let transfer_ref &#61; object::generate_transfer_ref(&amp;constructor_ref);
    let linear_transfer_ref &#61; object::generate_linear_transfer_ref(&amp;transfer_ref);
    object::transfer_with_ref(linear_transfer_ref, soul_bound_to);
    object::disable_ungated_transfer(&amp;transfer_ref);

    object::object_from_constructor_ref(&amp;constructor_ref)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_mint_internal"></a>

## Function `mint_internal`



<pre><code>fun mint_internal(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;): object::ConstructorRef
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun mint_internal(
    creator: &amp;signer,
    collection: String,
    description: String,
    name: String,
    uri: String,
    property_keys: vector&lt;String&gt;,
    property_types: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
): ConstructorRef acquires AptosCollection &#123;
    let constructor_ref &#61; token::create(creator, collection, description, name, option::none(), uri);

    let object_signer &#61; object::generate_signer(&amp;constructor_ref);

    let collection_obj &#61; collection_object(creator, &amp;collection);
    let collection &#61; borrow_collection(&amp;collection_obj);

    let mutator_ref &#61; if (
        collection.mutable_token_description
            &#124;&#124; collection.mutable_token_name
            &#124;&#124; collection.mutable_token_uri
    ) &#123;
        option::some(token::generate_mutator_ref(&amp;constructor_ref))
    &#125; else &#123;
        option::none()
    &#125;;

    let burn_ref &#61; if (collection.tokens_burnable_by_creator) &#123;
        option::some(token::generate_burn_ref(&amp;constructor_ref))
    &#125; else &#123;
        option::none()
    &#125;;

    let aptos_token &#61; AptosToken &#123;
        burn_ref,
        transfer_ref: option::none(),
        mutator_ref,
        property_mutator_ref: property_map::generate_mutator_ref(&amp;constructor_ref),
    &#125;;
    move_to(&amp;object_signer, aptos_token);

    let properties &#61; property_map::prepare_input(property_keys, property_types, property_values);
    property_map::init(&amp;constructor_ref, properties);

    constructor_ref
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_borrow"></a>

## Function `borrow`



<pre><code>fun borrow&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;): &amp;aptos_token::AptosToken
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow&lt;T: key&gt;(token: &amp;Object&lt;T&gt;): &amp;AptosToken &#123;
    let token_address &#61; object::object_address(token);
    assert!(
        exists&lt;AptosToken&gt;(token_address),
        error::not_found(ETOKEN_DOES_NOT_EXIST),
    );
    borrow_global&lt;AptosToken&gt;(token_address)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_are_properties_mutable"></a>

## Function `are_properties_mutable`



<pre><code>&#35;[view]
public fun are_properties_mutable&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun are_properties_mutable&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;
    let collection &#61; token::collection_object(token);
    borrow_collection(&amp;collection).mutable_token_properties
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_burnable"></a>

## Function `is_burnable`



<pre><code>&#35;[view]
public fun is_burnable&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_burnable&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosToken &#123;
    option::is_some(&amp;borrow(&amp;token).burn_ref)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_freezable_by_creator"></a>

## Function `is_freezable_by_creator`



<pre><code>&#35;[view]
public fun is_freezable_by_creator&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_freezable_by_creator&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;
    are_collection_tokens_freezable(token::collection_object(token))
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_description"></a>

## Function `is_mutable_description`



<pre><code>&#35;[view]
public fun is_mutable_description&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_description&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;
    is_mutable_collection_token_description(token::collection_object(token))
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_name"></a>

## Function `is_mutable_name`



<pre><code>&#35;[view]
public fun is_mutable_name&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_name&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;
    is_mutable_collection_token_name(token::collection_object(token))
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_uri"></a>

## Function `is_mutable_uri`



<pre><code>&#35;[view]
public fun is_mutable_uri&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_uri&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;
    is_mutable_collection_token_uri(token::collection_object(token))
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow"></a>

## Function `authorized_borrow`



<pre><code>fun authorized_borrow&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;, creator: &amp;signer): &amp;aptos_token::AptosToken
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun authorized_borrow&lt;T: key&gt;(token: &amp;Object&lt;T&gt;, creator: &amp;signer): &amp;AptosToken &#123;
    let token_address &#61; object::object_address(token);
    assert!(
        exists&lt;AptosToken&gt;(token_address),
        error::not_found(ETOKEN_DOES_NOT_EXIST),
    );

    assert!(
        token::creator(&#42;token) &#61;&#61; signer::address_of(creator),
        error::permission_denied(ENOT_CREATOR),
    );
    borrow_global&lt;AptosToken&gt;(token_address)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_burn"></a>

## Function `burn`



<pre><code>public entry fun burn&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn&lt;T: key&gt;(creator: &amp;signer, token: Object&lt;T&gt;) acquires AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        option::is_some(&amp;aptos_token.burn_ref),
        error::permission_denied(ETOKEN_NOT_BURNABLE),
    );
    move aptos_token;
    let aptos_token &#61; move_from&lt;AptosToken&gt;(object::object_address(&amp;token));
    let AptosToken &#123;
        burn_ref,
        transfer_ref: _,
        mutator_ref: _,
        property_mutator_ref,
    &#125; &#61; aptos_token;
    property_map::burn(property_mutator_ref);
    token::burn(option::extract(&amp;mut burn_ref));
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_freeze_transfer"></a>

## Function `freeze_transfer`



<pre><code>public entry fun freeze_transfer&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun freeze_transfer&lt;T: key&gt;(creator: &amp;signer, token: Object&lt;T&gt;) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_collection_tokens_freezable(token::collection_object(token))
            &amp;&amp; option::is_some(&amp;aptos_token.transfer_ref),
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    object::disable_ungated_transfer(option::borrow(&amp;aptos_token.transfer_ref));
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_unfreeze_transfer"></a>

## Function `unfreeze_transfer`



<pre><code>public entry fun unfreeze_transfer&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unfreeze_transfer&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;
) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_collection_tokens_freezable(token::collection_object(token))
            &amp;&amp; option::is_some(&amp;aptos_token.transfer_ref),
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    object::enable_ungated_transfer(option::borrow(&amp;aptos_token.transfer_ref));
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_description"></a>

## Function `set_description`



<pre><code>public entry fun set_description&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_description&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    description: String,
) acquires AptosCollection, AptosToken &#123;
    assert!(
        is_mutable_description(token),
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    token::set_description(option::borrow(&amp;aptos_token.mutator_ref), description);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_name"></a>

## Function `set_name`



<pre><code>public entry fun set_name&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, name: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_name&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    name: String,
) acquires AptosCollection, AptosToken &#123;
    assert!(
        is_mutable_name(token),
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    token::set_name(option::borrow(&amp;aptos_token.mutator_ref), name);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_uri"></a>

## Function `set_uri`



<pre><code>public entry fun set_uri&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_uri&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    uri: String,
) acquires AptosCollection, AptosToken &#123;
    assert!(
        is_mutable_uri(token),
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    token::set_uri(option::borrow(&amp;aptos_token.mutator_ref), uri);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_add_property"></a>

## Function `add_property`



<pre><code>public entry fun add_property&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, type: string::String, value: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_property&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    key: String,
    type: String,
    value: vector&lt;u8&gt;,
) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_properties_mutable(token),
        error::permission_denied(EPROPERTIES_NOT_MUTABLE),
    );

    property_map::add(&amp;aptos_token.property_mutator_ref, key, type, value);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_add_typed_property"></a>

## Function `add_typed_property`



<pre><code>public entry fun add_typed_property&lt;T: key, V: drop&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_typed_property&lt;T: key, V: drop&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    key: String,
    value: V,
) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_properties_mutable(token),
        error::permission_denied(EPROPERTIES_NOT_MUTABLE),
    );

    property_map::add_typed(&amp;aptos_token.property_mutator_ref, key, value);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_remove_property"></a>

## Function `remove_property`



<pre><code>public entry fun remove_property&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun remove_property&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    key: String,
) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_properties_mutable(token),
        error::permission_denied(EPROPERTIES_NOT_MUTABLE),
    );

    property_map::remove(&amp;aptos_token.property_mutator_ref, &amp;key);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_update_property"></a>

## Function `update_property`



<pre><code>public entry fun update_property&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, type: string::String, value: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_property&lt;T: key&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    key: String,
    type: String,
    value: vector&lt;u8&gt;,
) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_properties_mutable(token),
        error::permission_denied(EPROPERTIES_NOT_MUTABLE),
    );

    property_map::update(&amp;aptos_token.property_mutator_ref, &amp;key, type, value);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_update_typed_property"></a>

## Function `update_typed_property`



<pre><code>public entry fun update_typed_property&lt;T: key, V: drop&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_typed_property&lt;T: key, V: drop&gt;(
    creator: &amp;signer,
    token: Object&lt;T&gt;,
    key: String,
    value: V,
) acquires AptosCollection, AptosToken &#123;
    let aptos_token &#61; authorized_borrow(&amp;token, creator);
    assert!(
        are_properties_mutable(token),
        error::permission_denied(EPROPERTIES_NOT_MUTABLE),
    );

    property_map::update_typed(&amp;aptos_token.property_mutator_ref, &amp;key, value);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_collection_object"></a>

## Function `collection_object`



<pre><code>fun collection_object(creator: &amp;signer, name: &amp;string::String): object::Object&lt;aptos_token::AptosCollection&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun collection_object(creator: &amp;signer, name: &amp;String): Object&lt;AptosCollection&gt; &#123;
    let collection_addr &#61; collection::create_collection_address(&amp;signer::address_of(creator), name);
    object::address_to_object&lt;AptosCollection&gt;(collection_addr)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_borrow_collection"></a>

## Function `borrow_collection`



<pre><code>fun borrow_collection&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;): &amp;aptos_token::AptosCollection
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_collection&lt;T: key&gt;(token: &amp;Object&lt;T&gt;): &amp;AptosCollection &#123;
    let collection_address &#61; object::object_address(token);
    assert!(
        exists&lt;AptosCollection&gt;(collection_address),
        error::not_found(ECOLLECTION_DOES_NOT_EXIST),
    );
    borrow_global&lt;AptosCollection&gt;(collection_address)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_description"></a>

## Function `is_mutable_collection_description`



<pre><code>public fun is_mutable_collection_description&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_description&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).mutable_description
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_royalty"></a>

## Function `is_mutable_collection_royalty`



<pre><code>public fun is_mutable_collection_royalty&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_royalty&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    option::is_some(&amp;borrow_collection(&amp;collection).royalty_mutator_ref)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_uri"></a>

## Function `is_mutable_collection_uri`



<pre><code>public fun is_mutable_collection_uri&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_uri&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).mutable_uri
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_description"></a>

## Function `is_mutable_collection_token_description`



<pre><code>public fun is_mutable_collection_token_description&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_description&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).mutable_token_description
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_name"></a>

## Function `is_mutable_collection_token_name`



<pre><code>public fun is_mutable_collection_token_name&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_name&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).mutable_token_name
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_uri"></a>

## Function `is_mutable_collection_token_uri`



<pre><code>public fun is_mutable_collection_token_uri&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_uri&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).mutable_token_uri
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_properties"></a>

## Function `is_mutable_collection_token_properties`



<pre><code>public fun is_mutable_collection_token_properties&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_properties&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).mutable_token_properties
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_burnable"></a>

## Function `are_collection_tokens_burnable`



<pre><code>public fun are_collection_tokens_burnable&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun are_collection_tokens_burnable&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).tokens_burnable_by_creator
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_freezable"></a>

## Function `are_collection_tokens_freezable`



<pre><code>public fun are_collection_tokens_freezable&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun are_collection_tokens_freezable&lt;T: key&gt;(
    collection: Object&lt;T&gt;,
): bool acquires AptosCollection &#123;
    borrow_collection(&amp;collection).tokens_freezable_by_creator
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow_collection"></a>

## Function `authorized_borrow_collection`



<pre><code>fun authorized_borrow_collection&lt;T: key&gt;(collection: &amp;object::Object&lt;T&gt;, creator: &amp;signer): &amp;aptos_token::AptosCollection
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun authorized_borrow_collection&lt;T: key&gt;(collection: &amp;Object&lt;T&gt;, creator: &amp;signer): &amp;AptosCollection &#123;
    let collection_address &#61; object::object_address(collection);
    assert!(
        exists&lt;AptosCollection&gt;(collection_address),
        error::not_found(ECOLLECTION_DOES_NOT_EXIST),
    );
    assert!(
        collection::creator(&#42;collection) &#61;&#61; signer::address_of(creator),
        error::permission_denied(ENOT_CREATOR),
    );
    borrow_global&lt;AptosCollection&gt;(collection_address)
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_description"></a>

## Function `set_collection_description`



<pre><code>public entry fun set_collection_description&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_collection_description&lt;T: key&gt;(
    creator: &amp;signer,
    collection: Object&lt;T&gt;,
    description: String,
) acquires AptosCollection &#123;
    let aptos_collection &#61; authorized_borrow_collection(&amp;collection, creator);
    assert!(
        aptos_collection.mutable_description,
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    collection::set_description(option::borrow(&amp;aptos_collection.mutator_ref), description);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties"></a>

## Function `set_collection_royalties`



<pre><code>public fun set_collection_royalties&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, royalty: royalty::Royalty)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_collection_royalties&lt;T: key&gt;(
    creator: &amp;signer,
    collection: Object&lt;T&gt;,
    royalty: royalty::Royalty,
) acquires AptosCollection &#123;
    let aptos_collection &#61; authorized_borrow_collection(&amp;collection, creator);
    assert!(
        option::is_some(&amp;aptos_collection.royalty_mutator_ref),
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    royalty::update(option::borrow(&amp;aptos_collection.royalty_mutator_ref), royalty);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties_call"></a>

## Function `set_collection_royalties_call`



<pre><code>entry fun set_collection_royalties_call&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, royalty_numerator: u64, royalty_denominator: u64, payee_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun set_collection_royalties_call&lt;T: key&gt;(
    creator: &amp;signer,
    collection: Object&lt;T&gt;,
    royalty_numerator: u64,
    royalty_denominator: u64,
    payee_address: address,
) acquires AptosCollection &#123;
    let royalty &#61; royalty::create(royalty_numerator, royalty_denominator, payee_address);
    set_collection_royalties(creator, collection, royalty);
&#125;
</code></pre>



</details>

<a id="0x4_aptos_token_set_collection_uri"></a>

## Function `set_collection_uri`



<pre><code>public entry fun set_collection_uri&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_collection_uri&lt;T: key&gt;(
    creator: &amp;signer,
    collection: Object&lt;T&gt;,
    uri: String,
) acquires AptosCollection &#123;
    let aptos_collection &#61; authorized_borrow_collection(&amp;collection, creator);
    assert!(
        aptos_collection.mutable_uri,
        error::permission_denied(EFIELD_NOT_MUTABLE),
    );
    collection::set_uri(option::borrow(&amp;aptos_collection.mutator_ref), uri);
&#125;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
