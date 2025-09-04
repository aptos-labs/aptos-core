
<a id="0x1_ordered_map"></a>

# Module `0x1::ordered_map`

This module provides an implementation for an ordered map.

Keys point to values, and each key in the map must be unique.

Currently, one implementation is provided, backed by a single sorted vector.

That means that keys can be found within O(log N) time.
Adds and removals take O(N) time, but the constant factor is small,
as it does only O(log N) comparisons, and does efficient mem-copy with vector operations.

Additionally, it provides a way to lookup and iterate over sorted keys, making range query
take O(log N + R) time (where R is number of elements in the range).

Most methods operate with OrderedMap being <code>self</code>.
All methods that start with iter_*, operate with IteratorPtr being <code>self</code>.

Uses cmp::compare for ordering, which compares primitive types natively, and uses common
lexicographical sorting for complex types.

TODO: all iterator functions are public(friend) for now, so that they can be modified in a
backward incompatible way. Type is also named IteratorPtr, so that Iterator is free to use later.
They are waiting for Move improvement that will allow references to be part of the struct,
allowing cleaner iterator APIs.


-  [Struct `Entry`](#0x1_ordered_map_Entry)
-  [Enum `OrderedMap`](#0x1_ordered_map_OrderedMap)
-  [Enum `IteratorPtr`](#0x1_ordered_map_IteratorPtr)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_ordered_map_new)
-  [Function `new_from`](#0x1_ordered_map_new_from)
-  [Function `length`](#0x1_ordered_map_length)
-  [Function `is_empty`](#0x1_ordered_map_is_empty)
-  [Function `add`](#0x1_ordered_map_add)
-  [Function `upsert`](#0x1_ordered_map_upsert)
-  [Function `remove`](#0x1_ordered_map_remove)
-  [Function `contains`](#0x1_ordered_map_contains)
-  [Function `borrow`](#0x1_ordered_map_borrow)
-  [Function `borrow_mut`](#0x1_ordered_map_borrow_mut)
-  [Function `replace_key_inplace`](#0x1_ordered_map_replace_key_inplace)
-  [Function `add_all`](#0x1_ordered_map_add_all)
-  [Function `upsert_all`](#0x1_ordered_map_upsert_all)
-  [Function `append`](#0x1_ordered_map_append)
-  [Function `append_disjoint`](#0x1_ordered_map_append_disjoint)
-  [Function `append_impl`](#0x1_ordered_map_append_impl)
-  [Function `trim`](#0x1_ordered_map_trim)
-  [Function `borrow_front`](#0x1_ordered_map_borrow_front)
-  [Function `borrow_back`](#0x1_ordered_map_borrow_back)
-  [Function `pop_front`](#0x1_ordered_map_pop_front)
-  [Function `pop_back`](#0x1_ordered_map_pop_back)
-  [Function `prev_key`](#0x1_ordered_map_prev_key)
-  [Function `next_key`](#0x1_ordered_map_next_key)
-  [Function `lower_bound`](#0x1_ordered_map_lower_bound)
-  [Function `find`](#0x1_ordered_map_find)
-  [Function `new_begin_iter`](#0x1_ordered_map_new_begin_iter)
-  [Function `new_end_iter`](#0x1_ordered_map_new_end_iter)
-  [Function `iter_next`](#0x1_ordered_map_iter_next)
-  [Function `iter_prev`](#0x1_ordered_map_iter_prev)
-  [Function `iter_is_begin`](#0x1_ordered_map_iter_is_begin)
-  [Function `iter_is_begin_from_non_empty`](#0x1_ordered_map_iter_is_begin_from_non_empty)
-  [Function `iter_is_end`](#0x1_ordered_map_iter_is_end)
-  [Function `iter_borrow_key`](#0x1_ordered_map_iter_borrow_key)
-  [Function `iter_borrow`](#0x1_ordered_map_iter_borrow)
-  [Function `iter_borrow_mut`](#0x1_ordered_map_iter_borrow_mut)
-  [Function `iter_remove`](#0x1_ordered_map_iter_remove)
-  [Function `iter_replace`](#0x1_ordered_map_iter_replace)
-  [Function `iter_add`](#0x1_ordered_map_iter_add)
-  [Function `destroy_empty`](#0x1_ordered_map_destroy_empty)
-  [Function `keys`](#0x1_ordered_map_keys)
-  [Function `values`](#0x1_ordered_map_values)
-  [Function `to_vec_pair`](#0x1_ordered_map_to_vec_pair)
-  [Function `destroy`](#0x1_ordered_map_destroy)
-  [Function `for_each`](#0x1_ordered_map_for_each)
-  [Function `for_each_ref`](#0x1_ordered_map_for_each_ref)
-  [Function `for_each_ref_friend`](#0x1_ordered_map_for_each_ref_friend)
-  [Function `for_each_mut`](#0x1_ordered_map_for_each_mut)
-  [Function `new_iter`](#0x1_ordered_map_new_iter)
-  [Function `binary_search`](#0x1_ordered_map_binary_search)
-  [Specification](#@Specification_1)
    -  [Enum `OrderedMap`](#@Specification_1_OrderedMap)
    -  [Function `new`](#@Specification_1_new)
    -  [Function `new_from`](#@Specification_1_new_from)
    -  [Function `length`](#@Specification_1_length)
    -  [Function `is_empty`](#@Specification_1_is_empty)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `upsert`](#@Specification_1_upsert)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `borrow_mut`](#@Specification_1_borrow_mut)
    -  [Function `replace_key_inplace`](#@Specification_1_replace_key_inplace)
    -  [Function `add_all`](#@Specification_1_add_all)
    -  [Function `upsert_all`](#@Specification_1_upsert_all)
    -  [Function `append`](#@Specification_1_append)
    -  [Function `append_disjoint`](#@Specification_1_append_disjoint)
    -  [Function `append_impl`](#@Specification_1_append_impl)
    -  [Function `trim`](#@Specification_1_trim)
    -  [Function `borrow_front`](#@Specification_1_borrow_front)
    -  [Function `borrow_back`](#@Specification_1_borrow_back)
    -  [Function `pop_front`](#@Specification_1_pop_front)
    -  [Function `pop_back`](#@Specification_1_pop_back)
    -  [Function `prev_key`](#@Specification_1_prev_key)
    -  [Function `next_key`](#@Specification_1_next_key)
    -  [Function `lower_bound`](#@Specification_1_lower_bound)
    -  [Function `find`](#@Specification_1_find)
    -  [Function `new_begin_iter`](#@Specification_1_new_begin_iter)
    -  [Function `new_end_iter`](#@Specification_1_new_end_iter)
    -  [Function `iter_next`](#@Specification_1_iter_next)
    -  [Function `iter_prev`](#@Specification_1_iter_prev)
    -  [Function `iter_is_begin`](#@Specification_1_iter_is_begin)
    -  [Function `iter_is_begin_from_non_empty`](#@Specification_1_iter_is_begin_from_non_empty)
    -  [Function `iter_is_end`](#@Specification_1_iter_is_end)
    -  [Function `iter_borrow_key`](#@Specification_1_iter_borrow_key)
    -  [Function `iter_borrow`](#@Specification_1_iter_borrow)
    -  [Function `iter_borrow_mut`](#@Specification_1_iter_borrow_mut)
    -  [Function `iter_remove`](#@Specification_1_iter_remove)
    -  [Function `iter_replace`](#@Specification_1_iter_replace)
    -  [Function `iter_add`](#@Specification_1_iter_add)
    -  [Function `destroy_empty`](#@Specification_1_destroy_empty)
    -  [Function `keys`](#@Specification_1_keys)
    -  [Function `values`](#@Specification_1_values)
    -  [Function `to_vec_pair`](#@Specification_1_to_vec_pair)
    -  [Function `binary_search`](#@Specification_1_binary_search)


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp">0x1::cmp</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_ordered_map_Entry"></a>

## Struct `Entry`

Individual entry holding (key, value) pair


<pre><code><b>struct</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K, V&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: K</code>
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

<a id="0x1_ordered_map_OrderedMap"></a>

## Enum `OrderedMap`

The OrderedMap datastructure.


<pre><code>enum <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>SortedVectorMap</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>
 List of entries, sorted by key.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_ordered_map_IteratorPtr"></a>

## Enum `IteratorPtr`

An iterator pointing to a valid position in an ordered map, or to the end.

TODO: Once fields can be (mutable) references, this class will be deprecated.


<pre><code>enum <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> <b>has</b> <b>copy</b>, drop
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
<summary>Position</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>index: u64</code>
</dt>
<dd>
 The index of the iterator pointing to.
</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ordered_map_EITER_OUT_OF_BOUNDS"></a>



<pre><code><b>const</b> <a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>: u64 = 3;
</code></pre>



<a id="0x1_ordered_map_EKEY_ALREADY_EXISTS"></a>

Map key already exists


<pre><code><b>const</b> <a href="ordered_map.md#0x1_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x1_ordered_map_EKEY_NOT_FOUND"></a>

Map key is not found


<pre><code><b>const</b> <a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>: u64 = 2;
</code></pre>



<a id="0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER"></a>

New key used in replace_key_inplace doesn't respect the order


<pre><code><b>const</b> <a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>: u64 = 4;
</code></pre>



<a id="0x1_ordered_map_new"></a>

## Function `new`

Create a new empty OrderedMap, using default (SortedVectorMap) implementation.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new">new</a>&lt;K, V&gt;(): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new">new</a>&lt;K, V&gt;(): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; {
    OrderedMap::SortedVectorMap {
        entries: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_new_from"></a>

## Function `new_from`

Create a OrderedMap from a vector of keys and values.
Aborts with EKEY_ALREADY_EXISTS if duplicate keys are passed in.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_from">new_from</a>&lt;K, V&gt;(keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_from">new_from</a>&lt;K, V&gt;(keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; {
    <b>let</b> map = <a href="ordered_map.md#0x1_ordered_map_new">new</a>();
    map.<a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>(keys, values);
    map
}
</code></pre>



</details>

<a id="0x1_ordered_map_length"></a>

## Function `length`

Number of elements in the map.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_length">length</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_length">length</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): u64 {
    self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>()
}
</code></pre>



</details>

<a id="0x1_ordered_map_is_empty"></a>

## Function `is_empty`

Whether map is empty.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): bool {
    self.entries.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()
}
</code></pre>



</details>

<a id="0x1_ordered_map_add"></a>

## Function `add`

Add a key/value pair to the map.
Aborts with EKEY_ALREADY_EXISTS if key already exist.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add">add</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add">add</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: K, value: V) {
    <b>let</b> len = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();
    <b>let</b> index = <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>(&key, &self.entries, 0, len);

    // key must not already be inside.
    <b>assert</b>!(index &gt;= len || &self.entries[index].key != &key, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
    self.entries.insert(index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
}
</code></pre>



</details>

<a id="0x1_ordered_map_upsert"></a>

## Function `upsert`

If the key doesn't exist in the map, inserts the key/value, and returns none.
Otherwise, updates the value under the given key, and returns the old value.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert">upsert</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert">upsert</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: K, value: V): Option&lt;V&gt; {
    <b>let</b> len = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();
    <b>let</b> index = <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>(&key, &self.entries, 0, len);

    <b>if</b> (index &lt; len && &self.entries[index].key == &key) {
        <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> {
            key: _,
            value: old_value,
        } = self.entries.replace(index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
        <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(old_value)
    } <b>else</b> {
        self.entries.insert(index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
        <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_remove"></a>

## Function `remove`

Remove a key/value pair from the map.
Aborts with EKEY_NOT_FOUND if <code>key</code> doesn't exist.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_remove">remove</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_remove">remove</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): V {
    <b>let</b> len = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();
    <b>let</b> index = <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>(key, &self.entries, 0, len);
    <b>assert</b>!(index &lt; len, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key: old_key, value } = self.entries.<a href="ordered_map.md#0x1_ordered_map_remove">remove</a>(index);
    <b>assert</b>!(key == &old_key, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    value
}
</code></pre>



</details>

<a id="0x1_ordered_map_contains"></a>

## Function `contains`

Returns whether map contains a given key.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_contains">contains</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_contains">contains</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): bool {
    !self.<a href="ordered_map.md#0x1_ordered_map_find">find</a>(key).<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)
}
</code></pre>



</details>

<a id="0x1_ordered_map_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): &V {
    self.<a href="ordered_map.md#0x1_ordered_map_find">find</a>(key).<a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>(self)
}
</code></pre>



</details>

<a id="0x1_ordered_map_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V {
    self.<a href="ordered_map.md#0x1_ordered_map_find">find</a>(key).<a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>(self)
}
</code></pre>



</details>

<a id="0x1_ordered_map_replace_key_inplace"></a>

## Function `replace_key_inplace`

Changes the key, while keeping the same value attached to it
Aborts with EKEY_NOT_FOUND if <code>old_key</code> doesn't exist.
Aborts with ENEW_KEY_NOT_IN_ORDER if <code>new_key</code> doesn't keep the order <code>old_key</code> was in.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_replace_key_inplace">replace_key_inplace</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, old_key: &K, new_key: K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_replace_key_inplace">replace_key_inplace</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, old_key: &K, new_key: K) {
    <b>let</b> len = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();
    <b>let</b> index = <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>(old_key, &self.entries, 0, len);
    <b>assert</b>!(index &lt; len, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));

    <b>assert</b>!(old_key == &self.entries[index].key, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));

    // check that after we <b>update</b> the key, order is going <b>to</b> be respected
    <b>if</b> (index &gt; 0) {
        <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&self.entries[index - 1].key, &new_key).is_lt(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>))
    };

    <b>if</b> (index + 1 &lt; len) {
        <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&new_key, &self.entries[index + 1].key).is_lt(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>))
    };

    self.entries[index].key = new_key;
}
</code></pre>



</details>

<a id="0x1_ordered_map_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the map. The keys must not already exist.
Aborts with EKEY_ALREADY_EXISTS if key already exist, or duplicate keys are passed in.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    // TODO: Can be optimized, by sorting keys and values, and then creating map.
    keys.zip(values, |key, value| {
        self.<a href="ordered_map.md#0x1_ordered_map_add">add</a>(key, value);
    });
}
</code></pre>



</details>

<a id="0x1_ordered_map_upsert_all"></a>

## Function `upsert_all`

Add multiple key/value pairs to the map, overwrites values if they exist already,
or if duplicate keys are passed in.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert_all">upsert_all</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert_all">upsert_all</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    // TODO: Can be optimized, by sorting keys and values, and then creating map.
    keys.zip(values, |key, value| {
        self.<a href="ordered_map.md#0x1_ordered_map_upsert">upsert</a>(key, value);
    });
}
</code></pre>



</details>

<a id="0x1_ordered_map_append"></a>

## Function `append`

Takes all elements from <code>other</code> and adds them to <code>self</code>,
overwritting if any key is already present in self.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append">append</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append">append</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;) {
    self.<a href="ordered_map.md#0x1_ordered_map_append_impl">append_impl</a>(other);
}
</code></pre>



</details>

<a id="0x1_ordered_map_append_disjoint"></a>

## Function `append_disjoint`

Takes all elements from <code>other</code> and adds them to <code>self</code>.
Aborts with EKEY_ALREADY_EXISTS if <code>other</code> has a key already present in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append_disjoint">append_disjoint</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append_disjoint">append_disjoint</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;) {
    <b>let</b> overwritten = self.<a href="ordered_map.md#0x1_ordered_map_append_impl">append_impl</a>(other);
    <b>assert</b>!(overwritten.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() == 0, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
    overwritten.<a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_ordered_map_append_impl"></a>

## Function `append_impl`

Takes all elements from <code>other</code> and adds them to <code>self</code>, returning list of entries in self that were overwritten.


<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append_impl">append_impl</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append_impl">append_impl</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K,V&gt;&gt; {
    <b>let</b> OrderedMap::SortedVectorMap {
        entries: other_entries,
    } = other;
    <b>let</b> overwritten = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    <b>if</b> (other_entries.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        other_entries.<a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>();
        <b>return</b> overwritten;
    };

    <b>if</b> (self.entries.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        self.entries.<a href="ordered_map.md#0x1_ordered_map_append">append</a>(other_entries);
        <b>return</b> overwritten;
    };

    // Optimization: <b>if</b> all elements in `other` are larger than all elements in `self`, we can just <b>move</b> them over.
    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&self.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1).key, &other_entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(0).key).is_lt()) {
        self.entries.<a href="ordered_map.md#0x1_ordered_map_append">append</a>(other_entries);
        <b>return</b> overwritten;
    };

    // In O(n), traversing from the back, build reverse sorted result, and then reverse it back
    <b>let</b> reverse_result = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> cur_i = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1;
    <b>let</b> other_i = other_entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1;

    // after the end of the <b>loop</b>, other_entries is empty, and <a href="../../velor-stdlib/doc/any.md#0x1_any">any</a> leftover is in entries
    <b>loop</b> {
        <b>let</b> ord = <a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&self.entries[cur_i].key, &other_entries[other_i].key);
        <b>if</b> (ord.is_gt()) {
            reverse_result.push_back(self.entries.<a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>());
            <b>if</b> (cur_i == 0) {
                // make other_entries empty, and rest in entries.
                // TODO cannot <b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/mem.md#0x1_mem_swap">mem::swap</a> until it is <b>public</b>/released
                // <a href="../../velor-stdlib/../move-stdlib/doc/mem.md#0x1_mem_swap">mem::swap</a>(&<b>mut</b> self.entries, &<b>mut</b> other_entries);
                self.entries.<a href="ordered_map.md#0x1_ordered_map_append">append</a>(other_entries);
                <b>break</b>;
            } <b>else</b> {
                cur_i -= 1;
            };
        } <b>else</b> {
            // is_lt or is_eq
            <b>if</b> (ord.is_eq()) {
                // we skip the entries one, and below put in the result one from other.
                overwritten.push_back(self.entries.<a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>());

                <b>if</b> (cur_i == 0) {
                    // make other_entries empty, and rest in entries.
                    // TODO cannot <b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/mem.md#0x1_mem_swap">mem::swap</a> until it is <b>public</b>/released
                    // <a href="../../velor-stdlib/../move-stdlib/doc/mem.md#0x1_mem_swap">mem::swap</a>(&<b>mut</b> self.entries, &<b>mut</b> other_entries);
                    self.entries.<a href="ordered_map.md#0x1_ordered_map_append">append</a>(other_entries);
                    <b>break</b>;
                } <b>else</b> {
                    cur_i -= 1;
                };
            };

            reverse_result.push_back(other_entries.<a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>());
            <b>if</b> (other_i == 0) {
                other_entries.<a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>();
                <b>break</b>;
            } <b>else</b> {
                other_i -= 1;
            };
        };
    };

    self.entries.reverse_append(reverse_result);

    overwritten
}
</code></pre>



</details>

<a id="0x1_ordered_map_trim"></a>

## Function `trim`

Splits the collection into two, such to leave <code>self</code> with <code>at</code> number of elements.
Returns a newly allocated map containing the elements in the range [at, len).
After the call, the original map will be left containing the elements [0, at).


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_trim">trim</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, at: u64): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_trim">trim</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, at: u64): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; {
    <b>let</b> rest = self.entries.<a href="ordered_map.md#0x1_ordered_map_trim">trim</a>(at);

    OrderedMap::SortedVectorMap {
        entries: rest
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_borrow_front"></a>

## Function `borrow_front`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_front">borrow_front</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (&K, &V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_front">borrow_front</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): (&K, &V) {
    <b>let</b> entry = self.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(0);
    (&entry.key, &entry.value)
}
</code></pre>



</details>

<a id="0x1_ordered_map_borrow_back"></a>

## Function `borrow_back`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_back">borrow_back</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (&K, &V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_back">borrow_back</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): (&K, &V) {
    <b>let</b> entry = self.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1);
    (&entry.key, &entry.value)
}
</code></pre>



</details>

<a id="0x1_ordered_map_pop_front"></a>

## Function `pop_front`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_pop_front">pop_front</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_pop_front">pop_front</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): (K, V) {
    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value } = self.entries.<a href="ordered_map.md#0x1_ordered_map_remove">remove</a>(0);
    (key, value)
}
</code></pre>



</details>

<a id="0x1_ordered_map_pop_back"></a>

## Function `pop_back`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (K, V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): (K, V) {
    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value } = self.entries.<a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>();
    (key, value)
}
</code></pre>



</details>

<a id="0x1_ordered_map_prev_key"></a>

## Function `prev_key`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_prev_key">prev_key</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_prev_key">prev_key</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): Option&lt;K&gt; {
    <b>let</b> it = self.<a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>(key);
    <b>if</b> (it.<a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>(self)) {
        <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*it.<a href="ordered_map.md#0x1_ordered_map_iter_prev">iter_prev</a>(self).<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self))
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_next_key"></a>

## Function `next_key`



<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_next_key">next_key</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_next_key">next_key</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): Option&lt;K&gt; {
    <b>let</b> it = self.<a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>(key);
    <b>if</b> (it.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <b>let</b> cur_key = it.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self);
        <b>if</b> (key == cur_key) {
            <b>let</b> it = it.<a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>(self);
            <b>if</b> (it.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
                <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
            } <b>else</b> {
                <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*it.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self))
            }
        } <b>else</b> {
            <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*cur_key)
        }
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_lower_bound"></a>

## Function `lower_bound`

Returns an iterator pointing to the first element that is greater or equal to the provided
key, or an end iterator if such element doesn't exist.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    <b>let</b> entries = &self.entries;
    <b>let</b> len = entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();

    <b>let</b> index = <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>(key, entries, 0, len);
    <b>if</b> (index == len) {
        self.<a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>()
    } <b>else</b> {
        <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index)
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_find"></a>

## Function `find`

Returns an iterator pointing to the element that equals to the provided key, or an end
iterator if the key is not found.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_find">find</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_find">find</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    <b>let</b> lower_bound = self.<a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>(key);
    <b>if</b> (lower_bound.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        lower_bound
    } <b>else</b> <b>if</b> (lower_bound.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self) == key) {
        lower_bound
    } <b>else</b> {
        self.<a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>()
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_new_begin_iter"></a>

## Function `new_begin_iter`

Returns the begin iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    <b>if</b> (self.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        <b>return</b> IteratorPtr::End;
    };

    <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(0)
}
</code></pre>



</details>

<a id="0x1_ordered_map_new_end_iter"></a>

## Function `new_end_iter`

Returns the end iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    IteratorPtr::End
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_next"></a>

## Function `iter_next`

Returns the next iterator, or none if already at the end iterator.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    <b>assert</b>!(!self.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(map), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    <b>let</b> index = self.index + 1;
    <b>if</b> (index &lt; map.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>()) {
        <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index)
    } <b>else</b> {
        map.<a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>()
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_prev"></a>

## Function `iter_prev`

Returns the previous iterator, or none if already at the begin iterator.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_prev">iter_prev</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_prev">iter_prev</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    <b>assert</b>!(!self.<a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>(map), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    <b>let</b> index = <b>if</b> (self is IteratorPtr::End) {
        map.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1
    } <b>else</b> {
        self.index - 1
    };

    <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index)
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_is_begin"></a>

## Function `iter_is_begin`

Returns whether the iterator is a begin iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): bool {
    <b>if</b> (self is IteratorPtr::End) {
        map.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()
    } <b>else</b> {
        self.index == 0
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_is_begin_from_non_empty"></a>

## Function `iter_is_begin_from_non_empty`

Returns true iff the iterator is a begin iterator from a non-empty collection.
(I.e. if iterator points to a valid element)
This method doesn't require having access to map, unlike iter_is_begin.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin_from_non_empty">iter_is_begin_from_non_empty</a>(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin_from_non_empty">iter_is_begin_from_non_empty</a>(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>): bool {
    <b>if</b> (self is IteratorPtr::End) {
        <b>false</b>
    } <b>else</b> {
        self.index == 0
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_is_end"></a>

## Function `iter_is_end`

Returns whether the iterator is an end iterator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, _map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, _map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): bool {
    self is IteratorPtr::End
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_borrow_key"></a>

## Function `iter_borrow_key`

Borrows the key given iterator points to.
Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &K
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): &K {
    <b>assert</b>!(!(self is IteratorPtr::End), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    &map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.index).key
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_borrow"></a>

## Function `iter_borrow`

Borrows the value given iterator points to.
Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): &V {
    <b>assert</b>!(!(self is IteratorPtr::End), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));
    &map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.index).value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_borrow_mut"></a>

## Function `iter_borrow_mut`

Mutably borrows the value iterator points to.
Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): &<b>mut</b> V {
    <b>assert</b>!(!(self is IteratorPtr::End), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));
    &<b>mut</b> map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>(self.index).value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_remove"></a>

## Function `iter_remove`

Removes (key, value) pair iterator points to, returning the previous value.
Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_remove">iter_remove</a>&lt;K: drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_remove">iter_remove</a>&lt;K: drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): V {
    <b>assert</b>!(!(self is IteratorPtr::End), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key: _, value } = map.entries.<a href="ordered_map.md#0x1_ordered_map_remove">remove</a>(self.index);
    value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_replace"></a>

## Function `iter_replace`

Replaces the value iterator is pointing to, returning the previous value.
Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_replace">iter_replace</a>&lt;K: <b>copy</b>, drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, value: V): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_replace">iter_replace</a>&lt;K: <b>copy</b> + drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, value: V): V {
    <b>assert</b>!(!(self is IteratorPtr::End), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    // TODO once <a href="../../velor-stdlib/../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a> is <b>public</b>/released, <b>update</b> <b>to</b>:
    // <b>let</b> entry = map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>(self.index);
    // <a href="../../velor-stdlib/../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(&<b>mut</b> entry.value, value)
    <b>let</b> key = map.entries[self.index].key;
    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> {
        key: _,
        value: prev_value,
    } = map.entries.replace(self.index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
    prev_value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_add"></a>

## Function `iter_add`

Add key/value pair to the map, at the iterator position (before the element at the iterator position).
Aborts with ENEW_KEY_NOT_IN_ORDER is key is not larger than the key before the iterator,
or smaller than the key at the iterator position.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_add">iter_add</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_add">iter_add</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: K, value: V) {
    <b>let</b> len = map.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();
    <b>let</b> insert_index = <b>if</b> (self is IteratorPtr::End) {
        len
    } <b>else</b> {
        self.index
    };

    <b>if</b> (insert_index &gt; 0) {
        <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&map.entries[insert_index - 1].key, &key).is_lt(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>))
    };

    <b>if</b> (insert_index &lt; len) {
        <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&key, &map.entries[insert_index].key).is_lt(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>))
    };

    map.entries.insert(insert_index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
}
</code></pre>



</details>

<a id="0x1_ordered_map_destroy_empty"></a>

## Function `destroy_empty`

Destroys empty map.
Aborts if <code>self</code> is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;) {
    <b>let</b> OrderedMap::SortedVectorMap { entries } = self;
    // <b>assert</b>!(entries.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>(), E_NOT_EMPTY);
    entries.<a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_ordered_map_keys"></a>

## Function `keys`

Return all keys in the map. This requires keys to be copyable.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    self.entries.map_ref(|e| {
        <b>let</b> e: &<a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K, V&gt; = e;
        e.key
    })
}
</code></pre>



</details>

<a id="0x1_ordered_map_values"></a>

## Function `values`

Return all values in the map. This requires values to be copyable.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_values">values</a>&lt;K, V: <b>copy</b>&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_values">values</a>&lt;K, V: <b>copy</b>&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt; {
    self.entries.map_ref(|e| {
        <b>let</b> e: &<a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K, V&gt; = e;
        e.value
    })
}
</code></pre>



</details>

<a id="0x1_ordered_map_to_vec_pair"></a>

## Function `to_vec_pair`

Transform the map into two vectors with the keys and values respectively
Primarily used to destroy a map


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): (<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    <b>let</b> keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt; = <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> OrderedMap::SortedVectorMap { entries } = self;
    entries.<a href="ordered_map.md#0x1_ordered_map_for_each">for_each</a>(|e| {
        <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value } = e;
        keys.push_back(key);
        values.push_back(value);
    });
    (keys, values)
}
</code></pre>



</details>

<a id="0x1_ordered_map_destroy"></a>

## Function `destroy`

For maps that cannot be dropped this is a utility to destroy them
using lambdas to destroy the individual keys and values.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_destroy">destroy</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, dk: |K|, dv: |V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_destroy">destroy</a>&lt;K, V&gt;(
    self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;,
    dk: |K|,
    dv: |V|
) {
    <b>let</b> (keys, values) = self.<a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>();
    keys.<a href="ordered_map.md#0x1_ordered_map_destroy">destroy</a>(|_k| dk(_k));
    values.<a href="ordered_map.md#0x1_ordered_map_destroy">destroy</a>(|_v| dv(_v));
}
</code></pre>



</details>

<a id="0x1_ordered_map_for_each"></a>

## Function `for_each`

Apply the function to each key-value pair in the map, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each">for_each</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, f: |K, V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each">for_each</a>&lt;K, V&gt;(
    self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;,
    f: |K, V|
) {
    <b>let</b> (keys, values) = self.<a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>();
    keys.zip(values, |k, v| f(k, v));
}
</code></pre>



</details>

<a id="0x1_ordered_map_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each key-value pair in the map.

Current implementation is O(n * log(n)). After function values will be optimized
to O(n).


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_ref">for_each_ref</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, f: |&K, &V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_ref">for_each_ref</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, f: |&K, &V|) {
    // This implementation is innefficient: O(log(n)) for next_key / borrow lookups every time,
    // but is the only one available through the <b>public</b> API.
    <b>if</b> (!self.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        <b>let</b> (k, v) = self.<a href="ordered_map.md#0x1_ordered_map_borrow_front">borrow_front</a>();
        f(k, v);

        <b>let</b> cur_k = self.<a href="ordered_map.md#0x1_ordered_map_next_key">next_key</a>(k);
        <b>while</b> (cur_k.is_some()) {
            <b>let</b> k = cur_k.destroy_some();
            f(&k, self.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(&k));

            cur_k = self.<a href="ordered_map.md#0x1_ordered_map_next_key">next_key</a>(&k);
        };
    };

    // TODO: <b>if</b> we make iterator api <b>public</b> <b>update</b> <b>to</b>:
    // <b>let</b> iter = self.<a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>();
    // <b>while</b> (!iter.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
    //     f(iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self), iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>(self));
    //     iter = iter.<a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>(self);
    // }

    // TODO: once <b>move</b> supports private functions udpate <b>to</b>:
    // <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(
    //     &self.entries,
    //     |entry| {
    //         f(&entry.key, &entry.value)
    //     }
    // );
}
</code></pre>



</details>

<a id="0x1_ordered_map_for_each_ref_friend"></a>

## Function `for_each_ref_friend`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_ref_friend">for_each_ref_friend</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, f: |&K, &V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_ref_friend">for_each_ref_friend</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, f: |&K, &V|) {
    <b>let</b> iter = self.<a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>();
    <b>while</b> (!iter.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        f(iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self), iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>(self));
        iter = iter.<a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>(self);
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference of each key-value pair in the map.

Current implementation is O(n * log(n)). After function values will be optimized
to O(n).


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_mut">for_each_mut</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, f: |&K, &<b>mut</b> V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_mut">for_each_mut</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, f: |&K, &<b>mut</b> V|) {
    // This implementation is innefficient: O(log(n)) for next_key / borrow lookups every time,
    // but is the only one available through the <b>public</b> API.
    <b>if</b> (!self.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        <b>let</b> (k, _v) = self.<a href="ordered_map.md#0x1_ordered_map_borrow_front">borrow_front</a>();

        <b>let</b> k = *k;
        <b>let</b> done = <b>false</b>;
        <b>while</b> (!done) {
            f(&k, self.<a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>(&k));

            <b>let</b> cur_k = self.<a href="ordered_map.md#0x1_ordered_map_next_key">next_key</a>(&k);
            <b>if</b> (cur_k.is_some()) {
                k = cur_k.destroy_some();
            } <b>else</b> {
                done = <b>true</b>;
            }
        };
    };

    // TODO: <b>if</b> we make iterator api <b>public</b> <b>update</b> <b>to</b>:
    // <b>let</b> iter = self.<a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>();
    // <b>while</b> (!iter.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
    //     <b>let</b> key = *iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self);
    //     f(key, iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>(self));
    //     iter = iter.<a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>(self);
    // }

    // TODO: once <b>move</b> supports private functions udpate <b>to</b>:
    // <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_mut">vector::for_each_mut</a>(
    //     &<b>mut</b> self.entries,
    //     |entry| {
    //         f(&<b>mut</b> entry.key, &<b>mut</b> entry.value)
    //     }
    // );
}
</code></pre>



</details>

<a id="0x1_ordered_map_new_iter"></a>

## Function `new_iter`



<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index: u64): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index: u64): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">IteratorPtr</a> {
    IteratorPtr::Position {
        index: index,
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_binary_search"></a>

## Function `binary_search`



<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>&lt;K, V&gt;(key: &K, entries: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>&lt;K, V&gt;(key: &K, entries: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64 {
    <b>let</b> l = start;
    <b>let</b> r = end;
    <b>while</b> (l != r) {
        <b>let</b> mid = l + ((r - l) &gt;&gt; 1);
        <b>let</b> comparison = <a href="../../velor-stdlib/../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(mid).key, key);
        <b>if</b> (comparison.is_lt()) {
            l = mid + 1;
        } <b>else</b> {
            r = mid;
        };
    };
    l
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_OrderedMap"></a>

### Enum `OrderedMap`


<pre><code>enum <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<dl>

<details>
<summary>SortedVectorMap</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>
 List of entries, sorted by key.
</dd>
</dl>


</details>

</details>
</dl>



<pre><code><b>pragma</b> intrinsic = map,
    map_new = new,
    map_len = length,
    map_destroy_empty = destroy_empty,
    map_has_key = contains,
    map_add_no_override = add,
    map_borrow = borrow,
    map_borrow_mut = borrow_mut,
    map_spec_get = spec_get,
    map_spec_set = spec_set,
    map_spec_del = spec_remove,
    map_spec_len = spec_len,
    map_spec_has_key = spec_contains_key,
    map_is_empty = is_empty;
</code></pre>




<a id="0x1_ordered_map_spec_len"></a>


<pre><code><b>native</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>&lt;K, V&gt;(t: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): num;
</code></pre>




<a id="0x1_ordered_map_spec_contains_key"></a>


<pre><code><b>native</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>&lt;K, V&gt;(t: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, k: K): bool;
</code></pre>




<a id="0x1_ordered_map_spec_set"></a>


<pre><code><b>native</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_spec_set">spec_set</a>&lt;K, V&gt;(t: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, k: K, v: V): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;;
</code></pre>




<a id="0x1_ordered_map_spec_remove"></a>


<pre><code><b>native</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_spec_remove">spec_remove</a>&lt;K, V&gt;(t: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, k: K): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;;
</code></pre>




<a id="0x1_ordered_map_spec_get"></a>


<pre><code><b>native</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>&lt;K, V&gt;(t: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, k: K): V;
</code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new">new</a>&lt;K, V&gt;(): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_new_from"></a>

### Function `new_from`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_from">new_from</a>&lt;K, V&gt;(keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> [abstract] <b>exists</b> i in 0..len(keys), j in 0..len(keys) <b>where</b> i != j : keys[i] == keys[j];
<b>aborts_if</b> [abstract] len(keys) != len(values);
<b>ensures</b> [abstract] <b>forall</b> k: K {<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(result, k)} : <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(keys,k) &lt;==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(result, k);
<b>ensures</b> [abstract] <b>forall</b> i in 0..len(keys) : <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(result, keys[i]) == values[i];
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(result) == len(keys);
</code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_length">length</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): u64
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_is_empty"></a>

### Function `is_empty`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add">add</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert">upsert</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;V&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>ensures</b> [abstract] !<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), key) ==&gt; <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(result);
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, key);
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(self, key) == value;
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), key) ==&gt; ((<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(result)) && (<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result) == <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(<b>old</b>(
    self), key)));
<b>ensures</b> [abstract] !<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), key) ==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(<b>old</b>(self)) + 1 == <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(self);
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), key) ==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(<b>old</b>(self)) == <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(self);
<b>ensures</b> [abstract] <b>forall</b> k: K: <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), k) && k != key ==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(<b>old</b>(self), k) == <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(self, k);
<b>ensures</b> [abstract] <b>forall</b> k: K: <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), k) ==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k);
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_remove">remove</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): V
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> [abstract] !<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, key);
<b>ensures</b> [abstract] !<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, key);
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(<b>old</b>(self), key) == result;
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(<b>old</b>(self)) == <a href="ordered_map.md#0x1_ordered_map_spec_len">spec_len</a>(self) + 1;
<b>ensures</b> [abstract] <b>forall</b> k: K <b>where</b> k != key: <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k) ==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(self, k) == <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(<b>old</b>(self), k);
<b>ensures</b> [abstract] <b>forall</b> k: K <b>where</b> k != key: <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(<b>old</b>(self), k) == <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k);
</code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_contains">contains</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): &V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_replace_key_inplace"></a>

### Function `replace_key_inplace`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_replace_key_inplace">replace_key_inplace</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, old_key: &K, new_key: K)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_upsert_all"></a>

### Function `upsert_all`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert_all">upsert_all</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append">append</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_append_disjoint"></a>

### Function `append_disjoint`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append_disjoint">append_disjoint</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_append_impl"></a>

### Function `append_impl`


<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append_impl">append_impl</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_trim"></a>

### Function `trim`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_trim">trim</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, at: u64): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_front"></a>

### Function `borrow_front`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_front">borrow_front</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (&K, &V)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, result_1);
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(self, result_1) == result_2;
<b>ensures</b> [abstract] <b>forall</b> k: K <b>where</b> k != result_1: <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k) ==&gt;
std::cmp::compare(result_1, k) == std::cmp::Ordering::Less;
</code></pre>



<a id="@Specification_1_borrow_back"></a>

### Function `borrow_back`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_borrow_back">borrow_back</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (&K, &V)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, result_1);
<b>ensures</b> [abstract] <a href="ordered_map.md#0x1_ordered_map_spec_get">spec_get</a>(self, result_1) == result_2;
<b>ensures</b> [abstract] <b>forall</b> k: K <b>where</b> k != result_1: <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k) ==&gt;
std::cmp::compare(result_1, k) == std::cmp::Ordering::Greater;
</code></pre>



<a id="@Specification_1_pop_front"></a>

### Function `pop_front`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_pop_front">pop_front</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (K, V)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_pop_back">pop_back</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (K, V)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_prev_key"></a>

### Function `prev_key`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_prev_key">prev_key</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>ensures</b> [abstract] result == std::option::spec_none() &lt;==&gt;
(<b>forall</b> k: K {<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k)} <b>where</b> <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k)
&& k != key: std::cmp::compare(key, k) == std::cmp::Ordering::Less);
<b>ensures</b> [abstract] result.is_some() &lt;==&gt;
    <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result)) &&
    (std::cmp::compare(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result), key) == std::cmp::Ordering::Less)
    && (<b>forall</b> k: K {<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k), std::cmp::compare(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result), k), std::cmp::compare(key, k)} <b>where</b> k != <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result): ((<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k) &&
    std::cmp::compare(k, key) == std::cmp::Ordering::Less)) ==&gt;
    std::cmp::compare(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result), k) == std::cmp::Ordering::Greater);
</code></pre>



<a id="@Specification_1_next_key"></a>

### Function `next_key`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_next_key">next_key</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
<b>ensures</b> [abstract] result == std::option::spec_none() &lt;==&gt;
(<b>forall</b> k: K {<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k)} <b>where</b> <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k) && k != key:
std::cmp::compare(key, k) == std::cmp::Ordering::Greater);
<b>ensures</b> [abstract] result.is_some() &lt;==&gt;
    <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result)) &&
    (std::cmp::compare(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result), key) == std::cmp::Ordering::Greater)
    && (<b>forall</b> k: K {<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k)} <b>where</b> k != <a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result): ((<a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k) &&
    std::cmp::compare(k, key) == std::cmp::Ordering::Greater)) ==&gt;
    std::cmp::compare(<a href="../../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(result), k) == std::cmp::Ordering::Less);
</code></pre>



<a id="@Specification_1_lower_bound"></a>

### Function `lower_bound`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_find"></a>

### Function `find`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_find">find</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_new_begin_iter"></a>

### Function `new_begin_iter`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_new_end_iter"></a>

### Function `new_end_iter`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_next"></a>

### Function `iter_next`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_prev"></a>

### Function `iter_prev`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_prev">iter_prev</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_is_begin"></a>

### Function `iter_is_begin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_is_begin_from_non_empty"></a>

### Function `iter_is_begin_from_non_empty`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin_from_non_empty">iter_is_begin_from_non_empty</a>(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_is_end"></a>

### Function `iter_is_end`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, _map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_borrow_key"></a>

### Function `iter_borrow_key`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &K
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_borrow"></a>

### Function `iter_borrow`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &V
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_borrow_mut"></a>

### Function `iter_borrow_mut`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &<b>mut</b> V
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_remove"></a>

### Function `iter_remove`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_remove">iter_remove</a>&lt;K: drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): V
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_replace"></a>

### Function `iter_replace`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_replace">iter_replace</a>&lt;K: <b>copy</b>, drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, value: V): V
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_iter_add"></a>

### Function `iter_add`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_add">iter_add</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_IteratorPtr">ordered_map::IteratorPtr</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> opaque;
<b>ensures</b> [abstract] <b>forall</b> k: K: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(result, k) &lt;==&gt; <a href="ordered_map.md#0x1_ordered_map_spec_contains_key">spec_contains_key</a>(self, k);
</code></pre>



<a id="@Specification_1_values"></a>

### Function `values`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_values">values</a>&lt;K, V: <b>copy</b>&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_to_vec_pair"></a>

### Function `to_vec_pair`


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_binary_search"></a>

### Function `binary_search`


<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>&lt;K, V&gt;(key: &K, entries: &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
