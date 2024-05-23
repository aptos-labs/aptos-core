
<a id="0x4_token"></a>

# Module `0x4::token`

This defines an object&#45;based Token. The key differentiating features from the Aptos standard<br/> token are:<br/> &#42; Decoupled token ownership from token data.<br/> &#42; Explicit data model for token metadata via adjacent resources<br/> &#42; Extensible framework for tokens<br/>


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


<pre><code>use 0x1::aggregator_v2;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x1::string_utils;<br/>use 0x1::vector;<br/>use 0x4::collection;<br/>use 0x4::royalty;<br/></code></pre>



<a id="0x4_token_Token"></a>

## Resource `Token`

Represents the common fields to all tokens.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct Token has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection: object::Object&lt;collection::Collection&gt;</code>
</dt>
<dd>
 The collection from which this token resides.
</dd>
<dt>
<code>index: u64</code>
</dt>
<dd>
 Deprecated in favor of <code>index</code> inside TokenIdentifiers.<br/> Will be populated until concurrent_token_v2_enabled feature flag is enabled.<br/><br/> Unique identifier within the collection, optional, 0 means unassigned
</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>
 A brief description of the token.
</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 Deprecated in favor of <code>name</code> inside TokenIdentifiers.<br/> Will be populated until concurrent_token_v2_enabled feature flag is enabled.<br/><br/> The name of the token, which should be unique within the collection; the length of name<br/> should be smaller than 128, characters, eg: &quot;Aptos Animal &#35;1234&quot;
</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off&#45;chain<br/> storage; the URL length will likely need a maximum any suggestions?
</dd>
<dt>
<code>mutation_events: event::EventHandle&lt;token::MutationEvent&gt;</code>
</dt>
<dd>
 Emitted upon any mutation of the token.
</dd>
</dl>


</details>

<a id="0x4_token_TokenIdentifiers"></a>

## Resource `TokenIdentifiers`

Represents first addition to the common fields for all tokens<br/> Starts being populated once aggregator_v2_api_enabled is enabled.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct TokenIdentifiers has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: aggregator_v2::AggregatorSnapshot&lt;u64&gt;</code>
</dt>
<dd>
 Unique identifier within the collection, optional, 0 means unassigned
</dd>
<dt>
<code>name: aggregator_v2::DerivedStringSnapshot</code>
</dt>
<dd>
 The name of the token, which should be unique within the collection; the length of name<br/> should be smaller than 128, characters, eg: &quot;Aptos Animal &#35;1234&quot;
</dd>
</dl>


</details>

<a id="0x4_token_ConcurrentTokenIdentifiers"></a>

## Resource `ConcurrentTokenIdentifiers`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>&#35;[deprecated]<br/>struct ConcurrentTokenIdentifiers has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: aggregator_v2::AggregatorSnapshot&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>name: aggregator_v2::AggregatorSnapshot&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_token_BurnRef"></a>

## Struct `BurnRef`

This enables burning an NFT, if possible, it will also delete the object. Note, the data<br/> in inner and self occupies 32&#45;bytes each, rather than have both, this data structure makes<br/> a small optimization to support either and take a fixed amount of 34&#45;bytes.


<pre><code>struct BurnRef has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: option::Option&lt;object::DeleteRef&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>self: option::Option&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x4_token_MutatorRef"></a>

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

<a id="0x4_token_MutationEvent"></a>

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

<a id="0x4_token_Mutation"></a>

## Struct `Mutation`



<pre><code>&#35;[event]<br/>struct Mutation has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>mutated_field_name: string::String</code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x4_token_EURI_TOO_LONG"></a>

The URI is over the maximum length


<pre><code>const EURI_TOO_LONG: u64 &#61; 5;<br/></code></pre>



<a id="0x4_token_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;<br/></code></pre>



<a id="0x4_token_EDESCRIPTION_TOO_LONG"></a>

The description is over the maximum length


<pre><code>const EDESCRIPTION_TOO_LONG: u64 &#61; 6;<br/></code></pre>



<a id="0x4_token_MAX_DESCRIPTION_LENGTH"></a>



