
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
<code>prev: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a></code>
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
<code>node_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a></code>
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

</dd>
<dt>
<code>root_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a></code>
</dt>
<dd>
 The node index of the root node.
</dd>
<dt>
<code>nodes: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_StorageSlotsAllocator">storage_slots_allocator::StorageSlotsAllocator</a>&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>
 Mapping of node_index -> node.
</dd>
<dt>
<code>min_leaf_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a></code>
</dt>
<dd>
 The node index of the leftmost node.
</dd>
<dt>
<code>max_leaf_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a></code>
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



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>: u64 = 7;
</code></pre>



<a id="0x1_big_ordered_map_EITER_OUT_OF_BOUNDS"></a>



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



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EARGUMENT_BYTES_TOO_LARGE">EARGUMENT_BYTES_TOO_LARGE</a>: u64 = 6;
</code></pre>



<a id="0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EINVALID_CONFIG_PARAMETER">EINVALID_CONFIG_PARAMETER</a>: u64 = 4;
</code></pre>



<a id="0x1_big_ordered_map_EMAP_NOT_EMPTY"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_EMAP_NOT_EMPTY">EMAP_NOT_EMPTY</a>: u64 = 5;
</code></pre>



<a id="0x1_big_ordered_map_MAX_DEGREE"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_MAX_DEGREE">MAX_DEGREE</a>: u64 = 4096;
</code></pre>



<a id="0x1_big_ordered_map_MAX_NODE_BYTES"></a>



