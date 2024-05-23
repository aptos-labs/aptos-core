
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


<pre><code>use 0x1::account;<br/>use 0x1::any;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x3::property_map;<br/></code></pre>



<a id="0x3_token_event_store_CollectionDescriptionMutateEvent"></a>

## Struct `CollectionDescriptionMutateEvent`

Event emitted when collection description is mutated


<pre><code>struct CollectionDescriptionMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct CollectionDescriptionMutate has drop, store<br/></code></pre>



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


<pre><code>struct CollectionUriMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct CollectionUriMutate has drop, store<br/></code></pre>



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


<pre><code>struct CollectionMaxiumMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct CollectionMaxiumMutate has drop, store<br/></code></pre>



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

Event emitted when an user opt&#45;in the direct transfer


<pre><code>struct OptInTransferEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>opt_in: bool</code>
</dt>
<dd>
 True if the user opt in, false if the user opt&#45;out
</dd>
</dl>


</details>

<a id="0x3_token_event_store_OptInTransfer"></a>

## Struct `OptInTransfer`

Event emitted when an user opt&#45;in the direct transfer


<pre><code>&#35;[event]<br/>struct OptInTransfer has drop, store<br/></code></pre>



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
 True if the user opt in, false if the user opt&#45;out
</dd>
</dl>


</details>

<a id="0x3_token_event_store_UriMutationEvent"></a>

## Struct `UriMutationEvent`

Event emitted when the tokendata uri mutates


<pre><code>struct UriMutationEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct UriMutation has drop, store<br/></code></pre>



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


<pre><code>struct DefaultPropertyMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct DefaultPropertyMutate has drop, store<br/></code></pre>



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


<pre><code>struct DescriptionMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct DescriptionMutate has drop, store<br/></code></pre>



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


<pre><code>struct RoyaltyMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct RoyaltyMutate has drop, store<br/></code></pre>



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


<pre><code>struct MaxiumMutateEvent has drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct MaximumMutate has drop, store<br/></code></pre>



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



<pre><code>struct TokenEventStoreV1 has key<br/></code></pre>



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
 token transfer opt&#45;in event
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



