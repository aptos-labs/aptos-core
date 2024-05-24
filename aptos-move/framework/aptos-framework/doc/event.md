
<a id="0x1_event"></a>

# Module `0x1::event`

The Event module defines an <code>EventHandleGenerator</code> that is used to create
<code><a href="event.md#0x1_event_EventHandle">EventHandle</a></code>s with unique GUIDs. It contains a counter for the number
of <code><a href="event.md#0x1_event_EventHandle">EventHandle</a></code>s it generates. An <code><a href="event.md#0x1_event_EventHandle">EventHandle</a></code> is used to count the number of
events emitted to a handle and emit events to the event store.


-  [Struct `EventHandle`](#0x1_event_EventHandle)
-  [Function `emit`](#0x1_event_emit)
-  [Function `write_module_event_to_store`](#0x1_event_write_module_event_to_store)
-  [Function `new_event_handle`](#0x1_event_new_event_handle)
-  [Function `emit_event`](#0x1_event_emit_event)
-  [Function `guid`](#0x1_event_guid)
-  [Function `counter`](#0x1_event_counter)
-  [Function `write_to_event_store`](#0x1_event_write_to_event_store)
-  [Function `destroy_handle`](#0x1_event_destroy_handle)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `emit`](#@Specification_0_emit)
    -  [Function `write_module_event_to_store`](#@Specification_0_write_module_event_to_store)
    -  [Function `emit_event`](#@Specification_0_emit_event)
    -  [Function `guid`](#@Specification_0_guid)
    -  [Function `counter`](#@Specification_0_counter)
    -  [Function `write_to_event_store`](#@Specification_0_write_to_event_store)
    -  [Function `destroy_handle`](#@Specification_0_destroy_handle)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;<br /></code></pre>



<a id="0x1_event_EventHandle"></a>

## Struct `EventHandle`

A handle for an event such that:
1. Other modules can emit events to this handle.
2. Storage can use this handle to prove the total number of events that happened in the past.


<pre><code>&#35;[deprecated]<br /><b>struct</b> <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T: drop, store&gt; <b>has</b> store<br /></code></pre>



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

<a id="0x1_event_emit"></a>

## Function `emit`

Emit a module event with payload <code>msg</code>.


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit">emit</a>&lt;T: drop, store&gt;(msg: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit">emit</a>&lt;T: store &#43; drop&gt;(msg: T) &#123;<br />    <a href="event.md#0x1_event_write_module_event_to_store">write_module_event_to_store</a>&lt;T&gt;(msg);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_event_write_module_event_to_store"></a>

## Function `write_module_event_to_store`

Log <code>msg</code> with the event stream identified by <code>T</code>


<pre><code><b>fun</b> <a href="event.md#0x1_event_write_module_event_to_store">write_module_event_to_store</a>&lt;T: drop, store&gt;(msg: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="event.md#0x1_event_write_module_event_to_store">write_module_event_to_store</a>&lt;T: drop &#43; store&gt;(msg: T);<br /></code></pre>



</details>

<a id="0x1_event_new_event_handle"></a>

## Function `new_event_handle`

Use EventHandleGenerator to generate a unique event handle for <code>sig</code>


<pre><code>&#35;[deprecated]<br /><b>public</b>(<b>friend</b>) <b>fun</b> <a href="event.md#0x1_event_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="event.md#0x1_event_new_event_handle">new_event_handle</a>&lt;T: drop &#43; store&gt;(<a href="guid.md#0x1_guid">guid</a>: GUID): <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt; &#123;<br />    <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt; &#123;<br />        counter: 0,<br />        <a href="guid.md#0x1_guid">guid</a>,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_event_emit_event"></a>

## Function `emit_event`

Emit an event with payload <code>msg</code> by using <code>handle_ref</code>&apos;s key and counter.


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit_event">emit_event</a>&lt;T: drop, store&gt;(handle_ref: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;, msg: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit_event">emit_event</a>&lt;T: drop &#43; store&gt;(handle_ref: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;, msg: T) &#123;<br />    <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;handle_ref.<a href="guid.md#0x1_guid">guid</a>), handle_ref.counter, msg);<br />    <b>spec</b> &#123;<br />        <b>assume</b> handle_ref.counter &#43; 1 &lt;&#61; MAX_U64;<br />    &#125;;<br />    handle_ref.counter &#61; handle_ref.counter &#43; 1;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_event_guid"></a>

## Function `guid`

Return the GUID associated with this EventHandle


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid">guid</a>&lt;T: drop, store&gt;(handle_ref: &amp;<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): &amp;<a href="guid.md#0x1_guid_GUID">guid::GUID</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid">guid</a>&lt;T: drop &#43; store&gt;(handle_ref: &amp;<a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;): &amp;GUID &#123;<br />    &amp;handle_ref.<a href="guid.md#0x1_guid">guid</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_event_counter"></a>

## Function `counter`

Return the current counter associated with this EventHandle


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="event.md#0x1_event_counter">counter</a>&lt;T: drop, store&gt;(handle_ref: &amp;<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_counter">counter</a>&lt;T: drop &#43; store&gt;(handle_ref: &amp;<a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;): u64 &#123;<br />    handle_ref.counter<br />&#125;<br /></code></pre>



</details>

<a id="0x1_event_write_to_event_store"></a>

## Function `write_to_event_store`

Log <code>msg</code> as the <code>count</code>th event associated with the event stream identified by <code><a href="guid.md#0x1_guid">guid</a></code>


<pre><code>&#35;[deprecated]<br /><b>fun</b> <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T: drop, store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, count: u64, msg: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T: drop &#43; store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, count: u64, msg: T);<br /></code></pre>



</details>

<a id="0x1_event_destroy_handle"></a>

## Function `destroy_handle`

Destroy a unique handle.


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="event.md#0x1_event_destroy_handle">destroy_handle</a>&lt;T: drop, store&gt;(handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_destroy_handle">destroy_handle</a>&lt;T: drop &#43; store&gt;(handle: <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt;) &#123;<br />    <a href="event.md#0x1_event_EventHandle">EventHandle</a>&lt;T&gt; &#123; counter: _, <a href="guid.md#0x1_guid">guid</a>: _ &#125; &#61; handle;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Each event handle possesses a distinct and unique GUID.</td>
<td>Critical</td>
<td>The new_event_handle function creates an EventHandle object with a unique GUID, ensuring distinct identification.</td>
<td>Audited: GUIDs are created in guid::create. Each time the function is called, it increments creation_num_ref. Multiple calls to the function will result in distinct GUID values.</td>
</tr>

<tr>
<td>2</td>
<td>Unable to publish two events with the same GUID &amp; sequence number.</td>
<td>Critical</td>
<td>Two events may either have the same GUID with a different counter or the same counter with a different GUID.</td>
<td>This is implied by <a href="#high-level-req">high&#45;level requirement 1</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Event native functions respect normal Move rules around object creation and destruction.</td>
<td>Critical</td>
<td>Must follow the same rules and principles that apply to object creation and destruction in Move when using event native functions.</td>
<td>The native functions of this module have been manually audited.</td>
</tr>

<tr>
<td>4</td>
<td>Counter increases monotonically between event emissions</td>
<td>Medium</td>
<td>With each event emission, the emit_event function increments the counter of the EventHandle by one.</td>
<td>Formally verified in the post condition of <a href="#high-level-req-4">emit_event</a>.</td>
</tr>

<tr>
<td>5</td>
<td>For a given EventHandle, it should always be possible to: (1) return the GUID associated with this EventHandle, (2) return the current counter associated with this EventHandle, and (3) destroy the handle.</td>
<td>Low</td>
<td>The following functions should not abort if EventHandle exists: guid(), counter(), destroy_handle().</td>
<td>Formally verified via <a href="#high-level-req-5.1">guid</a>, <a href="#high-level-req-5.2">counter</a> and <a href="#high-level-req-5.3">destroy_handle</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_0_emit"></a>

### Function `emit`


<pre><code><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit">emit</a>&lt;T: drop, store&gt;(msg: T)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_0_write_module_event_to_store"></a>

### Function `write_module_event_to_store`


<pre><code><b>fun</b> <a href="event.md#0x1_event_write_module_event_to_store">write_module_event_to_store</a>&lt;T: drop, store&gt;(msg: T)<br /></code></pre>


Native function use opaque.


<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_0_emit_event"></a>

### Function `emit_event`


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="event.md#0x1_event_emit_event">emit_event</a>&lt;T: drop, store&gt;(handle_ref: &amp;<b>mut</b> <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;, msg: T)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>ensures</b> [concrete] handle_ref.counter &#61;&#61; <b>old</b>(handle_ref.counter) &#43; 1;<br /></code></pre>



<a id="@Specification_0_guid"></a>

### Function `guid`


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid">guid</a>&lt;T: drop, store&gt;(handle_ref: &amp;<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): &amp;<a href="guid.md#0x1_guid_GUID">guid::GUID</a><br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-5.1" href="#high-level-req">high&#45;level requirement 5</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_0_counter"></a>

### Function `counter`


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="event.md#0x1_event_counter">counter</a>&lt;T: drop, store&gt;(handle_ref: &amp;<a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;): u64<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-5.2" href="#high-level-req">high&#45;level requirement 5</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_0_write_to_event_store"></a>

### Function `write_to_event_store`


<pre><code>&#35;[deprecated]<br /><b>fun</b> <a href="event.md#0x1_event_write_to_event_store">write_to_event_store</a>&lt;T: drop, store&gt;(<a href="guid.md#0x1_guid">guid</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, count: u64, msg: T)<br /></code></pre>


Native function use opaque.


<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_0_destroy_handle"></a>

### Function `destroy_handle`


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="event.md#0x1_event_destroy_handle">destroy_handle</a>&lt;T: drop, store&gt;(handle: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;)<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-5.3" href="#high-level-req">high&#45;level requirement 5</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
