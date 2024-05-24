
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


<pre><code><b>use</b> <a href="../../aptos-framework/doc/account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="property_map.md#0x3_property_map">0x3::property_map</a>;<br /></code></pre>



<a id="0x3_token_event_store_CollectionDescriptionMutateEvent"></a>

## Struct `CollectionDescriptionMutateEvent`

Event emitted when collection description is mutated


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionDescriptionMutate"></a>

## Struct `CollectionDescriptionMutate`

Event emitted when collection description is mutated


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutate">CollectionDescriptionMutate</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionUriMutateEvent"></a>

## Struct `CollectionUriMutateEvent`

Event emitted when collection uri is mutated


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_CollectionUriMutateEvent">CollectionUriMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionUriMutate"></a>

## Struct `CollectionUriMutate`

Event emitted when collection uri is mutated


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_CollectionUriMutate">CollectionUriMutate</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_CollectionMaxiumMutateEvent"></a>

## Struct `CollectionMaxiumMutateEvent`

Event emitted when the collection maximum is mutated


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutate">CollectionMaxiumMutate</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_OptInTransferEvent">OptInTransferEvent</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_OptInTransfer">OptInTransfer</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: <b>address</b></code>
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


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_UriMutationEvent">UriMutationEvent</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_UriMutation"></a>

## Struct `UriMutation`

Event emitted when the tokendata uri mutates


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_UriMutation">UriMutation</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DefaultPropertyMutateEvent"></a>

## Struct `DefaultPropertyMutateEvent`

Event emitted when mutating the default the token properties stored at tokendata


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;&gt;</code>
</dt>
<dd>
 we allow upsert so the old values might be none
</dd>
<dt>
<code>new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DefaultPropertyMutate"></a>

## Struct `DefaultPropertyMutate`

Event emitted when mutating the default the token properties stored at tokendata


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutate">DefaultPropertyMutate</a> <b>has</b> drop, store<br /></code></pre>



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
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;&gt;</code>
</dt>
<dd>
 we allow upsert so the old values might be none
</dd>
<dt>
<code>new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DescriptionMutateEvent"></a>

## Struct `DescriptionMutateEvent`

Event emitted when the tokendata description is mutated


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_DescriptionMutateEvent">DescriptionMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_DescriptionMutate"></a>

## Struct `DescriptionMutate`

Event emitted when the tokendata description is mutated


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_DescriptionMutate">DescriptionMutate</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_RoyaltyMutateEvent"></a>

## Struct `RoyaltyMutateEvent`

Event emitted when the token royalty is mutated


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_RoyaltyMutateEvent">RoyaltyMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_royalty_payee_addr: <b>address</b></code>
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
<code>new_royalty_payee_addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_RoyaltyMutate"></a>

## Struct `RoyaltyMutate`

Event emitted when the token royalty is mutated


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_RoyaltyMutate">RoyaltyMutate</a> <b>has</b> drop, store<br /></code></pre>



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
<code>old_royalty_payee_addr: <b>address</b></code>
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
<code>new_royalty_payee_addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_event_store_MaxiumMutateEvent"></a>

## Struct `MaxiumMutateEvent`

Event emitted when the token maximum is mutated


<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_MaxiumMutateEvent">MaxiumMutateEvent</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_MaximumMutate">MaximumMutate</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_uri_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionUriMutateEvent">token_event_store::CollectionUriMutateEvent</a>&gt;</code>
</dt>
<dd>
 collection mutation events
</dd>
<dt>
<code>collection_maximum_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutateEvent">token_event_store::CollectionMaxiumMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_description_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutateEvent">token_event_store::CollectionDescriptionMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>opt_in_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_OptInTransferEvent">token_event_store::OptInTransferEvent</a>&gt;</code>
</dt>
<dd>
 token transfer opt&#45;in event
</dd>
<dt>
<code>uri_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_UriMutationEvent">token_event_store::UriMutationEvent</a>&gt;</code>
</dt>
<dd>
 token mutation events
</dd>
<dt>
<code>default_property_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutateEvent">token_event_store::DefaultPropertyMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>description_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_DescriptionMutateEvent">token_event_store::DescriptionMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_RoyaltyMutateEvent">token_event_store::RoyaltyMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum_mutate_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_MaxiumMutateEvent">token_event_store::MaxiumMutateEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>extension: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any_Any">any::Any</a>&gt;</code>
</dt>
<dd>
 This is for adding new events in future
