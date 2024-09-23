
<a id="0x1_btree_map"></a>

# Module `0x1::btree_map`

Type of large-scale search trees.

It internally uses BTree to organize the search tree data structure for keys. Comparing with
other common search trees like AVL or Red-black tree, a BTree node has more children, and packs
more metadata into one node, which is more disk friendly (and gas friendly).


-  [Struct `Node`](#0x1_btree_map_Node)
-  [Enum `Child`](#0x1_btree_map_Child)
-  [Enum `Iterator`](#0x1_btree_map_Iterator)
-  [Struct `BTreeMap`](#0x1_btree_map_BTreeMap)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_btree_map_new)
-  [Function `new_with_config`](#0x1_btree_map_new_with_config)
-  [Function `destroy_empty`](#0x1_btree_map_destroy_empty)
-  [Function `insert`](#0x1_btree_map_insert)
-  [Function `upsert`](#0x1_btree_map_upsert)
-  [Function `remove`](#0x1_btree_map_remove)
-  [Function `is_null_index`](#0x1_btree_map_is_null_index)
-  [Function `is_begin_iter`](#0x1_btree_map_is_begin_iter)
-  [Function `is_end_iter`](#0x1_btree_map_is_end_iter)
-  [Function `lower_bound`](#0x1_btree_map_lower_bound)
-  [Function `find`](#0x1_btree_map_find)
-  [Function `contains`](#0x1_btree_map_contains)
-  [Function `get_key`](#0x1_btree_map_get_key)
-  [Function `borrow`](#0x1_btree_map_borrow)
-  [Function `borrow_mut`](#0x1_btree_map_borrow_mut)
-  [Function `size`](#0x1_btree_map_size)
-  [Function `size_for_node`](#0x1_btree_map_size_for_node)
-  [Function `empty`](#0x1_btree_map_empty)
-  [Function `new_begin_iter`](#0x1_btree_map_new_begin_iter)
-  [Function `new_end_iter`](#0x1_btree_map_new_end_iter)
-  [Function `next_iter`](#0x1_btree_map_next_iter)
-  [Function `next_iter_or_die`](#0x1_btree_map_next_iter_or_die)
-  [Function `prev_iter`](#0x1_btree_map_prev_iter)
-  [Function `prev_iter_or_die`](#0x1_btree_map_prev_iter_or_die)
-  [Function `destroy_inner_child`](#0x1_btree_map_destroy_inner_child)
-  [Function `destroy_empty_node`](#0x1_btree_map_destroy_empty_node)
-  [Function `new_node`](#0x1_btree_map_new_node)
-  [Function `new_node_with_children`](#0x1_btree_map_new_node_with_children)
-  [Function `new_inner_child`](#0x1_btree_map_new_inner_child)
-  [Function `new_leaf_child`](#0x1_btree_map_new_leaf_child)
-  [Function `new_iter`](#0x1_btree_map_new_iter)
-  [Function `find_leaf`](#0x1_btree_map_find_leaf)
-  [Function `binary_search`](#0x1_btree_map_binary_search)
-  [Function `insert_at`](#0x1_btree_map_insert_at)
-  [Function `update_key`](#0x1_btree_map_update_key)
-  [Function `remove_at`](#0x1_btree_map_remove_at)


<pre><code><b>use</b> <a href="cmp.md#0x1_cmp">0x1::cmp</a>;
<b>use</b> <a href="debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_btree_map_Node"></a>

## Struct `Node`

A node of the BTreeMap.


<pre><code><b>struct</b> <a href="btree_map.md#0x1_btree_map_Node">Node</a>&lt;K: store, V: store&gt; <b>has</b> store
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
<code>parent: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>children: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;&gt;</code>
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

<a id="0x1_btree_map_Child"></a>

## Enum `Child`

The metadata of a child of a node.


<pre><code>enum <a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K: store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Inner</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>max_key: K</code>
</dt>
<dd>

</dd>
<dt>
<code>node_index: u64</code>
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
<code>max_key: K</code>
</dt>
<dd>

</dd>
<dt>
<code>value: V</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_btree_map_Iterator"></a>

## Enum `Iterator`

An iterator to iterate all keys in the BTreeMap.


<pre><code>enum <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; <b>has</b> <b>copy</b>, drop
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

</dd>
<dt>
<code>child_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>key: K</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_btree_map_BTreeMap"></a>

## Struct `BTreeMap`

The BTreeMap data structure.


<pre><code><b>struct</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K: store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>nodes: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="btree_map.md#0x1_btree_map_Node">btree_map::Node</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>order: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>min_leaf_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>max_leaf_index: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_btree_map_DEFAULT_ORDER"></a>



<pre><code><b>const</b> <a href="btree_map.md#0x1_btree_map_DEFAULT_ORDER">DEFAULT_ORDER</a>: u8 = 32;
</code></pre>



<a id="0x1_btree_map_E_INTERNAL"></a>



<pre><code><b>const</b> <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>: u64 = 0;
</code></pre>



<a id="0x1_btree_map_E_INVALID_PARAMETER"></a>



<pre><code><b>const</b> <a href="btree_map.md#0x1_btree_map_E_INVALID_PARAMETER">E_INVALID_PARAMETER</a>: u64 = 3;
</code></pre>



<a id="0x1_btree_map_E_TREE_NOT_EMPTY"></a>



<pre><code><b>const</b> <a href="btree_map.md#0x1_btree_map_E_TREE_NOT_EMPTY">E_TREE_NOT_EMPTY</a>: u64 = 1;
</code></pre>



<a id="0x1_btree_map_E_TREE_TOO_BIG"></a>



<pre><code><b>const</b> <a href="btree_map.md#0x1_btree_map_E_TREE_TOO_BIG">E_TREE_TOO_BIG</a>: u64 = 2;
</code></pre>



<a id="0x1_btree_map_NULL_INDEX"></a>



<pre><code><b>const</b> <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>: u64 = 0;
</code></pre>



<a id="0x1_btree_map_new"></a>

## Function `new`

Returns a new BTreeMap with the default configuration.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new">new</a>&lt;K: store, V: store&gt;(): <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new">new</a>&lt;K: store, V: store&gt;(): <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt; {
    <a href="btree_map.md#0x1_btree_map_new_with_config">new_with_config</a>(<a href="btree_map.md#0x1_btree_map_DEFAULT_ORDER">DEFAULT_ORDER</a>)
}
</code></pre>



</details>

<a id="0x1_btree_map_new_with_config"></a>

## Function `new_with_config`

Returns a new BTreeMap with the provided order (the maximum # of children a node can have).


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new_with_config">new_with_config</a>&lt;K: store, V: store&gt;(order: u8): <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new_with_config">new_with_config</a>&lt;K: store, V: store&gt;(order: u8): <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt; {
    <b>assert</b>!(order &gt;= 5, <a href="btree_map.md#0x1_btree_map_E_INVALID_PARAMETER">E_INVALID_PARAMETER</a>);
    <b>let</b> root_node = <a href="btree_map.md#0x1_btree_map_new_node">new_node</a>(/*is_leaf=*/<b>true</b>, /*parent=*/<a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>);
    <b>let</b> nodes = <a href="table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>();
    <b>let</b> root_index = 1;
    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> nodes, root_index, root_node);
    <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a> {
        root_index: root_index,
        nodes: nodes,
        order: order,
        min_leaf_index: root_index,
        max_leaf_index: root_index,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_destroy_empty"></a>

## Function `destroy_empty`

Destroys the tree if it's empty, otherwise aborts.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_destroy_empty">destroy_empty</a>&lt;K: store, V: store&gt;(tree: <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_destroy_empty">destroy_empty</a>&lt;K: store, V: store&gt;(tree: <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;) {
    <b>let</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a> { nodes, root_index, order: _, min_leaf_index: _, max_leaf_index: _ } = tree;
    aptos_std::debug::print(&nodes);
    <b>assert</b>!(<a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&nodes) == 1, <a href="btree_map.md#0x1_btree_map_E_TREE_NOT_EMPTY">E_TREE_NOT_EMPTY</a>);
    <a href="btree_map.md#0x1_btree_map_destroy_empty_node">destroy_empty_node</a>(<a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> nodes, root_index));
    <a href="table_with_length.md#0x1_table_with_length_destroy_empty">table_with_length::destroy_empty</a>(nodes);
}
</code></pre>



</details>

<a id="0x1_btree_map_insert"></a>

## Function `insert`

Inserts the key/value into the BTreeMap.
Aborts if the key is already in the tree.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_insert">insert</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_insert">insert</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K, value: V) {
    <b>let</b> leaf = <a href="btree_map.md#0x1_btree_map_find_leaf">find_leaf</a>(tree, key);

    <b>if</b> (leaf == <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        // In this case, the key is greater than all keys in the tree.
        leaf = tree.max_leaf_index;
        <b>let</b> current = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, leaf).parent;
        <b>while</b> (current != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <b>let</b> current_node = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, current);
            <b>let</b> last_index = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&current_node.children) - 1;
            <b>let</b> last_element = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> current_node.children, last_index);
            last_element.max_key = key;
            current = current_node.parent;
        }
    };

    <a href="btree_map.md#0x1_btree_map_insert_at">insert_at</a>(tree, leaf, <a href="btree_map.md#0x1_btree_map_new_leaf_child">new_leaf_child</a>(key, value));
}
</code></pre>



</details>

<a id="0x1_btree_map_upsert"></a>

## Function `upsert`

If the key doesn't exist in the tree, inserts the key/value, and returns none.
Otherwise updates the value under the given key, and returns the old value.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_upsert">upsert</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K, value: V): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_upsert">upsert</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K, value: V): Option&lt;V&gt; {
    <b>let</b> iter = <a href="btree_map.md#0x1_btree_map_find">find</a>(tree, key);
    <b>if</b> (<a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>(tree, &iter)) {
        <a href="btree_map.md#0x1_btree_map_insert">insert</a>(tree, key, value);
        <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, iter.node_index);
        <b>let</b> children = &<b>mut</b> node.children;

        // Field swap doesn't compile.
        // <b>let</b> child = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(children, iter.child_index);
        // <b>assert</b>!(child.max_key == key, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);
        // <b>let</b> <b>old</b> = child.value;
        // child.value = value;
        // <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<b>old</b>)

        <b>let</b> Child::Leaf {
            max_key: old_max_key,
            value: old_value,
        } = <a href="../../move-stdlib/doc/vector.md#0x1_vector_replace">vector::replace</a>(children, iter.child_index, Child::Leaf { max_key: key, value: value });
        <b>assert</b>!(old_max_key == key, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(old_value)
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_remove"></a>

## Function `remove`

Removes the entry from BTreeMap and returns the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_remove">remove</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_remove">remove</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): V {
    <b>let</b> iter = <a href="btree_map.md#0x1_btree_map_find">find</a>(tree, key);
    <b>assert</b>!(!<a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>(tree, &iter), <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);

    <b>let</b> Child::Leaf {
        value,
        max_key: _,
    } = <a href="btree_map.md#0x1_btree_map_remove_at">remove_at</a>(tree, iter.node_index, key);

    value
}
</code></pre>



</details>

<a id="0x1_btree_map_is_null_index"></a>

## Function `is_null_index`



<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_is_null_index">is_null_index</a>(node_index: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_is_null_index">is_null_index</a>(node_index: u64): bool {
    node_index == <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>
}
</code></pre>



</details>

<a id="0x1_btree_map_is_begin_iter"></a>

## Function `is_begin_iter`



<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_is_begin_iter">is_begin_iter</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, iter: &<a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_is_begin_iter">is_begin_iter</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, iter: &<a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): bool {
    <b>if</b> (iter is Iterator::End&lt;K&gt;) {
        <a href="btree_map.md#0x1_btree_map_empty">empty</a>(tree)
    } <b>else</b> {
        (iter.node_index == tree.min_leaf_index && iter.child_index == 0)
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_is_end_iter"></a>

## Function `is_end_iter`



<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>&lt;K: store, V: store&gt;(_tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, iter: &<a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>&lt;K: store, V: store&gt;(_tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, iter: &<a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): bool {
    iter is Iterator::End&lt;K&gt;
}
</code></pre>



</details>

<a id="0x1_btree_map_lower_bound"></a>

## Function `lower_bound`

Returns an iterator pointing to the first element that is greater or equal to the provided
key, or an end iterator if such element doesn't exist.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_lower_bound">lower_bound</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_lower_bound">lower_bound</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> leaf = <a href="btree_map.md#0x1_btree_map_find_leaf">find_leaf</a>(tree, key);
    <b>if</b> (leaf == <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        <b>return</b> <a href="btree_map.md#0x1_btree_map_new_end_iter">new_end_iter</a>(tree)
    };

    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, leaf);
    <b>assert</b>!(node.is_leaf, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);

    <b>let</b> keys = &node.children;

    <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(keys);

    <b>let</b> index = <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>(key, keys, 0, len);
    <b>if</b> (index == len) {
        <a href="btree_map.md#0x1_btree_map_new_end_iter">new_end_iter</a>(tree)
    } <b>else</b> {
        <a href="btree_map.md#0x1_btree_map_new_iter">new_iter</a>(leaf, index, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(keys, index).max_key)
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_find"></a>

## Function `find`

Returns an iterator pointing to the element that equals to the provided key, or an end
iterator if the key is not found.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_find">find</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_find">find</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> lower_bound = <a href="btree_map.md#0x1_btree_map_lower_bound">lower_bound</a>(tree, key);
    <b>if</b> (<a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>(tree, &lower_bound)) {
        lower_bound
    } <b>else</b> <b>if</b> (lower_bound.key == key) {
        lower_bound
    } <b>else</b> {
        <a href="btree_map.md#0x1_btree_map_new_end_iter">new_end_iter</a>(tree)
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_contains"></a>

## Function `contains`

Returns true iff the key exists in the tree.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_contains">contains</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_contains">contains</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): bool {
    <b>let</b> lower_bound = <a href="btree_map.md#0x1_btree_map_lower_bound">lower_bound</a>(tree, key);
    <b>if</b> (<a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>(tree, &lower_bound)) {
        <b>false</b>
    } <b>else</b> <b>if</b> (lower_bound.key == key) {
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_get_key"></a>

## Function `get_key`

Returns the key of the given iterator.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_get_key">get_key</a>&lt;K: <b>copy</b>&gt;(iter: &<a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): K
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_get_key">get_key</a>&lt;K: <b>copy</b>&gt;(iter: &<a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): K {
    <b>assert</b>!(!(iter is Iterator::End&lt;K&gt;), <a href="btree_map.md#0x1_btree_map_E_INVALID_PARAMETER">E_INVALID_PARAMETER</a>);
    iter.key
}
</code></pre>



</details>

<a id="0x1_btree_map_borrow"></a>

## Function `borrow`

Returns a reference to the element with its key, aborts if the key is not found.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_borrow">borrow</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_borrow">borrow</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): &V {
    <b>let</b> iter = <a href="btree_map.md#0x1_btree_map_find">find</a>(tree, key);

    <b>assert</b>!(<a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>(tree, &iter), <a href="btree_map.md#0x1_btree_map_E_INVALID_PARAMETER">E_INVALID_PARAMETER</a>);
    <b>let</b> children = &<a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, iter.node_index).children;
    &<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, iter.child_index).value
}
</code></pre>



</details>

<a id="0x1_btree_map_borrow_mut"></a>

## Function `borrow_mut`

Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_borrow_mut">borrow_mut</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): &<b>mut</b> V {
    <b>let</b> iter = <a href="btree_map.md#0x1_btree_map_find">find</a>(tree, key);

    <b>assert</b>!(<a href="btree_map.md#0x1_btree_map_is_end_iter">is_end_iter</a>(tree, &iter), <a href="btree_map.md#0x1_btree_map_E_INVALID_PARAMETER">E_INVALID_PARAMETER</a>);
    <b>let</b> children = &<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, iter.node_index).children;
    &<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(children, iter.child_index).value
}
</code></pre>



</details>

<a id="0x1_btree_map_size"></a>

## Function `size`

Returns the number of elements in the BTreeMap.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_size">size</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_size">size</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;): u64 {
    <a href="btree_map.md#0x1_btree_map_size_for_node">size_for_node</a>(tree, tree.root_index)
}
</code></pre>



</details>

<a id="0x1_btree_map_size_for_node"></a>

## Function `size_for_node`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_size_for_node">size_for_node</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, node_index: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_size_for_node">size_for_node</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, node_index: u64): u64 {
    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, node_index);
    <b>if</b> (node.is_leaf) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node.children)
    } <b>else</b> {
        <b>let</b> size = 0;

        for (i in 0..<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node.children)) {
            size = size + <a href="btree_map.md#0x1_btree_map_size_for_node">size_for_node</a>(tree, node.children[i].node_index);
        };
        size
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_empty"></a>

## Function `empty`

Returns true iff the BTreeMap is empty.


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_empty">empty</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_empty">empty</a>&lt;K: store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;): bool {
    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, tree.min_leaf_index);

    <a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&node.children)
}
</code></pre>



</details>

<a id="0x1_btree_map_new_begin_iter"></a>

## Function `new_begin_iter`

Return the begin iterator.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new_begin_iter">new_begin_iter</a>&lt;K: <b>copy</b>, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new_begin_iter">new_begin_iter</a>&lt;K: <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>if</b> (<a href="btree_map.md#0x1_btree_map_empty">empty</a>(tree)) {
        <b>return</b> Iterator::End;
    };

    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, tree.min_leaf_index);
    <b>let</b> key = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&node.children, 0).max_key;

    <a href="btree_map.md#0x1_btree_map_new_iter">new_iter</a>(tree.min_leaf_index, 0, key)
}
</code></pre>



</details>

<a id="0x1_btree_map_new_end_iter"></a>

## Function `new_end_iter`

Return the end iterator.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new_end_iter">new_end_iter</a>&lt;K: <b>copy</b>, store, V: store&gt;(_tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_new_end_iter">new_end_iter</a>&lt;K: <b>copy</b> + store, V: store&gt;(_tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    Iterator::End
}
</code></pre>



</details>

<a id="0x1_btree_map_next_iter"></a>

## Function `next_iter`

Returns the next iterator, or none if already at the end iterator.
Requires the tree is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_next_iter">next_iter</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_next_iter">next_iter</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): Option&lt;<a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;&gt; {
    <b>if</b> (iter is Iterator::End&lt;K&gt;) {
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="btree_map.md#0x1_btree_map_next_iter_or_die">next_iter_or_die</a>(tree, iter))
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_next_iter_or_die"></a>

## Function `next_iter_or_die`

Returns the next iterator, aborts if already at the end iterator.
Requires the tree is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_next_iter_or_die">next_iter_or_die</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_next_iter_or_die">next_iter_or_die</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>assert</b>!(!(iter is Iterator::End&lt;K&gt;), <a href="btree_map.md#0x1_btree_map_E_INVALID_PARAMETER">E_INVALID_PARAMETER</a>);

    <b>let</b> node_index = iter.node_index;

    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, node_index);
    iter.child_index = iter.child_index + 1;
    <b>if</b> (iter.child_index &lt; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node.children)) {
        iter.key = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&node.children, iter.child_index).max_key;
        <b>return</b> iter
    };

    <b>let</b> next_index = node.next;
    <b>if</b> (next_index != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        <b>let</b> next_node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, next_index);
        iter.node_index = next_index;
        iter.child_index = 0;
        iter.key = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&next_node.children, 0).max_key;
        <b>return</b> iter
    };

    <a href="btree_map.md#0x1_btree_map_new_end_iter">new_end_iter</a>(tree)
}
</code></pre>



</details>

<a id="0x1_btree_map_prev_iter"></a>

## Function `prev_iter`

Returns the previous iterator, or none if already at the begin iterator.
Requires the tree is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_prev_iter">prev_iter</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_prev_iter">prev_iter</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): Option&lt;<a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;&gt; {
    <b>if</b> (iter.node_index == tree.min_leaf_index && iter.child_index == 0) {
        <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="btree_map.md#0x1_btree_map_prev_iter_or_die">prev_iter_or_die</a>(tree, iter))
}
</code></pre>



</details>

<a id="0x1_btree_map_prev_iter_or_die"></a>

## Function `prev_iter_or_die`

Returns the previous iterator, aborts if already at the begin iterator.
Requires the tree is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_prev_iter_or_die">prev_iter_or_die</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="btree_map.md#0x1_btree_map_prev_iter_or_die">prev_iter_or_die</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, iter: <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt;): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    <b>let</b> prev_index = <b>if</b> (iter is Iterator::End&lt;K&gt;) {
        tree.max_leaf_index
    } <b>else</b> {
        <b>let</b> node_index = iter.node_index;
        <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, node_index);
        <b>if</b> (iter.child_index &gt;= 1) {
            iter.child_index = iter.child_index - 1;
            iter.key = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&node.children, iter.child_index).max_key;
            <b>return</b> iter
        };
        node.prev
    };

    <b>assert</b>!(prev_index != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);

    <b>let</b> prev_node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, prev_index);
    <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&prev_node.children);

    Iterator::Some {
        node_index: prev_index,
        child_index: len - 1,
        key: <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&prev_node.children, len - 1).max_key,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_destroy_inner_child"></a>

## Function `destroy_inner_child`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_destroy_inner_child">destroy_inner_child</a>&lt;K: drop, store, V: store&gt;(child: <a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_destroy_inner_child">destroy_inner_child</a>&lt;K: drop + store, V: store&gt;(child: <a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt;) {
    <b>let</b> Child::Inner {
        max_key: _,
        node_index: _,
    } = child;
}
</code></pre>



</details>

<a id="0x1_btree_map_destroy_empty_node"></a>

## Function `destroy_empty_node`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_destroy_empty_node">destroy_empty_node</a>&lt;K: store, V: store&gt;(node: <a href="btree_map.md#0x1_btree_map_Node">btree_map::Node</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_destroy_empty_node">destroy_empty_node</a>&lt;K: store, V: store&gt;(node: <a href="btree_map.md#0x1_btree_map_Node">Node</a>&lt;K, V&gt;) {
    <b>let</b> <a href="btree_map.md#0x1_btree_map_Node">Node</a> { children, is_leaf: _, parent: _, prev: _, next: _ } = node;
    <b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&children), <a href="btree_map.md#0x1_btree_map_E_TREE_NOT_EMPTY">E_TREE_NOT_EMPTY</a>);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(children);
}
</code></pre>



</details>

<a id="0x1_btree_map_new_node"></a>

## Function `new_node`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_node">new_node</a>&lt;K: store, V: store&gt;(is_leaf: bool, parent: u64): <a href="btree_map.md#0x1_btree_map_Node">btree_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_node">new_node</a>&lt;K: store, V: store&gt;(is_leaf: bool, parent: u64): <a href="btree_map.md#0x1_btree_map_Node">Node</a>&lt;K, V&gt; {
    <a href="btree_map.md#0x1_btree_map_Node">Node</a> {
        is_leaf: is_leaf,
        parent: parent,
        children: <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        prev: <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>,
        next: <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_new_node_with_children"></a>

## Function `new_node_with_children`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_node_with_children">new_node_with_children</a>&lt;K: store, V: store&gt;(is_leaf: bool, parent: u64, children: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;&gt;): <a href="btree_map.md#0x1_btree_map_Node">btree_map::Node</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_node_with_children">new_node_with_children</a>&lt;K: store, V: store&gt;(is_leaf: bool, parent: u64, children: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt;&gt;): <a href="btree_map.md#0x1_btree_map_Node">Node</a>&lt;K, V&gt; {
    <a href="btree_map.md#0x1_btree_map_Node">Node</a> {
        is_leaf: is_leaf,
        parent: parent,
        children: children,
        prev: <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>,
        next: <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_new_inner_child"></a>

## Function `new_inner_child`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_inner_child">new_inner_child</a>&lt;K: store, V: store&gt;(max_key: K, node_index: u64): <a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_inner_child">new_inner_child</a>&lt;K: store, V: store&gt;(max_key: K, node_index: u64): <a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt; {
    Child::Inner {
        max_key: max_key,
        node_index: node_index,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_new_leaf_child"></a>

## Function `new_leaf_child`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_leaf_child">new_leaf_child</a>&lt;K: store, V: store&gt;(max_key: K, value: V): <a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_leaf_child">new_leaf_child</a>&lt;K: store, V: store&gt;(max_key: K, value: V): <a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt; {
    Child::Leaf {
        max_key: max_key,
        value: value,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_new_iter"></a>

## Function `new_iter`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_iter">new_iter</a>&lt;K: store&gt;(node_index: u64, child_index: u64, key: K): <a href="btree_map.md#0x1_btree_map_Iterator">btree_map::Iterator</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_new_iter">new_iter</a>&lt;K: store&gt;(node_index: u64, child_index: u64, key: K): <a href="btree_map.md#0x1_btree_map_Iterator">Iterator</a>&lt;K&gt; {
    Iterator::Some {
        node_index: node_index,
        child_index: child_index,
        key: key,
    }
}
</code></pre>



</details>

<a id="0x1_btree_map_find_leaf"></a>

## Function `find_leaf`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_find_leaf">find_leaf</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, key: K): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_find_leaf">find_leaf</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, key: K): u64 {
    <b>let</b> current = tree.root_index;
    <b>while</b> (current != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, current);
        <b>if</b> (node.is_leaf) {
            <b>return</b> current
        };
        <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node.children);
        <b>if</b> (<a href="cmp.md#0x1_cmp_compare">cmp::compare</a>(&<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&node.children, len - 1).max_key, &key).is_less_than()) {
            <b>return</b> <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>
        };

        <b>let</b> index = <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>(key, &node.children, 0, len);

        current = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&node.children, index).node_index;
    };

    <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>
}
</code></pre>



</details>

<a id="0x1_btree_map_binary_search"></a>

## Function `binary_search`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>&lt;K: drop, store, V: store&gt;(key: K, children: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>&lt;K: drop + store, V: store&gt;(key: K, children: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64 {
    <b>let</b> l = start;
    <b>let</b> r = end;
    <b>while</b> (l != r) {
        <b>let</b> mid = l + (r - l) / 2;
        <b>if</b> (<a href="cmp.md#0x1_cmp_compare">cmp::compare</a>(&<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, mid).max_key, &key).is_less_than()) {
            l = mid + 1;
        } <b>else</b> {
            r = mid;
        };
    };
    l
}
</code></pre>



</details>

<a id="0x1_btree_map_insert_at"></a>

## Function `insert_at`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_insert_at">insert_at</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, node_index: u64, child: <a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_insert_at">insert_at</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, node_index: u64, child: <a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt;) {
    <b>let</b> current_size = {
        <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, node_index);
        <b>let</b> children = &<b>mut</b> node.children;
        <b>let</b> current_size = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(children);
        <b>let</b> key = child.max_key;

        <b>if</b> (current_size &lt; (tree.order <b>as</b> u64)) {
            <b>let</b> index = <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>(key, children, 0, current_size);
            <b>assert</b>!(index &gt;= current_size || children[index].max_key != key, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>); // key cannot already be inside.
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(children, index, child);
            <b>return</b>
        };
        current_size
    };

    // # of children in the current node exceeds the threshold, need <b>to</b> split into two nodes.
    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> tree.nodes, node_index);
    <b>let</b> parent_index = node.parent;
    <b>let</b> is_leaf = &<b>mut</b> node.is_leaf;
    <b>let</b> next = &<b>mut</b> node.next;
    <b>let</b> prev = &<b>mut</b> node.prev;
    <b>let</b> children = &<b>mut</b> node.children;
    <b>let</b> key = child.max_key;

    <b>let</b> target_size = ((tree.order <b>as</b> u64) + 1) / 2;

    <b>let</b> l = <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>(key, children, 0, current_size);

    <b>let</b> left_node_index = <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&tree.nodes) + 2;

    <b>if</b> (parent_index == <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        // Splitting root now, need <b>to</b> create a new root.
        parent_index = <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&tree.nodes) + 3;
        node.parent = parent_index;

        tree.root_index = parent_index;
        <b>let</b> parent_node = <a href="btree_map.md#0x1_btree_map_new_node">new_node</a>(/*is_leaf=*/<b>false</b>, /*parent=*/<a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>);
        <b>let</b> max_element = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, current_size - 1).max_key;
        <b>if</b> (<a href="cmp.md#0x1_cmp_compare">cmp::compare</a>(&max_element, &key).is_less_than()) {
            max_element = key;
        };
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> parent_node.children, <a href="btree_map.md#0x1_btree_map_new_inner_child">new_inner_child</a>(max_element, node_index));
        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, parent_index, parent_node);
    };

    <b>let</b> new_node_children = <b>if</b> (l &lt; target_size) {
        <b>let</b> new_node_children = <a href="../../move-stdlib/doc/vector.md#0x1_vector_split_off">vector::split_off</a>(children, target_size - 1);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(children, l, child);
        new_node_children
    } <b>else</b> {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(children, l, child);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_split_off">vector::split_off</a>(children, target_size)
    };

    <b>let</b> right_node = <a href="btree_map.md#0x1_btree_map_new_node_with_children">new_node_with_children</a>(*is_leaf, parent_index, new_node_children);

    right_node.next = *next;
    *next = node_index;
    right_node.prev = left_node_index;
    <b>if</b> (*prev != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, *prev).next = left_node_index;
    };

    <b>if</b> (!*is_leaf) {
        <b>let</b> i = 0;
        <b>while</b> (i &lt; target_size) {
            <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, i).node_index).parent = left_node_index;
            i = i + 1;
        };
    };

    <b>let</b> split_key = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, target_size - 1).max_key;

    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, left_node_index, node);
    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, node_index, right_node);
    <b>if</b> (node_index == tree.min_leaf_index) {
        tree.min_leaf_index = left_node_index;
    };
    <a href="btree_map.md#0x1_btree_map_insert_at">insert_at</a>(tree, parent_index, <a href="btree_map.md#0x1_btree_map_new_inner_child">new_inner_child</a>(split_key, left_node_index));
}
</code></pre>



