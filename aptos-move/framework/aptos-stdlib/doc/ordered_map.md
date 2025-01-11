
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
All methods that start with iter_*, operate with Iterator being <code>self</code>.

Uses cmp::compare for ordering, which compares primitive types natively, and uses common
lexicographical sorting for complex types.


-  [Struct `Entry`](#0x1_ordered_map_Entry)
-  [Enum `OrderedMap`](#0x1_ordered_map_OrderedMap)
-  [Enum `Iterator`](#0x1_ordered_map_Iterator)
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
-  [Function `append`](#0x1_ordered_map_append)
-  [Function `trim`](#0x1_ordered_map_trim)
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
-  [Function `destroy_empty`](#0x1_ordered_map_destroy_empty)
-  [Function `keys`](#0x1_ordered_map_keys)
-  [Function `values`](#0x1_ordered_map_values)
-  [Function `to_vec_pair`](#0x1_ordered_map_to_vec_pair)
-  [Function `destroy`](#0x1_ordered_map_destroy)
-  [Function `for_each_ref`](#0x1_ordered_map_for_each_ref)
-  [Function `for_each_mut`](#0x1_ordered_map_for_each_mut)
-  [Function `new_iter`](#0x1_ordered_map_new_iter)
-  [Function `binary_search`](#0x1_ordered_map_binary_search)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/cmp.md#0x1_cmp">0x1::cmp</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/mem.md#0x1_mem">0x1::mem</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
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
<code>entries: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>
 List of entries, sorted by key.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_ordered_map_Iterator"></a>

## Enum `Iterator`

An iterator pointing to a position between two elements in the


<pre><code>enum <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> <b>has</b> <b>copy</b>, drop
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

Creates a new empty OrderedMap, using default (SortedVectorMap) implementation.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new">new</a>&lt;K, V&gt;(): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new">new</a>&lt;K, V&gt;(): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; {
    OrderedMap::SortedVectorMap {
        entries: <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_new_from"></a>

## Function `new_from`

Create a OrderedMap from a vector of keys and values.
Aborts with EKEY_ALREADY_EXISTS if duplicate keys are passed in.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_from">new_from</a>&lt;K, V&gt;(keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_from">new_from</a>&lt;K, V&gt;(keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt; {
    <b>let</b> map = <a href="ordered_map.md#0x1_ordered_map_new">new</a>();
    <a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>(&<b>mut</b> map, keys, values);
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
    <b>assert</b>!(index &gt;= len || &self.entries[index].key != &key, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
    self.entries.insert(index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
}
</code></pre>



</details>

<a id="0x1_ordered_map_upsert"></a>

## Function `upsert`

If the key doesn't exist in the map, inserts the key/value, and returns none.
Otherwise, updates the value under the given key, and returns the old value.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_upsert">upsert</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: K, value: V): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;V&gt;
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
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(old_value)
    } <b>else</b> {
        self.entries.insert(index, <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value });
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
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
    <b>assert</b>!(index &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key: old_key, value } = self.entries.<a href="ordered_map.md#0x1_ordered_map_remove">remove</a>(index);
    <b>assert</b>!(key == &old_key, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
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

Changes the key, with keeping the same value attached to it
Aborts with EKEY_NOT_FOUND if <code>old_key</code> doesn't exist.
Aborts with ENEW_KEY_NOT_IN_ORDER if <code>new_key</code> doesn't keep the order <code>old_key</code> was in.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_replace_key_inplace">replace_key_inplace</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, old_key: &K, new_key: K)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_replace_key_inplace">replace_key_inplace</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, old_key: &K, new_key: K) {
    <b>let</b> len = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>();
    <b>let</b> index = <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>(old_key, &self.entries, 0, len);
    <b>assert</b>!(index &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));

    <b>if</b> (index &gt; 0) {
        <b>assert</b>!(<a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&self.entries[index - 1].key, &new_key).is_lt(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>))
    };

    <b>if</b> (index + 1 &lt; len) {
        <b>assert</b>!(<a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&new_key, &self.entries[index + 1].key).is_lt(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_ENEW_KEY_NOT_IN_ORDER">ENEW_KEY_NOT_IN_ORDER</a>))
    };

    <b>let</b> entry = self.entries.<a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>(index);
    <b>assert</b>!(old_key == &entry.key, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    entry.key = new_key;
}
</code></pre>



</details>

<a id="0x1_ordered_map_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the map. The keys must not already exist.
Aborts with EKEY_ALREADY_EXISTS if key already exist, or duplicate keys are passed in.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_zip">vector::zip</a>(keys, values, |key, value| {
        <a href="ordered_map.md#0x1_ordered_map_add">add</a>(self, key, value);
    });
}
</code></pre>



</details>

<a id="0x1_ordered_map_append"></a>

## Function `append`

Takes all elements from <code>other</code> and adds them to <code>self</code>.
Aborts with EKEY_ALREADY_EXISTS if <code>other</code> has a key already present in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append">append</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_append">append</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, other: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;) {
    <b>let</b> OrderedMap::SortedVectorMap {
        entries: other_entries,
    } = other;

    <b>if</b> (other_entries.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        other_entries.<a href="ordered_map.md#0x1_ordered_map_destroy_empty">destroy_empty</a>();
        <b>return</b>;
    };

    <b>if</b> (self.entries.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        self.entries.<a href="ordered_map.md#0x1_ordered_map_append">append</a>(other_entries);
        <b>return</b>;
    };

    // Optimization: <b>if</b> all elements in `other` are larger than all elements in `self`, we can just <b>move</b> them over.
    <b>if</b> (<a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&self.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1).key, &other_entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(0).key).is_lt()) {
        self.entries.<a href="ordered_map.md#0x1_ordered_map_append">append</a>(other_entries);
        <b>return</b>;
    };

    // In O(n), traversing from the back, build reverse sorted result, and then reverse it back
    <b>let</b> reverse_result = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> cur_i = self.entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1;
    <b>let</b> other_i = other_entries.<a href="ordered_map.md#0x1_ordered_map_length">length</a>() - 1;

    // after the end of the <b>loop</b>, entries is empty, and <a href="any.md#0x1_any">any</a> leftover is in other_entries
    <b>loop</b> {
        <b>let</b> ord = <a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&self.entries[cur_i].key, &other_entries[other_i].key);
        <b>assert</b>!(!ord.is_eq(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
        <b>if</b> (ord.is_gt()) {
            reverse_result.push_back(self.entries.pop_back());
            <b>if</b> (cur_i == 0) {
                <b>break</b>;
            } <b>else</b> {
                cur_i = cur_i - 1;
            }
        } <b>else</b> {
            reverse_result.push_back(other_entries.pop_back());
            <b>if</b> (other_i == 0) {
                // make entries empty, and rest in other_entries.
                <a href="../../move-stdlib/doc/mem.md#0x1_mem_swap">mem::swap</a>(&<b>mut</b> other_entries, &<b>mut</b> self.entries);
                <b>break</b>;
            } <b>else</b> {
                other_i = other_i - 1;
            }
        };
    };

    reverse_result.reverse_append(other_entries);
    self.entries.reverse_append(reverse_result);
}
</code></pre>



</details>

<a id="0x1_ordered_map_trim"></a>

## Function `trim`

Splits the collection into two, such to leave <code>self</code> with <code>at</code> number of elements.
Returns a newly allocated map containing the elements in the range [at, len).
After the call, the original map will be left containing the elements [0, at)
with its previous capacity unchanged.


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

<a id="0x1_ordered_map_lower_bound"></a>

## Function `lower_bound`

Returns an iterator pointing to the first element that is greater or equal to the provided
key, or an end iterator if such element doesn't exist.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_lower_bound">lower_bound</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_find">find</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_find">find</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, key: &K): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
    <b>if</b> (self.<a href="ordered_map.md#0x1_ordered_map_is_empty">is_empty</a>()) {
        <b>return</b> Iterator::End;
    };

    <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(0)
}
</code></pre>



</details>

<a id="0x1_ordered_map_new_end_iter"></a>

## Function `new_end_iter`

Returns the end iterator.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_end_iter">new_end_iter</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
    Iterator::End
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_next"></a>

## Function `iter_next`

Returns the next iterator, or none if already at the end iterator.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
    <b>assert</b>!(!self.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(map), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_prev">iter_prev</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_prev">iter_prev</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
    <b>assert</b>!(!self.<a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>(map), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    <b>let</b> index = <b>if</b> (self is Iterator::End) {
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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin">iter_is_begin</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): bool {
    <b>if</b> (self is Iterator::End) {
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin_from_non_empty">iter_is_begin_from_non_empty</a>(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_begin_from_non_empty">iter_is_begin_from_non_empty</a>(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>): bool {
    <b>if</b> (self is Iterator::End) {
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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, _map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, _map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): bool {
    self is Iterator::End
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_borrow_key"></a>

## Function `iter_borrow_key`

Borrows the key given iterator points to.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &K
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): &K {
    <b>assert</b>!(!(self is Iterator::End), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));

    &map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.index).key
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_borrow"></a>

## Function `iter_borrow`

Borrows the value given iterator points to.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): &V {
    <b>assert</b>!(!(self is Iterator::End), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));
    &map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(self.index).value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_borrow_mut"></a>

## Function `iter_borrow_mut`

Mutably borrows the value iterator points to.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): &<b>mut</b> V {
    <b>assert</b>!(!(self is Iterator::End), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EITER_OUT_OF_BOUNDS">EITER_OUT_OF_BOUNDS</a>));
    &<b>mut</b> map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>(self.index).value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_remove"></a>

## Function `iter_remove`

Removes (key, value) pair iterator points to, returning the previous value.
Aborts with EKEY_NOT_FOUND if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_remove">iter_remove</a>&lt;K: drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_remove">iter_remove</a>&lt;K: drop, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): V {
    <b>assert</b>!(!(self is Iterator::End), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));

    <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key: _, value } = map.entries.<a href="ordered_map.md#0x1_ordered_map_remove">remove</a>(self.index);
    value
}
</code></pre>



</details>

<a id="0x1_ordered_map_iter_replace"></a>

## Function `iter_replace`

Replaces the value iterator is pointing to, returning the previous value.
Aborts with EKEY_NOT_FOUND if iterator is pointing to the end.
Note: Requires that the map is not changed after the input iterator is generated.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_replace">iter_replace</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, value: V): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_iter_replace">iter_replace</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a>, map: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, value: V): V {
    <b>assert</b>!(!(self is Iterator::End), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="ordered_map.md#0x1_ordered_map_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));

    <b>let</b> entry = map.entries.<a href="ordered_map.md#0x1_ordered_map_borrow_mut">borrow_mut</a>(self.index);
    <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(&<b>mut</b> entry.value, value)
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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_keys">keys</a>&lt;K: <b>copy</b>, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(&self.entries, |e| {
        <b>let</b> e: &<a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K, V&gt; = e;
        e.key
    })
}
</code></pre>



</details>

<a id="0x1_ordered_map_values"></a>

## Function `values`

Return all values in the map. This requires values to be copyable.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_values">values</a>&lt;K, V: <b>copy</b>&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_values">values</a>&lt;K, V: <b>copy</b>&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt; {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_map_ref">vector::map_ref</a>(&self.entries, |e| {
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


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>&lt;K, V&gt;(self: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    <b>let</b> keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt; = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> OrderedMap::SortedVectorMap { entries } = self;
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(entries, |e| {
        <b>let</b> <a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a> { key, value } = e;
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> keys, key);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> values, value);
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
    <b>let</b> (keys, values) = <a href="ordered_map.md#0x1_ordered_map_to_vec_pair">to_vec_pair</a>(self);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy">vector::destroy</a>(keys, |_k| dk(_k));
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy">vector::destroy</a>(values, |_v| dv(_v));
}
</code></pre>



</details>

<a id="0x1_ordered_map_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each key-value pair in the table.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_ref">for_each_ref</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, f: |(&K, &V)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_ref">for_each_ref</a>&lt;K, V&gt;(self: &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, f: |&K, &V|) {
    <b>let</b> iter = self.<a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>();
    <b>while</b> (!iter.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        f(iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self), iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow">iter_borrow</a>(self));
        iter = iter.<a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>(self);
    }
    // <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(
    //     &self.entries,
    //     |entry| {
    //         f(&entry.key, &entry.value)
    //     }
    // );
}
</code></pre>



</details>

<a id="0x1_ordered_map_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference of each key-value pair in the table.


<pre><code><b>public</b> <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_mut">for_each_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;K, V&gt;, f: |(K, &<b>mut</b> V)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_for_each_mut">for_each_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="ordered_map.md#0x1_ordered_map_OrderedMap">OrderedMap</a>&lt;K, V&gt;, f: |K, &<b>mut</b> V|) {
    <b>let</b> iter = self.<a href="ordered_map.md#0x1_ordered_map_new_begin_iter">new_begin_iter</a>();
    <b>while</b> (!iter.<a href="ordered_map.md#0x1_ordered_map_iter_is_end">iter_is_end</a>(self)) {
        <b>let</b> key = *iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_key">iter_borrow_key</a>(self);
        f(key, iter.<a href="ordered_map.md#0x1_ordered_map_iter_borrow_mut">iter_borrow_mut</a>(self));
        iter = iter.<a href="ordered_map.md#0x1_ordered_map_iter_next">iter_next</a>(self);
    }
    // <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_mut">vector::for_each_mut</a>(
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



<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index: u64): <a href="ordered_map.md#0x1_ordered_map_Iterator">ordered_map::Iterator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="ordered_map.md#0x1_ordered_map_new_iter">new_iter</a>(index: u64): <a href="ordered_map.md#0x1_ordered_map_Iterator">Iterator</a> {
    Iterator::Position {
        index: index,
    }
}
</code></pre>



</details>

<a id="0x1_ordered_map_binary_search"></a>

## Function `binary_search`



<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>&lt;K, V&gt;(key: &K, entries: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">ordered_map::Entry</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ordered_map.md#0x1_ordered_map_binary_search">binary_search</a>&lt;K, V&gt;(key: &K, entries: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_Entry">Entry</a>&lt;K, V&gt;&gt;, start: u64, end: u64): u64 {
    <b>let</b> l = start;
    <b>let</b> r = end;
    <b>while</b> (l != r) {
        <b>let</b> mid = l + (r - l) / 2;
        <b>let</b> comparison = <a href="../../move-stdlib/doc/cmp.md#0x1_cmp_compare">cmp::compare</a>(&entries.<a href="ordered_map.md#0x1_ordered_map_borrow">borrow</a>(mid).key, key);
        // TODO: check why this short-circuiting actually performs worse
        // <b>if</b> (comparison.is_equal()) {
        //     // there can only be one equal value, so end the search.
        //     <b>return</b> mid;
        // } <b>else</b>
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


[move-book]: https://aptos.dev/move/book/SUMMARY
