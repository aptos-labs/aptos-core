
<a id="0x1_storage_slots_allocator"></a>

# Module `0x1::storage_slots_allocator`

Abstraction to having "addressable" storage slots (i.e. items) in global storage.
Addresses are local u64 values (unique within a single StorageSlotsAllocator instance,
but can and do overlap across instances).

Allows optionally to initialize slots (and pay for them upfront), and then reuse them,
providing predictable storage costs.

If we need to mutate multiple slots at the same time, we can workaround borrow_mut preventing us from that,
via provided pair of <code>transiently_remove</code> and <code>add_transient_slot</code> methods, to do so in non-conflicting manner.

Similarly allows getting an address upfront via <code>create_transient_slot</code>, for a slot created
later (i.e. if we need address to initialize the value itself).

In the future, more sophisticated strategies can be added, without breaking/modifying callers,
for example:
* having one slot embeded into the struct itself
* having a fee-payer for any storage creation operations


-  [Enum `Link`](#0x1_storage_slots_allocator_Link)
-  [Enum `StorageSlotsAllocator`](#0x1_storage_slots_allocator_StorageSlotsAllocator)
-  [Struct `ReservedSlot`](#0x1_storage_slots_allocator_ReservedSlot)
-  [Constants](#@Constants_0)
-  [Function `new_storage_slots`](#0x1_storage_slots_allocator_new_storage_slots)
-  [Function `new_reuse_storage_slots`](#0x1_storage_slots_allocator_new_reuse_storage_slots)
-  [Function `add`](#0x1_storage_slots_allocator_add)
-  [Function `remove`](#0x1_storage_slots_allocator_remove)
-  [Function `destroy`](#0x1_storage_slots_allocator_destroy)
-  [Function `borrow`](#0x1_storage_slots_allocator_borrow)
-  [Function `borrow_mut`](#0x1_storage_slots_allocator_borrow_mut)
-  [Function `get_index`](#0x1_storage_slots_allocator_get_index)
-  [Function `reserve_slot`](#0x1_storage_slots_allocator_reserve_slot)
-  [Function `fill_reserved_slot`](#0x1_storage_slots_allocator_fill_reserved_slot)
-  [Function `remove_and_reserve`](#0x1_storage_slots_allocator_remove_and_reserve)
-  [Function `free_reserved_slot`](#0x1_storage_slots_allocator_free_reserved_slot)
-  [Function `push_to_reuse_queue_if_enabled`](#0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled)
-  [Function `next_slot_index`](#0x1_storage_slots_allocator_next_slot_index)


<pre><code><b>use</b> <a href="table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x1_storage_slots_allocator_Link"></a>

## Enum `Link`



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

<a id="0x1_storage_slots_allocator_StorageSlotsAllocator"></a>

## Enum `StorageSlotsAllocator`



<pre><code>enum <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Simple</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slots: <a href="table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">storage_slots_allocator::Link</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_slot_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Reuse</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slots: <a href="table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_Link">storage_slots_allocator::Link</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_slot_index: u64</code>
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

</details>

</details>

<a id="0x1_storage_slots_allocator_ReservedSlot"></a>

## Struct `ReservedSlot`



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_storage_slots_allocator_FIRST_INDEX"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_FIRST_INDEX">FIRST_INDEX</a>: u64 = 3;
</code></pre>



<a id="0x1_storage_slots_allocator_NULL_INDEX"></a>



<pre><code><b>const</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>: u64 = 0;
</code></pre>



<a id="0x1_storage_slots_allocator_new_storage_slots"></a>

## Function `new_storage_slots`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_storage_slots">new_storage_slots</a>&lt;T: store&gt;(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_storage_slots">new_storage_slots</a>&lt;T: store&gt;(): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt; {
    StorageSlotsAllocator::Simple {
        slots: <a href="table.md#0x1_table_new">table::new</a>(),
        new_slot_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_FIRST_INDEX">FIRST_INDEX</a>,
    }
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_new_reuse_storage_slots"></a>

## Function `new_reuse_storage_slots`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_reuse_storage_slots">new_reuse_storage_slots</a>&lt;T: store&gt;(num_to_preallocate: u64): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_reuse_storage_slots">new_reuse_storage_slots</a>&lt;T: store&gt;(num_to_preallocate: u64): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt; {
    <b>let</b> self = StorageSlotsAllocator::Reuse {
        slots: <a href="table.md#0x1_table_new">table::new</a>(),
        new_slot_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_FIRST_INDEX">FIRST_INDEX</a>,
        reuse_head_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>,
    };

    for (i in 0..num_to_preallocate) {
        <b>let</b> slot_index = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>();
        self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled">push_to_reuse_queue_if_enabled</a>(slot_index);
    };

    self
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, val: T): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, val: T): u64 {
    <b>if</b> (self is StorageSlotsAllocator::Reuse&lt;T&gt;) {
        <b>let</b> slot_index = self.reuse_head_index;
        <b>if</b> (slot_index != <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>) {
            <b>let</b> Link::Vacant { next } = self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>(slot_index);
            self.reuse_head_index = next;
            self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>(slot_index, Link::Occupied { value: val });
            <b>return</b> slot_index
        };
    };

    <b>let</b> slot_index = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>();
    self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>(slot_index, Link::Occupied { value: val });
    slot_index
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): T {
    <b>let</b> Link::Occupied { value } = self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>(slot_index);

    self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled">push_to_reuse_queue_if_enabled</a>(slot_index);

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
    match (self) {
        Simple {
            slots,
            new_slot_index: _,
        } =&gt; slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_destroy">destroy</a>(),
        Reuse {
            slots,
            new_slot_index: _,
            reuse_head_index,
        } =&gt; {
            <b>while</b> (reuse_head_index != <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>) {
                <b>let</b> Link::Vacant { next } = slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>(reuse_head_index);
                reuse_head_index = next;
            };
            slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_destroy">destroy</a>();
        },
    };
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): &T {
    &self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow">borrow</a>(slot_index).value
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): &<b>mut</b> T {
    &<b>mut</b> self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_borrow_mut">borrow_mut</a>(slot_index).value
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_get_index"></a>

## Function `get_index`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_get_index">get_index</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_get_index">get_index</a>(self: &<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>): u64 {
    self.slot_index
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_reserve_slot"></a>

## Function `reserve_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserve_slot">reserve_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_reserve_slot">reserve_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> {
    <b>if</b> (self is StorageSlotsAllocator::Reuse&lt;T&gt;) {
        <b>let</b> slot_index = self.reuse_head_index;
        <b>if</b> (slot_index != <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_NULL_INDEX">NULL_INDEX</a>) {
            <b>let</b> Link::Vacant { next } = self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>(slot_index);
            self.reuse_head_index = next;
            <b>return</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> {
                slot_index,
            };
        };
    };

    <b>let</b> slot_index = self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_next_slot_index">next_slot_index</a>();
    <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> {
        slot_index,
    }
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
    self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>(slot_index, Link::Occupied { value: val });
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_remove_and_reserve"></a>

## Function `remove_and_reserve`

Remove storage slot, but reserve it for later.


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_and_reserve">remove_and_reserve</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>, T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove_and_reserve">remove_and_reserve</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64): (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>, T) {
    <b>let</b> Link::Occupied { value } = self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_remove">remove</a>(slot_index);
    (<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> { slot_index }, value)
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_free_reserved_slot"></a>

## Function `free_reserved_slot`



<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_free_reserved_slot">free_reserved_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">storage_slots_allocator::ReservedSlot</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_free_reserved_slot">free_reserved_slot</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a>) {
    <b>let</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_ReservedSlot">ReservedSlot</a> { slot_index } = slot;
    self.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled">push_to_reuse_queue_if_enabled</a>(slot_index);
}
</code></pre>



</details>

<a id="0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled"></a>

## Function `push_to_reuse_queue_if_enabled`



<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled">push_to_reuse_queue_if_enabled</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_push_to_reuse_queue_if_enabled">push_to_reuse_queue_if_enabled</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">StorageSlotsAllocator</a>&lt;T&gt;, slot_index: u64) {
    <b>if</b> (self is StorageSlotsAllocator::Reuse&lt;T&gt;) {
        self.slots.<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_add">add</a>(slot_index, Link::Vacant { next: self.reuse_head_index });
        self.reuse_head_index = slot_index;
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
    slot_index
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
