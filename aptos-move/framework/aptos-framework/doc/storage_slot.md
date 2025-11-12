
<a id="0x1_storage_slot"></a>

# Module `0x1::storage_slot`



-  [Resource `StorageSlotResource`](#0x1_storage_slot_StorageSlotResource)
-  [Struct `StorageSlot`](#0x1_storage_slot_StorageSlot)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_storage_slot_new)
-  [Function `borrow_storage_slot_resource`](#0x1_storage_slot_borrow_storage_slot_resource)
-  [Function `borrow_storage_slot_resource_mut`](#0x1_storage_slot_borrow_storage_slot_resource_mut)
-  [Function `borrow`](#0x1_storage_slot_borrow)
-  [Function `borrow_mut`](#0x1_storage_slot_borrow_mut)
-  [Function `copy_storage_slot`](#0x1_storage_slot_copy_storage_slot)
-  [Function `destroy`](#0x1_storage_slot_destroy)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_storage_slot_StorageSlotResource"></a>

## Resource `StorageSlotResource`



<pre><code><b>struct</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlotResource">StorageSlotResource</a>&lt;T&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>val: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_storage_slot_StorageSlot"></a>

## Struct `StorageSlot`



<pre><code><b>struct</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_storage_slot_ESTORAGE_SLOT_NATIVES_NOT_ENABLED"></a>

Storage slot natives are not enabled.


<pre><code><b>const</b> <a href="storage_slot.md#0x1_storage_slot_ESTORAGE_SLOT_NATIVES_NOT_ENABLED">ESTORAGE_SLOT_NATIVES_NOT_ENABLED</a>: u64 = 1;
</code></pre>



<a id="0x1_storage_slot_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_new">new</a>&lt;T: store&gt;(value: T): <a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_new">new</a>&lt;T: store&gt;(value: T): <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt; {
    <b>let</b> unique_signer = <a href="object.md#0x1_object_create_unique_onchain_signer">object::create_unique_onchain_signer</a>().generate_signer_for_extending();
    <b>move_to</b>(&unique_signer, <a href="storage_slot.md#0x1_storage_slot_StorageSlotResource">StorageSlotResource</a> { val: value });
    <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a> { addr: unique_signer.address_of() }
}
</code></pre>



</details>

<a id="0x1_storage_slot_borrow_storage_slot_resource"></a>

## Function `borrow_storage_slot_resource`



<pre><code><b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow_storage_slot_resource">borrow_storage_slot_resource</a>&lt;T: store, BR&gt;(self: &<a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;): &BR
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow_storage_slot_resource">borrow_storage_slot_resource</a>&lt;T: store, BR&gt;(self: &<a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt;): &BR;
</code></pre>



</details>

<a id="0x1_storage_slot_borrow_storage_slot_resource_mut"></a>

## Function `borrow_storage_slot_resource_mut`



<pre><code><b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow_storage_slot_resource_mut">borrow_storage_slot_resource_mut</a>&lt;T: store, BR&gt;(self: &<b>mut</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;): &<b>mut</b> BR
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow_storage_slot_resource_mut">borrow_storage_slot_resource_mut</a>&lt;T: store, BR&gt;(self: &<b>mut</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt;): &<b>mut</b> BR;
</code></pre>



</details>

<a id="0x1_storage_slot_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt;): &T {
    <b>assert</b>!(std::features::is_storage_slot_natives_enabled(), <a href="storage_slot.md#0x1_storage_slot_ESTORAGE_SLOT_NATIVES_NOT_ENABLED">ESTORAGE_SLOT_NATIVES_NOT_ENABLED</a>);
    &self.<a href="storage_slot.md#0x1_storage_slot_borrow_storage_slot_resource">borrow_storage_slot_resource</a>&lt;T, <a href="storage_slot.md#0x1_storage_slot_StorageSlotResource">StorageSlotResource</a>&lt;T&gt;&gt;().val
}
</code></pre>



</details>

<a id="0x1_storage_slot_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt;): &<b>mut</b> T {
    <b>assert</b>!(std::features::is_storage_slot_natives_enabled(), <a href="storage_slot.md#0x1_storage_slot_ESTORAGE_SLOT_NATIVES_NOT_ENABLED">ESTORAGE_SLOT_NATIVES_NOT_ENABLED</a>);
    &<b>mut</b> self.<a href="storage_slot.md#0x1_storage_slot_borrow_storage_slot_resource_mut">borrow_storage_slot_resource_mut</a>&lt;T, <a href="storage_slot.md#0x1_storage_slot_StorageSlotResource">StorageSlotResource</a>&lt;T&gt;&gt;().val
}
</code></pre>



</details>

<a id="0x1_storage_slot_copy_storage_slot"></a>

## Function `copy_storage_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_copy_storage_slot">copy_storage_slot</a>&lt;T: <b>copy</b>, store&gt;(self: &<a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;): <a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_copy_storage_slot">copy_storage_slot</a>&lt;T: store + <b>copy</b>&gt;(self: &<a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt;): <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt; {
    <a href="storage_slot.md#0x1_storage_slot_new">new</a>(*self.<a href="storage_slot.md#0x1_storage_slot_borrow">borrow</a>())
}
</code></pre>



</details>

<a id="0x1_storage_slot_destroy"></a>

## Function `destroy`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_destroy">destroy</a>&lt;T: store&gt;(self: <a href="storage_slot.md#0x1_storage_slot_StorageSlot">storage_slot::StorageSlot</a>&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slot.md#0x1_storage_slot_destroy">destroy</a>&lt;T: store&gt;(self: <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a>&lt;T&gt;): T {
    <b>let</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlot">StorageSlot</a> { addr } = self;
    <b>let</b> <a href="storage_slot.md#0x1_storage_slot_StorageSlotResource">StorageSlotResource</a> { val } = <b>move_from</b>&lt;<a href="storage_slot.md#0x1_storage_slot_StorageSlotResource">StorageSlotResource</a>&lt;T&gt;&gt;(addr);
    val
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
