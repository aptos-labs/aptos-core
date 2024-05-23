
<a id="0x1_event"></a>

# Module `0x1::event`

The Event module defines an <code>EventHandleGenerator</code> that is used to create<br/> <code>EventHandle</code>s with unique GUIDs. It contains a counter for the number<br/> of <code>EventHandle</code>s it generates. An <code>EventHandle</code> is used to count the number of<br/> events emitted to a handle and emit events to the event store.


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


<pre><code>use 0x1::bcs;<br/>use 0x1::guid;<br/></code></pre>



<a id="0x1_event_EventHandle"></a>

## Struct `EventHandle`

A handle for an event such that:<br/> 1. Other modules can emit events to this handle.<br/> 2. Storage can use this handle to prove the total number of events that happened in the past.


<pre><code>&#35;[deprecated]<br/>struct EventHandle&lt;T: drop, store&gt; has store<br/></code></pre>



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
<code>guid: guid::GUID</code>
</dt>
<dd>
 A globally unique ID for this event stream.
</dd>
</dl>


</details>

<a id="0x1_event_emit"></a>

## Function `emit`

Emit a module event with payload <code>msg</code>.


<pre><code>public fun emit&lt;T: drop, store&gt;(msg: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun emit&lt;T: store &#43; drop&gt;(msg: T) &#123;<br/>    write_module_event_to_store&lt;T&gt;(msg);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_event_write_module_event_to_store"></a>

## Function `write_module_event_to_store`

Log <code>msg</code> with the event stream identified by <code>T</code>


<pre><code>fun write_module_event_to_store&lt;T: drop, store&gt;(msg: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun write_module_event_to_store&lt;T: drop &#43; store&gt;(msg: T);<br/></code></pre>



</details>

<a id="0x1_event_new_event_handle"></a>

## Function `new_event_handle`

Use EventHandleGenerator to generate a unique event handle for <code>sig</code>


<pre><code>&#35;[deprecated]<br/>public(friend) fun new_event_handle&lt;T: drop, store&gt;(guid: guid::GUID): event::EventHandle&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun new_event_handle&lt;T: drop &#43; store&gt;(guid: GUID): EventHandle&lt;T&gt; &#123;<br/>    EventHandle&lt;T&gt; &#123;<br/>        counter: 0,<br/>        guid,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_event_emit_event"></a>

## Function `emit_event`

Emit an event with payload <code>msg</code> by using <code>handle_ref</code>&apos;s key and counter.


<pre><code>&#35;[deprecated]<br/>public fun emit_event&lt;T: drop, store&gt;(handle_ref: &amp;mut event::EventHandle&lt;T&gt;, msg: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun emit_event&lt;T: drop &#43; store&gt;(handle_ref: &amp;mut EventHandle&lt;T&gt;, msg: T) &#123;<br/>    write_to_event_store&lt;T&gt;(bcs::to_bytes(&amp;handle_ref.guid), handle_ref.counter, msg);<br/>    spec &#123;<br/>        assume handle_ref.counter &#43; 1 &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    handle_ref.counter &#61; handle_ref.counter &#43; 1;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_event_guid"></a>

## Function `guid`

Return the GUID associated with this EventHandle


<pre><code>&#35;[deprecated]<br/>public fun guid&lt;T: drop, store&gt;(handle_ref: &amp;event::EventHandle&lt;T&gt;): &amp;guid::GUID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun guid&lt;T: drop &#43; store&gt;(handle_ref: &amp;EventHandle&lt;T&gt;): &amp;GUID &#123;<br/>    &amp;handle_ref.guid<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_event_counter"></a>

## Function `counter`

Return the current counter associated with this EventHandle


<pre><code>&#35;[deprecated]<br/>public fun counter&lt;T: drop, store&gt;(handle_ref: &amp;event::EventHandle&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun counter&lt;T: drop &#43; store&gt;(handle_ref: &amp;EventHandle&lt;T&gt;): u64 &#123;<br/>    handle_ref.counter<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_event_write_to_event_store"></a>

## Function `write_to_event_store`

Log <code>msg</code> as the <code>count</code>th event associated with the event stream identified by <code>guid</code>


<pre><code>&#35;[deprecated]<br/>fun write_to_event_store&lt;T: drop, store&gt;(guid: vector&lt;u8&gt;, count: u64, msg: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun write_to_event_store&lt;T: drop &#43; store&gt;(guid: vector&lt;u8&gt;, count: u64, msg: T);<br/></code></pre>



</details>

<a id="0x1_event_destroy_handle"></a>

## Function `destroy_handle`

Destroy a unique handle.


<pre><code>&#35;[deprecated]<br/>public fun destroy_handle&lt;T: drop, store&gt;(handle: event::EventHandle&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_handle&lt;T: drop &#43; store&gt;(handle: EventHandle&lt;T&gt;) &#123;<br/>    EventHandle&lt;T&gt; &#123; counter: _, guid: _ &#125; &#61; handle;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;Each event handle possesses a distinct and unique GUID.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The new_event_handle function creates an EventHandle object with a unique GUID, ensuring distinct identification.&lt;/td&gt;<br/>&lt;td&gt;Audited: GUIDs are created in guid::create. Each time the function is called, it increments creation_num_ref. Multiple calls to the function will result in distinct GUID values.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;Unable to publish two events with the same GUID &amp; sequence number.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;Two events may either have the same GUID with a different counter or the same counter with a different GUID.&lt;/td&gt;<br/>&lt;td&gt;This is implied by &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Event native functions respect normal Move rules around object creation and destruction.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;Must follow the same rules and principles that apply to object creation and destruction in Move when using event native functions.&lt;/td&gt;<br/>&lt;td&gt;The native functions of this module have been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;Counter increases monotonically between event emissions&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;With each event emission, the emit_event function increments the counter of the EventHandle by one.&lt;/td&gt;<br/>&lt;td&gt;Formally verified in the post condition of &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;emit_event&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;For a given EventHandle, it should always be possible to: (1) return the GUID associated with this EventHandle, (2) return the current counter associated with this EventHandle, and (3) destroy the handle.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The following functions should not abort if EventHandle exists: guid(), counter(), destroy_handle().&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.1&quot;&gt;guid&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.2&quot;&gt;counter&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.3&quot;&gt;destroy_handle&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_0_emit"></a>

### Function `emit`


<pre><code>public fun emit&lt;T: drop, store&gt;(msg: T)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_0_write_module_event_to_store"></a>

### Function `write_module_event_to_store`


<pre><code>fun write_module_event_to_store&lt;T: drop, store&gt;(msg: T)<br/></code></pre>


Native function use opaque.


<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_0_emit_event"></a>

### Function `emit_event`


<pre><code>&#35;[deprecated]<br/>public fun emit_event&lt;T: drop, store&gt;(handle_ref: &amp;mut event::EventHandle&lt;T&gt;, msg: T)<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
ensures [concrete] handle_ref.counter &#61;&#61; old(handle_ref.counter) &#43; 1;<br/></code></pre>



<a id="@Specification_0_guid"></a>

### Function `guid`


<pre><code>&#35;[deprecated]<br/>public fun guid&lt;T: drop, store&gt;(handle_ref: &amp;event::EventHandle&lt;T&gt;): &amp;guid::GUID<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_0_counter"></a>

### Function `counter`


<pre><code>&#35;[deprecated]<br/>public fun counter&lt;T: drop, store&gt;(handle_ref: &amp;event::EventHandle&lt;T&gt;): u64<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_0_write_to_event_store"></a>

### Function `write_to_event_store`


<pre><code>&#35;[deprecated]<br/>fun write_to_event_store&lt;T: drop, store&gt;(guid: vector&lt;u8&gt;, count: u64, msg: T)<br/></code></pre>


Native function use opaque.


<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_0_destroy_handle"></a>

### Function `destroy_handle`


<pre><code>&#35;[deprecated]<br/>public fun destroy_handle&lt;T: drop, store&gt;(handle: event::EventHandle&lt;T&gt;)<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
