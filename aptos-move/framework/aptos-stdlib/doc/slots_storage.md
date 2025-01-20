
<a id="0x1_slots_storage"></a>

# Module `0x1::slots_storage`



-  [Enum `Link`](#0x1_slots_storage_Link)
-  [Struct `SlotsStorage`](#0x1_slots_storage_SlotsStorage)
-  [Struct `TransientSlot`](#0x1_slots_storage_TransientSlot)
-  [Constants](#@Constants_0)
-  [Function `new_storage_slots`](#0x1_slots_storage_new_storage_slots)
-  [Function `add`](#0x1_slots_storage_add)
-  [Function `remove`](#0x1_slots_storage_remove)
-  [Function `destroy_empty`](#0x1_slots_storage_destroy_empty)
-  [Function `borrow`](#0x1_slots_storage_borrow)
-  [Function `borrow_mut`](#0x1_slots_storage_borrow_mut)
-  [Function `get_index`](#0x1_slots_storage_get_index)
-  [Function `create_transient_slot`](#0x1_slots_storage_create_transient_slot)
-  [Function `add_transient_slot`](#0x1_slots_storage_add_transient_slot)
-  [Function `transiently_remove`](#0x1_slots_storage_transiently_remove)
-  [Function `destroy_transient_slot`](#0x1_slots_storage_destroy_transient_slot)


<pre><code><b>use</b> <a href="table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x1_slots_storage_Link"></a>

## Enum `Link`



<pre><code>enum <a href="slots_storage.md#0x1_slots_storage_Link">Link</a>&lt;T: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Some</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Empty</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>next: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_slots_storage_SlotsStorage"></a>

## Struct `SlotsStorage`



<pre><code><b>struct</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slots: <a href="table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="slots_storage.md#0x1_slots_storage_Link">slots_storage::Link</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_slot_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>never_deallocate: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>reuse_head_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_slots_storage_TransientSlot"></a>

## Struct `TransientSlot`



<pre><code><b>struct</b> <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slot_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_slots_storage_NULL_INDEX"></a>



<pre><code><b>const</b> <a href="slots_storage.md#0x1_slots_storage_NULL_INDEX">NULL_INDEX</a>: u64 = 0;
</code></pre>



<a id="0x1_slots_storage_new_storage_slots"></a>

## Function `new_storage_slots`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_new_storage_slots">new_storage_slots</a>&lt;T: store&gt;(): <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_new_storage_slots">new_storage_slots</a>&lt;T: store&gt;(): <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt; {
    <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a> {
        slots: <a href="table.md#0x1_table_new">table::new</a>(),
        new_slot_index: 1,
        never_deallocate: <b>false</b>,
        reuse_head_index: <a href="slots_storage.md#0x1_slots_storage_NULL_INDEX">NULL_INDEX</a>,
    }
}
</code></pre>



</details>

<a id="0x1_slots_storage_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_add">add</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;, val: T): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_add">add</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;, val: T): u64 {
    <b>let</b> slot_index = self.new_slot_index;
    self.new_slot_index = self.new_slot_index + 1;
    self.slots.<a href="slots_storage.md#0x1_slots_storage_add">add</a>(slot_index, Link::Some { value: val });
    slot_index
}
</code></pre>



</details>

<a id="0x1_slots_storage_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_remove">remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;, slot_index: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_remove">remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;, slot_index: u64): T {
    <b>let</b> Link::Some { value } = self.slots.<a href="slots_storage.md#0x1_slots_storage_remove">remove</a>(slot_index);
    value
}
</code></pre>



</details>

<a id="0x1_slots_storage_destroy_empty"></a>

## Function `destroy_empty`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_destroy_empty">destroy_empty</a>&lt;T: store&gt;(self: <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_destroy_empty">destroy_empty</a>&lt;T: store&gt;(self: <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;) {
    <b>let</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a> {
        slots,
        new_slot_index: _,
        never_deallocate: _,
        reuse_head_index: _,
    } = self;
    slots.<a href="slots_storage.md#0x1_slots_storage_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_slots_storage_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;, slot_index: u64): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;, slot_index: u64): &T {
    &self.slots.<a href="slots_storage.md#0x1_slots_storage_borrow">borrow</a>(slot_index).value
}
</code></pre>



</details>

<a id="0x1_slots_storage_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;, slot_index: u64): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;, slot_index: u64): &<b>mut</b> T {
    &<b>mut</b> self.slots.<a href="slots_storage.md#0x1_slots_storage_borrow_mut">borrow_mut</a>(slot_index).value
}
</code></pre>



</details>

<a id="0x1_slots_storage_get_index"></a>

## Function `get_index`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_get_index">get_index</a>(self: &<a href="slots_storage.md#0x1_slots_storage_TransientSlot">slots_storage::TransientSlot</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_get_index">get_index</a>(self: &<a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a>): u64 {
    self.slot_index
}
</code></pre>



</details>

<a id="0x1_slots_storage_create_transient_slot"></a>

## Function `create_transient_slot`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_create_transient_slot">create_transient_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;): <a href="slots_storage.md#0x1_slots_storage_TransientSlot">slots_storage::TransientSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_create_transient_slot">create_transient_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;): <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a> {
    <b>let</b> slot_index = self.new_slot_index;
    self.new_slot_index = self.new_slot_index + 1;
    <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a> {
        slot_index,
    }
}
</code></pre>



</details>

<a id="0x1_slots_storage_add_transient_slot"></a>

## Function `add_transient_slot`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_add_transient_slot">add_transient_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;, slot: <a href="slots_storage.md#0x1_slots_storage_TransientSlot">slots_storage::TransientSlot</a>, val: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_add_transient_slot">add_transient_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;, slot: <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a>, val: T) {
    <b>let</b> <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a> { slot_index } = slot;
    self.slots.<a href="slots_storage.md#0x1_slots_storage_add">add</a>(slot_index, Link::Some { value: val });
}
</code></pre>



</details>

<a id="0x1_slots_storage_transiently_remove"></a>

## Function `transiently_remove`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_transiently_remove">transiently_remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">slots_storage::SlotsStorage</a>&lt;T&gt;, slot_index: u64): (<a href="slots_storage.md#0x1_slots_storage_TransientSlot">slots_storage::TransientSlot</a>, T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_transiently_remove">transiently_remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="slots_storage.md#0x1_slots_storage_SlotsStorage">SlotsStorage</a>&lt;T&gt;, slot_index: u64): (<a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a>, T) {
    <b>let</b> Link::Some { value } = self.slots.<a href="slots_storage.md#0x1_slots_storage_remove">remove</a>(slot_index);
    (<a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a> { slot_index }, value)
}
</code></pre>



</details>

<a id="0x1_slots_storage_destroy_transient_slot"></a>

## Function `destroy_transient_slot`



<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_destroy_transient_slot">destroy_transient_slot</a>(self: <a href="slots_storage.md#0x1_slots_storage_TransientSlot">slots_storage::TransientSlot</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="slots_storage.md#0x1_slots_storage_destroy_transient_slot">destroy_transient_slot</a>(self: <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a>) {
    <b>let</b> <a href="slots_storage.md#0x1_slots_storage_TransientSlot">TransientSlot</a> { slot_index: _ } = self;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
