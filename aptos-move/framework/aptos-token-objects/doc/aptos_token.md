
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


<pre><code>use 0x1::error;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x4::collection;<br/>use 0x4::property_map;<br/>use 0x4::royalty;<br/>use 0x4::token;<br/></code></pre>



<a id="0x4_aptos_token_AptosCollection"></a>

## Resource `AptosCollection`

Storage state for managing the no-code Collection.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct AptosCollection has key<br/></code></pre>



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


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct AptosToken has key<br/></code></pre>



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


<pre><code>const ECOLLECTION_DOES_NOT_EXIST: u64 &#61; 1;<br/></code></pre>



<a id="0x4_aptos_token_EFIELD_NOT_MUTABLE"></a>

The field being changed is not mutable


<pre><code>const EFIELD_NOT_MUTABLE: u64 &#61; 4;<br/></code></pre>



<a id="0x4_aptos_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code>const ENOT_CREATOR: u64 &#61; 3;<br/></code></pre>



<a id="0x4_aptos_token_ETOKEN_DOES_NOT_EXIST"></a>

The token does not exist


<pre><code>const ETOKEN_DOES_NOT_EXIST: u64 &#61; 2;<br/></code></pre>



<a id="0x4_aptos_token_EPROPERTIES_NOT_MUTABLE"></a>

The property map being mutated is not mutable


<pre><code>const EPROPERTIES_NOT_MUTABLE: u64 &#61; 6;<br/></code></pre>



<a id="0x4_aptos_token_ETOKEN_NOT_BURNABLE"></a>

The token being burned is not burnable


<pre><code>const ETOKEN_NOT_BURNABLE: u64 &#61; 5;<br/></code></pre>



<a id="0x4_aptos_token_create_collection"></a>

## Function `create_collection`

Create a new collection


