
<a id="0x1_big_ordered_map"></a>

# Module `0x1::big_ordered_map`

This module provides an implementation for an big ordered map.
Big means that it is stored across multiple resources, and doesn't have an
upper limit on number of elements it can contain.

Keys point to values, and each key in the map must be unique.

Currently, one implementation is provided - BPlusTreeMap, backed by a B+Tree,
with each node being a separate resource, internally containing OrderedMap.

BPlusTreeMap is chosen since the biggest (performance and gast)
costs are reading resources, and it:
* reduces number of resource accesses
* reduces number of rebalancing operations, and makes each rebalancing
operation touch only few resources
* it allows for parallelism for keys that are not close to each other,
once it contains enough keys


TODO: all iterator functions are public(friend) for now, so that they can be modified in a
backward incompatible way.
They are waiting for Move improvement that will allow references to be part of the struct
Allowing cleaner iterator APIs


-  [Struct `Node`](#0x1_big_ordered_map_Node)
-  [Enum `Child`](#0x1_big_ordered_map_Child)
-  [Enum `Iterator`](#0x1_big_ordered_map_Iterator)
-  [Enum `BigOrderedMap`](#0x1_big_ordered_map_BigOrderedMap)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_big_ordered_map_new)
-  [Function `new_with_config`](#0x1_big_ordered_map_new_with_config)
-  [Function `destroy_empty`](#0x1_big_ordered_map_destroy_empty)
-  [Function `add`](#0x1_big_ordered_map_add)
-  [Function `upsert`](#0x1_big_ordered_map_upsert)
-  [Function `remove`](#0x1_big_ordered_map_remove)
-  [Function `lower_bound`](#0x1_big_ordered_map_lower_bound)
-  [Function `find`](#0x1_big_ordered_map_find)
-  [Function `contains`](#0x1_big_ordered_map_contains)
-  [Function `borrow`](#0x1_big_ordered_map_borrow)
-  [Function `borrow_mut`](#0x1_big_ordered_map_borrow_mut)
-  [Function `new_begin_iter`](#0x1_big_ordered_map_new_begin_iter)
-  [Function `new_end_iter`](#0x1_big_ordered_map_new_end_iter)
-  [Function `iter_is_begin`](#0x1_big_ordered_map_iter_is_begin)
-  [Function `iter_is_end`](#0x1_big_ordered_map_iter_is_end)
-  [Function `iter_get_key`](#0x1_big_ordered_map_iter_get_key)
-  [Function `iter_next`](#0x1_big_ordered_map_iter_next)
-  [Function `iter_prev`](#0x1_big_ordered_map_iter_prev)
-  [Function `borrow_node`](#0x1_big_ordered_map_borrow_node)
-  [Function `borrow_node_mut`](#0x1_big_ordered_map_borrow_node_mut)
-  [Function `add_or_upsert_impl`](#0x1_big_ordered_map_add_or_upsert_impl)
-  [Function `validate_dynamic_size_and_init_max_degrees`](#0x1_big_ordered_map_validate_dynamic_size_and_init_max_degrees)
-  [Function `validate_static_size_and_init_max_degrees`](#0x1_big_ordered_map_validate_static_size_and_init_max_degrees)
-  [Function `validate_size_and_init_max_degrees`](#0x1_big_ordered_map_validate_size_and_init_max_degrees)
-  [Function `destroy_inner_child`](#0x1_big_ordered_map_destroy_inner_child)
-  [Function `destroy_empty_node`](#0x1_big_ordered_map_destroy_empty_node)
-  [Function `new_node`](#0x1_big_ordered_map_new_node)
-  [Function `new_node_with_children`](#0x1_big_ordered_map_new_node_with_children)
-  [Function `new_inner_child`](#0x1_big_ordered_map_new_inner_child)
-  [Function `new_leaf_child`](#0x1_big_ordered_map_new_leaf_child)
-  [Function `new_iter`](#0x1_big_ordered_map_new_iter)
-  [Function `find_leaf`](#0x1_big_ordered_map_find_leaf)
-  [Function `find_leaf_with_path`](#0x1_big_ordered_map_find_leaf_with_path)
-  [Function `get_max_degree`](#0x1_big_ordered_map_get_max_degree)
-  [Function `add_at`](#0x1_big_ordered_map_add_at)
-  [Function `update_key`](#0x1_big_ordered_map_update_key)
-  [Function `remove_at`](#0x1_big_ordered_map_remove_at)
-  [Function `length`](#0x1_big_ordered_map_length)
-  [Function `length_for_node`](#0x1_big_ordered_map_length_for_node)
-  [Function `is_empty`](#0x1_big_ordered_map_is_empty)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../move-stdlib/doc/cmp.md#0x1_cmp">0x1::cmp</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../move-stdlib/doc/mem.md#0x1_mem">0x1::mem</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="ordered_map.md#0x1_ordered_map">0x1::ordered_map</a>;
<b>use</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator">0x1::storage_slots_allocator</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_big_ordered_map_Node"></a>

## Struct `Node`

A node of the BigOrderedMap.

Inner node will have all children be Child::Inner, pointing to the child nodes.
Leaf node will have all children be Child::Leaf.
Basically - Leaf node is a single-resource OrderedMap, containing as much keys as can fit.
So Leaf node contains multiple values, not just one.


<pre><code><b>struct</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K: store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>is_leaf: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>children: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>prev: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>next: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_big_ordered_map_Child"></a>

## Enum `Child`

The metadata of a child of a node.


<pre><code>enum <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Inner</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>node_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>Leaf</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: V</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_big_ordered_map_Iterator"></a>

## Enum `Iterator`

An iterator to iterate all keys in the BigOrderedMap.


<pre><code>enum <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>End</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Some</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>node_index: u64</code>
</dt>
<dd>
 The node index of the iterator pointing to.
</dd>
<dt>
<code>child_iter: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a></code>
</dt>
<dd>
 Child iter it is pointing to
</dd>
<dt>
<code>key: K</code>
</dt>
<dd>
 <code>key</code> to which <code>(node_index, child_iter)</code> are pointing to
 cache to not require borrowing global resources to fetch again
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_big_ordered_map_BigOrderedMap"></a>

## Enum `BigOrderedMap`

The BigOrderedMap data structure.


<pre><code>enum <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K: store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>BPlusTreeMap</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: <a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;</code>
</dt>
<dd>
 Root node. It is stored directly in the resource itself, unlike all other nodes.
</dd>
<dt>
<code>nodes: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>
 Storage of all non-root nodes. They are stored in separate storage slots.
</dd>
<dt>
<code>min_leaf_index: u64</code>
</dt>
<dd>
 The node index of the leftmost node.
</dd>
<dt>
<code>max_leaf_index: u64</code>
</dt>
<dd>
 The node index of the rightmost node.
</dd>
<dt>
<code>constant_kv_size: bool</code>
</dt>
<dd>
 Whether Key and Value have constant serialized size, and if so
 optimize out size checks on every insert, if so.
</dd>
<dt>
<code>inner_max_degree: u16</code>
</dt>
<dd>
 The max number of children an inner node can have.
</dd>
<dt>
<code>leaf_max_degree: u16</code>
</dt>
<dd>
 The max number of children a leaf node can have.
</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN"></a>

Internal errors.


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>: u64 = 20;
</code></pre>



<a id="0x1_big_ordered_map_NULL_INDEX"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>: u64 = 0;
</code></pre>



<a id="0x1_big_ordered_map_EITER_OUT_OF_BOUNDS"></a>

Trying to do an operation on an Iterator that would go out of bounds


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>: u64 = 3;
</code></pre>



<a id="0x1_big_ordered_map_EKEY_ALREADY_EXISTS"></a>

Map key already exists


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x1_big_ordered_map_EKEY_NOT_FOUND"></a>

Map key is not found


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>: u64 = 2;
</code></pre>



<a id="0x1_big_ordered_map_DEFAULT_INNER_MIN_DEGREE"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_INNER_MIN_DEGREE">DEFAULT_INNER_MIN_DEGREE</a>: u16 = 4;
</code></pre>



<a id="0x1_big_ordered_map_DEFAULT_LEAF_MIN_DEGREE"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_LEAF_MIN_DEGREE">DEFAULT_LEAF_MIN_DEGREE</a>: u16 = 3;
</code></pre>



<a id="0x1_big_ordered_map_DEFAULT_TARGET_NODE_SIZE"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_TARGET_NODE_SIZE">DEFAULT_TARGET_NODE_SIZE</a>: u64 = 4096;
</code></pre>



<a id="0x1_big_ordered_map_EARGUMENT_BYTES_TOO_LARGE"></a>

Trying to insert too large of an object into the mp.


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EARGUMENT_BYTES_TOO_LARGE">EARGUMENT_BYTES_TOO_LARGE</a>: u64 = 6;
</code></pre>



<a id="0x1_big_ordered_map_EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE"></a>

borrow_mut requires that key and value types have constant size
(otherwise it wouldn't be able to guarantee size requirements are not violated)


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE">EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE</a>: u64 = 7;
</code></pre>



<a id="0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER"></a>

The provided configuration parameter is invalid.


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER">EINVALID_CONFIG_PARAMETER</a>: u64 = 4;
</code></pre>



<a id="0x1_big_ordered_map_EMAP_NOT_EMPTY"></a>

Map isn't empty


<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EMAP_NOT_EMPTY">EMAP_NOT_EMPTY</a>: u64 = 5;
</code></pre>



<a id="0x1_big_ordered_map_MAX_DEGREE"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_MAX_DEGREE">MAX_DEGREE</a>: u64 = 4096;
</code></pre>



<a id="0x1_big_ordered_map_MAX_NODE_BYTES"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_MAX_NODE_BYTES">MAX_NODE_BYTES</a>: u64 = 204800;
</code></pre>



<a id="0x1_big_ordered_map_ROOT_INDEX"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>: u64 = 1;
</code></pre>



<a id="0x1_big_ordered_map_new"></a>

## Function `new`

Returns a new BigOrderedMap with the default configuration.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new">new</a>&lt;K: store, V: store&gt;(): <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new">new</a>&lt;K: store, V: store&gt;(): <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt; {
    <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">new_with_config</a>(0, 0, <b>false</b>, 0)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_with_config"></a>

## Function `new_with_config`

Returns a new BigOrderedMap with the provided max degree consts (the maximum # of children a node can have).
If 0 is passed, then it is dynamically computed based on size of first key and value.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">new_with_config</a>&lt;K: store, V: store&gt;(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool, num_to_preallocate: u32): <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_config">new_with_config</a>&lt;K: store, V: store&gt;(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool, num_to_preallocate: u32): <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt; {
    <b>assert</b>!(inner_max_degree == 0 || inner_max_degree &gt;= <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_INNER_MIN_DEGREE">DEFAULT_INNER_MIN_DEGREE</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER">EINVALID_CONFIG_PARAMETER</a>));
    <b>assert</b>!(leaf_max_degree == 0 || leaf_max_degree &gt;= <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_LEAF_MIN_DEGREE">DEFAULT_LEAF_MIN_DEGREE</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER">EINVALID_CONFIG_PARAMETER</a>));
    <b>assert</b>!(reuse_slots || num_to_preallocate == 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER">EINVALID_CONFIG_PARAMETER</a>));

    // Assert that <a href="storage_slots_allocator.md#0x1_storage_slots_allocator">storage_slots_allocator</a> special indices are aligned:
    <b>assert</b>!(<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_is_null_index">storage_slots_allocator::is_null_index</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
    <b>assert</b>!(<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_is_special_unused_index">storage_slots_allocator::is_special_unused_index</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

    <b>let</b> nodes = <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new">storage_slots_allocator::new</a>(<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_config">storage_slots_allocator::new_config</a>(reuse_slots, num_to_preallocate));

    <b>let</b> self = BigOrderedMap::BPlusTreeMap {
        root: <a href="big_ordered_map.md#0x1_big_ordered_map_new_node">new_node</a>(/*is_leaf=*/<b>true</b>),
        nodes: nodes,
        min_leaf_index: <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>,
        max_leaf_index: <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>,
        constant_kv_size: <b>false</b>, // Will be initialized in validate_static_size_and_init_max_degrees below.
        inner_max_degree: inner_max_degree,
        leaf_max_degree: leaf_max_degree
    };
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_validate_static_size_and_init_max_degrees">validate_static_size_and_init_max_degrees</a>();
    self
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_destroy_empty"></a>

## Function `destroy_empty`

Destroys the map if it's empty, otherwise aborts.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty">destroy_empty</a>&lt;K: store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty">destroy_empty</a>&lt;K: store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;) {
    <b>let</b> BigOrderedMap::BPlusTreeMap { root, nodes, min_leaf_index: _, max_leaf_index: _, constant_kv_size: _, inner_max_degree: _, leaf_max_degree: _ } = self;
    root.<a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty_node">destroy_empty_node</a>();
    // If root node is empty, then we know that no storage slots are used,
    // and so we can safely destroy all nodes.
    nodes.destroy_known_empty_unsafe();
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_add"></a>

## Function `add`

Inserts the key/value into the BigOrderedMap.
Aborts if the key is already in the map.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: K, value: V) {
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_add_or_upsert_impl">add_or_upsert_impl</a>(key, value, <b>false</b>).destroy_none()
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_upsert"></a>

## Function `upsert`

If the key doesn't exist in the map, inserts the key/value, and returns none.
Otherwise updates the value under the given key, and returns the old value.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_upsert">upsert</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: K, value: V): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_upsert">upsert</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: K, value: V): Option&lt;V&gt; {
    <b>let</b> result = self.<a href="big_ordered_map.md#0x1_big_ordered_map_add_or_upsert_impl">add_or_upsert_impl</a>(key, value, <b>true</b>);
    <b>if</b> (result.is_some()) {
        <b>let</b> Child::Leaf {
            value: old_value,
        } = result.destroy_some();
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(old_value)
    } <b>else</b> {
        result.destroy_none();
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_remove"></a>

## Function `remove`

Removes the entry from BigOrderedMap and returns the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): V {
    // Optimize case <b>where</b> only root node <b>exists</b>
    // (optimizes out borrowing and path creation in `find_leaf_with_path`)
    <b>if</b> (self.root.is_leaf) {
        <b>let</b> Child::Leaf {
            value,
        } = self.root.children.<a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>(key);
        <b>return</b> value;
    };

    <b>let</b> path_to_leaf = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>(key);

    <b>assert</b>!(!path_to_leaf.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));

    <b>let</b> Child::Leaf {
        value,
    } = self.<a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>(path_to_leaf, key);
    value
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_lower_bound"></a>

## Function `lower_bound`

Returns an iterator pointing to the first element that is greater or equal to the provided
key, or an end iterator if such element doesn't exist.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> leaf = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf">find_leaf</a>(key);
    <b>if</b> (leaf == <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
        <b>return</b> self.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>()
    };

    <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(leaf);
    <b>assert</b>!(node.is_leaf, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

    <b>let</b> child_lower_bound = node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>(key);
    <b>if</b> (child_lower_bound.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(&node.children)) {
        self.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>()
    } <b>else</b> {
        <b>let</b> iter_key = *child_lower_bound.iter_borrow_key(&node.children);
        <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(leaf, child_lower_bound, iter_key)
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_find"></a>

## Function `find`

Returns an iterator pointing to the element that equals to the provided key, or an end
iterator if the key is not found.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> lower_bound = self.<a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>(key);
    <b>if</b> (lower_bound.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        lower_bound
    } <b>else</b> <b>if</b> (&lower_bound.key == key) {
        lower_bound
    } <b>else</b> {
        self.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>()
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_contains"></a>

## Function `contains`

Returns true iff the key exists in the map.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_contains">contains</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_contains">contains</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): bool {
    <b>let</b> lower_bound = self.<a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>(key);
    <b>if</b> (lower_bound.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        <b>false</b>
    } <b>else</b> <b>if</b> (&lower_bound.key == key) {
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_borrow"></a>

## Function `borrow`

Returns a reference to the element with its key, aborts if the key is not found.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: K): &V {
    <b>let</b> iter = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>(&key);

    <b>assert</b>!(iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(self), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> children = &self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(iter.node_index).children;
    &iter.child_iter.iter_borrow(children).value
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_borrow_mut"></a>

## Function `borrow_mut`

Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: K): &<b>mut</b> V {
    <b>assert</b>!(self.constant_kv_size, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE">EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE</a>));
    <b>let</b> iter = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>(&key);

    <b>assert</b>!(iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(self), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> children = &<b>mut</b> self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(iter.node_index).children;
    &<b>mut</b> iter.child_iter.iter_borrow_mut(children).value
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_begin_iter"></a>

## Function `new_begin_iter`

Return the begin iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K: <b>copy</b>, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K: <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>if</b> (self.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
        <b>return</b> Iterator::End;
    };

    <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(self.min_leaf_index);
    <b>assert</b>!(!node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
    <b>let</b> begin_child_iter = node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>();
    <b>let</b> begin_child_key = *begin_child_iter.iter_borrow_key(&node.children);
    <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(self.min_leaf_index, begin_child_iter, begin_child_key)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_end_iter"></a>

## Function `new_end_iter`

Return the end iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>&lt;K: <b>copy</b>, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>&lt;K: <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    Iterator::End
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_is_begin"></a>

## Function `iter_is_begin`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): bool {
    <b>if</b> (self is Iterator::End&lt;K&gt;) {
        map.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()
    } <b>else</b> {
        (self.node_index == map.min_leaf_index && self.child_iter.iter_is_begin_from_non_empty())
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_is_end"></a>

## Function `iter_is_end`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, _map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, _map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): bool {
    self is Iterator::End&lt;K&gt;
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_get_key"></a>

## Function `iter_get_key`

Returns the key of the given iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_get_key">iter_get_key</a>&lt;K&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;): &K
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_get_key">iter_get_key</a>&lt;K&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;): &K {
    <b>assert</b>!(!(self is Iterator::End&lt;K&gt;), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));
    &self.key
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_next"></a>

## Function `iter_next`

Returns the next iterator, or none if already at the end iterator.
Requires the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_next">iter_next</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_next">iter_next</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>assert</b>!(!(self is Iterator::End&lt;K&gt;), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    <b>let</b> node_index = self.node_index;
    <b>let</b> node = map.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(node_index);

    <b>let</b> child_iter = self.child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_next">iter_next</a>(&node.children);
    <b>if</b> (!child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(&node.children)) {
        // next is in the same leaf node
        <b>let</b> iter_key = *child_iter.iter_borrow_key(&node.children);
        <b>return</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(node_index, child_iter, iter_key);
    };

    // next is in a different leaf node
    <b>let</b> next_index = node.next;
    <b>if</b> (next_index != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
        <b>let</b> next_node = map.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(next_index);

        <b>let</b> child_iter = next_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>();
        <b>assert</b>!(!child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(&next_node.children), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
        <b>let</b> iter_key = *child_iter.iter_borrow_key(&next_node.children);
        <b>return</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(next_index, child_iter, iter_key);
    };

    <a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>(map)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_prev"></a>

## Function `iter_prev`

Returns the previous iterator, or none if already at the begin iterator.
Requires the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> prev_index = <b>if</b> (self is Iterator::End&lt;K&gt;) {
        map.max_leaf_index
    } <b>else</b> {
        <b>let</b> node_index = self.node_index;
        <b>let</b> node = map.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(node_index);

        <b>if</b> (!self.child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_begin">iter_is_begin</a>(&node.children)) {
            // next is in the same leaf node
            <b>let</b> child_iter = self.child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(&node.children);
            <b>let</b> key = *child_iter.iter_borrow_key(&node.children);
            <b>return</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(node_index, child_iter, key);
        };
        node.prev
    };

    <b>assert</b>!(prev_index != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    // next is in a different leaf node
    <b>let</b> prev_node = map.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(prev_index);

    <b>let</b> prev_children = &prev_node.children;
    <b>let</b> child_iter = prev_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(prev_children);
    <b>let</b> iter_key = *child_iter.iter_borrow_key(prev_children);
    <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(prev_index, child_iter, iter_key)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_borrow_node"></a>

## Function `borrow_node`

Borrow a node, given an index. Works for both root (i.e. inline) node and separately stored nodes


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, node_index: u64): &<a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, node_index: u64): &<a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt; {
    <b>if</b> (node_index == <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>) {
        &self.root
    } <b>else</b> {
        self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>(node_index)
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_borrow_node_mut"></a>

## Function `borrow_node_mut`

Borrow a node mutably, given an index. Works for both root (i.e. inline) node and separately stored nodes


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, node_index: u64): &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, node_index: u64): &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt; {
    <b>if</b> (node_index == <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>) {
        &<b>mut</b> self.root
    } <b>else</b> {
        self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>(node_index)
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_add_or_upsert_impl"></a>

## Function `add_or_upsert_impl`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add_or_upsert_impl">add_or_upsert_impl</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: K, value: V, allow_overwrite: bool): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add_or_upsert_impl">add_or_upsert_impl</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: K, value: V, allow_overwrite: bool): Option&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;&gt; {
    <b>if</b> (!self.constant_kv_size) {
        self.<a href="big_ordered_map.md#0x1_big_ordered_map_validate_dynamic_size_and_init_max_degrees">validate_dynamic_size_and_init_max_degrees</a>(&key, &value);
    };

    // Optimize case <b>where</b> only root node <b>exists</b>
    // (optimizes out borrowing and path creation in `find_leaf_with_path`)
    <b>if</b> (self.root.is_leaf) {
        <b>let</b> children = &<b>mut</b> self.root.children;
        <b>let</b> current_size = children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>();

        <b>if</b> (current_size &lt; (self.leaf_max_degree <b>as</b> u64)) {
            <b>let</b> result = children.<a href="big_ordered_map.md#0x1_big_ordered_map_upsert">upsert</a>(key, <a href="big_ordered_map.md#0x1_big_ordered_map_new_leaf_child">new_leaf_child</a>(value));
            <b>assert</b>!(allow_overwrite || result.is_none(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
            <b>return</b> result;
        };
    };

    <b>let</b> path_to_leaf = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>(&key);

    <b>if</b> (path_to_leaf.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
        // In this case, the key is greater than all keys in the map.
        // So we need <b>to</b> <b>update</b> `key` in the pointers <b>to</b> the last child,
        // <b>to</b> maintain the <b>invariant</b> of `add_at`
        <b>let</b> current = <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>;

        <b>loop</b> {
            path_to_leaf.push_back(current);

            <b>let</b> current_node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(current);
            <b>if</b> (current_node.is_leaf) {
                <b>break</b>;
            };
            <b>let</b> last_value = current_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(&current_node.children).iter_remove(&<b>mut</b> current_node.children);
            current = last_value.node_index.stored_to_index();
            current_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(key, last_value);
        };
    };

    self.<a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>(path_to_leaf, key, <a href="big_ordered_map.md#0x1_big_ordered_map_new_leaf_child">new_leaf_child</a>(value), allow_overwrite)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_validate_dynamic_size_and_init_max_degrees"></a>

## Function `validate_dynamic_size_and_init_max_degrees`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_validate_dynamic_size_and_init_max_degrees">validate_dynamic_size_and_init_max_degrees</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K, value: &V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_validate_dynamic_size_and_init_max_degrees">validate_dynamic_size_and_init_max_degrees</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K, value: &V) {
    <b>let</b> key_size = <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_serialized_size">bcs::serialized_size</a>(key);
    <b>let</b> value_size = <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_serialized_size">bcs::serialized_size</a>(value);
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_validate_size_and_init_max_degrees">validate_size_and_init_max_degrees</a>(key_size, value_size)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_validate_static_size_and_init_max_degrees"></a>

## Function `validate_static_size_and_init_max_degrees`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_validate_static_size_and_init_max_degrees">validate_static_size_and_init_max_degrees</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_validate_static_size_and_init_max_degrees">validate_static_size_and_init_max_degrees</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;) {
    <b>let</b> key_size = <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_constant_serialized_size">bcs::constant_serialized_size</a>&lt;K&gt;();
    <b>let</b> value_size = <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_constant_serialized_size">bcs::constant_serialized_size</a>&lt;V&gt;();

    <b>if</b> (key_size.is_some() && value_size.is_some()) {
        self.<a href="big_ordered_map.md#0x1_big_ordered_map_validate_size_and_init_max_degrees">validate_size_and_init_max_degrees</a>(key_size.destroy_some(), value_size.destroy_some());
        self.constant_kv_size = <b>true</b>;
    };
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_validate_size_and_init_max_degrees"></a>

## Function `validate_size_and_init_max_degrees`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_validate_size_and_init_max_degrees">validate_size_and_init_max_degrees</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key_size: u64, value_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_validate_size_and_init_max_degrees">validate_size_and_init_max_degrees</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key_size: u64, value_size: u64) {
    <b>let</b> entry_size = key_size + value_size;

    <b>if</b> (self.inner_max_degree == 0) {
        self.inner_max_degree = max(<b>min</b>(<a href="big_ordered_map.md#0x1_big_ordered_map_MAX_DEGREE">MAX_DEGREE</a>, <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_TARGET_NODE_SIZE">DEFAULT_TARGET_NODE_SIZE</a> / key_size), <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_INNER_MIN_DEGREE">DEFAULT_INNER_MIN_DEGREE</a> <b>as</b> u64) <b>as</b> u16;
    };

    <b>if</b> (self.leaf_max_degree == 0) {
        self.leaf_max_degree = max(<b>min</b>(<a href="big_ordered_map.md#0x1_big_ordered_map_MAX_DEGREE">MAX_DEGREE</a>, <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_TARGET_NODE_SIZE">DEFAULT_TARGET_NODE_SIZE</a> / entry_size), <a href="big_ordered_map.md#0x1_big_ordered_map_DEFAULT_LEAF_MIN_DEGREE">DEFAULT_LEAF_MIN_DEGREE</a> <b>as</b> u64) <b>as</b> u16;
    };

    // Make sure that no nodes can exceed the upper size limit.
    <b>assert</b>!(key_size * (self.inner_max_degree <b>as</b> u64) &lt;= <a href="big_ordered_map.md#0x1_big_ordered_map_MAX_NODE_BYTES">MAX_NODE_BYTES</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EARGUMENT_BYTES_TOO_LARGE">EARGUMENT_BYTES_TOO_LARGE</a>));
    <b>assert</b>!(entry_size * (self.leaf_max_degree <b>as</b> u64) &lt;= <a href="big_ordered_map.md#0x1_big_ordered_map_MAX_NODE_BYTES">MAX_NODE_BYTES</a>, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EARGUMENT_BYTES_TOO_LARGE">EARGUMENT_BYTES_TOO_LARGE</a>));
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_destroy_inner_child"></a>

## Function `destroy_inner_child`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_inner_child">destroy_inner_child</a>&lt;V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_inner_child">destroy_inner_child</a>&lt;V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;): StoredSlot {
    <b>let</b> Child::Inner {
        node_index,
    } = self;

    node_index
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_destroy_empty_node"></a>

## Function `destroy_empty_node`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty_node">destroy_empty_node</a>&lt;K: store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty_node">destroy_empty_node</a>&lt;K: store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt;) {
    <b>let</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> { children, is_leaf: _, prev: _, next: _ } = self;
    <b>assert</b>!(children.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EMAP_NOT_EMPTY">EMAP_NOT_EMPTY</a>));
    children.<a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_node"></a>

## Function `new_node`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_node">new_node</a>&lt;K: store, V: store&gt;(is_leaf: bool): <a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_node">new_node</a>&lt;K: store, V: store&gt;(is_leaf: bool): <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt; {
    <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> {
        is_leaf: is_leaf,
        children: <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>(),
        prev: <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>,
        next: <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>,
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_node_with_children"></a>

## Function `new_node_with_children`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_node_with_children">new_node_with_children</a>&lt;K: store, V: store&gt;(is_leaf: bool, children: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_node_with_children">new_node_with_children</a>&lt;K: store, V: store&gt;(is_leaf: bool, children: OrderedMap&lt;K, <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt; {
    <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> {
        is_leaf: is_leaf,
        children: children,
        prev: <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>,
        next: <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>,
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_inner_child"></a>

## Function `new_inner_child`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_inner_child">new_inner_child</a>&lt;V: store&gt;(node_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StoredSlot">storage_slots_allocator::StoredSlot</a>): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_inner_child">new_inner_child</a>&lt;V: store&gt;(node_index: StoredSlot): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt; {
    Child::Inner {
        node_index: node_index,
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_leaf_child"></a>

## Function `new_leaf_child`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_leaf_child">new_leaf_child</a>&lt;V: store&gt;(value: V): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_leaf_child">new_leaf_child</a>&lt;V: store&gt;(value: V): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt; {
    Child::Leaf {
        value: value,
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_new_iter"></a>

## Function `new_iter`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>&lt;K&gt;(node_index: u64, child_iter: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, key: K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>&lt;K&gt;(node_index: u64, child_iter: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, key: K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    Iterator::Some {
        node_index: node_index,
        child_iter: child_iter,
        key: key,
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_find_leaf"></a>

## Function `find_leaf`

Find leaf where the given key would fall in.
So the largest leaf with it's <code>max_key &lt;= key</code>.
return NULL_INDEX if <code>key</code> is larger than any key currently stored in the map.


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf">find_leaf</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf">find_leaf</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): u64 {
    <b>let</b> current = <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>;
    <b>loop</b> {
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(current);
        <b>if</b> (node.is_leaf) {
            <b>return</b> current;
        };
        <b>let</b> children = &node.children;
        <b>let</b> child_iter = children.<a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>(key);
        <b>if</b> (child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(children)) {
            <b>return</b> <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>;
        } <b>else</b> {
            current = child_iter.iter_borrow(children).node_index.stored_to_index();
        };
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_find_leaf_with_path"></a>

## Function `find_leaf_with_path`

Find leaf where the given key would fall in.
So the largest leaf with it's <code>max_key &lt;= key</code>.
Return the path from root to that leaf (including the leaf itself)
Return empty path if <code>key</code> is larger than any key currently stored in the map.


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>let</b> vec = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    <b>let</b> current = <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>;
    <b>loop</b> {
        vec.push_back(current);

        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(current);
        <b>if</b> (node.is_leaf) {
            <b>return</b> vec;
        };
        <b>let</b> children = &node.children;
        <b>let</b> child_iter = children.<a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>(key);
        <b>if</b> (child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(children)) {
            <b>return</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
        } <b>else</b> {
            current = child_iter.iter_borrow(children).node_index.stored_to_index();
        };
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_get_max_degree"></a>

## Function `get_max_degree`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_get_max_degree">get_max_degree</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, leaf: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_get_max_degree">get_max_degree</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, leaf: bool): u64 {
    <b>if</b> (leaf) {
        self.leaf_max_degree <b>as</b> u64
    } <b>else</b> {
        self.inner_max_degree <b>as</b> u64
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_add_at"></a>

## Function `add_at`

Add a given child to a given node (last in the <code>path_to_node</code>), and update/rebalance the tree as necessary.
It is required that <code>key</code> pointers to the child node, on the <code>path_to_node</code> are greater or equal to the given key.
That means if we are adding a <code>key</code> larger than any currently existing in the map - we needed
to update <code>key</code> pointers on the <code>path_to_node</code> to include it, before calling this method.

If <code>allow_overwrite</code> is not set, function will abort if <code>key</code> is already present.


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, key: K, child: <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;, allow_overwrite: bool): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, key: K, child: <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;, allow_overwrite: bool): Option&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;&gt; {
    // Last node in the path is one <b>where</b> we need <b>to</b> add the child <b>to</b>.
    <b>let</b> node_index = path_to_node.pop_back();
    {
        // First check <b>if</b> we can perform this operation, without changing structure of the tree (i.e. without adding <a href="any.md#0x1_any">any</a> nodes).

        // For that we can just borrow the single node
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(node_index);
        <b>let</b> children = &<b>mut</b> node.children;
        <b>let</b> current_size = children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>();

        <b>let</b> max_degree = <b>if</b> (node.is_leaf) {
            self.leaf_max_degree <b>as</b> u64
        } <b>else</b> {
            self.inner_max_degree <b>as</b> u64
        };

        <b>if</b> (current_size &lt; max_degree) {
            // Adding a child <b>to</b> a current node doesn't exceed the size, so we can just do that.
            <b>let</b> result = children.<a href="big_ordered_map.md#0x1_big_ordered_map_upsert">upsert</a>(key, child);

            <b>if</b> (node.is_leaf) {
                <b>assert</b>!(allow_overwrite || result.is_none(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
                <b>return</b> result;
            } <b>else</b> {
                <b>assert</b>!(!allow_overwrite && result.is_none(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
                <b>return</b> result;
            };
        };

        // If we cannot add more nodes without exceeding the size,
        // but node <b>with</b> `key` already <b>exists</b>, we either need <b>to</b> replace or <b>abort</b>.
        <b>let</b> iter = children.<a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>(&key);
        <b>if</b> (!iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(children)) {
            <b>assert</b>!(node.is_leaf, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
            <b>assert</b>!(allow_overwrite, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));

            <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(iter.iter_replace(children, child));
        }
    };

    // # of children in the current node exceeds the threshold, need <b>to</b> split into two nodes.

    // If we are at the root, we need <b>to</b> <b>move</b> root node <b>to</b> become a child and have a new root node,
    // in order <b>to</b> be able <b>to</b> split the node on the level it is.
    <b>let</b> (reserved_slot, node) = <b>if</b> (node_index == <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>) {
        <b>assert</b>!(path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

        // Splitting root now, need <b>to</b> create a new root.
        // Since root is stored direclty in the resource, we will swap-in the new node there.
        <b>let</b> new_root_node = <a href="big_ordered_map.md#0x1_big_ordered_map_new_node">new_node</a>&lt;K, V&gt;(/*is_leaf=*/<b>false</b>);

        // Reserve a slot <b>where</b> the current root will be moved <b>to</b>.
        <b>let</b> (replacement_node_slot, replacement_node_reserved_slot) = self.nodes.reserve_slot();

        <b>let</b> max_key = {
            <b>let</b> root_children = &self.root.children;
            <b>let</b> max_key = *root_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(root_children).iter_borrow_key(root_children);
            // need <b>to</b> check <b>if</b> key is largest, <b>as</b> <b>invariant</b> is that "parent's pointers" have been updated,
            // but key itself can be larger than all previous ones.
            <b>if</b> (<a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&max_key, &key).is_lt()) {
                max_key = key;
            };
            max_key
        };
        // New root will have start <b>with</b> a single child - the existing root (which will be at replacement location).
        new_root_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(max_key, <a href="big_ordered_map.md#0x1_big_ordered_map_new_inner_child">new_inner_child</a>(replacement_node_slot));
        <b>let</b> node = <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(&<b>mut</b> self.root, new_root_node);

        // we moved the currently processing node one level down, so we need <b>to</b> <b>update</b> the path
        path_to_node.push_back(<a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>);

        <b>let</b> replacement_index = replacement_node_reserved_slot.reserved_to_index();
        <b>if</b> (node.is_leaf) {
            // replacement node is the only leaf, so we <b>update</b> the pointers:
            self.min_leaf_index = replacement_index;
            self.max_leaf_index = replacement_index;
        };
        (replacement_node_reserved_slot, node)
    } <b>else</b> {
        // In order <b>to</b> work on multiple nodes at the same time, we cannot borrow_mut, and need <b>to</b> be
        // remove_and_reserve existing node.
        <b>let</b> (cur_node_reserved_slot, node) = self.nodes.remove_and_reserve(node_index);
        (cur_node_reserved_slot, node)
    };

    // <b>move</b> node_index out of scope, <b>to</b> make sure we don't accidentally access it, <b>as</b> we are done <b>with</b> it.
    // (i.e. we should be using `reserved_slot` instead).
    <b>move</b> node_index;

    // Now we can perform the split at the current level, <b>as</b> we know we are not at the root level.
    <b>assert</b>!(!path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

    // Parent <b>has</b> a reference under max key <b>to</b> the current node, so existing index
    // needs <b>to</b> be the right node.
    // Since <a href="ordered_map.md#0x1_ordered_map_trim">ordered_map::trim</a> moves from the end (i.e. smaller keys stay),
    // we are going <b>to</b> put the contents of the current node on the left side,
    // and create a new right node.
    // So <b>if</b> we had before (node_index, node), we will change that <b>to</b> end up having:
    // (new_left_node_index, node trimmed off) and (node_index, new node <b>with</b> trimmed off children)
    //
    // So <b>let</b>'s rename variables cleanly:
    <b>let</b> right_node_reserved_slot = reserved_slot;
    <b>let</b> left_node = node;


    <b>let</b> is_leaf = left_node.is_leaf;
    <b>let</b> left_children = &<b>mut</b> left_node.children;

    <b>let</b> right_node_index = right_node_reserved_slot.reserved_to_index();
    <b>let</b> left_next = &<b>mut</b> left_node.next;
    <b>let</b> left_prev = &<b>mut</b> left_node.prev;

    // compute the target size for the left node:
    <b>let</b> max_degree = <b>if</b> (is_leaf) {
        self.leaf_max_degree <b>as</b> u64
    } <b>else</b> {
        self.inner_max_degree <b>as</b> u64
    };
    <b>let</b> target_size = (max_degree + 1) / 2;

    // Add child (which will exceed the size), and then trim off <b>to</b> create two sets of children of correct sizes.
    left_children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(key, child);
    <b>let</b> right_node_children = left_children.trim(target_size);

    <b>assert</b>!(left_children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() &lt;= max_degree, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
    <b>assert</b>!(right_node_children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() &lt;= max_degree, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

    <b>let</b> right_node = <a href="big_ordered_map.md#0x1_big_ordered_map_new_node_with_children">new_node_with_children</a>(is_leaf, right_node_children);

    <b>let</b> (left_node_slot, left_node_reserved_slot) = self.nodes.reserve_slot();
    <b>let</b> left_node_index = left_node_slot.stored_to_index();

    // right nodes next is the node that was next of the left (previous) node, and next of left node is the right node.
    right_node.next = *left_next;
    *left_next = right_node_index;

    // right nodes previous is current left node
    right_node.prev = left_node_index;
    // Since the previuosly used index is going <b>to</b> the right node, `prev` pointer of the next node is correct,
    // and we need <b>to</b> <b>update</b> next pointer of the previous node (<b>if</b> <b>exists</b>)
    <b>if</b> (*left_prev != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
        self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>(*left_prev).next = left_node_index;
    };
    // Otherwise, we were the smallest node on the level. <b>if</b> this is the leaf level, <b>update</b> the pointer.
    <b>if</b> (right_node_index == self.min_leaf_index) {
        self.min_leaf_index = left_node_index;
    };

    // Largest left key is the split key.
    <b>let</b> max_left_key = *left_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(left_children).iter_borrow_key(left_children);

    self.nodes.fill_reserved_slot(left_node_reserved_slot, left_node);
    self.nodes.fill_reserved_slot(right_node_reserved_slot, right_node);

    // Add new <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a> (i.e. pointer <b>to</b> the left node) in the parent.
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>(path_to_node, max_left_key, <a href="big_ordered_map.md#0x1_big_ordered_map_new_inner_child">new_inner_child</a>(left_node_slot), <b>false</b>).destroy_none();
    <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_update_key"></a>

## Function `update_key`

Given a path to node (excluding the node itself), which is currently stored under "old_key", update "old_key" to "new_key".


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, old_key: &K, new_key: K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, old_key: &K, new_key: K) {
    <b>while</b> (!path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
        <b>let</b> node_index = path_to_node.pop_back();
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(node_index);
        <b>let</b> children = &<b>mut</b> node.children;
        children.replace_key_inplace(old_key, new_key);

        // If we were not updating the largest child, we don't need <b>to</b> <b>continue</b>.
        <b>if</b> (children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children) != &new_key) {
            <b>return</b>
        };
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_remove_at"></a>

## Function `remove_at`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt; {
    // Last node in the path is one <b>where</b> we need <b>to</b> add the child <b>to</b>.
    <b>let</b> node_index = path_to_node.pop_back();
    <b>let</b> old_child = {
        // First check <b>if</b> we can perform this operation, without changing structure of the tree (i.e. without adding <a href="any.md#0x1_any">any</a> nodes).

        // For that we can just borrow the single node
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(node_index);

        <b>let</b> children = &<b>mut</b> node.children;
        <b>let</b> is_leaf = node.is_leaf;

        <b>let</b> old_child = children.<a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>(key);
        <b>if</b> (node_index == <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>) {
            // If current node is root, lower limit of max_degree/2 nodes doesn't <b>apply</b>.
            // So we can adjust internally

            <b>assert</b>!(path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

            <b>if</b> (!is_leaf && children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() == 1) {
                // If root is not leaf, but <b>has</b> a single child, promote only child <b>to</b> root,
                // and drop current root. Since root is stored directly in the resource, we
                // "<b>move</b>" the child into the root.

                <b>let</b> Child::Inner {
                    node_index: inner_child_index,
                } = children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_remove(children);

                <b>let</b> inner_child = self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>(inner_child_index);
                <b>if</b> (inner_child.is_leaf) {
                    self.min_leaf_index = <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>;
                    self.max_leaf_index = <a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>;
                };

                <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(&<b>mut</b> self.root, inner_child).<a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty_node">destroy_empty_node</a>();
            };
            <b>return</b> old_child;
        };

        <b>let</b> max_degree = <b>if</b> (is_leaf) {
            self.leaf_max_degree <b>as</b> u64
        } <b>else</b> {
            self.inner_max_degree <b>as</b> u64
        };
        <b>let</b> current_size = children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>();

        // See <b>if</b> the node is big enough, or we need <b>to</b> merge it <b>with</b> another node on this level.
        <b>let</b> big_enough = current_size * 2 &gt;= max_degree;

        <b>let</b> new_max_key = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);

        // See <b>if</b> max key was updated for the current node, and <b>if</b> so - <b>update</b> it on the path.
        <b>let</b> max_key_updated = <a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&new_max_key, key).is_lt();
        <b>if</b> (max_key_updated) {
            <b>assert</b>!(current_size &gt;= 1, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

            self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, key, new_max_key);
        };

        // If node is big enough after removal, we are done.
        <b>if</b> (big_enough) {
            <b>return</b> old_child;
        };

        old_child
    };

    // Children size is below threshold, we need <b>to</b> rebalance <b>with</b> a neighbor on the same level.

    // In order <b>to</b> work on multiple nodes at the same time, we cannot borrow_mut, and need <b>to</b> be
    // remove_and_reserve existing node.
    <b>let</b> (node_slot, node) = self.nodes.remove_and_reserve(node_index);

    <b>let</b> is_leaf = node.is_leaf;
    <b>let</b> max_degree = self.<a href="big_ordered_map.md#0x1_big_ordered_map_get_max_degree">get_max_degree</a>(is_leaf);
    <b>let</b> prev = node.prev;
    <b>let</b> next = node.next;

    // index of the node we will rebalance <b>with</b>.
    <b>let</b> sibling_index = {
        <b>let</b> parent_children = &self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(*path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>(path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() - 1)).children;
        <b>assert</b>!(parent_children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() &gt;= 2, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
        // We merge <b>with</b> previous node - <b>if</b> it <b>has</b> the same parent, otherwise <b>with</b> next node (which then needs <b>to</b> have the same parent)
        <b>if</b> (parent_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(parent_children).iter_borrow(parent_children).node_index.stored_to_index() == node_index) {
            prev
        } <b>else</b> {
            next
        }
    };

    <b>let</b> children = &<b>mut</b> node.children;

    <b>let</b> (sibling_slot, sibling_node) = self.nodes.remove_and_reserve(sibling_index);
    <b>let</b> sibling_children = &<b>mut</b> sibling_node.children;

    <b>if</b> ((sibling_children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() - 1) * 2 &gt;= max_degree) {
        // The sibling node <b>has</b> enough elements, we can just borrow an element from the sibling node.
        <b>if</b> (sibling_index == next) {
            // <b>if</b> sibling is a larger node, we remove a child from the start
            <b>let</b> old_max_key = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);
            <b>let</b> sibling_begin_iter = sibling_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>();
            <b>let</b> borrowed_max_key = *sibling_begin_iter.iter_borrow_key(sibling_children);
            <b>let</b> borrowed_element = sibling_begin_iter.iter_remove(sibling_children);

            children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(borrowed_max_key, borrowed_element);

            // max_key of the current node changed, so <b>update</b>
            self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, &old_max_key, borrowed_max_key);
        } <b>else</b> {
            // <b>if</b> sibling is a smaller node, we remove a child from the end
            <b>let</b> sibling_end_iter = sibling_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(sibling_children);
            <b>let</b> borrowed_max_key = *sibling_end_iter.iter_borrow_key(sibling_children);
            <b>let</b> borrowed_element = sibling_end_iter.iter_remove(sibling_children);

            children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(borrowed_max_key, borrowed_element);

            // max_key of the sibling node changed, so <b>update</b>
            self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, &borrowed_max_key, *sibling_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(sibling_children).iter_borrow_key(sibling_children));
        };

        self.nodes.fill_reserved_slot(node_slot, node);
        self.nodes.fill_reserved_slot(sibling_slot, sibling_node);
        <b>return</b> old_child;
    };

    // The sibling node doesn't have enough elements <b>to</b> borrow, merge <b>with</b> the sibling node.
    // Keep the slot of the larger node of the two, <b>to</b> not require updating key on the parent nodes.
    // But append <b>to</b> the smaller node, <b>as</b> <a href="ordered_map.md#0x1_ordered_map_append">ordered_map::append</a> is more efficient when adding <b>to</b> the end.
    <b>let</b> (key_to_remove, reserved_slot_to_remove) = <b>if</b> (sibling_index == next) {
        // destroying larger sibling node, keeping sibling_slot.
        <b>let</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> { children: sibling_children, is_leaf: _, prev: _, next: sibling_next } = sibling_node;
        <b>let</b> key_to_remove = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);
        children.append(sibling_children);
        node.next = sibling_next;

        <b>if</b> (node.next != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
            <b>assert</b>!(self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>(node.next).prev == sibling_index, 1);
        };

        // we are removing node_index, which previous's node's next was pointing <b>to</b>,
        // so <b>update</b> the pointer
        <b>if</b> (node.prev != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
            self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>(node.prev).next = sibling_index;
        };
        // Otherwise, we were the smallest node on the level. <b>if</b> this is the leaf level, <b>update</b> the pointer.
        <b>if</b> (self.min_leaf_index == node_index) {
            self.min_leaf_index = sibling_index;
        };

        self.nodes.fill_reserved_slot(sibling_slot, node);

        (key_to_remove, node_slot)
    } <b>else</b> {
        // destroying larger current node, keeping node_slot
        <b>let</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> { children: node_children, is_leaf: _, prev: _, next: node_next } = node;
        <b>let</b> key_to_remove = *sibling_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(sibling_children).iter_borrow_key(sibling_children);
        sibling_children.append(node_children);
        sibling_node.next = node_next;

        <b>if</b> (sibling_node.next != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
            <b>assert</b>!(self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>(sibling_node.next).prev == node_index, 1);
        };
        // we are removing sibling node_index, which previous's node's next was pointing <b>to</b>,
        // so <b>update</b> the pointer
        <b>if</b> (sibling_node.prev != <a href="big_ordered_map.md#0x1_big_ordered_map_NULL_INDEX">NULL_INDEX</a>) {
            self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_mut">borrow_mut</a>(sibling_node.prev).next = node_index;
        };
        // Otherwise, sibling was the smallest node on the level. <b>if</b> this is the leaf level, <b>update</b> the pointer.
        <b>if</b> (self.min_leaf_index == sibling_index) {
            self.min_leaf_index = node_index;
        };

        self.nodes.fill_reserved_slot(node_slot, sibling_node);

        (key_to_remove, sibling_slot)
    };

    <b>assert</b>!(!path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
    <b>let</b> slot_to_remove = <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_inner_child">destroy_inner_child</a>(self.<a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>(path_to_node, &key_to_remove));
    self.nodes.free_reserved_slot(reserved_slot_to_remove, slot_to_remove);

    old_child
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_length"></a>

## Function `length`

Returns the number of elements in the BigOrderedMap.


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): u64 {
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_ROOT_INDEX">ROOT_INDEX</a>)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_length_for_node"></a>

## Function `length_for_node`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, node_index: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, node_index: u64): u64 {
    <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(node_index);
    <b>if</b> (node.is_leaf) {
        node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>()
    } <b>else</b> {
        <b>let</b> size = 0;

        node.children.for_each_ref(|_key, child| {
            size = size + self.<a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>(child.node_index.stored_to_index());
        });
        size
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_is_empty"></a>

## Function `is_empty`

Returns true iff the BigOrderedMap is empty.


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): bool {
    <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(self.min_leaf_index);

    node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