</details>

<a id="0x1_btree_map_update_key"></a>

## Function `update_key`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_update_key">update_key</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, node_index: u64, old_key: K, new_key: K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_update_key">update_key</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, node_index: u64, old_key: K, new_key: K) {
    <b>if</b> (node_index == <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
        <b>return</b>
    };

    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, node_index);
    <b>let</b> keys = &<b>mut</b> node.children;
    <b>let</b> current_size = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(keys);

    <b>let</b> index = <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>(old_key, keys, 0, current_size);

    <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(keys, index).max_key = new_key;
    <b>move</b> keys;

    <b>if</b> (index == current_size - 1) {
        <a href="btree_map.md#0x1_btree_map_update_key">update_key</a>(tree, node.parent, old_key, new_key);
    };
}
</code></pre>



</details>

<a id="0x1_btree_map_remove_at"></a>

## Function `remove_at`



<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_remove_at">remove_at</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">btree_map::BTreeMap</a>&lt;K, V&gt;, node_index: u64, key: K): <a href="btree_map.md#0x1_btree_map_Child">btree_map::Child</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="btree_map.md#0x1_btree_map_remove_at">remove_at</a>&lt;K: drop + <b>copy</b> + store, V: store&gt;(tree: &<b>mut</b> <a href="btree_map.md#0x1_btree_map_BTreeMap">BTreeMap</a>&lt;K, V&gt;, node_index: u64, key: K): <a href="btree_map.md#0x1_btree_map_Child">Child</a>&lt;K, V&gt; {
    <b>let</b> (old_child, current_size) = {
        <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, node_index);

        <b>let</b> children = &<b>mut</b> node.children;
        <b>let</b> current_size = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(children);

        <b>if</b> (current_size == 1) {
            // Remove the only element at root node.
            <b>assert</b>!(node_index == tree.root_index, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);
            <b>assert</b>!(key == <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, 0).max_key, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);
            <b>return</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(children);
        };

        <b>let</b> is_leaf = node.is_leaf;

        <b>let</b> index = <a href="btree_map.md#0x1_btree_map_binary_search">binary_search</a>(key, children, 0, current_size);

        <b>assert</b>!(index &lt; current_size, <a href="btree_map.md#0x1_btree_map_E_INTERNAL">E_INTERNAL</a>);

        <b>let</b> max_key_updated = index == (current_size - 1);
        <b>let</b> old_child = <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(children, index);
        current_size = current_size - 1;

        <b>let</b> big_enough = current_size * 2 &gt;= (tree.order <b>as</b> u64);
        <b>if</b> (!max_key_updated && big_enough) {
            <b>return</b> old_child;
        };

        <b>if</b> (!big_enough && node_index == tree.root_index) {
            // promote only child <b>to</b> root, and drop current root.
            <b>if</b> (current_size == 1 && !is_leaf) {
                <b>let</b> Child::Inner {
                    node_index: inner_child_index,
                    max_key: _,
                } = <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(children);
                tree.root_index = inner_child_index;
                <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, tree.root_index).parent = <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>;
                <a href="btree_map.md#0x1_btree_map_destroy_empty_node">destroy_empty_node</a>(<a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> tree.nodes, node_index));
            } <b>else</b> {
                // nothing <b>to</b> change
            };
            <b>return</b> old_child;
        };

        <b>if</b> (max_key_updated) {
            <b>let</b> new_max_key = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, current_size - 1).max_key;
            <b>let</b> parent = node.parent;

            <a href="btree_map.md#0x1_btree_map_update_key">update_key</a>(tree, parent, key, new_max_key);

            <b>if</b> (big_enough) {
                <b>return</b> old_child;
            }
        };

        (old_child, current_size)
    };

    // We need <b>to</b> <b>update</b> tree beyond the current node

    <b>let</b> node = <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> tree.nodes, node_index);

    <b>let</b> prev = node.prev;
    <b>let</b> next = node.next;
    <b>let</b> parent = node.parent;
    <b>let</b> is_leaf = node.is_leaf;

    <b>let</b> children = &<b>mut</b> node.children;

    // Children size is below threshold, we need <b>to</b> rebalance

    <b>let</b> brother_index = next;
    <b>if</b> (brother_index == <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a> || <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&tree.nodes, brother_index).parent != parent) {
        brother_index = prev;
    };
    <b>let</b> brother_node = <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> tree.nodes, brother_index);
    <b>let</b> brother_children = &<b>mut</b> brother_node.children;
    <b>let</b> brother_size = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(brother_children);

    <b>if</b> ((brother_size - 1) * 2 &gt;= (tree.order <b>as</b> u64)) {
        // The brother node <b>has</b> enough elements, borrow an element from the brother node.
        brother_size = brother_size - 1;
        <b>if</b> (brother_index == next) {
            <b>let</b> borrowed_element = <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(brother_children, 0);
            <b>if</b> (borrowed_element is Child::Inner&lt;K, V&gt;) {
                <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, borrowed_element.node_index).parent = node_index;
            };
            <b>let</b> borrowed_max_key = borrowed_element.max_key;
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(children, borrowed_element);
            <a href="btree_map.md#0x1_btree_map_update_key">update_key</a>(tree, parent, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, current_size - 2).max_key, borrowed_max_key);
        } <b>else</b> {
            <b>let</b> borrowed_element = <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(brother_children);
            <b>if</b> (borrowed_element is Child::Inner&lt;K, V&gt;) {
                <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, borrowed_element.node_index).parent = node_index;
            };
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(children, 0, borrowed_element);
            <a href="btree_map.md#0x1_btree_map_update_key">update_key</a>(tree, parent, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, 0).max_key, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(brother_children, brother_size - 1).max_key);
        };

        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, node_index, node);
        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, brother_index, brother_node);
        <b>return</b> old_child;
    };

    // The brother node doesn't have enough elements <b>to</b> borrow, merge <b>with</b> the brother node.
    <b>if</b> (brother_index == next) {
        <b>if</b> (!is_leaf) {
            <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(children);
            <b>let</b> i = 0;
            <b>while</b> (i &lt; len) {
                <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, i).node_index).parent = brother_index;
                i = i + 1;
            };
        };
        <b>let</b> <a href="btree_map.md#0x1_btree_map_Node">Node</a> { children: brother_children, is_leaf: _, parent: _, prev: _, next: brother_next } = brother_node;
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(children, brother_children);
        node.next = brother_next;
        <b>let</b> key_to_remove = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(children, current_size - 1).max_key;

        <b>move</b> children;

        <b>if</b> (node.next != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, node.next).prev = brother_index;
        };
        <b>if</b> (node.prev != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, node.prev).next = brother_index;
        };

        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, brother_index, node);
        <b>if</b> (tree.min_leaf_index == node_index) {
            tree.min_leaf_index = brother_index;
        };

        <b>if</b> (parent != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <a href="btree_map.md#0x1_btree_map_destroy_inner_child">destroy_inner_child</a>(<a href="btree_map.md#0x1_btree_map_remove_at">remove_at</a>(tree, parent, key_to_remove));
        };
    } <b>else</b> {
        <b>if</b> (!is_leaf) {
            <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(brother_children);
            <b>let</b> i = 0;
            <b>while</b> (i &lt; len) {
                <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(brother_children, i).node_index).parent = node_index;
                i = i + 1;
            };
        };
        <b>let</b> <a href="btree_map.md#0x1_btree_map_Node">Node</a> { children: node_children, is_leaf: _, parent: _, prev: _, next: node_next } = node;
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(brother_children, node_children);
        brother_node.next = node_next;
        <b>let</b> key_to_remove = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(brother_children, brother_size - 1).max_key;

        <b>move</b> brother_children;

        <b>if</b> (brother_node.next != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, brother_node.next).prev = node_index;
        };
        <b>if</b> (brother_node.prev != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> tree.nodes, brother_node.prev).next = node_index;
        };

        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> tree.nodes, node_index, brother_node);
        <b>if</b> (tree.min_leaf_index == brother_index) {
            tree.min_leaf_index = node_index;
        };

        <b>if</b> (parent != <a href="btree_map.md#0x1_btree_map_NULL_INDEX">NULL_INDEX</a>) {
            <a href="btree_map.md#0x1_btree_map_destroy_inner_child">destroy_inner_child</a>(<a href="btree_map.md#0x1_btree_map_remove_at">remove_at</a>(tree, parent, key_to_remove));
        };
    };
    old_child
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