<pre><code>public entry fun create_collection(creator: &amp;signer, description: string::String, max_supply: u64, name: string::String, uri: string::String, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_collection(<br/>    creator: &amp;signer,<br/>    description: String,<br/>    max_supply: u64,<br/>    name: String,<br/>    uri: String,<br/>    mutable_description: bool,<br/>    mutable_royalty: bool,<br/>    mutable_uri: bool,<br/>    mutable_token_description: bool,<br/>    mutable_token_name: bool,<br/>    mutable_token_properties: bool,<br/>    mutable_token_uri: bool,<br/>    tokens_burnable_by_creator: bool,<br/>    tokens_freezable_by_creator: bool,<br/>    royalty_numerator: u64,<br/>    royalty_denominator: u64,<br/>) &#123;<br/>    create_collection_object(<br/>        creator,<br/>        description,<br/>        max_supply,<br/>        name,<br/>        uri,<br/>        mutable_description,<br/>        mutable_royalty,<br/>        mutable_uri,<br/>        mutable_token_description,<br/>        mutable_token_name,<br/>        mutable_token_properties,<br/>        mutable_token_uri,<br/>        tokens_burnable_by_creator,<br/>        tokens_freezable_by_creator,<br/>        royalty_numerator,<br/>        royalty_denominator<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_create_collection_object"></a>

## Function `create_collection_object`



<pre><code>public fun create_collection_object(creator: &amp;signer, description: string::String, max_supply: u64, name: string::String, uri: string::String, mutable_description: bool, mutable_royalty: bool, mutable_uri: bool, mutable_token_description: bool, mutable_token_name: bool, mutable_token_properties: bool, mutable_token_uri: bool, tokens_burnable_by_creator: bool, tokens_freezable_by_creator: bool, royalty_numerator: u64, royalty_denominator: u64): object::Object&lt;aptos_token::AptosCollection&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_object(<br/>    creator: &amp;signer,<br/>    description: String,<br/>    max_supply: u64,<br/>    name: String,<br/>    uri: String,<br/>    mutable_description: bool,<br/>    mutable_royalty: bool,<br/>    mutable_uri: bool,<br/>    mutable_token_description: bool,<br/>    mutable_token_name: bool,<br/>    mutable_token_properties: bool,<br/>    mutable_token_uri: bool,<br/>    tokens_burnable_by_creator: bool,<br/>    tokens_freezable_by_creator: bool,<br/>    royalty_numerator: u64,<br/>    royalty_denominator: u64,<br/>): Object&lt;AptosCollection&gt; &#123;<br/>    let creator_addr &#61; signer::address_of(creator);<br/>    let royalty &#61; royalty::create(royalty_numerator, royalty_denominator, creator_addr);<br/>    let constructor_ref &#61; collection::create_fixed_collection(<br/>        creator,<br/>        description,<br/>        max_supply,<br/>        name,<br/>        option::some(royalty),<br/>        uri,<br/>    );<br/><br/>    let object_signer &#61; object::generate_signer(&amp;constructor_ref);<br/>    let mutator_ref &#61; if (mutable_description &#124;&#124; mutable_uri) &#123;<br/>        option::some(collection::generate_mutator_ref(&amp;constructor_ref))<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;;<br/><br/>    let royalty_mutator_ref &#61; if (mutable_royalty) &#123;<br/>        option::some(royalty::generate_mutator_ref(object::generate_extend_ref(&amp;constructor_ref)))<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;;<br/><br/>    let aptos_collection &#61; AptosCollection &#123;<br/>        mutator_ref,<br/>        royalty_mutator_ref,<br/>        mutable_description,<br/>        mutable_uri,<br/>        mutable_token_description,<br/>        mutable_token_name,<br/>        mutable_token_properties,<br/>        mutable_token_uri,<br/>        tokens_burnable_by_creator,<br/>        tokens_freezable_by_creator,<br/>    &#125;;<br/>    move_to(&amp;object_signer, aptos_collection);<br/>    object::object_from_constructor_ref(&amp;constructor_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_mint"></a>

## Function `mint`

With an existing collection, directly mint a viable token into the creators account.


<pre><code>public entry fun mint(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    description: String,<br/>    name: String,<br/>    uri: String,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_types: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    mint_token_object(creator, collection, description, name, uri, property_keys, property_types, property_values);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_mint_token_object"></a>

## Function `mint_token_object`

Mint a token into an existing collection, and retrieve the object / address of the token.


<pre><code>public fun mint_token_object(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;): object::Object&lt;aptos_token::AptosToken&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_token_object(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    description: String,<br/>    name: String,<br/>    uri: String,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_types: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>): Object&lt;AptosToken&gt; acquires AptosCollection, AptosToken &#123;<br/>    let constructor_ref &#61; mint_internal(<br/>        creator,<br/>        collection,<br/>        description,<br/>        name,<br/>        uri,<br/>        property_keys,<br/>        property_types,<br/>        property_values,<br/>    );<br/><br/>    let collection &#61; collection_object(creator, &amp;collection);<br/><br/>    // If tokens are freezable, add a transfer ref to be able to freeze transfers<br/>    let freezable_by_creator &#61; are_collection_tokens_freezable(collection);<br/>    if (freezable_by_creator) &#123;<br/>        let aptos_token_addr &#61; object::address_from_constructor_ref(&amp;constructor_ref);<br/>        let aptos_token &#61; borrow_global_mut&lt;AptosToken&gt;(aptos_token_addr);<br/>        let transfer_ref &#61; object::generate_transfer_ref(&amp;constructor_ref);<br/>        option::fill(&amp;mut aptos_token.transfer_ref, transfer_ref);<br/>    &#125;;<br/><br/>    object::object_from_constructor_ref(&amp;constructor_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound"></a>

## Function `mint_soul_bound`

With an existing collection, directly mint a soul bound token into the recipient's account.


<pre><code>public entry fun mint_soul_bound(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, soul_bound_to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint_soul_bound(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    description: String,<br/>    name: String,<br/>    uri: String,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_types: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    soul_bound_to: address,<br/>) acquires AptosCollection &#123;<br/>    mint_soul_bound_token_object(<br/>        creator,<br/>        collection,<br/>        description,<br/>        name,<br/>        uri,<br/>        property_keys,<br/>        property_types,<br/>        property_values,<br/>        soul_bound_to<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_mint_soul_bound_token_object"></a>

## Function `mint_soul_bound_token_object`

With an existing collection, directly mint a soul bound token into the recipient's account.


<pre><code>public fun mint_soul_bound_token_object(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, soul_bound_to: address): object::Object&lt;aptos_token::AptosToken&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_soul_bound_token_object(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    description: String,<br/>    name: String,<br/>    uri: String,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_types: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    soul_bound_to: address,<br/>): Object&lt;AptosToken&gt; acquires AptosCollection &#123;<br/>    let constructor_ref &#61; mint_internal(<br/>        creator,<br/>        collection,<br/>        description,<br/>        name,<br/>        uri,<br/>        property_keys,<br/>        property_types,<br/>        property_values,<br/>    );<br/><br/>    let transfer_ref &#61; object::generate_transfer_ref(&amp;constructor_ref);<br/>    let linear_transfer_ref &#61; object::generate_linear_transfer_ref(&amp;transfer_ref);<br/>    object::transfer_with_ref(linear_transfer_ref, soul_bound_to);<br/>    object::disable_ungated_transfer(&amp;transfer_ref);<br/><br/>    object::object_from_constructor_ref(&amp;constructor_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_mint_internal"></a>

## Function `mint_internal`



<pre><code>fun mint_internal(creator: &amp;signer, collection: string::String, description: string::String, name: string::String, uri: string::String, property_keys: vector&lt;string::String&gt;, property_types: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun mint_internal(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    description: String,<br/>    name: String,<br/>    uri: String,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_types: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>): ConstructorRef acquires AptosCollection &#123;<br/>    let constructor_ref &#61; token::create(creator, collection, description, name, option::none(), uri);<br/><br/>    let object_signer &#61; object::generate_signer(&amp;constructor_ref);<br/><br/>    let collection_obj &#61; collection_object(creator, &amp;collection);<br/>    let collection &#61; borrow_collection(&amp;collection_obj);<br/><br/>    let mutator_ref &#61; if (<br/>        collection.mutable_token_description<br/>            &#124;&#124; collection.mutable_token_name<br/>            &#124;&#124; collection.mutable_token_uri<br/>    ) &#123;<br/>        option::some(token::generate_mutator_ref(&amp;constructor_ref))<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;;<br/><br/>    let burn_ref &#61; if (collection.tokens_burnable_by_creator) &#123;<br/>        option::some(token::generate_burn_ref(&amp;constructor_ref))<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;;<br/><br/>    let aptos_token &#61; AptosToken &#123;<br/>        burn_ref,<br/>        transfer_ref: option::none(),<br/>        mutator_ref,<br/>        property_mutator_ref: property_map::generate_mutator_ref(&amp;constructor_ref),<br/>    &#125;;<br/>    move_to(&amp;object_signer, aptos_token);<br/><br/>    let properties &#61; property_map::prepare_input(property_keys, property_types, property_values);<br/>    property_map::init(&amp;constructor_ref, properties);<br/><br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_borrow"></a>

## Function `borrow`



<pre><code>fun borrow&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;): &amp;aptos_token::AptosToken<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow&lt;T: key&gt;(token: &amp;Object&lt;T&gt;): &amp;AptosToken &#123;<br/>    let token_address &#61; object::object_address(token);<br/>    assert!(<br/>        exists&lt;AptosToken&gt;(token_address),<br/>        error::not_found(ETOKEN_DOES_NOT_EXIST),<br/>    );<br/>    borrow_global&lt;AptosToken&gt;(token_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_are_properties_mutable"></a>

## Function `are_properties_mutable`



<pre><code>&#35;[view]<br/>public fun are_properties_mutable&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun are_properties_mutable&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;<br/>    let collection &#61; token::collection_object(token);<br/>    borrow_collection(&amp;collection).mutable_token_properties<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_burnable"></a>

## Function `is_burnable`



<pre><code>&#35;[view]<br/>public fun is_burnable&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_burnable&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosToken &#123;<br/>    option::is_some(&amp;borrow(&amp;token).burn_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_freezable_by_creator"></a>

## Function `is_freezable_by_creator`



<pre><code>&#35;[view]<br/>public fun is_freezable_by_creator&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_freezable_by_creator&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;<br/>    are_collection_tokens_freezable(token::collection_object(token))<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_description"></a>

## Function `is_mutable_description`



<pre><code>&#35;[view]<br/>public fun is_mutable_description&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_description&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;<br/>    is_mutable_collection_token_description(token::collection_object(token))<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_name"></a>

## Function `is_mutable_name`



<pre><code>&#35;[view]<br/>public fun is_mutable_name&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_name&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;<br/>    is_mutable_collection_token_name(token::collection_object(token))<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_uri"></a>

## Function `is_mutable_uri`



<pre><code>&#35;[view]<br/>public fun is_mutable_uri&lt;T: key&gt;(token: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_uri&lt;T: key&gt;(token: Object&lt;T&gt;): bool acquires AptosCollection &#123;<br/>    is_mutable_collection_token_uri(token::collection_object(token))<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow"></a>

## Function `authorized_borrow`



<pre><code>fun authorized_borrow&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;, creator: &amp;signer): &amp;aptos_token::AptosToken<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun authorized_borrow&lt;T: key&gt;(token: &amp;Object&lt;T&gt;, creator: &amp;signer): &amp;AptosToken &#123;<br/>    let token_address &#61; object::object_address(token);<br/>    assert!(<br/>        exists&lt;AptosToken&gt;(token_address),<br/>        error::not_found(ETOKEN_DOES_NOT_EXIST),<br/>    );<br/><br/>    assert!(<br/>        token::creator(&#42;token) &#61;&#61; signer::address_of(creator),<br/>        error::permission_denied(ENOT_CREATOR),<br/>    );<br/>    borrow_global&lt;AptosToken&gt;(token_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_burn"></a>

## Function `burn`



<pre><code>public entry fun burn&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn&lt;T: key&gt;(creator: &amp;signer, token: Object&lt;T&gt;) acquires AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        option::is_some(&amp;aptos_token.burn_ref),<br/>        error::permission_denied(ETOKEN_NOT_BURNABLE),<br/>    );<br/>    move aptos_token;<br/>    let aptos_token &#61; move_from&lt;AptosToken&gt;(object::object_address(&amp;token));<br/>    let AptosToken &#123;<br/>        burn_ref,<br/>        transfer_ref: _,<br/>        mutator_ref: _,<br/>        property_mutator_ref,<br/>    &#125; &#61; aptos_token;<br/>    property_map::burn(property_mutator_ref);<br/>    token::burn(option::extract(&amp;mut burn_ref));<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_freeze_transfer"></a>

## Function `freeze_transfer`



<pre><code>public entry fun freeze_transfer&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun freeze_transfer&lt;T: key&gt;(creator: &amp;signer, token: Object&lt;T&gt;) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_collection_tokens_freezable(token::collection_object(token))<br/>            &amp;&amp; option::is_some(&amp;aptos_token.transfer_ref),<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    object::disable_ungated_transfer(option::borrow(&amp;aptos_token.transfer_ref));<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_unfreeze_transfer"></a>

## Function `unfreeze_transfer`



<pre><code>public entry fun unfreeze_transfer&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun unfreeze_transfer&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;<br/>) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_collection_tokens_freezable(token::collection_object(token))<br/>            &amp;&amp; option::is_some(&amp;aptos_token.transfer_ref),<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    object::enable_ungated_transfer(option::borrow(&amp;aptos_token.transfer_ref));<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_description"></a>