</dd>
</dl>


</details>

<a id="0x3_token_event_store_initialize_token_event_store"></a>

## Function `initialize_token_event_store`



<pre><code><b>fun</b> <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(acct: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(acct: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)&#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(acct))) &#123;<br />        <b>move_to</b>(acct, <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />            collection_uri_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionUriMutateEvent">CollectionUriMutateEvent</a>&gt;(acct),<br />            collection_maximum_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a>&gt;(acct),<br />            collection_description_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a>&gt;(acct),<br />            opt_in_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_OptInTransferEvent">OptInTransferEvent</a>&gt;(acct),<br />            uri_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_UriMutationEvent">UriMutationEvent</a>&gt;(acct),<br />            default_property_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a>&gt;(acct),<br />            description_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_DescriptionMutateEvent">DescriptionMutateEvent</a>&gt;(acct),<br />            royalty_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_RoyaltyMutateEvent">RoyaltyMutateEvent</a>&gt;(acct),<br />            maximum_mutate_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_event_store.md#0x3_token_event_store_MaxiumMutateEvent">MaxiumMutateEvent</a>&gt;(acct),<br />            extension: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;Any&gt;(),<br />        &#125;);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_uri_mutate_event"></a>

## Function `emit_collection_uri_mutate_event`

Emit the collection uri mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_uri_mutate_event">emit_collection_uri_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_uri_mutate_event">emit_collection_uri_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: String, old_uri: String, new_uri: String) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_CollectionUriMutateEvent">CollectionUriMutateEvent</a> &#123;<br />        creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />        collection_name: collection,<br />        old_uri,<br />        new_uri,<br />    &#125;;<br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_CollectionUriMutate">CollectionUriMutate</a> &#123;<br />                creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />                collection_name: collection,<br />                old_uri,<br />                new_uri,<br />            &#125;<br />        );<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionUriMutateEvent">CollectionUriMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.collection_uri_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_description_mutate_event"></a>

## Function `emit_collection_description_mutate_event`

Emit the collection description mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_description_mutate_event">emit_collection_description_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_description_mutate_event">emit_collection_description_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: String, old_description: String, new_description: String) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a> &#123;<br />        creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />        collection_name: collection,<br />        old_description,<br />        new_description,<br />    &#125;;<br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutate">CollectionDescriptionMutate</a> &#123;<br />                creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />                collection_name: collection,<br />                old_description,<br />                new_description,<br />            &#125;<br />        );<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionDescriptionMutateEvent">CollectionDescriptionMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.collection_description_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_collection_maximum_mutate_event"></a>

## Function `emit_collection_maximum_mutate_event`

Emit the collection maximum mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_maximum_mutate_event">emit_collection_maximum_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_maximum: u64, new_maximum: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_maximum_mutate_event">emit_collection_maximum_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: String, old_maximum: u64, new_maximum: u64) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a> &#123;<br />        creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />        collection_name: collection,<br />        old_maximum,<br />        new_maximum,<br />    &#125;;<br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator));<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutate">CollectionMaxiumMutate</a> &#123;<br />                creator_addr: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator),<br />                collection_name: collection,<br />                old_maximum,<br />                new_maximum,<br />            &#125;<br />        );<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_CollectionMaxiumMutateEvent">CollectionMaxiumMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.collection_maximum_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_opt_in_event"></a>

## Function `emit_token_opt_in_event`

Emit the direct opt&#45;in event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_opt_in_event">emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_opt_in_event">emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> opt_in_event &#61; <a href="token_event_store.md#0x3_token_event_store_OptInTransferEvent">OptInTransferEvent</a> &#123;<br />      opt_in,<br />    &#125;;<br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>));<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_OptInTransfer">OptInTransfer</a> &#123;<br />                account_address: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />                opt_in,<br />            &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_OptInTransferEvent">OptInTransferEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.opt_in_events,<br />        opt_in_event,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_uri_mutate_event"></a>

## Function `emit_token_uri_mutate_event`

