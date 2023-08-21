
<a name="0x1_event"></a>

# Module `0x1::event`

The Event module defines an <code>EventHandleGenerator</code> that is used to create
<code><a href="event.md#0x1_event_EventHandle">EventHandle</a></code>s with unique GUIDs. It contains a counter for the number
of <code><a href="event.md#0x1_event_EventHandle">EventHandle</a></code>s it generates. An <code><a href="event.md#0x1_event_EventHandle">EventHandle</a></code> is used to count the number of
events emitted to a handle and emit events to the event store.


-  [Struct `EventHandle`](#0x1_event_EventHandle)
-  [Constants](#@Constants_0)
-  [Function `emit`](#0x1_event_emit)
-  [Function `write_to_module_event_store`](#0x1_event_write_to_module_event_store)
-  [Function `new_event_handle`](#0x1_event_new_event_handle)
-  [Function `emit_event`](#0x1_event_emit_event)
-  [Function `guid`](#0x1_event_guid)
-  [Function `counter`](#0x1_event_counter)
-  [Function `write_to_event_store`](#0x1_event_write_to_event_store)
-  [Function `destroy_handle`](#0x1_event_destroy_handle)
-  [Specification](#@Specification_1)
    -  [Function `emit_event`](#@Specification_1_emit_event)
    -  [Function `guid`](#@Specification_1_guid)
    -  [Function `counter`](#@Specification_1_counter)
    -  [Function `write_to_event_store`](#@Specification_1_write_to_event_store)
    -  [Function `destroy_handle`](#@Specification_1_destroy_handle)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;
</code></pre>



<a name="0x1_event_EventHandle"></a>

## Struct `EventHandle`



<pre><code>#[deprecated]
<b>struct</b> <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T: drop, store&gt; <b>has</b> store
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
<code><a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a></code>
</dt>
<dd>
 A globally unique ID for this event stream.
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_event_ENOT_EVENT_STRUCT"></a>

The module event passed into <code>emit</code> function is not a valid #[event]-attributed struct type.


<pre><code><b>const</b> <a href="event.md#0x1_event_ENOT_EVENT_STRUCT">ENOT_EVENT_STRUCT</a>: u64 = 1;
</code></pre>



<a name="0x1_event_emit"></a>

## Function `emit`

Emit an event with payload <code>msg</code> by using <code>handle_ref</code>'s key and counter.


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit">emit</a>&lt;T: drop, store&gt;(msg: &T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit">emit</a>&lt;T: drop + store&gt;(msg: &T) {
    <a href="event.md#0x1_event_write_to_module_event_store">write_to_module_event_store</a>&lt;T&gt;(msg);
}
</code></pre>



</details>

<a name="0x1_event_write_to_module_event_store"></a>

## Function `write_to_module_event_store`

Log <code>msg</code> with the event stream identified by <code>T</code>


<pre><code><b>fun</b> <a href="event.md#0x1_event_write_to_module_event_store">write_to_module_event_store</a>&lt;T: drop, store&gt;(msg: &T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="event.md#0x1_event_write_to_module_event_store">write_to_module_event_store</a>&lt;T: drop + store&gt;(msg: &T);
</code></pre>



</details>

<a name="0x1_event_new_event_handle"></a>

## Function `new_event_handle`



<pre><code>#[deprecated]
<b>public</b>(<b>friend</b>) <b>fun</b> <a href="event.md#0x1_event_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="event.md#0x1_event_new_event_handle">new_event_handle</a>&lt;T: drop + store&gt;(<a href="guid.md#0x1_guid">guid</a>: GUID): <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt; {
    <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt; {
        counter: 0,
        <a href="guid.md#0x1_guid">guid</a>,
    }
}
</code></pre>



</details>

<a name="0x1_event_emit_event"></a>

## Function `emit_event`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit_event">emit_event</a>&lt;T: drop, store&gt;(handle_ref: &<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;, msg: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit_event">emit_event</a>&lt;T: drop + store&gt;(handle_ref: &<b>mut</b> <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;, msg: T) {
    <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&handle_ref.<a href="guid.md#0x1_guid">guid</a>), handle_ref.counter, msg);
    <b>spec</b> {
        <b>assume</b> handle_ref.counter + 1 &lt;= MAX_U64;
    };
    handle_ref.counter = handle_ref.counter + 1;
}
</code></pre>



</details>

<a name="0x1_event_guid"></a>

## Function `guid`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="guid.md#0x1_guid">guid</a>&lt;T: drop, store&gt;(handle_ref: &<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid">guid</a>&lt;T: drop + store&gt;(handle_ref: &<a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;): &GUID {
    &handle_ref.<a href="guid.md#0x1_guid">guid</a>
}
</code></pre>



</details>

<a name="0x1_event_counter"></a>

## Function `counter`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="event.md#0x1_event_counter">counter</a>&lt;T: drop, store&gt;(handle_ref: &<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_counter">counter</a>&lt;T: drop + store&gt;(handle_ref: &<a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;): u64 {
    handle_ref.counter
}
</code></pre>



</details>

<a name="0x1_event_write_to_event_store"></a>

## Function `write_to_event_store`



<pre><code>#[deprecated]
<b>fun</b> <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T: drop, store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, count: u64, msg: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T: drop + store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, count: u64, msg: T);
</code></pre>



</details>

<a name="0x1_event_destroy_handle"></a>

## Function `destroy_handle`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="event.md#0x1_event_destroy_handle">destroy_handle</a>&lt;T: drop, store&gt;(handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_destroy_handle">destroy_handle</a>&lt;T: drop + store&gt;(handle: <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;) {
    <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt; { counter: _, <a href="guid.md#0x1_guid">guid</a>: _ } = handle;
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a name="@Specification_1_emit_event"></a>

### Function `emit_event`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit_event">emit_event</a>&lt;T: drop, store&gt;(handle_ref: &<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;, msg: T)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [concrete] handle_ref.counter == <b>old</b>(handle_ref.counter) + 1;
</code></pre>



<a name="@Specification_1_guid"></a>

### Function `guid`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="guid.md#0x1_guid">guid</a>&lt;T: drop, store&gt;(handle_ref: &<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a name="@Specification_1_counter"></a>

### Function `counter`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="event.md#0x1_event_counter">counter</a>&lt;T: drop, store&gt;(handle_ref: &<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): u64
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a name="@Specification_1_write_to_event_store"></a>

### Function `write_to_event_store`


<pre><code>#[deprecated]
<b>fun</b> <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T: drop, store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, count: u64, msg: T)
</code></pre>


Native function use opaque.


<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_1_destroy_handle"></a>

### Function `destroy_handle`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="event.md#0x1_event_destroy_handle">destroy_handle</a>&lt;T: drop, store&gt;(handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;)
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
