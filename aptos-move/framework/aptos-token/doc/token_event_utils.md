
<a name="0x3_token_event_utils"></a>

# Module `0x3::token_event_utils`

This module provide utils to create and emit token events


-  [Struct `CollectionDescriptionMutateEvent`](#0x3_token_event_utils_CollectionDescriptionMutateEvent)
-  [Struct `CollectionUriMutateEvent`](#0x3_token_event_utils_CollectionUriMutateEvent)
-  [Struct `CollectionMaxiumMutateEvent`](#0x3_token_event_utils_CollectionMaxiumMutateEvent)
-  [Struct `OptInTransferEvent`](#0x3_token_event_utils_OptInTransferEvent)
-  [Struct `UriMutationEvent`](#0x3_token_event_utils_UriMutationEvent)
-  [Struct `DefaultPropertyMutateEvent`](#0x3_token_event_utils_DefaultPropertyMutateEvent)
-  [Struct `DescriptionMutateEvent`](#0x3_token_event_utils_DescriptionMutateEvent)
-  [Struct `RoyaltyMutateEvent`](#0x3_token_event_utils_RoyaltyMutateEvent)
-  [Struct `MaxiumMutateEvent`](#0x3_token_event_utils_MaxiumMutateEvent)
-  [Resource `TokenEventStore`](#0x3_token_event_utils_TokenEventStore)
-  [Function `initialize_token_event_store`](#0x3_token_event_utils_initialize_token_event_store)
-  [Function `emit_collection_uri_mutate_event`](#0x3_token_event_utils_emit_collection_uri_mutate_event)
-  [Function `emit_collection_description_mutate_event`](#0x3_token_event_utils_emit_collection_description_mutate_event)
-  [Function `emit_collection_maximum_mutate_event`](#0x3_token_event_utils_emit_collection_maximum_mutate_event)
-  [Function `emit_token_opt_in_event`](#0x3_token_event_utils_emit_token_opt_in_event)
-  [Function `emit_token_uri_mutate_event`](#0x3_token_event_utils_emit_token_uri_mutate_event)
-  [Function `emit_default_property_mutate_event`](#0x3_token_event_utils_emit_default_property_mutate_event)
-  [Function `emit_token_descrition_mutate_event`](#0x3_token_event_utils_emit_token_descrition_mutate_event)
-  [Function `emit_token_royalty_mutate_event`](#0x3_token_event_utils_emit_token_royalty_mutate_event)
-  [Function `emit_token_maximum_mutate_event`](#0x3_token_event_utils_emit_token_maximum_mutate_event)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x3_token_event_utils_CollectionDescriptionMutateEvent"></a>

## Struct `CollectionDescriptionMutateEvent`

Event emitted when collection description is mutated


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 new description
</dd>
</dl>


</details>

<a name="0x3_token_event_utils_CollectionUriMutateEvent"></a>

## Struct `CollectionUriMutateEvent`

Event emitted when collection uri is mutated


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_CollectionUriMutateEvent">CollectionUriMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 new uri
</dd>
</dl>


</details>

<a name="0x3_token_event_utils_CollectionMaxiumMutateEvent"></a>

## Struct `CollectionMaxiumMutateEvent`

Event emitted when the collection maximum is mutated


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>
 new maximum
</dd>
</dl>


</details>

<a name="0x3_token_event_utils_OptInTransferEvent"></a>

## Struct `OptInTransferEvent`

Event emitted when an user opt-in the direct transfer


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_OptInTransferEvent">OptInTransferEvent</a> <b>has</b> drop, store
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

<a name="0x3_token_event_utils_UriMutationEvent"></a>

## Struct `UriMutationEvent`

Event emitted when the tokendata uri mutates


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_UriMutationEvent">UriMutationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 new URI
</dd>
</dl>


</details>

<a name="0x3_token_event_utils_DefaultPropertyMutateEvent"></a>

## Struct `DefaultPropertyMutateEvent`

Event emitted when mutating the default the token properties stored at tokendata


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_token_event_utils_DescriptionMutateEvent"></a>

## Struct `DescriptionMutateEvent`

Event emitted when the tokendata description is mutated


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_DescriptionMutateEvent">DescriptionMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_token_event_utils_RoyaltyMutateEvent"></a>

## Struct `RoyaltyMutateEvent`

Event emitted when the token royalty is mutated


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_RoyaltyMutateEvent">RoyaltyMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_token_event_utils_MaxiumMutateEvent"></a>

## Struct `MaxiumMutateEvent`

Event emitted when the token maximum is mutated


<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_MaxiumMutateEvent">MaxiumMutateEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_token_event_utils_TokenEventStore"></a>

## Resource `TokenEventStore`



<pre><code><b>struct</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_uri_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionUriMutateEvent">token_event_utils::CollectionUriMutateEvent</a>&gt;</code>
</dt>
<dd>
 collection mutation events
</dd>
<dt>
<code>collection_maximum_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionMaxiumMutateEvent">token_event_utils::CollectionMaxiumMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_description_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionDescriptionMutateEvent">token_event_utils::CollectionDescriptionMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>opt_in_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_OptInTransferEvent">token_event_utils::OptInTransferEvent</a>&gt;</code>
</dt>
<dd>
 token transfer opt-in event
</dd>
<dt>
<code>uri_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_UriMutationEvent">token_event_utils::UriMutationEvent</a>&gt;</code>
</dt>
<dd>
 token mutation events
</dd>
<dt>
<code>default_property_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_DefaultPropertyMutateEvent">token_event_utils::DefaultPropertyMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>description_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_DescriptionMutateEvent">token_event_utils::DescriptionMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_RoyaltyMutateEvent">token_event_utils::RoyaltyMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_MaxiumMutateEvent">token_event_utils::MaxiumMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>extention: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any_Any">any::Any</a>&gt;</code>
</dt>
<dd>
 This is for adding new events in future
</dd>
</dl>


</details>

<a name="0x3_token_event_utils_initialize_token_event_store"></a>

## Function `initialize_token_event_store`



<pre><code><b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(acct: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(acct: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>){
    <b>if</b> (!<b>exists</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(acct))) {
        <b>move_to</b>(acct, <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
            collection_uri_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionUriMutateEvent">CollectionUriMutateEvent</a>&gt;(acct),
            collection_maximum_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a>&gt;(acct),
            collection_description_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a>&gt;(acct),
            opt_in_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_OptInTransferEvent">OptInTransferEvent</a>&gt;(acct),
            uri_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_UriMutationEvent">UriMutationEvent</a>&gt;(acct),
            default_property_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a>&gt;(acct),
            description_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_DescriptionMutateEvent">DescriptionMutateEvent</a>&gt;(acct),
            royalty_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_RoyaltyMutateEvent">RoyaltyMutateEvent</a>&gt;(acct),
            maximum_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_MaxiumMutateEvent">MaxiumMutateEvent</a>&gt;(acct),
            extention: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;Any&gt;(),
        });
    };
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_collection_uri_mutate_event"></a>

## Function `emit_collection_uri_mutate_event`

Emit the collection uri mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_collection_uri_mutate_event">emit_collection_uri_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_collection_uri_mutate_event">emit_collection_uri_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: String, uri: String) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_CollectionUriMutateEvent">CollectionUriMutateEvent</a> {
        creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),
        collection_name: collection,
        uri,
    };
    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionUriMutateEvent">CollectionUriMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.collection_uri_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_collection_description_mutate_event"></a>

## Function `emit_collection_description_mutate_event`

Emit the collection description mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_collection_description_mutate_event">emit_collection_description_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_collection_description_mutate_event">emit_collection_description_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: String, description: String) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a> {
        creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),
        collection_name: collection,
        description,
    };
    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.collection_description_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_collection_maximum_mutate_event"></a>

## Function `emit_collection_maximum_mutate_event`

Emit the collection maximum mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_collection_maximum_mutate_event">emit_collection_maximum_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_collection_maximum_mutate_event">emit_collection_maximum_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: String, maximum: u64) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a> {
        creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),
        collection_name: collection,
        maximum,
    };
    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.collection_maximum_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_token_opt_in_event"></a>

## Function `emit_token_opt_in_event`

Emit the direct opt-in event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_opt_in_event">emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_opt_in_event">emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> opt_in_event = <a href="token_event_utils.md#0x3_token_event_utils_OptInTransferEvent">OptInTransferEvent</a> {
      opt_in,
    };
    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>));
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_OptInTransferEvent">OptInTransferEvent</a>&gt;(
        &<b>mut</b> token_event_store.opt_in_events,
        opt_in_event,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_token_uri_mutate_event"></a>

## Function `emit_token_uri_mutate_event`

Emit URI mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_uri_mutate_event">emit_token_uri_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_uri_mutate_event">emit_token_uri_mutate_event</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    <a href="token.md#0x3_token">token</a>: String,
    new_uri: String,
) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> creator_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);

    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_UriMutationEvent">UriMutationEvent</a> {
        creator: creator_addr,
        collection,
        <a href="token.md#0x3_token">token</a>,
        new_uri,
    };

    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(creator_addr);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_UriMutationEvent">UriMutationEvent</a>&gt;(
        &<b>mut</b> token_event_store.uri_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_default_property_mutate_event"></a>

## Function `emit_default_property_mutate_event`

Emit tokendata property map mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_default_property_mutate_event">emit_default_property_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_default_property_mutate_event">emit_default_property_mutate_event</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    <a href="token.md#0x3_token">token</a>: String,
    new_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> creator_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);

    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a> {
        creator: creator_addr,
        collection,
        <a href="token.md#0x3_token">token</a>,
        new_keys,
        new_values,
        new_types,
    };

    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(creator_addr);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.default_property_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_token_descrition_mutate_event"></a>

## Function `emit_token_descrition_mutate_event`

Emit description mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_descrition_mutate_event">emit_token_descrition_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_descrition_mutate_event">emit_token_descrition_mutate_event</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    <a href="token.md#0x3_token">token</a>: String,
    description: String,
) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> creator_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);

    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_DescriptionMutateEvent">DescriptionMutateEvent</a> {
        creator: creator_addr,
        collection,
        <a href="token.md#0x3_token">token</a>,
        description,
    };

    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(creator_addr);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_DescriptionMutateEvent">DescriptionMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.description_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_token_royalty_mutate_event"></a>

## Function `emit_token_royalty_mutate_event`

Emit royalty mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_royalty_mutate_event">emit_token_royalty_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_numerator: u64, royalty_denominator: u64, royalty_payee_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_royalty_mutate_event">emit_token_royalty_mutate_event</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    <a href="token.md#0x3_token">token</a>: String,
    royalty_numerator: u64,
    royalty_denominator: u64,
    royalty_payee_addr: <b>address</b>,
) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> creator_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_RoyaltyMutateEvent">RoyaltyMutateEvent</a> {
        creator: creator_addr,
        collection,
        <a href="token.md#0x3_token">token</a>,
        royalty_numerator,
        royalty_denominator,
        royalty_payee_addr,
    };

    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store = <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(creator_addr);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_RoyaltyMutateEvent">RoyaltyMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.royalty_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>

<a name="0x3_token_event_utils_emit_token_maximum_mutate_event"></a>

## Function `emit_token_maximum_mutate_event`

Emit maximum mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_maximum_mutate_event">emit_token_maximum_mutate_event</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_utils.md#0x3_token_event_utils_emit_token_maximum_mutate_event">emit_token_maximum_mutate_event</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    <a href="token.md#0x3_token">token</a>: String,
    maximum: u64,
) <b>acquires</b> <a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a> {
    <b>let</b> creator_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);

    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> = <a href="token_event_utils.md#0x3_token_event_utils_MaxiumMutateEvent">MaxiumMutateEvent</a> {
        creator: creator_addr,
        collection,
        <a href="token.md#0x3_token">token</a>,
        maximum,
    };

    <a href="token_event_utils.md#0x3_token_event_utils_initialize_token_event_store">initialize_token_event_store</a>(creator);
    <b>let</b> token_event_store =  <b>borrow_global_mut</b>&lt;<a href="token_event_utils.md#0x3_token_event_utils_TokenEventStore">TokenEventStore</a>&gt;(creator_addr);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_utils.md#0x3_token_event_utils_MaxiumMutateEvent">MaxiumMutateEvent</a>&gt;(
        &<b>mut</b> token_event_store.maximum_mutate_events,
        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,
    );
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