Emit URI mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_uri_mutate_event">emit_token_uri_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_uri_mutate_event">emit_token_uri_mutate_event</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    <a href="token.md#0x3_token">token</a>: String,<br />    old_uri: String,<br />    new_uri: String,<br />) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> creator_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_UriMutationEvent">UriMutationEvent</a> &#123;<br />        creator: creator_addr,<br />        collection,<br />        <a href="token.md#0x3_token">token</a>,<br />        old_uri,<br />        new_uri,<br />    &#125;;<br /><br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(creator_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_UriMutation">UriMutation</a> &#123;<br />                creator: creator_addr,<br />                collection,<br />                <a href="token.md#0x3_token">token</a>,<br />                old_uri,<br />                new_uri,<br />            &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_UriMutationEvent">UriMutationEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.uri_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_default_property_mutate_event"></a>

## Function `emit_default_property_mutate_event`

Emit tokendata property map mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_default_property_mutate_event">emit_default_property_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;&gt;, new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_default_property_mutate_event">emit_default_property_mutate_event</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    <a href="token.md#0x3_token">token</a>: String,<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;PropertyValue&gt;&gt;,<br />    new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;PropertyValue&gt;,<br />) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> creator_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a> &#123;<br />        creator: creator_addr,<br />        collection,<br />        <a href="token.md#0x3_token">token</a>,<br />        keys,<br />        old_values,<br />        new_values,<br />    &#125;;<br /><br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(creator_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutate">DefaultPropertyMutate</a> &#123;<br />                creator: creator_addr,<br />                collection,<br />                <a href="token.md#0x3_token">token</a>,<br />                keys,<br />                old_values,<br />                new_values,<br />            &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_DefaultPropertyMutateEvent">DefaultPropertyMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.default_property_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_descrition_mutate_event"></a>

## Function `emit_token_descrition_mutate_event`

Emit description mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_descrition_mutate_event">emit_token_descrition_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_descrition_mutate_event">emit_token_descrition_mutate_event</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    <a href="token.md#0x3_token">token</a>: String,<br />    old_description: String,<br />    new_description: String,<br />) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> creator_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_DescriptionMutateEvent">DescriptionMutateEvent</a> &#123;<br />        creator: creator_addr,<br />        collection,<br />        <a href="token.md#0x3_token">token</a>,<br />        old_description,<br />        new_description,<br />    &#125;;<br /><br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(creator_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_DescriptionMutate">DescriptionMutate</a> &#123;<br />                creator: creator_addr,<br />                collection,<br />                <a href="token.md#0x3_token">token</a>,<br />                old_description,<br />                new_description,<br />            &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_DescriptionMutateEvent">DescriptionMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.description_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_royalty_mutate_event"></a>

## Function `emit_token_royalty_mutate_event`

Emit royalty mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_royalty_mutate_event">emit_token_royalty_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_royalty_numerator: u64, old_royalty_denominator: u64, old_royalty_payee_addr: <b>address</b>, new_royalty_numerator: u64, new_royalty_denominator: u64, new_royalty_payee_addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_royalty_mutate_event">emit_token_royalty_mutate_event</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    <a href="token.md#0x3_token">token</a>: String,<br />    old_royalty_numerator: u64,<br />    old_royalty_denominator: u64,<br />    old_royalty_payee_addr: <b>address</b>,<br />    new_royalty_numerator: u64,<br />    new_royalty_denominator: u64,<br />    new_royalty_payee_addr: <b>address</b>,<br />) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> creator_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_RoyaltyMutateEvent">RoyaltyMutateEvent</a> &#123;<br />        creator: creator_addr,<br />        collection,<br />        <a href="token.md#0x3_token">token</a>,<br />        old_royalty_numerator,<br />        old_royalty_denominator,<br />        old_royalty_payee_addr,<br />        new_royalty_numerator,<br />        new_royalty_denominator,<br />        new_royalty_payee_addr,<br />    &#125;;<br /><br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61; <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(creator_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_RoyaltyMutate">RoyaltyMutate</a> &#123;<br />                creator: creator_addr,<br />                collection,<br />                <a href="token.md#0x3_token">token</a>,<br />                old_royalty_numerator,<br />                old_royalty_denominator,<br />                old_royalty_payee_addr,<br />                new_royalty_numerator,<br />                new_royalty_denominator,<br />                new_royalty_payee_addr,<br />            &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_RoyaltyMutateEvent">RoyaltyMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.royalty_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_event_store_emit_token_maximum_mutate_event"></a>

## Function `emit_token_maximum_mutate_event`