<pre><code>const MAX_DESCRIPTION_LENGTH: u64 &#61; 2048;<br/></code></pre>



<a id="0x4_token_EFIELD_NOT_MUTABLE"></a>

The field being changed is not mutable


<pre><code>const EFIELD_NOT_MUTABLE: u64 &#61; 3;<br/></code></pre>



<a id="0x4_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code>const ENOT_CREATOR: u64 &#61; 2;<br/></code></pre>



<a id="0x4_token_ESEED_TOO_LONG"></a>

The seed is over the maximum length


<pre><code>const ESEED_TOO_LONG: u64 &#61; 7;<br/></code></pre>



<a id="0x4_token_ETOKEN_DOES_NOT_EXIST"></a>

The token does not exist


<pre><code>const ETOKEN_DOES_NOT_EXIST: u64 &#61; 1;<br/></code></pre>



<a id="0x4_token_ETOKEN_NAME_TOO_LONG"></a>

The token name is over the maximum length


<pre><code>const ETOKEN_NAME_TOO_LONG: u64 &#61; 4;<br/></code></pre>



<a id="0x4_token_MAX_TOKEN_NAME_LENGTH"></a>



<pre><code>const MAX_TOKEN_NAME_LENGTH: u64 &#61; 128;<br/></code></pre>



<a id="0x4_token_MAX_TOKEN_SEED_LENGTH"></a>



<pre><code>const MAX_TOKEN_SEED_LENGTH: u64 &#61; 128;<br/></code></pre>



<a id="0x4_token_create_common"></a>

## Function `create_common`



