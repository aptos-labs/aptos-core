
<a name="0x1_Event"></a>

# Module `0x1::Event`

The Event module defines an <code><a href="Event.md#0x1_Event_EventHandleGenerator">EventHandleGenerator</a></code> that is used to create
<code><a href="Event.md#0x1_Event_EventHandle">EventHandle</a></code>s with unique GUIDs. It contains a counter for the number
of <code><a href="Event.md#0x1_Event_EventHandle">EventHandle</a></code>s it generates. An <code><a href="Event.md#0x1_Event_EventHandle">EventHandle</a></code> is used to count the number of
events emitted to a handle and emit events to the event store.


-  [Struct `GUIDWrapper`](#0x1_Event_GUIDWrapper)
-  [Struct `EventHandle`](#0x1_Event_EventHandle)
-  [Resource `EventHandleGenerator`](#0x1_Event_EventHandleGenerator)
-  [Function `new_event_handle`](#0x1_Event_new_event_handle)
-  [Function `emit_event`](#0x1_Event_emit_event)
-  [Function `guid`](#0x1_Event_guid)
-  [Function `write_to_event_store`](#0x1_Event_write_to_event_store)
-  [Function `destroy_handle`](#0x1_Event_destroy_handle)
-  [Module Specification](#@Module_Specification_0)


<pre><code><b>use</b> <a href="BCS.md#0x1_BCS">0x1::BCS</a>;
<b>use</b> <a href="GUID.md#0x1_GUID">0x1::GUID</a>;
</code></pre>



<a name="0x1_Event_GUIDWrapper"></a>

## Struct `GUIDWrapper`

Wrapper for a GUID for layout compatibility with legacy EventHandle id's


<pre><code><b>struct</b> <a href="Event.md#0x1_Event_GUIDWrapper">GUIDWrapper</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>len_bytes: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>guid: <a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Event_EventHandle"></a>

## Struct `EventHandle`

A handle for an event such that:
1. Other modules can emit events to this handle.
2. Storage can use this handle to prove the total number of events that happened in the past.


<pre><code><b>struct</b> <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T: drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>counter: u64</code>
</dt>
<dd>
 Total number of events emitted to this event stream.
</dd>
<dt>
<code>guid: <a href="Event.md#0x1_Event_GUIDWrapper">Event::GUIDWrapper</a></code>
</dt>
<dd>
 A globally unique ID for this event stream.
</dd>
</dl>


</details>

<a name="0x1_Event_EventHandleGenerator"></a>

## Resource `EventHandleGenerator`

Deprecated. Only kept around so Diem clients know how to deserialize existing EventHandleGenerator's


<pre><code><b>struct</b> <a href="Event.md#0x1_Event_EventHandleGenerator">EventHandleGenerator</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>counter: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Event_new_event_handle"></a>

## Function `new_event_handle`

Use EventHandleGenerator to generate a unique event handle for <code>sig</code>


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(account: &signer): <a href="Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_new_event_handle">new_event_handle</a>&lt;T: drop + store&gt;(account: &signer): <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt; {
    // must be 24 for compatibility <b>with</b> legacy <a href="Event.md#0x1_Event">Event</a> ID's--see comment on <a href="Event.md#0x1_Event_GUIDWrapper">GUIDWrapper</a>
    <b>let</b> len_bytes = 24u8;
     <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt; {
        counter: 0,
        guid: <a href="Event.md#0x1_Event_GUIDWrapper">GUIDWrapper</a> { len_bytes, guid: <a href="GUID.md#0x1_GUID_create">GUID::create</a>(account) }
    }
}
</code></pre>



</details>

<a name="0x1_Event_emit_event"></a>

## Function `emit_event`

Emit an event with payload <code>msg</code> by using <code>handle_ref</code>'s key and counter.


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_emit_event">emit_event</a>&lt;T: drop, store&gt;(handle_ref: &<b>mut</b> <a href="Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;T&gt;, msg: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_emit_event">emit_event</a>&lt;T: drop + store&gt;(handle_ref: &<b>mut</b> <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt;, msg: T) {
    <a href="Event.md#0x1_Event_write_to_event_store">write_to_event_store</a>&lt;T&gt;(<a href="BCS.md#0x1_BCS_to_bytes">BCS::to_bytes</a>(&handle_ref.guid.guid), handle_ref.counter, msg);
    handle_ref.counter = handle_ref.counter + 1;
}
</code></pre>



</details>

<a name="0x1_Event_guid"></a>

## Function `guid`

Return the GUIID associated with this EventHandle


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_guid">guid</a>&lt;T: drop, store&gt;(handle_ref: &<a href="Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;T&gt;): &<a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_guid">guid</a>&lt;T: drop + store&gt;(handle_ref: &<a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt;): &<a href="GUID.md#0x1_GUID">GUID</a> {
    &handle_ref.guid.guid
}
</code></pre>



</details>

<a name="0x1_Event_write_to_event_store"></a>

## Function `write_to_event_store`

Log <code>msg</code> as the <code>count</code>th event associated with the event stream identified by <code>guid</code>


<pre><code><b>fun</b> <a href="Event.md#0x1_Event_write_to_event_store">write_to_event_store</a>&lt;T: drop, store&gt;(guid: vector&lt;u8&gt;, count: u64, msg: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Event.md#0x1_Event_write_to_event_store">write_to_event_store</a>&lt;T: drop + store&gt;(guid: vector&lt;u8&gt;, count: u64, msg: T);
</code></pre>



</details>

<a name="0x1_Event_destroy_handle"></a>

## Function `destroy_handle`

Destroy a unique handle.


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_destroy_handle">destroy_handle</a>&lt;T: drop, store&gt;(handle: <a href="Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Event.md#0x1_Event_destroy_handle">destroy_handle</a>&lt;T: drop + store&gt;(handle: <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt;) {
    <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt; { counter: _, guid: _ } = handle;
}
</code></pre>



</details>

<a name="@Module_Specification_0"></a>

## Module Specification



Functions of the event module are mocked out using the intrinsic
pragma. They are implemented in the prover's prelude.


<pre><code><b>pragma</b> intrinsic = <b>true</b>;
</code></pre>


Determines equality between the guids of two event handles. Since fields of intrinsic
structs cannot be accessed, this function is provided.


<a name="0x1_Event_spec_guid_eq"></a>


<pre><code><b>fun</b> <a href="Event.md#0x1_Event_spec_guid_eq">spec_guid_eq</a>&lt;T&gt;(h1: <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt;, h2: <a href="Event.md#0x1_Event_EventHandle">EventHandle</a>&lt;T&gt;): bool {
    // The implementation currently can just <b>use</b> <b>native</b> equality since the mocked prover
    // representation does not have the `counter` field.
    h1 == h2
}
</code></pre>


[//]: # ("File containing references which can be used from documentation")
