
<a id="0x3_token_event_store"></a>

# Module `0x3::token_event_store`

This module provides utils to add and emit new token events that are not in token.move


-  [Struct `CollectionDescriptionMutateEvent`](#0x3_token_event_store_CollectionDescriptionMutateEvent)
-  [Struct `CollectionDescriptionMutate`](#0x3_token_event_store_CollectionDescriptionMutate)
-  [Struct `CollectionUriMutateEvent`](#0x3_token_event_store_CollectionUriMutateEvent)
-  [Struct `CollectionUriMutate`](#0x3_token_event_store_CollectionUriMutate)
-  [Struct `CollectionMaxiumMutateEvent`](#0x3_token_event_store_CollectionMaxiumMutateEvent)
-  [Struct `CollectionMaxiumMutate`](#0x3_token_event_store_CollectionMaxiumMutate)
-  [Struct `OptInTransferEvent`](#0x3_token_event_store_OptInTransferEvent)
-  [Struct `OptInTransfer`](#0x3_token_event_store_OptInTransfer)
-  [Struct `UriMutationEvent`](#0x3_token_event_store_UriMutationEvent)
-  [Struct `UriMutation`](#0x3_token_event_store_UriMutation)
-  [Struct `DefaultPropertyMutateEvent`](#0x3_token_event_store_DefaultPropertyMutateEvent)
-  [Struct `DefaultPropertyMutate`](#0x3_token_event_store_DefaultPropertyMutate)
-  [Struct `DescriptionMutateEvent`](#0x3_token_event_store_DescriptionMutateEvent)
-  [Struct `DescriptionMutate`](#0x3_token_event_store_DescriptionMutate)
-  [Struct `RoyaltyMutateEvent`](#0x3_token_event_store_RoyaltyMutateEvent)
-  [Struct `RoyaltyMutate`](#0x3_token_event_store_RoyaltyMutate)
-  [Struct `MaxiumMutateEvent`](#0x3_token_event_store_MaxiumMutateEvent)
-  [Struct `MaximumMutate`](#0x3_token_event_store_MaximumMutate)
-  [Resource `TokenEventStoreV1`](#0x3_token_event_store_TokenEventStoreV1)
-  [Function `initialize_token_event_store`](#0x3_token_event_store_initialize_token_event_store)
-  [Function `emit_collection_uri_mutate_event`](#0x3_token_event_store_emit_collection_uri_mutate_event)
-  [Function `emit_collection_description_mutate_event`](#0x3_token_event_store_emit_collection_description_mutate_event)
-  [Function `emit_collection_maximum_mutate_event`](#0x3_token_event_store_emit_collection_maximum_mutate_event)
-  [Function `emit_token_opt_in_event`](#0x3_token_event_store_emit_token_opt_in_event)
-  [Function `emit_token_uri_mutate_event`](#0x3_token_event_store_emit_token_uri_mutate_event)
-  [Function `emit_default_property_mutate_event`](#0x3_token_event_store_emit_default_property_mutate_event)
-  [Function `emit_token_descrition_mutate_event`](#0x3_token_event_store_emit_token_descrition_mutate_event)
-  [Function `emit_token_royalty_mutate_event`](#0x3_token_event_store_emit_token_royalty_mutate_event)
-  [Function `emit_token_maximum_mutate_event`](#0x3_token_event_store_emit_token_maximum_mutate_event)
-  [Specification](#@Specification_0)
    -  [Function `initialize_token_event_store`](#@Specification_0_initialize_token_event_store)
    -  [Function `emit_collection_uri_mutate_event`](#@Specification_0_emit_collection_uri_mutate_event)
    -  [Function `emit_collection_description_mutate_event`](#@Specification_0_emit_collection_description_mutate_event)
    -  [Function `emit_collection_maximum_mutate_event`](#@Specification_0_emit_collection_maximum_mutate_event)
    -  [Function `emit_token_opt_in_event`](#@Specification_0_emit_token_opt_in_event)
    -  [Function `emit_token_uri_mutate_event`](#@Specification_0_emit_token_uri_mutate_event)
    -  [Function `emit_default_property_mutate_event`](#@Specification_0_emit_default_property_mutate_event)
    -  [Function `emit_token_descrition_mutate_event`](#@Specification_0_emit_token_descrition_mutate_event)
    -  [Function `emit_token_royalty_mutate_event`](#@Specification_0_emit_token_royalty_mutate_event)
    -  [Function `emit_token_maximum_mutate_event`](#@Specification_0_emit_token_maximum_mutate_event)


<pre><code>use 0x1::account;
use 0x1::any;
use 0x1::event;
use 0x1::features;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
use 0x3::property_map;
</code></pre>



<a id="0x3_token_event_store_CollectionDescriptionMutateEvent"></a>

## Struct `CollectionDescriptionMutateEvent`

Event emitted when collection description is mutated


<pre><code>struct CollectionDescriptionMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionDescriptionMutate"></a>

## Struct `CollectionDescriptionMutate`

Event emitted when collection description is mutated


<pre><code>&#35;[event]
struct CollectionDescriptionMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionUriMutateEvent"></a>

## Struct `CollectionUriMutateEvent`

Event emitted when collection uri is mutated


<pre><code>struct CollectionUriMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionUriMutate"></a>

## Struct `CollectionUriMutate`

Event emitted when collection uri is mutated


<pre><code>&#35;[event]
struct CollectionUriMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionMaxiumMutateEvent"></a>

## Struct `CollectionMaxiumMutateEvent`

Event emitted when the collection maximum is mutated


<pre><code>struct CollectionMaxiumMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionMaxiumMutate"></a>

## Struct `CollectionMaxiumMutate`

Event emitted when the collection maximum is mutated


<pre><code>&#35;[event]
struct CollectionMaxiumMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_OptInTransferEvent"></a>

## Struct `OptInTransferEvent`

Event emitted when an user opt-in the direct transfer


<pre><code>struct OptInTransferEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>opt_in: bool</code>
</dt>
<dd>
 True if the user opt in, false if the user opt-out
</dd>
</dl>


</details>

<a id="0x3_token_event_store_OptInTransfer"></a>

## Struct `OptInTransfer`

Event emitted when an user opt-in the direct transfer


<pre><code>&#35;[event]
struct OptInTransfer has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>opt_in: bool</code>
</dt>
<dd>
 True if the user opt in, false if the user opt-out
</dd>
</dl>


</details>

<a id="0x3_token_event_store_UriMutationEvent"></a>

## Struct `UriMutationEvent`

Event emitted when the tokendata uri mutates


<pre><code>struct UriMutationEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_UriMutation"></a>

## Struct `UriMutation`

Event emitted when the tokendata uri mutates


<pre><code>&#35;[event]
struct UriMutation has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DefaultPropertyMutateEvent"></a>

## Struct `DefaultPropertyMutateEvent`

Event emitted when mutating the default the token properties stored at tokendata


<pre><code>struct DefaultPropertyMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>keys: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_values: vector&lt;option::Option&lt;property_map::PropertyValue&gt;&gt;</code>
</dt>
<dd>
 we allow upsert so the old values might be none
</dd>
<dt>
<code>new_values: vector&lt;property_map::PropertyValue&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DefaultPropertyMutate"></a>

## Struct `DefaultPropertyMutate`

Event emitted when mutating the default the token properties stored at tokendata


<pre><code>&#35;[event]
struct DefaultPropertyMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>keys: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_values: vector&lt;option::Option&lt;property_map::PropertyValue&gt;&gt;</code>
</dt>
<dd>
 we allow upsert so the old values might be none
</dd>
<dt>
<code>new_values: vector&lt;property_map::PropertyValue&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DescriptionMutateEvent"></a>

## Struct `DescriptionMutateEvent`

Event emitted when the tokendata description is mutated


<pre><code>struct DescriptionMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DescriptionMutate"></a>

## Struct `DescriptionMutate`

Event emitted when the tokendata description is mutated


<pre><code>&#35;[event]
struct DescriptionMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_RoyaltyMutateEvent"></a>

## Struct `RoyaltyMutateEvent`

Event emitted when the token royalty is mutated


<pre><code>struct RoyaltyMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_royalty_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_royalty_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_royalty_payee_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_royalty_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_royalty_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_royalty_payee_addr: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_RoyaltyMutate"></a>

## Struct `RoyaltyMutate`

Event emitted when the token royalty is mutated


<pre><code>&#35;[event]
struct RoyaltyMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_royalty_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_royalty_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_royalty_payee_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_royalty_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_royalty_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_royalty_payee_addr: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_MaxiumMutateEvent"></a>

## Struct `MaxiumMutateEvent`

Event emitted when the token maximum is mutated


<pre><code>struct MaxiumMutateEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_MaximumMutate"></a>

## Struct `MaximumMutate`

Event emitted when the token maximum is mutated


<pre><code>&#35;[event]
struct MaximumMutate has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>token: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>old_maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_TokenEventStoreV1"></a>

## Resource `TokenEventStoreV1`



<pre><code>struct TokenEventStoreV1 has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_uri_mutate_events: event::EventHandle&lt;token_event_store::CollectionUriMutateEvent&gt;</code>
</dt>
<dd>
 collection mutation events
</dd>
<dt>
<code>collection_maximum_mutate_events: event::EventHandle&lt;token_event_store::CollectionMaxiumMutateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_description_mutate_events: event::EventHandle&lt;token_event_store::CollectionDescriptionMutateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>opt_in_events: event::EventHandle&lt;token_event_store::OptInTransferEvent&gt;</code>
</dt>
<dd>
 token transfer opt-in event
</dd>
<dt>
<code>uri_mutate_events: event::EventHandle&lt;token_event_store::UriMutationEvent&gt;</code>
</dt>
<dd>
 token mutation events
</dd>
<dt>
<code>default_property_mutate_events: event::EventHandle&lt;token_event_store::DefaultPropertyMutateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>description_mutate_events: event::EventHandle&lt;token_event_store::DescriptionMutateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_mutate_events: event::EventHandle&lt;token_event_store::RoyaltyMutateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum_mutate_events: event::EventHandle&lt;token_event_store::MaxiumMutateEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>extension: option::Option&lt;any::Any&gt;</code>
</dt>
<dd>
 This is for adding new events in future
</dd>
</dl>


</details>

<a id="0x3_token_event_store_initialize_token_event_store"></a>

## Function `initialize_token_event_store`



<pre><code>fun initialize_token_event_store(acct: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_token_event_store(acct: &amp;signer)&#123;
    if (!exists&lt;TokenEventStoreV1&gt;(signer::address_of(acct))) &#123;
        move_to(acct, TokenEventStoreV1 &#123;
            collection_uri_mutate_events: account::new_event_handle&lt;CollectionUriMutateEvent&gt;(acct),
            collection_maximum_mutate_events: account::new_event_handle&lt;CollectionMaxiumMutateEvent&gt;(acct),
            collection_description_mutate_events: account::new_event_handle&lt;CollectionDescriptionMutateEvent&gt;(acct),
            opt_in_events: account::new_event_handle&lt;OptInTransferEvent&gt;(acct),
            uri_mutate_events: account::new_event_handle&lt;UriMutationEvent&gt;(acct),
            default_property_mutate_events: account::new_event_handle&lt;DefaultPropertyMutateEvent&gt;(acct),
            description_mutate_events: account::new_event_handle&lt;DescriptionMutateEvent&gt;(acct),
            royalty_mutate_events: account::new_event_handle&lt;RoyaltyMutateEvent&gt;(acct),
            maximum_mutate_events: account::new_event_handle&lt;MaxiumMutateEvent&gt;(acct),
            extension: option::none&lt;Any&gt;(),
        &#125;);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_uri_mutate_event"></a>

## Function `emit_collection_uri_mutate_event`

Emit the collection uri mutation event


<pre><code>public(friend) fun emit_collection_uri_mutate_event(creator: &amp;signer, collection: string::String, old_uri: string::String, new_uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_collection_uri_mutate_event(creator: &amp;signer, collection: String, old_uri: String, new_uri: String) acquires TokenEventStoreV1 &#123;
    let event &#61; CollectionUriMutateEvent &#123;
        creator_addr: signer::address_of(creator),
        collection_name: collection,
        old_uri,
        new_uri,
    &#125;;
    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(creator));
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CollectionUriMutate &#123;
                creator_addr: signer::address_of(creator),
                collection_name: collection,
                old_uri,
                new_uri,
            &#125;
        );
    &#125;;
    event::emit_event&lt;CollectionUriMutateEvent&gt;(
        &amp;mut token_event_store.collection_uri_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_description_mutate_event"></a>

## Function `emit_collection_description_mutate_event`

Emit the collection description mutation event


<pre><code>public(friend) fun emit_collection_description_mutate_event(creator: &amp;signer, collection: string::String, old_description: string::String, new_description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_collection_description_mutate_event(creator: &amp;signer, collection: String, old_description: String, new_description: String) acquires TokenEventStoreV1 &#123;
    let event &#61; CollectionDescriptionMutateEvent &#123;
        creator_addr: signer::address_of(creator),
        collection_name: collection,
        old_description,
        new_description,
    &#125;;
    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(creator));
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CollectionDescriptionMutate &#123;
                creator_addr: signer::address_of(creator),
                collection_name: collection,
                old_description,
                new_description,
            &#125;
        );
    &#125;;
    event::emit_event&lt;CollectionDescriptionMutateEvent&gt;(
        &amp;mut token_event_store.collection_description_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_maximum_mutate_event"></a>

## Function `emit_collection_maximum_mutate_event`

Emit the collection maximum mutation event


<pre><code>public(friend) fun emit_collection_maximum_mutate_event(creator: &amp;signer, collection: string::String, old_maximum: u64, new_maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_collection_maximum_mutate_event(creator: &amp;signer, collection: String, old_maximum: u64, new_maximum: u64) acquires TokenEventStoreV1 &#123;
    let event &#61; CollectionMaxiumMutateEvent &#123;
        creator_addr: signer::address_of(creator),
        collection_name: collection,
        old_maximum,
        new_maximum,
    &#125;;
    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(creator));
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CollectionMaxiumMutate &#123;
                creator_addr: signer::address_of(creator),
                collection_name: collection,
                old_maximum,
                new_maximum,
            &#125;
        );
    &#125;;
    event::emit_event&lt;CollectionMaxiumMutateEvent&gt;(
        &amp;mut token_event_store.collection_maximum_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_token_opt_in_event"></a>

## Function `emit_token_opt_in_event`

Emit the direct opt-in event


<pre><code>public(friend) fun emit_token_opt_in_event(account: &amp;signer, opt_in: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_opt_in_event(account: &amp;signer, opt_in: bool) acquires TokenEventStoreV1 &#123;
    let opt_in_event &#61; OptInTransferEvent &#123;
      opt_in,
    &#125;;
    initialize_token_event_store(account);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(account));
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            OptInTransfer &#123;
                account_address: signer::address_of(account),
                opt_in,
            &#125;);
    &#125;;
    event::emit_event&lt;OptInTransferEvent&gt;(
        &amp;mut token_event_store.opt_in_events,
        opt_in_event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_token_uri_mutate_event"></a>

## Function `emit_token_uri_mutate_event`

Emit URI mutation event


<pre><code>public(friend) fun emit_token_uri_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_uri: string::String, new_uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_uri_mutate_event(
    creator: &amp;signer,
    collection: String,
    token: String,
    old_uri: String,
    new_uri: String,
) acquires TokenEventStoreV1 &#123;
    let creator_addr &#61; signer::address_of(creator);

    let event &#61; UriMutationEvent &#123;
        creator: creator_addr,
        collection,
        token,
        old_uri,
        new_uri,
    &#125;;

    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            UriMutation &#123;
                creator: creator_addr,
                collection,
                token,
                old_uri,
                new_uri,
            &#125;);
    &#125;;
    event::emit_event&lt;UriMutationEvent&gt;(
        &amp;mut token_event_store.uri_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_default_property_mutate_event"></a>

## Function `emit_default_property_mutate_event`

Emit tokendata property map mutation event


<pre><code>public(friend) fun emit_default_property_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, keys: vector&lt;string::String&gt;, old_values: vector&lt;option::Option&lt;property_map::PropertyValue&gt;&gt;, new_values: vector&lt;property_map::PropertyValue&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_default_property_mutate_event(
    creator: &amp;signer,
    collection: String,
    token: String,
    keys: vector&lt;String&gt;,
    old_values: vector&lt;Option&lt;PropertyValue&gt;&gt;,
    new_values: vector&lt;PropertyValue&gt;,
) acquires TokenEventStoreV1 &#123;
    let creator_addr &#61; signer::address_of(creator);

    let event &#61; DefaultPropertyMutateEvent &#123;
        creator: creator_addr,
        collection,
        token,
        keys,
        old_values,
        new_values,
    &#125;;

    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            DefaultPropertyMutate &#123;
                creator: creator_addr,
                collection,
                token,
                keys,
                old_values,
                new_values,
            &#125;);
    &#125;;
    event::emit_event&lt;DefaultPropertyMutateEvent&gt;(
        &amp;mut token_event_store.default_property_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_token_descrition_mutate_event"></a>

## Function `emit_token_descrition_mutate_event`

Emit description mutation event


<pre><code>public(friend) fun emit_token_descrition_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_description: string::String, new_description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_descrition_mutate_event(
    creator: &amp;signer,
    collection: String,
    token: String,
    old_description: String,
    new_description: String,
) acquires TokenEventStoreV1 &#123;
    let creator_addr &#61; signer::address_of(creator);

    let event &#61; DescriptionMutateEvent &#123;
        creator: creator_addr,
        collection,
        token,
        old_description,
        new_description,
    &#125;;

    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            DescriptionMutate &#123;
                creator: creator_addr,
                collection,
                token,
                old_description,
                new_description,
            &#125;);
    &#125;;
    event::emit_event&lt;DescriptionMutateEvent&gt;(
        &amp;mut token_event_store.description_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_token_royalty_mutate_event"></a>

## Function `emit_token_royalty_mutate_event`

Emit royalty mutation event


<pre><code>public(friend) fun emit_token_royalty_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_royalty_numerator: u64, old_royalty_denominator: u64, old_royalty_payee_addr: address, new_royalty_numerator: u64, new_royalty_denominator: u64, new_royalty_payee_addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_royalty_mutate_event(
    creator: &amp;signer,
    collection: String,
    token: String,
    old_royalty_numerator: u64,
    old_royalty_denominator: u64,
    old_royalty_payee_addr: address,
    new_royalty_numerator: u64,
    new_royalty_denominator: u64,
    new_royalty_payee_addr: address,
) acquires TokenEventStoreV1 &#123;
    let creator_addr &#61; signer::address_of(creator);
    let event &#61; RoyaltyMutateEvent &#123;
        creator: creator_addr,
        collection,
        token,
        old_royalty_numerator,
        old_royalty_denominator,
        old_royalty_payee_addr,
        new_royalty_numerator,
        new_royalty_denominator,
        new_royalty_payee_addr,
    &#125;;

    initialize_token_event_store(creator);
    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            RoyaltyMutate &#123;
                creator: creator_addr,
                collection,
                token,
                old_royalty_numerator,
                old_royalty_denominator,
                old_royalty_payee_addr,
                new_royalty_numerator,
                new_royalty_denominator,
                new_royalty_payee_addr,
            &#125;);
    &#125;;
    event::emit_event&lt;RoyaltyMutateEvent&gt;(
        &amp;mut token_event_store.royalty_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_event_store_emit_token_maximum_mutate_event"></a>

## Function `emit_token_maximum_mutate_event`

Emit maximum mutation event


<pre><code>public(friend) fun emit_token_maximum_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_maximum: u64, new_maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_maximum_mutate_event(
    creator: &amp;signer,
    collection: String,
    token: String,
    old_maximum: u64,
    new_maximum: u64,
) acquires TokenEventStoreV1 &#123;
    let creator_addr &#61; signer::address_of(creator);

    let event &#61; MaxiumMutateEvent &#123;
        creator: creator_addr,
        collection,
        token,
        old_maximum,
        new_maximum,
    &#125;;

    initialize_token_event_store(creator);
    let token_event_store &#61;  borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            MaximumMutate &#123;
                creator: creator_addr,
                collection,
                token,
                old_maximum,
                new_maximum,
            &#125;);
    &#125;;
    event::emit_event&lt;MaxiumMutateEvent&gt;(
        &amp;mut token_event_store.maximum_mutate_events,
        event,
    );
&#125;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_0_initialize_token_event_store"></a>

### Function `initialize_token_event_store`


<pre><code>fun initialize_token_event_store(acct: &amp;signer)
</code></pre>




<pre><code>pragma verify &#61; true;
let addr &#61; signer::address_of(acct);
include InitializeTokenEventStoreAbortsIf &#123;creator : acct&#125;;
</code></pre>


Adjust the overflow value according to the
number of registered events


<a id="0x3_token_event_store_InitializeTokenEventStoreAbortsIf"></a>


<pre><code>schema InitializeTokenEventStoreAbortsIf &#123;
    creator: &amp;signer;
    let addr &#61; signer::address_of(creator);
    let account &#61; global&lt;Account&gt;(addr);
    aborts_if !exists&lt;TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;Account&gt;(addr);
    aborts_if !exists&lt;TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if !exists&lt;TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
&#125;
</code></pre>




<a id="0x3_token_event_store_TokenEventStoreAbortsIf"></a>


<pre><code>schema TokenEventStoreAbortsIf &#123;
    creator: &amp;signer;
    let addr &#61; signer::address_of(creator);
    let account &#61; global&lt;Account&gt;(addr);
    aborts_if !exists&lt;Account&gt;(addr);
    aborts_if account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if account.guid_creation_num &#43; 9 &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_0_emit_collection_uri_mutate_event"></a>

### Function `emit_collection_uri_mutate_event`


<pre><code>public(friend) fun emit_collection_uri_mutate_event(creator: &amp;signer, collection: string::String, old_uri: string::String, new_uri: string::String)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_collection_description_mutate_event"></a>

### Function `emit_collection_description_mutate_event`


<pre><code>public(friend) fun emit_collection_description_mutate_event(creator: &amp;signer, collection: string::String, old_description: string::String, new_description: string::String)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_collection_maximum_mutate_event"></a>

### Function `emit_collection_maximum_mutate_event`


<pre><code>public(friend) fun emit_collection_maximum_mutate_event(creator: &amp;signer, collection: string::String, old_maximum: u64, new_maximum: u64)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_token_opt_in_event"></a>

### Function `emit_token_opt_in_event`


<pre><code>public(friend) fun emit_token_opt_in_event(account: &amp;signer, opt_in: bool)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf &#123;creator : account&#125;;
</code></pre>



<a id="@Specification_0_emit_token_uri_mutate_event"></a>

### Function `emit_token_uri_mutate_event`


<pre><code>public(friend) fun emit_token_uri_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_uri: string::String, new_uri: string::String)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_default_property_mutate_event"></a>

### Function `emit_default_property_mutate_event`


<pre><code>public(friend) fun emit_default_property_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, keys: vector&lt;string::String&gt;, old_values: vector&lt;option::Option&lt;property_map::PropertyValue&gt;&gt;, new_values: vector&lt;property_map::PropertyValue&gt;)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_token_descrition_mutate_event"></a>

### Function `emit_token_descrition_mutate_event`


<pre><code>public(friend) fun emit_token_descrition_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_description: string::String, new_description: string::String)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_token_royalty_mutate_event"></a>

### Function `emit_token_royalty_mutate_event`


<pre><code>public(friend) fun emit_token_royalty_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_royalty_numerator: u64, old_royalty_denominator: u64, old_royalty_payee_addr: address, new_royalty_numerator: u64, new_royalty_denominator: u64, new_royalty_payee_addr: address)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>



<a id="@Specification_0_emit_token_maximum_mutate_event"></a>

### Function `emit_token_maximum_mutate_event`


<pre><code>public(friend) fun emit_token_maximum_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_maximum: u64, new_maximum: u64)
</code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