## Function `set_description`



<pre><code>public entry fun set_description&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_description&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    description: String,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    assert!(<br/>        is_mutable_description(token),<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    token::set_description(option::borrow(&amp;aptos_token.mutator_ref), description);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_name"></a>

## Function `set_name`



<pre><code>public entry fun set_name&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, name: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_name&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    name: String,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    assert!(<br/>        is_mutable_name(token),<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    token::set_name(option::borrow(&amp;aptos_token.mutator_ref), name);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_uri"></a>

## Function `set_uri`



<pre><code>public entry fun set_uri&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_uri&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    uri: String,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    assert!(<br/>        is_mutable_uri(token),<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    token::set_uri(option::borrow(&amp;aptos_token.mutator_ref), uri);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_add_property"></a>

## Function `add_property`



<pre><code>public entry fun add_property&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, type: string::String, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_property&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    key: String,<br/>    type: String,<br/>    value: vector&lt;u8&gt;,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_properties_mutable(token),<br/>        error::permission_denied(EPROPERTIES_NOT_MUTABLE),<br/>    );<br/><br/>    property_map::add(&amp;aptos_token.property_mutator_ref, key, type, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_add_typed_property"></a>

## Function `add_typed_property`



<pre><code>public entry fun add_typed_property&lt;T: key, V: drop&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, value: V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_typed_property&lt;T: key, V: drop&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    key: String,<br/>    value: V,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_properties_mutable(token),<br/>        error::permission_denied(EPROPERTIES_NOT_MUTABLE),<br/>    );<br/><br/>    property_map::add_typed(&amp;aptos_token.property_mutator_ref, key, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_remove_property"></a>

## Function `remove_property`



<pre><code>public entry fun remove_property&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun remove_property&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    key: String,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_properties_mutable(token),<br/>        error::permission_denied(EPROPERTIES_NOT_MUTABLE),<br/>    );<br/><br/>    property_map::remove(&amp;aptos_token.property_mutator_ref, &amp;key);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_update_property"></a>

## Function `update_property`



<pre><code>public entry fun update_property&lt;T: key&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, type: string::String, value: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_property&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    key: String,<br/>    type: String,<br/>    value: vector&lt;u8&gt;,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_properties_mutable(token),<br/>        error::permission_denied(EPROPERTIES_NOT_MUTABLE),<br/>    );<br/><br/>    property_map::update(&amp;aptos_token.property_mutator_ref, &amp;key, type, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_update_typed_property"></a>

## Function `update_typed_property`



<pre><code>public entry fun update_typed_property&lt;T: key, V: drop&gt;(creator: &amp;signer, token: object::Object&lt;T&gt;, key: string::String, value: V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun update_typed_property&lt;T: key, V: drop&gt;(<br/>    creator: &amp;signer,<br/>    token: Object&lt;T&gt;,<br/>    key: String,<br/>    value: V,<br/>) acquires AptosCollection, AptosToken &#123;<br/>    let aptos_token &#61; authorized_borrow(&amp;token, creator);<br/>    assert!(<br/>        are_properties_mutable(token),<br/>        error::permission_denied(EPROPERTIES_NOT_MUTABLE),<br/>    );<br/><br/>    property_map::update_typed(&amp;aptos_token.property_mutator_ref, &amp;key, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_collection_object"></a>

## Function `collection_object`



<pre><code>fun collection_object(creator: &amp;signer, name: &amp;string::String): object::Object&lt;aptos_token::AptosCollection&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun collection_object(creator: &amp;signer, name: &amp;String): Object&lt;AptosCollection&gt; &#123;<br/>    let collection_addr &#61; collection::create_collection_address(&amp;signer::address_of(creator), name);<br/>    object::address_to_object&lt;AptosCollection&gt;(collection_addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_borrow_collection"></a>

## Function `borrow_collection`



<pre><code>fun borrow_collection&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;): &amp;aptos_token::AptosCollection<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_collection&lt;T: key&gt;(token: &amp;Object&lt;T&gt;): &amp;AptosCollection &#123;<br/>    let collection_address &#61; object::object_address(token);<br/>    assert!(<br/>        exists&lt;AptosCollection&gt;(collection_address),<br/>        error::not_found(ECOLLECTION_DOES_NOT_EXIST),<br/>    );<br/>    borrow_global&lt;AptosCollection&gt;(collection_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_description"></a>

## Function `is_mutable_collection_description`



<pre><code>public fun is_mutable_collection_description&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_description&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).mutable_description<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_royalty"></a>

## Function `is_mutable_collection_royalty`



<pre><code>public fun is_mutable_collection_royalty&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_royalty&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    option::is_some(&amp;borrow_collection(&amp;collection).royalty_mutator_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_uri"></a>

## Function `is_mutable_collection_uri`



<pre><code>public fun is_mutable_collection_uri&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_uri&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).mutable_uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_description"></a>

## Function `is_mutable_collection_token_description`



<pre><code>public fun is_mutable_collection_token_description&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_description&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).mutable_token_description<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_name"></a>