<pre><code>fun initialize_token_event_store(acct: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_token_event_store(acct: &amp;signer)&#123;<br/>    if (!exists&lt;TokenEventStoreV1&gt;(signer::address_of(acct))) &#123;<br/>        move_to(acct, TokenEventStoreV1 &#123;<br/>            collection_uri_mutate_events: account::new_event_handle&lt;CollectionUriMutateEvent&gt;(acct),<br/>            collection_maximum_mutate_events: account::new_event_handle&lt;CollectionMaxiumMutateEvent&gt;(acct),<br/>            collection_description_mutate_events: account::new_event_handle&lt;CollectionDescriptionMutateEvent&gt;(acct),<br/>            opt_in_events: account::new_event_handle&lt;OptInTransferEvent&gt;(acct),<br/>            uri_mutate_events: account::new_event_handle&lt;UriMutationEvent&gt;(acct),<br/>            default_property_mutate_events: account::new_event_handle&lt;DefaultPropertyMutateEvent&gt;(acct),<br/>            description_mutate_events: account::new_event_handle&lt;DescriptionMutateEvent&gt;(acct),<br/>            royalty_mutate_events: account::new_event_handle&lt;RoyaltyMutateEvent&gt;(acct),<br/>            maximum_mutate_events: account::new_event_handle&lt;MaxiumMutateEvent&gt;(acct),<br/>            extension: option::none&lt;Any&gt;(),<br/>        &#125;);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_uri_mutate_event"></a>

## Function `emit_collection_uri_mutate_event`

Emit the collection uri mutation event


<pre><code>public(friend) fun emit_collection_uri_mutate_event(creator: &amp;signer, collection: string::String, old_uri: string::String, new_uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_collection_uri_mutate_event(creator: &amp;signer, collection: String, old_uri: String, new_uri: String) acquires TokenEventStoreV1 &#123;<br/>    let event &#61; CollectionUriMutateEvent &#123;<br/>        creator_addr: signer::address_of(creator),<br/>        collection_name: collection,<br/>        old_uri,<br/>        new_uri,<br/>    &#125;;<br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(creator));<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CollectionUriMutate &#123;<br/>                creator_addr: signer::address_of(creator),<br/>                collection_name: collection,<br/>                old_uri,<br/>                new_uri,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;CollectionUriMutateEvent&gt;(<br/>        &amp;mut token_event_store.collection_uri_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_description_mutate_event"></a>

## Function `emit_collection_description_mutate_event`

Emit the collection description mutation event


<pre><code>public(friend) fun emit_collection_description_mutate_event(creator: &amp;signer, collection: string::String, old_description: string::String, new_description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_collection_description_mutate_event(creator: &amp;signer, collection: String, old_description: String, new_description: String) acquires TokenEventStoreV1 &#123;<br/>    let event &#61; CollectionDescriptionMutateEvent &#123;<br/>        creator_addr: signer::address_of(creator),<br/>        collection_name: collection,<br/>        old_description,<br/>        new_description,<br/>    &#125;;<br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(creator));<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CollectionDescriptionMutate &#123;<br/>                creator_addr: signer::address_of(creator),<br/>                collection_name: collection,<br/>                old_description,<br/>                new_description,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;CollectionDescriptionMutateEvent&gt;(<br/>        &amp;mut token_event_store.collection_description_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_maximum_mutate_event"></a>

## Function `emit_collection_maximum_mutate_event`

Emit the collection maximum mutation event


<pre><code>public(friend) fun emit_collection_maximum_mutate_event(creator: &amp;signer, collection: string::String, old_maximum: u64, new_maximum: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_collection_maximum_mutate_event(creator: &amp;signer, collection: String, old_maximum: u64, new_maximum: u64) acquires TokenEventStoreV1 &#123;<br/>    let event &#61; CollectionMaxiumMutateEvent &#123;<br/>        creator_addr: signer::address_of(creator),<br/>        collection_name: collection,<br/>        old_maximum,<br/>        new_maximum,<br/>    &#125;;<br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(creator));<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CollectionMaxiumMutate &#123;<br/>                creator_addr: signer::address_of(creator),<br/>                collection_name: collection,<br/>                old_maximum,<br/>                new_maximum,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;CollectionMaxiumMutateEvent&gt;(<br/>        &amp;mut token_event_store.collection_maximum_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_opt_in_event"></a>

## Function `emit_token_opt_in_event`

Emit the direct opt&#45;in event


<pre><code>public(friend) fun emit_token_opt_in_event(account: &amp;signer, opt_in: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_opt_in_event(account: &amp;signer, opt_in: bool) acquires TokenEventStoreV1 &#123;<br/>    let opt_in_event &#61; OptInTransferEvent &#123;<br/>      opt_in,<br/>    &#125;;<br/>    initialize_token_event_store(account);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(signer::address_of(account));<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            OptInTransfer &#123;<br/>                account_address: signer::address_of(account),<br/>                opt_in,<br/>            &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;OptInTransferEvent&gt;(<br/>        &amp;mut token_event_store.opt_in_events,<br/>        opt_in_event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_uri_mutate_event"></a>

## Function `emit_token_uri_mutate_event`

Emit URI mutation event


<pre><code>public(friend) fun emit_token_uri_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_uri: string::String, new_uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_uri_mutate_event(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    token: String,<br/>    old_uri: String,<br/>    new_uri: String,<br/>) acquires TokenEventStoreV1 &#123;<br/>    let creator_addr &#61; signer::address_of(creator);<br/><br/>    let event &#61; UriMutationEvent &#123;<br/>        creator: creator_addr,<br/>        collection,<br/>        token,<br/>        old_uri,<br/>        new_uri,<br/>    &#125;;<br/><br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            UriMutation &#123;<br/>                creator: creator_addr,<br/>                collection,<br/>                token,<br/>                old_uri,<br/>                new_uri,<br/>            &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;UriMutationEvent&gt;(<br/>        &amp;mut token_event_store.uri_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_default_property_mutate_event"></a>

## Function `emit_default_property_mutate_event`

Emit tokendata property map mutation event


<pre><code>public(friend) fun emit_default_property_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, keys: vector&lt;string::String&gt;, old_values: vector&lt;option::Option&lt;property_map::PropertyValue&gt;&gt;, new_values: vector&lt;property_map::PropertyValue&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_default_property_mutate_event(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    token: String,<br/>    keys: vector&lt;String&gt;,<br/>    old_values: vector&lt;Option&lt;PropertyValue&gt;&gt;,<br/>    new_values: vector&lt;PropertyValue&gt;,<br/>) acquires TokenEventStoreV1 &#123;<br/>    let creator_addr &#61; signer::address_of(creator);<br/><br/>    let event &#61; DefaultPropertyMutateEvent &#123;<br/>        creator: creator_addr,<br/>        collection,<br/>        token,<br/>        keys,<br/>        old_values,<br/>        new_values,<br/>    &#125;;<br/><br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            DefaultPropertyMutate &#123;<br/>                creator: creator_addr,<br/>                collection,<br/>                token,<br/>                keys,<br/>                old_values,<br/>                new_values,<br/>            &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;DefaultPropertyMutateEvent&gt;(<br/>        &amp;mut token_event_store.default_property_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_descrition_mutate_event"></a>

## Function `emit_token_descrition_mutate_event`

Emit description mutation event


<pre><code>public(friend) fun emit_token_descrition_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_description: string::String, new_description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_descrition_mutate_event(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    token: String,<br/>    old_description: String,<br/>    new_description: String,<br/>) acquires TokenEventStoreV1 &#123;<br/>    let creator_addr &#61; signer::address_of(creator);<br/><br/>    let event &#61; DescriptionMutateEvent &#123;<br/>        creator: creator_addr,<br/>        collection,<br/>        token,<br/>        old_description,<br/>        new_description,<br/>    &#125;;<br/><br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            DescriptionMutate &#123;<br/>                creator: creator_addr,<br/>                collection,<br/>                token,<br/>                old_description,<br/>                new_description,<br/>            &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;DescriptionMutateEvent&gt;(<br/>        &amp;mut token_event_store.description_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_royalty_mutate_event"></a>

## Function `emit_token_royalty_mutate_event`

Emit royalty mutation event


<pre><code>public(friend) fun emit_token_royalty_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_royalty_numerator: u64, old_royalty_denominator: u64, old_royalty_payee_addr: address, new_royalty_numerator: u64, new_royalty_denominator: u64, new_royalty_payee_addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_royalty_mutate_event(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    token: String,<br/>    old_royalty_numerator: u64,<br/>    old_royalty_denominator: u64,<br/>    old_royalty_payee_addr: address,<br/>    new_royalty_numerator: u64,<br/>    new_royalty_denominator: u64,<br/>    new_royalty_payee_addr: address,<br/>) acquires TokenEventStoreV1 &#123;<br/>    let creator_addr &#61; signer::address_of(creator);<br/>    let event &#61; RoyaltyMutateEvent &#123;<br/>        creator: creator_addr,<br/>        collection,<br/>        token,<br/>        old_royalty_numerator,<br/>        old_royalty_denominator,<br/>        old_royalty_payee_addr,<br/>        new_royalty_numerator,<br/>        new_royalty_denominator,<br/>        new_royalty_payee_addr,<br/>    &#125;;<br/><br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61; borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            RoyaltyMutate &#123;<br/>                creator: creator_addr,<br/>                collection,<br/>                token,<br/>                old_royalty_numerator,<br/>                old_royalty_denominator,<br/>                old_royalty_payee_addr,<br/>                new_royalty_numerator,<br/>                new_royalty_denominator,<br/>                new_royalty_payee_addr,<br/>            &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;RoyaltyMutateEvent&gt;(<br/>        &amp;mut token_event_store.royalty_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_maximum_mutate_event"></a>

## Function `emit_token_maximum_mutate_event`

Emit maximum mutation event


<pre><code>public(friend) fun emit_token_maximum_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_maximum: u64, new_maximum: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun emit_token_maximum_mutate_event(<br/>    creator: &amp;signer,<br/>    collection: String,<br/>    token: String,<br/>    old_maximum: u64,<br/>    new_maximum: u64,<br/>) acquires TokenEventStoreV1 &#123;<br/>    let creator_addr &#61; signer::address_of(creator);<br/><br/>    let event &#61; MaxiumMutateEvent &#123;<br/>        creator: creator_addr,<br/>        collection,<br/>        token,<br/>        old_maximum,<br/>        new_maximum,<br/>    &#125;;<br/><br/>    initialize_token_event_store(creator);<br/>    let token_event_store &#61;  borrow_global_mut&lt;TokenEventStoreV1&gt;(creator_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            MaximumMutate &#123;<br/>                creator: creator_addr,<br/>                collection,<br/>                token,<br/>                old_maximum,<br/>                new_maximum,<br/>            &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;MaxiumMutateEvent&gt;(<br/>        &amp;mut token_event_store.maximum_mutate_events,<br/>        event,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_0_initialize_token_event_store"></a>

### Function `initialize_token_event_store`


<pre><code>fun initialize_token_event_store(acct: &amp;signer)<br/></code></pre>




<pre><code>pragma verify &#61; true;<br/>let addr &#61; signer::address_of(acct);<br/>include InitializeTokenEventStoreAbortsIf &#123;creator : acct&#125;;<br/></code></pre>


Adjust the overflow value according to the<br/> number of registered events


<a id="0x3_token_event_store_InitializeTokenEventStoreAbortsIf"></a>


<pre><code>schema InitializeTokenEventStoreAbortsIf &#123;<br/>creator: &amp;signer;<br/>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/>&#125;<br/></code></pre>




<a id="0x3_token_event_store_TokenEventStoreAbortsIf"></a>


<pre><code>schema TokenEventStoreAbortsIf &#123;<br/>creator: &amp;signer;<br/>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/>&#125;<br/></code></pre>



<a id="@Specification_0_emit_collection_uri_mutate_event"></a>

### Function `emit_collection_uri_mutate_event`


<pre><code>public(friend) fun emit_collection_uri_mutate_event(creator: &amp;signer, collection: string::String, old_uri: string::String, new_uri: string::String)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_collection_description_mutate_event"></a>

### Function `emit_collection_description_mutate_event`


<pre><code>public(friend) fun emit_collection_description_mutate_event(creator: &amp;signer, collection: string::String, old_description: string::String, new_description: string::String)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_collection_maximum_mutate_event"></a>

### Function `emit_collection_maximum_mutate_event`


<pre><code>public(friend) fun emit_collection_maximum_mutate_event(creator: &amp;signer, collection: string::String, old_maximum: u64, new_maximum: u64)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_token_opt_in_event"></a>

### Function `emit_token_opt_in_event`


<pre><code>public(friend) fun emit_token_opt_in_event(account: &amp;signer, opt_in: bool)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf &#123;creator : account&#125;;<br/></code></pre>



<a id="@Specification_0_emit_token_uri_mutate_event"></a>

### Function `emit_token_uri_mutate_event`


<pre><code>public(friend) fun emit_token_uri_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_uri: string::String, new_uri: string::String)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_default_property_mutate_event"></a>

### Function `emit_default_property_mutate_event`


<pre><code>public(friend) fun emit_default_property_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, keys: vector&lt;string::String&gt;, old_values: vector&lt;option::Option&lt;property_map::PropertyValue&gt;&gt;, new_values: vector&lt;property_map::PropertyValue&gt;)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_token_descrition_mutate_event"></a>

### Function `emit_token_descrition_mutate_event`


<pre><code>public(friend) fun emit_token_descrition_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_description: string::String, new_description: string::String)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_token_royalty_mutate_event"></a>

### Function `emit_token_royalty_mutate_event`


<pre><code>public(friend) fun emit_token_royalty_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_royalty_numerator: u64, old_royalty_denominator: u64, old_royalty_payee_addr: address, new_royalty_numerator: u64, new_royalty_denominator: u64, new_royalty_payee_addr: address)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>



<a id="@Specification_0_emit_token_maximum_mutate_event"></a>

### Function `emit_token_maximum_mutate_event`


<pre><code>public(friend) fun emit_token_maximum_mutate_event(creator: &amp;signer, collection: string::String, token: string::String, old_maximum: u64, new_maximum: u64)<br/></code></pre>




<pre><code>include InitializeTokenEventStoreAbortsIf;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