<pre><code>fun create_common(constructor_ref: &amp;object::ConstructorRef, creator_address: address, collection_name: string::String, description: string::String, name_prefix: string::String, name_with_index_suffix: option::Option&lt;string::String&gt;, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun create_common(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    creator_address: address,<br/>    collection_name: String,<br/>    description: String,<br/>    name_prefix: String,<br/>    // If option::some, numbered token is created &#45; i.e. index is appended to the name.<br/>    // If option::none, name_prefix is the full name of the token.<br/>    name_with_index_suffix: Option&lt;String&gt;,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>) &#123;<br/>    let collection_addr &#61; collection::create_collection_address(&amp;creator_address, &amp;collection_name);<br/>    let collection &#61; object::address_to_object&lt;Collection&gt;(collection_addr);<br/><br/>    create_common_with_collection(<br/>        constructor_ref,<br/>        collection,<br/>        description,<br/>        name_prefix,<br/>        name_with_index_suffix,<br/>        royalty,<br/>        uri<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_common_with_collection"></a>

## Function `create_common_with_collection`



<pre><code>fun create_common_with_collection(constructor_ref: &amp;object::ConstructorRef, collection: object::Object&lt;collection::Collection&gt;, description: string::String, name_prefix: string::String, name_with_index_suffix: option::Option&lt;string::String&gt;, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun create_common_with_collection(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    collection: Object&lt;Collection&gt;,<br/>    description: String,<br/>    name_prefix: String,<br/>    // If option::some, numbered token is created &#45; i.e. index is appended to the name.<br/>    // If option::none, name_prefix is the full name of the token.<br/>    name_with_index_suffix: Option&lt;String&gt;,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>) &#123;<br/>    if (option::is_some(&amp;name_with_index_suffix)) &#123;<br/>        // Be conservative, as we don&apos;t know what length the index will be, and assume worst case (20 chars in MAX_U64)<br/>        assert!(<br/>            string::length(&amp;name_prefix) &#43; 20 &#43; string::length(<br/>                option::borrow(&amp;name_with_index_suffix)<br/>            ) &lt;&#61; MAX_TOKEN_NAME_LENGTH,<br/>            error::out_of_range(ETOKEN_NAME_TOO_LONG)<br/>        );<br/>    &#125; else &#123;<br/>        assert!(string::length(&amp;name_prefix) &lt;&#61; MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));<br/>    &#125;;<br/>    assert!(string::length(&amp;description) &lt;&#61; MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));<br/><br/>    let object_signer &#61; object::generate_signer(constructor_ref);<br/><br/>    // TODO[agg_v2](cleanup) once this flag is enabled, cleanup code for aggregator_api_enabled &#61; false.<br/>    // Flag which controls whether any functions from aggregator_v2 module can be called.<br/>    let aggregator_api_enabled &#61; features::aggregator_v2_api_enabled();<br/>    // Flag which controls whether we are going to still continue writing to deprecated fields.<br/>    let concurrent_token_v2_enabled &#61; features::concurrent_token_v2_enabled();<br/><br/>    let (deprecated_index, deprecated_name) &#61; if (aggregator_api_enabled) &#123;<br/>        let index &#61; option::destroy_with_default(<br/>            collection::increment_concurrent_supply(&amp;collection, signer::address_of(&amp;object_signer)),<br/>            aggregator_v2::create_snapshot&lt;u64&gt;(0)<br/>        );<br/><br/>        // If create_numbered_token called us, add index to the name.<br/>        let name &#61; if (option::is_some(&amp;name_with_index_suffix)) &#123;<br/>            aggregator_v2::derive_string_concat(name_prefix, &amp;index, option::extract(&amp;mut name_with_index_suffix))<br/>        &#125; else &#123;<br/>            aggregator_v2::create_derived_string(name_prefix)<br/>        &#125;;<br/><br/>        // Until concurrent_token_v2_enabled is enabled, we still need to write to deprecated fields.<br/>        // Otherwise we put empty values there.<br/>        // (we need to do these calls before creating token_concurrent, to avoid copying objects)<br/>        let deprecated_index &#61; if (concurrent_token_v2_enabled) &#123;<br/>            0<br/>        &#125; else &#123;<br/>            aggregator_v2::read_snapshot(&amp;index)<br/>        &#125;;<br/>        let deprecated_name &#61; if (concurrent_token_v2_enabled) &#123;<br/>            string::utf8(b&quot;&quot;)<br/>        &#125; else &#123;<br/>            aggregator_v2::read_derived_string(&amp;name)<br/>        &#125;;<br/><br/>        // If aggregator_api_enabled, we always populate newly added fields<br/>        let token_concurrent &#61; TokenIdentifiers &#123;<br/>            index,<br/>            name,<br/>        &#125;;<br/>        move_to(&amp;object_signer, token_concurrent);<br/><br/>        (deprecated_index, deprecated_name)<br/>    &#125; else &#123;<br/>        // If aggregator_api_enabled is disabled, we cannot use increment_concurrent_supply or<br/>        // create TokenIdentifiers, so we fallback to the old behavior.<br/>        let id &#61; collection::increment_supply(&amp;collection, signer::address_of(&amp;object_signer));<br/>        let index &#61; option::get_with_default(&amp;mut id, 0);<br/><br/>        // If create_numbered_token called us, add index to the name.<br/>        let name &#61; if (option::is_some(&amp;name_with_index_suffix)) &#123;<br/>            let name &#61; name_prefix;<br/>            string::append(&amp;mut name, to_string&lt;u64&gt;(&amp;index));<br/>            string::append(&amp;mut name, option::extract(&amp;mut name_with_index_suffix));<br/>            name<br/>        &#125; else &#123;<br/>            name_prefix<br/>        &#125;;<br/><br/>        (index, name)<br/>    &#125;;<br/><br/>    let token &#61; Token &#123;<br/>        collection,<br/>        index: deprecated_index,<br/>        description,<br/>        name: deprecated_name,<br/>        uri,<br/>        mutation_events: object::new_event_handle(&amp;object_signer),<br/>    &#125;;<br/>    move_to(&amp;object_signer, token);<br/><br/>    if (option::is_some(&amp;royalty)) &#123;<br/>        royalty::init(constructor_ref, option::extract(&amp;mut royalty))<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_token"></a>

## Function `create_token`

Creates a new token object with a unique address and returns the ConstructorRef<br/> for additional specialization.<br/> This takes in the collection object instead of the collection name.<br/> This function must be called if the collection name has been previously changed.


<pre><code>public fun create_token(creator: &amp;signer, collection: object::Object&lt;collection::Collection&gt;, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;Collection&gt;,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let constructor_ref &#61; object::create_object(creator_address);<br/>    create_common_with_collection(<br/>        &amp;constructor_ref,<br/>        collection,<br/>        description,<br/>        name,<br/>        option::none(),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create"></a>

## Function `create`

Creates a new token object with a unique address and returns the ConstructorRef<br/> for additional specialization.


<pre><code>public fun create(creator: &amp;signer, collection_name: string::String, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create(<br/>    creator: &amp;signer,<br/>    collection_name: String,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let constructor_ref &#61; object::create_object(creator_address);<br/>    create_common(<br/>        &amp;constructor_ref,<br/>        creator_address,<br/>        collection_name,<br/>        description,<br/>        name,<br/>        option::none(),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_numbered_token_object"></a>

## Function `create_numbered_token_object`

Creates a new token object with a unique address and returns the ConstructorRef<br/> for additional specialization.<br/> The name is created by concatenating the (name_prefix, index, name_suffix).<br/> After flag concurrent_token_v2_enabled is enabled, this function will allow<br/> creating tokens in parallel, from the same collection, while providing sequential names.<br/><br/> This takes in the collection object instead of the collection name.<br/> This function must be called if the collection name has been previously changed.


<pre><code>public fun create_numbered_token_object(creator: &amp;signer, collection: object::Object&lt;collection::Collection&gt;, description: string::String, name_with_index_prefix: string::String, name_with_index_suffix: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_numbered_token_object(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;Collection&gt;,<br/>    description: String,<br/>    name_with_index_prefix: String,<br/>    name_with_index_suffix: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let constructor_ref &#61; object::create_object(creator_address);<br/>    create_common_with_collection(<br/>        &amp;constructor_ref,<br/>        collection,<br/>        description,<br/>        name_with_index_prefix,<br/>        option::some(name_with_index_suffix),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_numbered_token"></a>

## Function `create_numbered_token`

Creates a new token object with a unique address and returns the ConstructorRef<br/> for additional specialization.<br/> The name is created by concatenating the (name_prefix, index, name_suffix).<br/> After flag concurrent_token_v2_enabled is enabled, this function will allow<br/> creating tokens in parallel, from the same collection, while providing sequential names.


<pre><code>public fun create_numbered_token(creator: &amp;signer, collection_name: string::String, description: string::String, name_with_index_prefix: string::String, name_with_index_suffix: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_numbered_token(<br/>    creator: &amp;signer,<br/>    collection_name: String,<br/>    description: String,<br/>    name_with_index_prefix: String,<br/>    name_with_index_suffix: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let constructor_ref &#61; object::create_object(creator_address);<br/>    create_common(<br/>        &amp;constructor_ref,<br/>        creator_address,<br/>        collection_name,<br/>        description,<br/>        name_with_index_prefix,<br/>        option::some(name_with_index_suffix),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_named_token_object"></a>

## Function `create_named_token_object`

Creates a new token object from a token name and returns the ConstructorRef for<br/> additional specialization.<br/> This function must be called if the collection name has been previously changed.


<pre><code>public fun create_named_token_object(creator: &amp;signer, collection: object::Object&lt;collection::Collection&gt;, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_named_token_object(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;Collection&gt;,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let seed &#61; create_token_seed(&amp;collection::name(collection), &amp;name);<br/>    let constructor_ref &#61; object::create_named_object(creator, seed);<br/>    create_common_with_collection(<br/>        &amp;constructor_ref,<br/>        collection,<br/>        description,<br/>        name,<br/>        option::none(),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_named_token"></a>

## Function `create_named_token`

Creates a new token object from a token name and returns the ConstructorRef for<br/> additional specialization.


<pre><code>public fun create_named_token(creator: &amp;signer, collection_name: string::String, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_named_token(<br/>    creator: &amp;signer,<br/>    collection_name: String,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let seed &#61; create_token_seed(&amp;collection_name, &amp;name);<br/><br/>    let constructor_ref &#61; object::create_named_object(creator, seed);<br/>    create_common(<br/>        &amp;constructor_ref,<br/>        creator_address,<br/>        collection_name,<br/>        description,<br/>        name,<br/>        option::none(),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_named_token_from_seed"></a>

## Function `create_named_token_from_seed`

Creates a new token object from a token name and seed.<br/> Returns the ConstructorRef for additional specialization.<br/> This function must be called if the collection name has been previously changed.


<pre><code>public fun create_named_token_from_seed(creator: &amp;signer, collection: object::Object&lt;collection::Collection&gt;, description: string::String, name: string::String, seed: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_named_token_from_seed(<br/>    creator: &amp;signer,<br/>    collection: Object&lt;Collection&gt;,<br/>    description: String,<br/>    name: String,<br/>    seed: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let seed &#61; create_token_name_with_seed(&amp;collection::name(collection), &amp;name, &amp;seed);<br/>    let constructor_ref &#61; object::create_named_object(creator, seed);<br/>    create_common_with_collection(&amp;constructor_ref, collection, description, name, option::none(), royalty, uri);<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_from_account"></a>

## Function `create_from_account`

DEPRECATED: Use <code>create</code> instead for identical behavior.<br/><br/> Creates a new token object from an account GUID and returns the ConstructorRef for<br/> additional specialization.


<pre><code>&#35;[deprecated]<br/>public fun create_from_account(creator: &amp;signer, collection_name: string::String, description: string::String, name: string::String, royalty: option::Option&lt;royalty::Royalty&gt;, uri: string::String): object::ConstructorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_account(<br/>    creator: &amp;signer,<br/>    collection_name: String,<br/>    description: String,<br/>    name: String,<br/>    royalty: Option&lt;Royalty&gt;,<br/>    uri: String,<br/>): ConstructorRef &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    let constructor_ref &#61; object::create_object_from_account(creator);<br/>    create_common(<br/>        &amp;constructor_ref,<br/>        creator_address,<br/>        collection_name,<br/>        description,<br/>        name,<br/>        option::none(),<br/>        royalty,<br/>        uri<br/>    );<br/>    constructor_ref<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_token_address"></a>

## Function `create_token_address`

Generates the token&apos;s address based upon the creator&apos;s address, the collection&apos;s name and the token&apos;s name.


<pre><code>public fun create_token_address(creator: &amp;address, collection: &amp;string::String, name: &amp;string::String): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_address(creator: &amp;address, collection: &amp;String, name: &amp;String): address &#123;<br/>    object::create_object_address(creator, create_token_seed(collection, name))<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_token_address_with_seed"></a>

## Function `create_token_address_with_seed`

Generates the token&apos;s address based upon the creator&apos;s address, the collection object and the token&apos;s name and seed.


<pre><code>&#35;[view]<br/>public fun create_token_address_with_seed(creator: address, collection: string::String, name: string::String, seed: string::String): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_address_with_seed(creator: address, collection: String, name: String, seed: String): address &#123;<br/>    let seed &#61; create_token_name_with_seed(&amp;collection, &amp;name, &amp;seed);<br/>    object::create_object_address(&amp;creator, seed)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_token_seed"></a>

## Function `create_token_seed`

Named objects are derived from a seed, the token&apos;s seed is its name appended to the collection&apos;s name.


<pre><code>public fun create_token_seed(collection: &amp;string::String, name: &amp;string::String): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_seed(collection: &amp;String, name: &amp;String): vector&lt;u8&gt; &#123;<br/>    assert!(string::length(name) &lt;&#61; MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));<br/>    let seed &#61; &#42;string::bytes(collection);<br/>    vector::append(&amp;mut seed, b&quot;::&quot;);<br/>    vector::append(&amp;mut seed, &#42;string::bytes(name));<br/>    seed<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_create_token_name_with_seed"></a>

## Function `create_token_name_with_seed`



<pre><code>public fun create_token_name_with_seed(collection: &amp;string::String, name: &amp;string::String, seed: &amp;string::String): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_name_with_seed(collection: &amp;String, name: &amp;String, seed: &amp;String): vector&lt;u8&gt; &#123;<br/>    assert!(string::length(seed) &lt;&#61; MAX_TOKEN_SEED_LENGTH, error::out_of_range(ESEED_TOO_LONG));<br/>    let seeds &#61; create_token_seed(collection, name);<br/>    vector::append(&amp;mut seeds, &#42;string::bytes(seed));<br/>    seeds<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code>public fun generate_mutator_ref(ref: &amp;object::ConstructorRef): token::MutatorRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_mutator_ref(ref: &amp;ConstructorRef): MutatorRef &#123;<br/>    let object &#61; object::object_from_constructor_ref&lt;Token&gt;(ref);<br/>    MutatorRef &#123; self: object::object_address(&amp;object) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a BurnRef, which gates the ability to burn the given token.


<pre><code>public fun generate_burn_ref(ref: &amp;object::ConstructorRef): token::BurnRef<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_burn_ref(ref: &amp;ConstructorRef): BurnRef &#123;<br/>    let (inner, self) &#61; if (object::can_generate_delete_ref(ref)) &#123;<br/>        let delete_ref &#61; object::generate_delete_ref(ref);<br/>        (option::some(delete_ref), option::none())<br/>    &#125; else &#123;<br/>        let addr &#61; object::address_from_constructor_ref(ref);<br/>        (option::none(), option::some(addr))<br/>    &#125;;<br/>    BurnRef &#123; self, inner &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_address_from_burn_ref"></a>

## Function `address_from_burn_ref`

Extracts the tokens address from a BurnRef.


<pre><code>public fun address_from_burn_ref(ref: &amp;token::BurnRef): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_burn_ref(ref: &amp;BurnRef): address &#123;<br/>    if (option::is_some(&amp;ref.inner)) &#123;<br/>        object::address_from_delete_ref(option::borrow(&amp;ref.inner))<br/>    &#125; else &#123;<br/>        &#42;option::borrow(&amp;ref.self)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_borrow"></a>

## Function `borrow`



<pre><code>fun borrow&lt;T: key&gt;(token: &amp;object::Object&lt;T&gt;): &amp;token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow&lt;T: key&gt;(token: &amp;Object&lt;T&gt;): &amp;Token acquires Token &#123;<br/>    let token_address &#61; object::object_address(token);<br/>    assert!(<br/>        exists&lt;Token&gt;(token_address),<br/>        error::not_found(ETOKEN_DOES_NOT_EXIST),<br/>    );<br/>    borrow_global&lt;Token&gt;(token_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_creator"></a>

## Function `creator`



<pre><code>&#35;[view]<br/>public fun creator&lt;T: key&gt;(token: object::Object&lt;T&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun creator&lt;T: key&gt;(token: Object&lt;T&gt;): address acquires Token &#123;<br/>    collection::creator(borrow(&amp;token).collection)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_collection_name"></a>

## Function `collection_name`



<pre><code>&#35;[view]<br/>public fun collection_name&lt;T: key&gt;(token: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun collection_name&lt;T: key&gt;(token: Object&lt;T&gt;): String acquires Token &#123;<br/>    collection::name(borrow(&amp;token).collection)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_collection_object"></a>

## Function `collection_object`



<pre><code>&#35;[view]<br/>public fun collection_object&lt;T: key&gt;(token: object::Object&lt;T&gt;): object::Object&lt;collection::Collection&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun collection_object&lt;T: key&gt;(token: Object&lt;T&gt;): Object&lt;Collection&gt; acquires Token &#123;<br/>    borrow(&amp;token).collection<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_description"></a>

## Function `description`



<pre><code>&#35;[view]<br/>public fun description&lt;T: key&gt;(token: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun description&lt;T: key&gt;(token: Object&lt;T&gt;): String acquires Token &#123;<br/>    borrow(&amp;token).description<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_name"></a>

## Function `name`

Avoid this method in the same transaction as the token is minted<br/> as that would prohibit transactions to be executed in parallel.


<pre><code>&#35;[view]<br/>public fun name&lt;T: key&gt;(token: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun name&lt;T: key&gt;(token: Object&lt;T&gt;): String acquires Token, TokenIdentifiers &#123;<br/>    let token_address &#61; object::object_address(&amp;token);<br/>    if (exists&lt;TokenIdentifiers&gt;(token_address)) &#123;<br/>        aggregator_v2::read_derived_string(&amp;borrow_global&lt;TokenIdentifiers&gt;(token_address).name)<br/>    &#125; else &#123;<br/>        borrow(&amp;token).name<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_uri"></a>

## Function `uri`



<pre><code>&#35;[view]<br/>public fun uri&lt;T: key&gt;(token: object::Object&lt;T&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun uri&lt;T: key&gt;(token: Object&lt;T&gt;): String acquires Token &#123;<br/>    borrow(&amp;token).uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_royalty"></a>

## Function `royalty`



<pre><code>&#35;[view]<br/>public fun royalty&lt;T: key&gt;(token: object::Object&lt;T&gt;): option::Option&lt;royalty::Royalty&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun royalty&lt;T: key&gt;(token: Object&lt;T&gt;): Option&lt;Royalty&gt; acquires Token &#123;<br/>    borrow(&amp;token);<br/>    let royalty &#61; royalty::get(token);<br/>    if (option::is_some(&amp;royalty)) &#123;<br/>        royalty<br/>    &#125; else &#123;<br/>        let creator &#61; creator(token);<br/>        let collection_name &#61; collection_name(token);<br/>        let collection_address &#61; collection::create_collection_address(&amp;creator, &amp;collection_name);<br/>        let collection &#61; object::address_to_object&lt;collection::Collection&gt;(collection_address);<br/>        royalty::get(collection)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_index"></a>

## Function `index`

Avoid this method in the same transaction as the token is minted<br/> as that would prohibit transactions to be executed in parallel.


<pre><code>&#35;[view]<br/>public fun index&lt;T: key&gt;(token: object::Object&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun index&lt;T: key&gt;(token: Object&lt;T&gt;): u64 acquires Token, TokenIdentifiers &#123;<br/>    let token_address &#61; object::object_address(&amp;token);<br/>    if (exists&lt;TokenIdentifiers&gt;(token_address)) &#123;<br/>        aggregator_v2::read_snapshot(&amp;borrow_global&lt;TokenIdentifiers&gt;(token_address).index)<br/>    &#125; else &#123;<br/>        borrow(&amp;token).index<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_borrow_mut"></a>

## Function `borrow_mut`



<pre><code>fun borrow_mut(mutator_ref: &amp;token::MutatorRef): &amp;mut token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun borrow_mut(mutator_ref: &amp;MutatorRef): &amp;mut Token acquires Token &#123;<br/>    assert!(<br/>        exists&lt;Token&gt;(mutator_ref.self),<br/>        error::not_found(ETOKEN_DOES_NOT_EXIST),<br/>    );<br/>    borrow_global_mut&lt;Token&gt;(mutator_ref.self)<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_burn"></a>

## Function `burn`



<pre><code>public fun burn(burn_ref: token::BurnRef)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn(burn_ref: BurnRef) acquires Token, TokenIdentifiers &#123;<br/>    let (addr, previous_owner) &#61; if (option::is_some(&amp;burn_ref.inner)) &#123;<br/>        let delete_ref &#61; option::extract(&amp;mut burn_ref.inner);<br/>        let addr &#61; object::address_from_delete_ref(&amp;delete_ref);<br/>        let previous_owner &#61; object::owner(object::address_to_object&lt;Token&gt;(addr));<br/>        object::delete(delete_ref);<br/>        (addr, previous_owner)<br/>    &#125; else &#123;<br/>        let addr &#61; option::extract(&amp;mut burn_ref.self);<br/>        let previous_owner &#61; object::owner(object::address_to_object&lt;Token&gt;(addr));<br/>        (addr, previous_owner)<br/>    &#125;;<br/><br/>    if (royalty::exists_at(addr)) &#123;<br/>        royalty::delete(addr)<br/>    &#125;;<br/><br/>    let Token &#123;<br/>        collection,<br/>        index: deprecated_index,<br/>        description: _,<br/>        name: _,<br/>        uri: _,<br/>        mutation_events,<br/>    &#125; &#61; move_from&lt;Token&gt;(addr);<br/><br/>    let index &#61; if (exists&lt;TokenIdentifiers&gt;(addr)) &#123;<br/>        let TokenIdentifiers &#123;<br/>            index,<br/>            name: _,<br/>        &#125; &#61; move_from&lt;TokenIdentifiers&gt;(addr);<br/>        aggregator_v2::read_snapshot(&amp;index)<br/>    &#125; else &#123;<br/>        deprecated_index<br/>    &#125;;<br/><br/>    event::destroy_handle(mutation_events);<br/>    collection::decrement_supply(&amp;collection, addr, option::some(index), previous_owner);<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_set_description"></a>

## Function `set_description`



<pre><code>public fun set_description(mutator_ref: &amp;token::MutatorRef, description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_description(mutator_ref: &amp;MutatorRef, description: String) acquires Token &#123;<br/>    assert!(string::length(&amp;description) &lt;&#61; MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));<br/>    let token &#61; borrow_mut(mutator_ref);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Mutation &#123;<br/>            token_address: mutator_ref.self,<br/>            mutated_field_name: string::utf8(b&quot;description&quot;),<br/>            old_value: token.description,<br/>            new_value: description<br/>        &#125;)<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut token.mutation_events,<br/>        MutationEvent &#123;<br/>            mutated_field_name: string::utf8(b&quot;description&quot;),<br/>            old_value: token.description,<br/>            new_value: description<br/>        &#125;,<br/>    );<br/>    token.description &#61; description;<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_set_name"></a>

## Function `set_name`



<pre><code>public fun set_name(mutator_ref: &amp;token::MutatorRef, name: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_name(mutator_ref: &amp;MutatorRef, name: String) acquires Token, TokenIdentifiers &#123;<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));<br/><br/>    let token &#61; borrow_mut(mutator_ref);<br/><br/>    let old_name &#61; if (exists&lt;TokenIdentifiers&gt;(mutator_ref.self)) &#123;<br/>        let token_concurrent &#61; borrow_global_mut&lt;TokenIdentifiers&gt;(mutator_ref.self);<br/>        let old_name &#61; aggregator_v2::read_derived_string(&amp;token_concurrent.name);<br/>        token_concurrent.name &#61; aggregator_v2::create_derived_string(name);<br/>        old_name<br/>    &#125; else &#123;<br/>        let old_name &#61; token.name;<br/>        token.name &#61; name;<br/>        old_name<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Mutation &#123;<br/>            token_address: mutator_ref.self,<br/>            mutated_field_name: string::utf8(b&quot;name&quot;),<br/>            old_value: old_name,<br/>            new_value: name<br/>        &#125;)<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut token.mutation_events,<br/>        MutationEvent &#123;<br/>            mutated_field_name: string::utf8(b&quot;name&quot;),<br/>            old_value: old_name,<br/>            new_value: name<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x4_token_set_uri"></a>

## Function `set_uri`



<pre><code>public fun set_uri(mutator_ref: &amp;token::MutatorRef, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_uri(mutator_ref: &amp;MutatorRef, uri: String) acquires Token &#123;<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));<br/>    let token &#61; borrow_mut(mutator_ref);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Mutation &#123;<br/>            token_address: mutator_ref.self,<br/>            mutated_field_name: string::utf8(b&quot;uri&quot;),<br/>            old_value: token.uri,<br/>            new_value: uri,<br/>        &#125;)<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut token.mutation_events,<br/>        MutationEvent &#123;<br/>            mutated_field_name: string::utf8(b&quot;uri&quot;),<br/>            old_value: token.uri,<br/>            new_value: uri,<br/>        &#125;,<br/>    );<br/>    token.uri &#61; uri;<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