## Function `is_mutable_collection_token_name`



<pre><code>public fun is_mutable_collection_token_name&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_name&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).mutable_token_name<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_uri"></a>

## Function `is_mutable_collection_token_uri`



<pre><code>public fun is_mutable_collection_token_uri&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_uri&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).mutable_token_uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_is_mutable_collection_token_properties"></a>

## Function `is_mutable_collection_token_properties`



<pre><code>public fun is_mutable_collection_token_properties&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_mutable_collection_token_properties&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).mutable_token_properties<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_burnable"></a>

## Function `are_collection_tokens_burnable`



<pre><code>public fun are_collection_tokens_burnable&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun are_collection_tokens_burnable&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).tokens_burnable_by_creator<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_are_collection_tokens_freezable"></a>

## Function `are_collection_tokens_freezable`



<pre><code>public fun are_collection_tokens_freezable&lt;T: key&gt;(collection: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun are_collection_tokens_freezable&lt;T: key&gt;(<br/>    collection: Object&lt;T&gt;,<br/>): bool acquires AptosCollection &#123;<br/>    borrow_collection(&amp;collection).tokens_freezable_by_creator<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_authorized_borrow_collection"></a>

## Function `authorized_borrow_collection`



<pre><code>fun authorized_borrow_collection&lt;T: key&gt;(collection: &amp;object::Object&lt;T&gt;, creator: &amp;signer): &amp;aptos_token::AptosCollection<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun authorized_borrow_collection&lt;T: key&gt;(collection: &amp;Object&lt;T&gt;, creator: &amp;signer): &amp;AptosCollection &#123;<br/>    let collection_address &#61; object::object_address(collection);<br/>    assert!(<br/>        exists&lt;AptosCollection&gt;(collection_address),<br/>        error::not_found(ECOLLECTION_DOES_NOT_EXIST),<br/>    );<br/>    assert!(<br/>        collection::creator(&#42;collection) &#61;&#61; signer::address_of(creator),<br/>        error::permission_denied(ENOT_CREATOR),<br/>    );<br/>    borrow_global&lt;AptosCollection&gt;(collection_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_description"></a>

## Function `set_collection_description`



<pre><code>public entry fun set_collection_description&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_collection_description&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;T&gt;,<br/>    description: String,<br/>) acquires AptosCollection &#123;<br/>    let aptos_collection &#61; authorized_borrow_collection(&amp;collection, creator);<br/>    assert!(<br/>        aptos_collection.mutable_description,<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    collection::set_description(option::borrow(&amp;aptos_collection.mutator_ref), description);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties"></a>

