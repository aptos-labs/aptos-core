
<a id="0x1_storage_slot_or_inline"></a>

# Module `0x1::storage_slot_or_inline`



-  [Enum `StorageSlotOrInline`](#0x1_storage_slot_or_inline_StorageSlotOrInline)
-  [Struct `Dummy`](#0x1_storage_slot_or_inline_Dummy)
-  [Constants](#@Constants_0)
-  [Function `new_inline`](#0x1_storage_slot_or_inline_new_inline)
-  [Function `new_storage_slot`](#0x1_storage_slot_or_inline_new_storage_slot)
-  [Function `borrow`](#0x1_storage_slot_or_inline_borrow)
-  [Function `borrow_mut`](#0x1_storage_slot_or_inline_borrow_mut)
-  [Function `destroy`](#0x1_storage_slot_or_inline_destroy)
-  [Function `move_to_inline`](#0x1_storage_slot_or_inline_move_to_inline)
-  [Function `move_to_storage_slot`](#0x1_storage_slot_or_inline_move_to_storage_slot)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/mem.md#0x1_mem">0x1::mem</a>;
<b>use</b> <a href="storage_slot.md#0x1_storage_slot">0x1::storage_slot</a>;
</code></pre>



<a id="0x1_storage_slot_or_inline_StorageSlotOrInline"></a>

## Enum `StorageSlotOrInline`



<pre><code>enum <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Inline</summary>


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
<summary>StorageSlot</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slot: <a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Transient</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_storage_slot_or_inline_Dummy"></a>

## Struct `Dummy`



<pre><code><b>struct</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_Dummy">Dummy</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE"></a>

StorageSlotOrInline found in inconsistent (transient) state, should never happen.


<pre><code><b>const</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE">ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE</a>: u64 = 1;
</code></pre>



<a id="0x1_storage_slot_or_inline_new_inline"></a>

## Function `new_inline`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_new_inline">new_inline</a>&lt;T: store&gt;(value: T): <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_new_inline">new_inline</a>&lt;T: store&gt;(value: T): <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt; {
    StorageSlotOrInline::Inline { value }
}
</code></pre>



</details>

<a id="0x1_storage_slot_or_inline_new_storage_slot"></a>

## Function `new_storage_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_new_storage_slot">new_storage_slot</a>&lt;T: store&gt;(value: T): <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_new_storage_slot">new_storage_slot</a>&lt;T: store&gt;(value: T): <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt; {
    StorageSlotOrInline::StorageSlot { slot: <a href="storage_slot.md#0x1_storage_slot_new">storage_slot::new</a>(value) }
}
</code></pre>



</details>

<a id="0x1_storage_slot_or_inline_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt;): &T {
    match (self) {
        StorageSlotOrInline::Inline { value } =&gt; value,
        StorageSlotOrInline::StorageSlot { slot } =&gt; slot.<a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_borrow">borrow</a>(),
        StorageSlotOrInline::Transient =&gt; <b>abort</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE">ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE</a>,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slot_or_inline_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt;): &<b>mut</b> T {
    match (self) {
        StorageSlotOrInline::Inline { value } =&gt; value,
        StorageSlotOrInline::StorageSlot { slot } =&gt; slot.<a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_borrow_mut">borrow_mut</a>(),
        StorageSlotOrInline::Transient =&gt; <b>abort</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE">ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE</a>,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slot_or_inline_destroy"></a>

## Function `destroy`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_destroy">destroy</a>&lt;T: store&gt;(self: <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_destroy">destroy</a>&lt;T: store&gt;(self: <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt;): T {
    match (self) {
        StorageSlotOrInline::Inline { value } =&gt; value,
        StorageSlotOrInline::StorageSlot { slot } =&gt; slot.<a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_destroy">destroy</a>(),
        StorageSlotOrInline::Transient =&gt; <b>abort</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE">ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE</a>,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slot_or_inline_move_to_inline"></a>

## Function `move_to_inline`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_move_to_inline">move_to_inline</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_move_to_inline">move_to_inline</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt;) {
    match (self) {
        StorageSlotOrInline::Inline { value: _ } =&gt; {},
        StorageSlotOrInline::StorageSlot { slot: _ } =&gt; {
            <b>let</b> StorageSlotOrInline::StorageSlot { slot } = <a href="../../aptos-stdlib/../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(self, StorageSlotOrInline::Transient);
            <b>let</b> StorageSlotOrInline::Transient = <a href="../../aptos-stdlib/../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(self, <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_new_inline">new_inline</a>(slot.<a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_destroy">destroy</a>()));
        },
        StorageSlotOrInline::Transient =&gt; <b>abort</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE">ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE</a>,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slot_or_inline_move_to_storage_slot"></a>

## Function `move_to_storage_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_move_to_storage_slot">move_to_storage_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">storage_slot_or_inline::StorageSlotOrInline</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_move_to_storage_slot">move_to_storage_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_StorageSlotOrInline">StorageSlotOrInline</a>&lt;T&gt;) {
    match (self) {
        StorageSlotOrInline::Inline { value: _ } =&gt; {
            <b>let</b> StorageSlotOrInline::Inline { value } = <a href="../../aptos-stdlib/../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(self, StorageSlotOrInline::Transient);
            <b>let</b> StorageSlotOrInline::Transient = <a href="../../aptos-stdlib/../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(self, <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_new_storage_slot">new_storage_slot</a>(value));
        },
        StorageSlotOrInline::StorageSlot { slot: _ } =&gt; {},
        StorageSlotOrInline::Transient =&gt; <b>abort</b> <a href="storage_slot_or_inline.md#0x1_storage_slot_or_inline_ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE">ESTORAGE_SLOT_INCORRECTLY_IN_TRANSIENT_STATE</a>,
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