Emit maximum mutation event


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_maximum_mutate_event">emit_token_maximum_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_maximum: u64, new_maximum: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_maximum_mutate_event">emit_token_maximum_mutate_event</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    <a href="token.md#0x3_token">token</a>: String,<br />    old_maximum: u64,<br />    new_maximum: u64,<br />) <b>acquires</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a> &#123;<br />    <b>let</b> creator_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><br />    <b>let</b> <a href="../../aptos-framework/doc/event.md#0x1_event">event</a> &#61; <a href="token_event_store.md#0x3_token_event_store_MaxiumMutateEvent">MaxiumMutateEvent</a> &#123;<br />        creator: creator_addr,<br />        collection,<br />        <a href="token.md#0x3_token">token</a>,<br />        old_maximum,<br />        new_maximum,<br />    &#125;;<br /><br />    <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(creator);<br />    <b>let</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a> &#61;  <b>borrow_global_mut</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(creator_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_event_store.md#0x3_token_event_store_MaximumMutate">MaximumMutate</a> &#123;<br />                creator: creator_addr,<br />                collection,<br />                <a href="token.md#0x3_token">token</a>,<br />                old_maximum,<br />                new_maximum,<br />            &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_event_store.md#0x3_token_event_store_MaxiumMutateEvent">MaxiumMutateEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="token_event_store.md#0x3_token_event_store">token_event_store</a>.maximum_mutate_events,<br />        <a href="../../aptos-framework/doc/event.md#0x1_event">event</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_0_initialize_token_event_store"></a>

### Function `initialize_token_event_store`


<pre><code><b>fun</b> <a href="token_event_store.md#0x3_token_event_store_initialize_token_event_store">initialize_token_event_store</a>(acct: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(acct);<br /><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a> &#123;creator : acct&#125;;<br /></code></pre>


Adjust the overflow value according to the
number of registered events


<a id="0x3_token_event_store_InitializeTokenEventStoreAbortsIf"></a>


<pre><code><b>schema</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a> &#123;<br />creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br />&#125;<br /></code></pre>




<a id="0x3_token_event_store_TokenEventStoreAbortsIf"></a>


<pre><code><b>schema</b> <a href="token_event_store.md#0x3_token_event_store_TokenEventStoreAbortsIf">TokenEventStoreAbortsIf</a> &#123;<br />creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br />&#125;<br /></code></pre>



<a id="@Specification_0_emit_collection_uri_mutate_event"></a>

### Function `emit_collection_uri_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_uri_mutate_event">emit_collection_uri_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_collection_description_mutate_event"></a>

### Function `emit_collection_description_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_description_mutate_event">emit_collection_description_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_collection_maximum_mutate_event"></a>

### Function `emit_collection_maximum_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_collection_maximum_mutate_event">emit_collection_maximum_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_maximum: u64, new_maximum: u64)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_token_opt_in_event"></a>

### Function `emit_token_opt_in_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_opt_in_event">emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a> &#123;creator : <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>&#125;;<br /></code></pre>



<a id="@Specification_0_emit_token_uri_mutate_event"></a>

### Function `emit_token_uri_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_uri_mutate_event">emit_token_uri_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_default_property_mutate_event"></a>

### Function `emit_default_property_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_default_property_mutate_event">emit_default_property_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;&gt;, new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="property_map.md#0x3_property_map_PropertyValue">property_map::PropertyValue</a>&gt;)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_token_descrition_mutate_event"></a>

### Function `emit_token_descrition_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_descrition_mutate_event">emit_token_descrition_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, new_description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_token_royalty_mutate_event"></a>

### Function `emit_token_royalty_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_royalty_mutate_event">emit_token_royalty_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_royalty_numerator: u64, old_royalty_denominator: u64, old_royalty_payee_addr: <b>address</b>, new_royalty_numerator: u64, new_royalty_denominator: u64, new_royalty_payee_addr: <b>address</b>)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_emit_token_maximum_mutate_event"></a>

### Function `emit_token_maximum_mutate_event`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="token_event_store.md#0x3_token_event_store_emit_token_maximum_mutate_event">emit_token_maximum_mutate_event</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token">token</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, old_maximum: u64, new_maximum: u64)<br /></code></pre>




<pre><code><b>include</b> <a href="token_event_store.md#0x3_token_event_store_InitializeTokenEventStoreAbortsIf">InitializeTokenEventStoreAbortsIf</a>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