## Function `set_collection_royalties`



<pre><code>public fun set_collection_royalties&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, royalty: royalty::Royalty)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_collection_royalties&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;T&gt;,<br/>    royalty: royalty::Royalty,<br/>) acquires AptosCollection &#123;<br/>    let aptos_collection &#61; authorized_borrow_collection(&amp;collection, creator);<br/>    assert!(<br/>        option::is_some(&amp;aptos_collection.royalty_mutator_ref),<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    royalty::update(option::borrow(&amp;aptos_collection.royalty_mutator_ref), royalty);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_royalties_call"></a>

## Function `set_collection_royalties_call`



<pre><code>entry fun set_collection_royalties_call&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, royalty_numerator: u64, royalty_denominator: u64, payee_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun set_collection_royalties_call&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;T&gt;,<br/>    royalty_numerator: u64,<br/>    royalty_denominator: u64,<br/>    payee_address: address,<br/>) acquires AptosCollection &#123;<br/>    let royalty &#61; royalty::create(royalty_numerator, royalty_denominator, payee_address);<br/>    set_collection_royalties(creator, collection, royalty);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_aptos_token_set_collection_uri"></a>

## Function `set_collection_uri`



<pre><code>public entry fun set_collection_uri&lt;T: key&gt;(creator: &amp;signer, collection: object::Object&lt;T&gt;, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_collection_uri&lt;T: key&gt;(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;T&gt;,<br/>    uri: String,<br/>) acquires AptosCollection &#123;<br/>    let aptos_collection &#61; authorized_borrow_collection(&amp;collection, creator);<br/>    assert!(<br/>        aptos_collection.mutable_uri,<br/>        error::permission_denied(EFIELD_NOT_MUTABLE),<br/>    );<br/>    collection::set_uri(option::borrow(&amp;aptos_collection.mutator_ref), uri);<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