<pre><code><b>const</b> <a href="big_ordered_map.md#0x1_big_ordered_map_MAX_NODE_BYTES">MAX_NODE_BYTES</a>: u64 = 204800;
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

    <b>let</b> nodes = <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new">storage_slots_allocator::new</a>(<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_new_config">storage_slots_allocator::new_config</a>(reuse_slots, num_to_preallocate));

    <b>let</b> root_ref = <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_special_ref">storage_slots_allocator::special_ref</a>();
    <b>let</b> self = BigOrderedMap::BPlusTreeMap {
        root: <a href="big_ordered_map.md#0x1_big_ordered_map_new_node">new_node</a>(/*is_leaf=*/<b>true</b>),
        root_index: root_ref,
        nodes: nodes,
        min_leaf_index: root_ref,
        max_leaf_index: root_ref,
        constant_kv_size: <b>false</b>,
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
    <b>let</b> BigOrderedMap::BPlusTreeMap { root, nodes, root_index: _, min_leaf_index: _, max_leaf_index: _, constant_kv_size: _, inner_max_degree: _, leaf_max_degree: _ } = self;
    root.<a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty_node">destroy_empty_node</a>();
    nodes.destroy();
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


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> leaf = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf">find_leaf</a>(key);
    <b>if</b> (leaf.ref_is_null()) {
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


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
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

<a id="0x1_big_ordered_map_new_begin_iter"></a>

## Function `new_begin_iter`

Return the begin iterator.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K: <b>copy</b>, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K: <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
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


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>&lt;K: <b>copy</b>, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>&lt;K: <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    Iterator::End
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_is_begin"></a>

## Function `iter_is_begin`



<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): bool {
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



<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, _map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, _map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): bool {
    self is Iterator::End&lt;K&gt;
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_get_key"></a>

## Function `iter_get_key`

Returns the key of the given iterator.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_get_key">iter_get_key</a>&lt;K&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;): &K
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_get_key">iter_get_key</a>&lt;K&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;): &K {
    <b>assert</b>!(!(self is Iterator::End&lt;K&gt;), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));
    &self.key
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_iter_next"></a>

## Function `iter_next`

Returns the next iterator, or none if already at the end iterator.
Requires the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_next">iter_next</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_next">iter_next</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>assert</b>!(!(self is Iterator::End&lt;K&gt;), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    <b>let</b> node_index = self.node_index;
    <b>let</b> node = map.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(node_index);

    <b>let</b> child_iter = self.child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_next">iter_next</a>(&node.children);
    <b>if</b> (!child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(&node.children)) {
        <b>let</b> iter_key = *child_iter.iter_borrow_key(&node.children);
        <b>return</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(node_index, child_iter, iter_key);
    };

    <b>let</b> next_index = node.next;
    <b>if</b> (!next_index.ref_is_null()) {
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


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt;, map: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> prev_index = <b>if</b> (self is Iterator::End&lt;K&gt;) {
        map.max_leaf_index
    } <b>else</b> {
        <b>let</b> node_index = self.node_index;
        <b>let</b> node = map.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(node_index);

        <b>if</b> (!self.child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_begin">iter_is_begin</a>(&node.children)) {
            <b>let</b> child_iter = self.child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(&node.children);
            <b>let</b> key = *child_iter.iter_borrow_key(&node.children);
            <b>return</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>(node_index, child_iter, key);
        };
        node.prev
    };

    <b>assert</b>!(!prev_index.ref_is_null(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

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



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, node: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): &<a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, node: RefToSlot): &<a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt; {
    <b>if</b> (self.root_index == node) {
        &self.root
    } <b>else</b> {
        self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>(node)
    }
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_borrow_node_mut"></a>

## Function `borrow_node_mut`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, node: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">big_ordered_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>&lt;K: store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, node: RefToSlot): &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a>&lt;K, V&gt; {
    <b>if</b> (self.root_index == node) {
        &<b>mut</b> self.root
    } <b>else</b> {
        self.nodes.borrow_mut(node)
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

    <b>let</b> path_to_leaf = self.<a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>(&key);

    <b>if</b> (path_to_leaf.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
        // In this case, the key is greater than all keys in the map.

        <b>let</b> current = self.root_index;

        <b>loop</b> {
            path_to_leaf.push_back(current);

            <b>let</b> current_node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(current);
            <b>if</b> (current_node.is_leaf) {
                <b>break</b>;
            };
            <b>let</b> last_value = current_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(&current_node.children).iter_remove(&<b>mut</b> current_node.children);
            current = last_value.node_index.stored_as_ref();
            current_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(key, last_value);
        };
    };

    // aptos_std::debug::print(&std::string::utf8(b"add_or_upsert_impl::path_to_leaf"));
    // aptos_std::debug::print(&path_to_leaf);
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
        prev: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">storage_slots_allocator::null_ref</a>(),
        next: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">storage_slots_allocator::null_ref</a>(),
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
        prev: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">storage_slots_allocator::null_ref</a>(),
        next: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">storage_slots_allocator::null_ref</a>(),
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



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>&lt;K&gt;(node_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>, child_iter: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, key: K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">big_ordered_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_new_iter">new_iter</a>&lt;K&gt;(node_index: RefToSlot, child_iter: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, key: K): <a href="big_ordered_map.md#0x1_big_ordered_map_Iterator">Iterator</a>&lt;K&gt; {
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



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf">find_leaf</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf">find_leaf</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): RefToSlot {
    <b>let</b> current = self.root_index;
    <b>while</b> (!current.ref_is_null()) {
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(current);
        <b>if</b> (node.is_leaf) {
            <b>return</b> current;
        };
        <b>let</b> children = &node.children;
        <b>let</b> child_iter = children.<a href="big_ordered_map.md#0x1_big_ordered_map_lower_bound">lower_bound</a>(key);
        <b>if</b> (child_iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(children)) {
            <b>return</b> <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">storage_slots_allocator::null_ref</a>();
        } <b>else</b> {
            current = child_iter.iter_borrow(children).node_index.stored_as_ref();
        }
    };

    <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_null_ref">storage_slots_allocator::null_ref</a>()
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_find_leaf_with_path"></a>

## Function `find_leaf_with_path`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_find_leaf_with_path">find_leaf_with_path</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RefToSlot&gt; {
    <b>let</b> vec = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    <b>let</b> current = self.root_index;
    <b>while</b> (!current.ref_is_null()) {
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
            current = child_iter.iter_borrow(children).node_index.stored_as_ref();
        }
    };

    <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>)
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



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>&gt;, key: K, child: <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;, allow_overwrite: bool): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RefToSlot&gt;, key: K, child: <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;, allow_overwrite: bool): Option&lt;<a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt;&gt; {
    <b>let</b> node_index = path_to_node.pop_back();
    {
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(node_index);
        <b>let</b> children = &<b>mut</b> node.children;
        <b>let</b> current_size = children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>();

        <b>let</b> max_degree = <b>if</b> (node.is_leaf) {
            self.leaf_max_degree <b>as</b> u64
        } <b>else</b> {
            self.inner_max_degree <b>as</b> u64
        };

        <b>if</b> (current_size &lt; max_degree) {
            <b>let</b> result = children.<a href="big_ordered_map.md#0x1_big_ordered_map_upsert">upsert</a>(key, child);

            <b>if</b> (node.is_leaf) {
                <b>assert</b>!(allow_overwrite || result.is_none(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
                <b>return</b> result;
            } <b>else</b> {
                <b>assert</b>!(!allow_overwrite && result.is_none(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
                <b>return</b> result;
            };
        };

        <b>if</b> (allow_overwrite) {
            <b>let</b> iter = children.<a href="big_ordered_map.md#0x1_big_ordered_map_find">find</a>(&key);
            <b>if</b> (!iter.<a href="big_ordered_map.md#0x1_big_ordered_map_iter_is_end">iter_is_end</a>(children)) {
                <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(iter.iter_replace(children, child));
            }
        }
    };

    // # of children in the current node exceeds the threshold, need <b>to</b> split into two nodes.

    <b>let</b> (right_node_slot, node) = <b>if</b> (path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
        // If we are at the root, we need <b>to</b> <b>move</b> root node <b>to</b> become a child and have a new root node.

        <b>assert</b>!(node_index == self.root_index, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
        // aptos_std::debug::print(&std::string::utf8(b"changing root"));

        // Splitting root now, need <b>to</b> create a new root.
        // We keep root_index always the same
        <b>let</b> new_root_node = <a href="big_ordered_map.md#0x1_big_ordered_map_new_node">new_node</a>&lt;K, V&gt;(/*is_leaf=*/<b>false</b>);

        <b>let</b> (replacement_node_stored_slot, replacement_node_slot) = self.nodes.reserve_slot();
        // aptos_std::debug::print(&replacement_node_slot);

        <b>let</b> root_children = &self.root.children;
        <b>let</b> max_element = *root_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(root_children).iter_borrow_key(root_children);
        <b>if</b> (<a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&max_element, &key).is_less_than()) {
            max_element = key;
        };
        new_root_node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(max_element, <a href="big_ordered_map.md#0x1_big_ordered_map_new_inner_child">new_inner_child</a>(replacement_node_stored_slot));

        // aptos_std::debug::print(&cur_node_slot);
        path_to_node.push_back(self.root_index);

        <b>let</b> node = <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(&<b>mut</b> self.root, new_root_node);

        <b>let</b> replacement_ref = replacement_node_slot.reserved_as_ref();
        <b>if</b> (node.is_leaf) {
            self.min_leaf_index = replacement_ref;
            self.max_leaf_index = replacement_ref;
        };
        (replacement_node_slot, node)
    } <b>else</b> {
        <b>let</b> (cur_node_slot, node) = self.nodes.remove_and_reserve(node_index);
        (cur_node_slot, node)
    };

    // aptos_std::debug::print(&std::string::utf8(b"node that needs <b>to</b> be split"));
    // aptos_std::debug::print(&node);

    <b>move</b> node_index;
    <b>let</b> is_leaf = node.is_leaf;
    <b>let</b> children = &<b>mut</b> node.children;

    <b>let</b> right_node_ref = right_node_slot.reserved_as_ref();
    <b>let</b> next = &<b>mut</b> node.next;
    <b>let</b> prev = &<b>mut</b> node.prev;

    <b>let</b> max_degree = <b>if</b> (is_leaf) {
        self.leaf_max_degree <b>as</b> u64
    } <b>else</b> {
        self.inner_max_degree <b>as</b> u64
    };
    <b>let</b> target_size = (max_degree + 1) / 2;

    children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(key, child);
    <b>let</b> new_node_children = children.trim(target_size);

    <b>assert</b>!(children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() &lt;= max_degree, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
    <b>assert</b>!(new_node_children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() &lt;= max_degree, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

    <b>let</b> right_node = <a href="big_ordered_map.md#0x1_big_ordered_map_new_node_with_children">new_node_with_children</a>(is_leaf, new_node_children);

    <b>let</b> (left_node_stored_slot, left_node_slot) = self.nodes.reserve_slot();
    <b>let</b> left_node_ref = left_node_stored_slot.stored_as_ref();
    right_node.next = *next;
    *next = right_node_ref;
    right_node.prev = left_node_ref;
    <b>if</b> (!prev.ref_is_null()) {
        self.nodes.borrow_mut(*prev).next = left_node_ref;
    };

    <b>let</b> split_key = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);

    // aptos_std::debug::print(&std::string::utf8(b"creating right node"));
    // aptos_std::debug::print(&right_node_slot);
    // aptos_std::debug::print(&right_node);

    // aptos_std::debug::print(&std::string::utf8(b"updating left node"));
    // aptos_std::debug::print(&left_node_slot);
    // aptos_std::debug::print(&node);

    self.nodes.fill_reserved_slot(left_node_slot, node);
    self.nodes.fill_reserved_slot(right_node_slot, right_node);

    <b>if</b> (right_node_ref == self.min_leaf_index) {
        self.min_leaf_index = left_node_ref;
    };
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_add_at">add_at</a>(path_to_node, split_key, <a href="big_ordered_map.md#0x1_big_ordered_map_new_inner_child">new_inner_child</a>(left_node_stored_slot), <b>false</b>).destroy_none();
    <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_update_key"></a>

## Function `update_key`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>&gt;, old_key: &K, new_key: K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RefToSlot&gt;, old_key: &K, new_key: K) {
    <b>if</b> (path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
        <b>return</b>
    };

    <b>let</b> node_index = path_to_node.pop_back();
    <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(node_index);
    <b>let</b> children = &<b>mut</b> node.children;
    children.replace_key_inplace(old_key, new_key);

    <b>if</b> (children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children) == &new_key) {
        self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, old_key, new_key);
    };
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_remove_at"></a>

## Function `remove_at`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">big_ordered_map::Child</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(self: &<b>mut</b> <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, path_to_node: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RefToSlot&gt;, key: &K): <a href="big_ordered_map.md#0x1_big_ordered_map_Child">Child</a>&lt;V&gt; {
    <b>let</b> node_index = path_to_node.pop_back();
    <b>let</b> old_child = {
        <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node_mut">borrow_node_mut</a>(node_index);

        <b>let</b> children = &<b>mut</b> node.children;

        <b>let</b> is_leaf = node.is_leaf;

        <b>let</b> old_child = children.<a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>(key);
        <b>if</b> (path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>()) {
            <b>assert</b>!(node_index == self.root_index, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

            <b>if</b> (!is_leaf && children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() == 1) {
                // promote only child <b>to</b> root, and drop current root.
                // keep the root index the same.
                <b>let</b> Child::Inner {
                    node_index: inner_child_index,
                } = children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_remove(children);
                <b>move</b> children;
                <b>move</b> node;

                <b>let</b> inner_child = self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_remove">remove</a>(inner_child_index);
                <b>if</b> (inner_child.is_leaf) {
                    <b>let</b> root_ref = self.root_index;
                    self.min_leaf_index = root_ref;
                    self.max_leaf_index = root_ref;
                };

                <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(&<b>mut</b> self.root, inner_child).<a href="big_ordered_map.md#0x1_big_ordered_map_destroy_empty_node">destroy_empty_node</a>();
            }; // <b>else</b>: nothing <b>to</b> change
            <b>return</b> old_child;
        };

        <b>let</b> max_degree = <b>if</b> (is_leaf) {
            self.leaf_max_degree <b>as</b> u64
        } <b>else</b> {
            self.inner_max_degree <b>as</b> u64
        };

        <b>let</b> current_size = children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>();
        <b>let</b> big_enough = current_size * 2 &gt;= max_degree;

        <b>let</b> new_max_key = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);
        <b>let</b> max_key_updated = <a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&new_max_key, key).is_less_than();
        <b>if</b> (!max_key_updated && big_enough) {
            <b>return</b> old_child;
        };

        <b>if</b> (max_key_updated) {
            <b>assert</b>!(current_size &gt;= 1, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));

            self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, key, new_max_key);

            <b>if</b> (big_enough) {
                <b>return</b> old_child;
            }
        };

        old_child
    };

    // Children size is below threshold, we need <b>to</b> rebalance

    <b>let</b> (node_slot, node) = self.nodes.remove_and_reserve(node_index);

    <b>let</b> is_leaf = node.is_leaf;
    <b>let</b> max_degree = self.<a href="big_ordered_map.md#0x1_big_ordered_map_get_max_degree">get_max_degree</a>(is_leaf);
    <b>let</b> prev = node.prev;
    <b>let</b> next = node.next;

    <b>let</b> brother_index = {
        <b>let</b> parent_children = &self.nodes.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>(*path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow">borrow</a>(path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() - 1)).children;
        <b>if</b> (parent_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(parent_children).iter_borrow(parent_children).node_index.stored_as_ref() == node_index) {
            prev
        } <b>else</b> {
            next
        }
    };

    <b>let</b> children = &<b>mut</b> node.children;
    <b>let</b> (brother_slot, brother_node) = self.nodes.remove_and_reserve(brother_index);

    <b>let</b> brother_children = &<b>mut</b> brother_node.children;

    <b>if</b> ((brother_children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>() - 1) * 2 &gt;= max_degree) {
        // The brother node <b>has</b> enough elements, borrow an element from the brother node.
        <b>if</b> (brother_index == next) {
            <b>let</b> old_max_key = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);
            <b>let</b> brother_begin_iter = brother_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_begin_iter">new_begin_iter</a>();
            <b>let</b> borrowed_max_key = *brother_begin_iter.iter_borrow_key(brother_children);
            <b>let</b> borrowed_element = brother_begin_iter.iter_remove(brother_children);

            children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(borrowed_max_key, borrowed_element);
            self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, &old_max_key, borrowed_max_key);
        } <b>else</b> {
            <b>let</b> brother_end_iter = brother_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(brother_children);
            <b>let</b> borrowed_max_key = *brother_end_iter.iter_borrow_key(brother_children);
            <b>let</b> borrowed_element = brother_end_iter.iter_remove(brother_children);

            children.<a href="big_ordered_map.md#0x1_big_ordered_map_add">add</a>(borrowed_max_key, borrowed_element);
            self.<a href="big_ordered_map.md#0x1_big_ordered_map_update_key">update_key</a>(path_to_node, &borrowed_max_key, *brother_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(brother_children).iter_borrow_key(brother_children));
        };

        self.nodes.fill_reserved_slot(node_slot, node);
        self.nodes.fill_reserved_slot(brother_slot, brother_node);
        <b>return</b> old_child;
    };

    // The brother node doesn't have enough elements <b>to</b> borrow, merge <b>with</b> the brother node.
    <b>if</b> (brother_index == next) {
        <b>let</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> { children: brother_children, is_leaf: _, prev: _, next: brother_next } = brother_node;
        <b>let</b> key_to_remove = *children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(children).iter_borrow_key(children);
        children.append(brother_children);
        node.next = brother_next;

        <b>move</b> children;

        <b>if</b> (!node.next.ref_is_null()) {
            self.nodes.borrow_mut(node.next).prev = brother_index;
        };
        <b>if</b> (!node.prev.ref_is_null()) {
            self.nodes.borrow_mut(node.prev).next = brother_index;
        };

        self.nodes.fill_reserved_slot(brother_slot, node);

        <b>if</b> (self.min_leaf_index == node_index) {
            self.min_leaf_index = brother_index;
        };

        <b>assert</b>!(!path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
        <b>let</b> node_stored_slot = <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_inner_child">destroy_inner_child</a>(self.<a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>(path_to_node, &key_to_remove));
        self.nodes.free_reserved_slot(node_slot, node_stored_slot);
    } <b>else</b> {
        <b>let</b> <a href="big_ordered_map.md#0x1_big_ordered_map_Node">Node</a> { children: node_children, is_leaf: _, prev: _, next: node_next } = node;
        <b>let</b> key_to_remove = *brother_children.<a href="big_ordered_map.md#0x1_big_ordered_map_new_end_iter">new_end_iter</a>().<a href="big_ordered_map.md#0x1_big_ordered_map_iter_prev">iter_prev</a>(brother_children).iter_borrow_key(brother_children);
        brother_children.append(node_children);
        brother_node.next = node_next;

        <b>move</b> brother_children;

        <b>if</b> (!brother_node.next.ref_is_null()) {
            self.nodes.borrow_mut(brother_node.next).prev = node_index;
        };
        <b>if</b> (!brother_node.prev.ref_is_null()) {
            self.nodes.borrow_mut(brother_node.prev).next = node_index;
        };

        self.nodes.fill_reserved_slot(node_slot, brother_node);

        <b>if</b> (self.min_leaf_index == brother_index) {
            self.min_leaf_index = node_index;
        };

        <b>assert</b>!(!path_to_node.<a href="big_ordered_map.md#0x1_big_ordered_map_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_ordered_map.md#0x1_big_ordered_map_EINTERNAL_INVARIANT_BROKEN">EINTERNAL_INVARIANT_BROKEN</a>));
        <b>let</b> node_stored_slot = <a href="big_ordered_map.md#0x1_big_ordered_map_destroy_inner_child">destroy_inner_child</a>(self.<a href="big_ordered_map.md#0x1_big_ordered_map_remove_at">remove_at</a>(path_to_node, &key_to_remove));
        self.nodes.free_reserved_slot(brother_slot, node_stored_slot);
    };
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
    self.<a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>(self.root_index)
}
</code></pre>



</details>

<a id="0x1_big_ordered_map_length_for_node"></a>

## Function `length_for_node`



<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;K, V&gt;, node_index: <a href="storage_slots_allocator.md#0x1_storage_slots_allocator_RefToSlot">storage_slots_allocator::RefToSlot</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>&lt;K: store, V: store&gt;(self: &<a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">BigOrderedMap</a>&lt;K, V&gt;, node_index: RefToSlot): u64 {
    <b>let</b> node = self.<a href="big_ordered_map.md#0x1_big_ordered_map_borrow_node">borrow_node</a>(node_index);
    <b>if</b> (node.is_leaf) {
        node.children.<a href="big_ordered_map.md#0x1_big_ordered_map_length">length</a>()
    } <b>else</b> {
        <b>let</b> size = 0;

        node.children.for_each_ref(|_key, child| {
            size = size + self.<a href="big_ordered_map.md#0x1_big_ordered_map_length_for_node">length_for_node</a>(child.node_index.stored_as_ref());
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
