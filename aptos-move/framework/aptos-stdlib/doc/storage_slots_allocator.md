
<a id="0x1_storage_slots_allocator"></a>

# Module `0x1::storage_slots_allocator`

Abstraction to having "addressable" storage slots (i.e. items) in global storage.
Addresses are local u64 values (unique within a single StorageSlotsAllocator instance,
but can and do overlap across instances).

Allows optionally to initialize slots (and pay for them upfront), and then reuse them,
providing predictable storage costs.

If we need to mutate multiple slots at the same time, we can workaround borrow_mut preventing us from that,
via provided pair of <code>remove_and_reserve</code> and <code>fill_reserved_slot</code> methods, to do so in non-conflicting manner.

Similarly allows getting an address upfront via <code>reserve_slot</code>, for a slot created
later (i.e. if we need address to initialize the value itself).

In the future, more sophisticated strategies can be added, without breaking/modifying callers,
for example:
* having a fee-payer for any storage creation operations


-  [Enum `Link`](#0x1_storage_slots_allocator_Link)
-  [Enum `StorageSlotsAllocatorConfig`](#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig)
-  [Enum `StorageSlotsAllocator`](#0x1_storage_slots_allocator_StorageSlotsAllocator)
-  [Struct `ReservedSlot`](#0x1_storage_slots_allocator_ReservedSlot)
-  [Struct `StoredSlot`](#0x1_storage_slots_allocator_StoredSlot)
-  [Struct `RefToSlot`](#0x1_storage_slots_allocator_RefToSlot)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_storage_slots_allocator_new)
-  [Function `new_default_config`](#0x1_storage_slots_allocator_new_default_config)
-  [Function `new_config`](#0x1_storage_slots_allocator_new_config)
-  [Function `add`](#0x1_storage_slots_allocator_add)
-  [Function `remove`](#0x1_storage_slots_allocator_remove)
-  [Function `destroy`](#0x1_storage_slots_allocator_destroy)
-  [Function `borrow`](#0x1_storage_slots_allocator_borrow)
-  [Function `borrow_mut`](#0x1_storage_slots_allocator_borrow_mut)
-  [Function `reserve_slot`](#0x1_storage_slots_allocator_reserve_slot)
-  [Function `fill_reserved_slot`](#0x1_storage_slots_allocator_fill_reserved_slot)
-  [Function `remove_and_reserve`](#0x1_storage_slots_allocator_remove_and_reserve)
-  [Function `free_reserved_slot`](#0x1_storage_slots_allocator_free_reserved_slot)
-  [Function `reserved_as_ref`](#0x1_storage_slots_allocator_reserved_as_ref)
-  [Function `stored_as_ref`](#0x1_storage_slots_allocator_stored_as_ref)
-  [Function `null_ref`](#0x1_storage_slots_allocator_null_ref)
-  [Function `special_ref`](#0x1_storage_slots_allocator_special_ref)
-  [Function `ref_is_null`](#0x1_storage_slots_allocator_ref_is_null)
-  [Function `maybe_pop_from_reuse_queue`](#0x1_storage_slots_allocator_maybe_pop_from_reuse_queue)
-  [Function `maybe_push_to_reuse_queue`](#0x1_storage_slots_allocator_maybe_push_to_reuse_queue)
-  [Function `next_slot_index`](#0x1_storage_slots_allocator_next_slot_index)
-  [Function `add_link`](#0x1_storage_slots_allocator_add_link)
-  [Function `remove_link`](#0x1_storage_slots_allocator_remove_link)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x1_storage_slots_allocator_Link"></a>

## Enum `Link`

Data stored in an individual slot


<pre><code>enum <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">Link</a>&lt;T: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Occupied</summary>


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
<summary>Vacant</summary>


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

<a id="0x1_storage_slots_allocator_StorageSlotsAllocatorConfig"></a>

## Enum `StorageSlotsAllocatorConfig`



<pre><code>enum <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">StorageSlotsAllocatorConfig</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>should_reuse: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>num_to_preallocate: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_storage_slots_allocator_StorageSlotsAllocator"></a>

## Enum `StorageSlotsAllocator`



<pre><code>enum <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slots: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">storage_slots_allocator::Link</a>&lt;T&gt;&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_slot_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>should_reuse: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>reuse_head_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>reuse_spare_count: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_storage_slots_allocator_ReservedSlot"></a>

## Struct `ReservedSlot`

Handle to a reserved slot within a transaction.
Not copy/drop/store-able, to guarantee reservation
is used or released within the transaction.


<pre><code><b>struct</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>
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

<a id="0x1_storage_slots_allocator_StoredSlot"></a>

## Struct `StoredSlot`

Ownership handle to a slot.
Not copy/drop-able to make sure slots are released when not needed,
and there is unique owner for each slot.


<pre><code><b>struct</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a> <b>has</b> store
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

<a id="0x1_storage_slots_allocator_RefToSlot"></a>

## Struct `RefToSlot`

(Weak) Reference to a slot.
We can have variety of <code><a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a></code>, but only a single <code><a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a></code>.
It is on the caller to make sure references are not used after slot is freed.


<pre><code><b>struct</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> <b>has</b> <b>copy</b>, drop, store
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


<a id="0x1_storage_slots_allocator_EINTERNAL_INVARIANT_BROKEN"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>: u64 = 7;
</code></pre>



<a id="0x1_storage_slots_allocator_EINVALID_ARGUMENT"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_EINVALID_ARGUMENT">EINVALID_ARGUMENT</a>: u64 = 1;
</code></pre>



<a id="0x1_storage_slots_allocator_FIRST_INDEX"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_FIRST_INDEX">FIRST_INDEX</a>: u64 = 10;
</code></pre>



<a id="0x1_storage_slots_allocator_NULL_INDEX"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>: u64 = 0;
</code></pre>



<a id="0x1_storage_slots_allocator_SPECIAL_SLOT_INDEX"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_SPECIAL_SLOT_INDEX">SPECIAL_SLOT_INDEX</a>: u64 = 1;
</code></pre>



<a id="0x1_storage_slots_allocator_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new">new</a>&lt;T: store&gt;(config: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">storage_slots_allocator::StorageSlotsAllocatorConfig</a>): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new">new</a>&lt;T: store&gt;(config: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">StorageSlotsAllocatorConfig</a>): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt; {
    <b>let</b> result = StorageSlotsAllocator::V1 {
        slots: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        new_slot_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_FIRST_INDEX">FIRST_INDEX</a>,
        should_reuse: config.should_reuse,
        reuse_head_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>,
        reuse_spare_count: 0,
    };

    for (i in 0..config.num_to_preallocate) {
        <b>let</b> slot_index = result.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>();
        result.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_push_to_reuse_queue">maybe_push_to_reuse_queue</a>(slot_index);
    };

    result
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_new_default_config"></a>

## Function `new_default_config`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_default_config">new_default_config</a>(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">storage_slots_allocator::StorageSlotsAllocatorConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_default_config">new_default_config</a>(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">StorageSlotsAllocatorConfig</a> {
    StorageSlotsAllocatorConfig::V1 {
        should_reuse: <b>false</b>,
        num_to_preallocate: 0,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_new_config"></a>

## Function `new_config`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_config">new_config</a>(should_reuse: bool, num_to_preallocate: u32): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">storage_slots_allocator::StorageSlotsAllocatorConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_config">new_config</a>(should_reuse: bool, num_to_preallocate: u32): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocatorConfig">StorageSlotsAllocatorConfig</a> {
    StorageSlotsAllocatorConfig::V1 {
        should_reuse,
        num_to_preallocate,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, val: T): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, val: T): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a> {
    <b>let</b> (stored_slot, reserved_slot) = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserve_slot">reserve_slot</a>();
    self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_fill_reserved_slot">fill_reserved_slot</a>(reserved_slot, val);
    stored_slot
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a>): T {
    <b>let</b> (reserved_slot, value) = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_and_reserve">remove_and_reserve</a>(slot.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_stored_as_ref">stored_as_ref</a>());
    self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_free_reserved_slot">free_reserved_slot</a>(reserved_slot, slot);
    value
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_destroy"></a>

## Function `destroy`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_destroy">destroy</a>&lt;T: store&gt;(self: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_destroy">destroy</a>&lt;T: store&gt;(self: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;) {
    <b>loop</b> {
        <b>let</b> reuse_index = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_pop_from_reuse_queue">maybe_pop_from_reuse_queue</a>();
        <b>if</b> (reuse_index == <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>) {
            <b>break</b>;
        };
    };
    match (self) {
        V1 {
            slots,
            new_slot_index: _,
            should_reuse: _,
            reuse_head_index,
            reuse_spare_count: _,
        } =&gt; {
            <b>assert</b>!(reuse_head_index == <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>);
            slots.destroy_some().<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_destroy">destroy</a>();
        },
    };
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a>): &T {
    &self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>().<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>(slot.slot_index).value
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a>): &<b>mut</b> T {
    &<b>mut</b> self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>().<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>(slot.slot_index).value
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_reserve_slot"></a>

## Function `reserve_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserve_slot">reserve_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;): (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserve_slot">reserve_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;): (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a>, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>) {
    <b>let</b> slot_index = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_pop_from_reuse_queue">maybe_pop_from_reuse_queue</a>();
    <b>if</b> (slot_index == <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>) {
        slot_index = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>();
    };

    (
        <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a> { slot_index },
        <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> { slot_index },
    )
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_fill_reserved_slot"></a>

## Function `fill_reserved_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_fill_reserved_slot">fill_reserved_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>, val: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_fill_reserved_slot">fill_reserved_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>, val: T) {
    <b>let</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> { slot_index } = slot;
    self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add_link">add_link</a>(slot_index, Link::Occupied { value: val });
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_remove_and_reserve"></a>

## Function `remove_and_reserve`

Remove storage slot, but reserve it for later.


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_and_reserve">remove_and_reserve</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>, T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_and_reserve">remove_and_reserve</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a>): (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>, T) {
    <b>let</b> slot_index = slot.slot_index;
    <b>let</b> Link::Occupied { value } = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_link">remove_link</a>(slot_index);
    (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> { slot_index }, value)
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_free_reserved_slot"></a>

## Function `free_reserved_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_free_reserved_slot">free_reserved_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, reserved_slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>, stored_slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_free_reserved_slot">free_reserved_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, reserved_slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>, stored_slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a>) {
    <b>let</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> { slot_index } = reserved_slot;
    <b>assert</b>!(slot_index == stored_slot.slot_index, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_EINVALID_ARGUMENT">EINVALID_ARGUMENT</a>);
    <b>let</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a> { slot_index: _ } = stored_slot;
    self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_push_to_reuse_queue">maybe_push_to_reuse_queue</a>(slot_index);
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_reserved_as_ref"></a>

## Function `reserved_as_ref`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserved_as_ref">reserved_as_ref</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserved_as_ref">reserved_as_ref</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> {
    <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> { slot_index: self.slot_index }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_stored_as_ref"></a>

## Function `stored_as_ref`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_stored_as_ref">stored_as_ref</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_stored_as_ref">stored_as_ref</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">StoredSlot</a>): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> {
    <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> { slot_index: self.slot_index }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_null_ref"></a>

## Function `null_ref`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">null_ref</a>(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">null_ref</a>(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> {
    <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> { slot_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a> }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_special_ref"></a>

## Function `special_ref`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_special_ref">special_ref</a>(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_special_ref">special_ref</a>(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> {
    <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a> { slot_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_SPECIAL_SLOT_INDEX">SPECIAL_SLOT_INDEX</a> }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_ref_is_null"></a>

## Function `ref_is_null`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ref_is_null">ref_is_null</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ref_is_null">ref_is_null</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">RefToSlot</a>): bool {
    self.slot_index == <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_maybe_pop_from_reuse_queue"></a>

## Function `maybe_pop_from_reuse_queue`



<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_pop_from_reuse_queue">maybe_pop_from_reuse_queue</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_pop_from_reuse_queue">maybe_pop_from_reuse_queue</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;): u64 {
    <b>let</b> slot_index = self.reuse_head_index;
    <b>if</b> (slot_index != <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>) {
        <b>let</b> Link::Vacant { next } = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_link">remove_link</a>(slot_index);
        self.reuse_head_index = next;
        self.reuse_spare_count = self.reuse_spare_count - 1;
    };
    slot_index
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_maybe_push_to_reuse_queue"></a>

## Function `maybe_push_to_reuse_queue`



<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_push_to_reuse_queue">maybe_push_to_reuse_queue</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_maybe_push_to_reuse_queue">maybe_push_to_reuse_queue</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64) {
    <b>if</b> (self.should_reuse) {
        self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add_link">add_link</a>(slot_index, Link::Vacant { next: self.reuse_head_index });
        self.reuse_head_index = slot_index;
        self.reuse_spare_count = self.reuse_spare_count + 1;
    };
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_next_slot_index"></a>

## Function `next_slot_index`



<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;): u64 {
    <b>let</b> slot_index = self.new_slot_index;
    self.new_slot_index = self.new_slot_index + 1;
    <b>if</b> (self.slots.is_none()) {
        self.slots.fill(<a href="table.md#0x1_table_new">table::new</a>&lt;u64, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">Link</a>&lt;T&gt;&gt;());
    };
    slot_index
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_add_link"></a>

## Function `add_link`



<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add_link">add_link</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64, link: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">storage_slots_allocator::Link</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add_link">add_link</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64, link: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">Link</a>&lt;T&gt;) {
    self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>().<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>(slot_index, link);
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_remove_link"></a>

## Function `remove_link`



<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_link">remove_link</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">storage_slots_allocator::Link</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_link">remove_link</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">Link</a>&lt;T&gt; {
    self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>().<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>(slot_index)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
