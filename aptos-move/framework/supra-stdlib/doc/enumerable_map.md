
<a id="0x1_enumerable_map"></a>

# Module `0x1::enumerable_map`

This module provides an implementation of an enumerable map, a data structure that maintains key-value pairs with
efficient operations for addition, removal, and retrieval.
It allows for enumeration of keys in insertion order, bulk operations, and updates while ensuring data consistency.
The module includes error handling and a suite of test functions for validation.


-  [Struct `EnumerableMap`](#0x1_enumerable_map_EnumerableMap)
-  [Struct `Tuple`](#0x1_enumerable_map_Tuple)
-  [Struct `KeyValue`](#0x1_enumerable_map_KeyValue)
-  [Constants](#@Constants_0)
-  [Function `new_map`](#0x1_enumerable_map_new_map)
-  [Function `add_value`](#0x1_enumerable_map_add_value)
-  [Function `add_value_bulk`](#0x1_enumerable_map_add_value_bulk)
-  [Function `update_value`](#0x1_enumerable_map_update_value)
-  [Function `remove_value`](#0x1_enumerable_map_remove_value)
-  [Function `remove_value_bulk`](#0x1_enumerable_map_remove_value_bulk)
-  [Function `clear`](#0x1_enumerable_map_clear)
-  [Function `get_value`](#0x1_enumerable_map_get_value)
-  [Function `get_value_ref`](#0x1_enumerable_map_get_value_ref)
-  [Function `get_key_by_index`](#0x1_enumerable_map_get_key_by_index)
-  [Function `get_value_mut`](#0x1_enumerable_map_get_value_mut)
-  [Function `get_map_list`](#0x1_enumerable_map_get_map_list)
-  [Function `contains`](#0x1_enumerable_map_contains)
-  [Function `length`](#0x1_enumerable_map_length)
-  [Function `for_each_value`](#0x1_enumerable_map_for_each_value)
-  [Function `for_each_value_ref`](#0x1_enumerable_map_for_each_value_ref)
-  [Function `for_each_value_mut`](#0x1_enumerable_map_for_each_value_mut)
-  [Function `for_each_keyval`](#0x1_enumerable_map_for_each_keyval)
-  [Function `filter`](#0x1_enumerable_map_filter)
-  [Function `map`](#0x1_enumerable_map_map)
-  [Function `map_ref`](#0x1_enumerable_map_map_ref)
-  [Function `filter_map`](#0x1_enumerable_map_filter_map)
-  [Function `filter_map_ref`](#0x1_enumerable_map_filter_map_ref)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_enumerable_map_EnumerableMap"></a>

## Struct `EnumerableMap`

Enumerable Map to store the key value pairs


<pre><code><b>struct</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>list: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;</code>
</dt>
<dd>
 List of all keys
</dd>
<dt>
<code>map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;K, <a href="enumerable_map.md#0x1_enumerable_map_Tuple">enumerable_map::Tuple</a>&lt;V&gt;&gt;</code>
</dt>
<dd>
 Key mapped to a tuple containing the (position of key in list and value corresponding to the key)
</dd>
</dl>


</details>

<a id="0x1_enumerable_map_Tuple"></a>

## Struct `Tuple`

Tuple to store the position of key in list and value corresponding to the key


<pre><code><b>struct</b> <a href="enumerable_map.md#0x1_enumerable_map_Tuple">Tuple</a>&lt;V: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>position: u64</code>
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

<a id="0x1_enumerable_map_KeyValue"></a>

## Struct `KeyValue`

Return type


<pre><code><b>struct</b> <a href="enumerable_map.md#0x1_enumerable_map_KeyValue">KeyValue</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_enumerable_map_EKEY_ABSENT"></a>

Key is absent in the map


<pre><code><b>const</b> <a href="enumerable_map.md#0x1_enumerable_map_EKEY_ABSENT">EKEY_ABSENT</a>: u64 = 2;
</code></pre>



<a id="0x1_enumerable_map_EKEY_ALREADY_ADDED"></a>

Key is already present in the map


<pre><code><b>const</b> <a href="enumerable_map.md#0x1_enumerable_map_EKEY_ALREADY_ADDED">EKEY_ALREADY_ADDED</a>: u64 = 1;
</code></pre>



<a id="0x1_enumerable_map_EVECTOR_EMPTY"></a>

Vector is empty


<pre><code><b>const</b> <a href="enumerable_map.md#0x1_enumerable_map_EVECTOR_EMPTY">EVECTOR_EMPTY</a>: u64 = 3;
</code></pre>



<a id="0x1_enumerable_map_new_map"></a>

## Function `new_map`

To create an empty enum map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_new_map">new_map</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(): <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_new_map">new_map</a>&lt;K: <b>copy</b> + drop, V: store + drop + <b>copy</b>&gt;(): <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt; {
    <b>return</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt; { list: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;K&gt;(), map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;K, <a href="enumerable_map.md#0x1_enumerable_map_Tuple">Tuple</a>&lt;V&gt;&gt;() }
}
</code></pre>



</details>

<a id="0x1_enumerable_map_add_value"></a>

## Function `add_value`

Add Single Key in the Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_add_value">add_value</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_add_value">add_value</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, key: K, value: V) {
    <b>assert</b>!(!<a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>(map, key), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="enumerable_map.md#0x1_enumerable_map_EKEY_ALREADY_ADDED">EKEY_ALREADY_ADDED</a>));
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> map.map, key, <a href="enumerable_map.md#0x1_enumerable_map_Tuple">Tuple</a>&lt;V&gt; { position: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&map.list), value });
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> map.list, key);
}
</code></pre>



</details>

<a id="0x1_enumerable_map_add_value_bulk"></a>

## Function `add_value_bulk`

Add Multiple Keys in the Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_add_value_bulk">add_value_bulk</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_add_value_bulk">add_value_bulk</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(
    map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;,
    keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;,
    values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&values), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="enumerable_map.md#0x1_enumerable_map_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));
    <b>let</b> current_key_list_length = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&map.list);
    <b>let</b> updated_keys = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;K&gt;();

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_zip_reverse">vector::zip_reverse</a>(keys, values, |key, value| {
        <b>if</b> (!<a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>(map, key)) {
            <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> map.map, key, <a href="enumerable_map.md#0x1_enumerable_map_Tuple">Tuple</a>&lt;V&gt; { position: current_key_list_length, value });
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> map.list, key);
            current_key_list_length = current_key_list_length + 1;

            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> updated_keys, key);
        };
    });

    <b>return</b> updated_keys
}
</code></pre>



</details>

<a id="0x1_enumerable_map_update_value"></a>

## Function `update_value`

Update the value of a key thats already present in the Enumerable Map and return old value


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_update_value">update_value</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K, new_value: V): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_update_value">update_value</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(
    map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;,
    key: K,
    new_value: V
): V {
    <b>assert</b>!(<a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>(map, key), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="enumerable_map.md#0x1_enumerable_map_EKEY_ABSENT">EKEY_ABSENT</a>));
    <b>let</b> old_value = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&<b>mut</b> map.map, key).value;
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> map.map, key).value = new_value;
    old_value
}
</code></pre>



</details>

<a id="0x1_enumerable_map_remove_value"></a>

## Function `remove_value`

Remove single Key from the Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_remove_value">remove_value</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_remove_value">remove_value</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, key: K): V {
    <b>assert</b>!(<a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>(map, key), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="enumerable_map.md#0x1_enumerable_map_EKEY_ABSENT">EKEY_ABSENT</a>));

    <b>let</b> map_last_index = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&map.list) - 1;
    <b>let</b> index_of_element = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&map.map, key).position;
    <b>let</b> tuple_to_modify = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> map.map, *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&map.list, map_last_index));

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(&<b>mut</b> map.list, index_of_element, map_last_index);
    tuple_to_modify.position = index_of_element;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> map.list);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&<b>mut</b> map.map, key).value
}
</code></pre>



</details>

<a id="0x1_enumerable_map_remove_value_bulk"></a>

## Function `remove_value_bulk`

Remove Multiple Keys from the Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_remove_value_bulk">remove_value_bulk</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_remove_value_bulk">remove_value_bulk</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(
    map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;,
    keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&keys), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="enumerable_map.md#0x1_enumerable_map_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));

    <b>let</b> removed_keys = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;K&gt;();

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_reverse">vector::for_each_reverse</a>(keys, |key| {
        <b>if</b> (<a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>(map, key)) {
            <a href="enumerable_map.md#0x1_enumerable_map_remove_value">remove_value</a>(map, key);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> removed_keys, key);
        };
    });

    <b>return</b> removed_keys
}
</code></pre>



</details>

<a id="0x1_enumerable_map_clear"></a>

## Function `clear`

Will clear the entire data from the Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_clear">clear</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_clear">clear</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;) {
    <b>let</b> list = <a href="enumerable_map.md#0x1_enumerable_map_get_map_list">get_map_list</a>(map);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&list)) {
        <b>return</b>
    };
    <a href="enumerable_map.md#0x1_enumerable_map_remove_value_bulk">remove_value_bulk</a>(map, list);
}
</code></pre>



</details>

<a id="0x1_enumerable_map_get_value"></a>

## Function `get_value`

Returns the value of a key that is present in Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_value">get_value</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_value">get_value</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: & <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, key: K): V {
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&map.map, key).value
}
</code></pre>



</details>

<a id="0x1_enumerable_map_get_value_ref"></a>

## Function `get_value_ref`

Returns reference to the value of a key that is present in Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_value_ref">get_value_ref</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_value_ref">get_value_ref</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: & <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, key: K): &V {
    &<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&map.map, key).value
}
</code></pre>



</details>

<a id="0x1_enumerable_map_get_key_by_index"></a>

## Function `get_key_by_index`

Retrieves the key at the specified index from the EnumerableMap's key list.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_key_by_index">get_key_by_index</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, index: u64): K
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_key_by_index">get_key_by_index</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, index: u64): K {
    *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&set.list, index)
}
</code></pre>



</details>

<a id="0x1_enumerable_map_get_value_mut"></a>

## Function `get_value_mut`

Returns the value of a key that is present in Enumerable Map


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_value_mut">get_value_mut</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_value_mut">get_value_mut</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, key: K): &<b>mut</b> V {
    &<b>mut</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> map.map, key).value
}
</code></pre>



</details>

<a id="0x1_enumerable_map_get_map_list"></a>

## Function `get_map_list`

Returns the list of keys that the Enumerable Map contains


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_map_list">get_map_list</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_get_map_list">get_map_list</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <b>return</b> map.list
}
</code></pre>



</details>

<a id="0x1_enumerable_map_contains"></a>

## Function `contains`

Check whether Key is present into the Enumerable map or not


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(map: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_contains">contains</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(map: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, key: K): bool {
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&map.map, key)
}
</code></pre>



</details>

<a id="0x1_enumerable_map_length"></a>

## Function `length`

Return current length of the EnumerableSetRing


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_length">length</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_length">length</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;): u64 {
    <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&set.list)
}
</code></pre>



</details>

<a id="0x1_enumerable_map_for_each_value"></a>

## Function `for_each_value`

Apply the function to each element in the EnumerableMap.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_value">for_each_value</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_value">for_each_value</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, f: |V|) {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="enumerable_map.md#0x1_enumerable_map_length">length</a>(set);
    <b>while</b> (i &lt; len) {
        <b>let</b> key = <a href="enumerable_map.md#0x1_enumerable_map_get_key_by_index">get_key_by_index</a>(set, i);
        f(*<a href="enumerable_map.md#0x1_enumerable_map_get_value_ref">get_value_ref</a>(set, key));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_enumerable_map_for_each_value_ref"></a>

## Function `for_each_value_ref`

Apply the function to a reference of each element in the EnumerableMap.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_ref">for_each_value_ref</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |&V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_ref">for_each_value_ref</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, f: |&V|) {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="enumerable_map.md#0x1_enumerable_map_length">length</a>(set);
    <b>while</b> (i &lt; len) {
        <b>let</b> key = <a href="enumerable_map.md#0x1_enumerable_map_get_key_by_index">get_key_by_index</a>(set, i);
        f(<a href="enumerable_map.md#0x1_enumerable_map_get_value_ref">get_value_ref</a>(set, key));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_enumerable_map_for_each_value_mut"></a>

## Function `for_each_value_mut`

Apply the function to a mutable reference in the EnumerableMap.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_mut">for_each_value_mut</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |&<b>mut</b> V|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_mut">for_each_value_mut</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<b>mut</b> <a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, f: |&<b>mut</b> V|) {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="enumerable_map.md#0x1_enumerable_map_length">length</a>(set);
    <b>while</b> (i &lt; len) {
        <b>let</b> key = <a href="enumerable_map.md#0x1_enumerable_map_get_key_by_index">get_key_by_index</a>(set, i);
        f(<a href="enumerable_map.md#0x1_enumerable_map_get_value_mut">get_value_mut</a>(set, key));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_enumerable_map_for_each_keyval"></a>

## Function `for_each_keyval`

Iterates over each key-value pair in an EnumerableMap and applies the provided function


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_keyval">for_each_keyval</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |(K, V)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_for_each_keyval">for_each_keyval</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, f: |K, V|) {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="enumerable_map.md#0x1_enumerable_map_length">length</a>(set);
    <b>while</b> (i &lt; len) {
        <b>let</b> key = <a href="enumerable_map.md#0x1_enumerable_map_get_key_by_index">get_key_by_index</a>(set, i);
        f(key, *<a href="enumerable_map.md#0x1_enumerable_map_get_value_ref">get_value_ref</a>(set, key));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_enumerable_map_filter"></a>

## Function `filter`

Filter the enumerableMap using the boolean function, removing all elements for which <code>p(v)</code> is not true.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_filter">filter</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, p: |&V|bool): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_filter">filter</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, p: |&V|bool): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt; {
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;[];
    <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_ref">for_each_value_ref</a>(set, |v| {
        <b>if</b> (p(v)) <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, *v);
    });
    result
}
</code></pre>



</details>

<a id="0x1_enumerable_map_map"></a>

## Function `map`

Transforms values in an EnumerableMap using the provided function and returns a vector of results.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_map">map</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store, T&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |V|T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_map">map</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>, T&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, f: |V|T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;[];
    <a href="enumerable_map.md#0x1_enumerable_map_for_each_value">for_each_value</a>(set, |elem| <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, f(elem)));
    result
}
</code></pre>



</details>

<a id="0x1_enumerable_map_map_ref"></a>

## Function `map_ref`

Transforms values in an EnumerableMap by reference using the provided function and returns a vector of results.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_map_ref">map_ref</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store, T&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |&V|T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_map_ref">map_ref</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>, T&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;, f: |&V|T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;[];
    <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_ref">for_each_value_ref</a>(set, |elem| <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, f(elem)));
    result
}
</code></pre>



</details>

<a id="0x1_enumerable_map_filter_map"></a>

## Function `filter_map`

Applies a filter and transformation function to values in an EnumerableMap, returning a vector of results.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_filter_map">filter_map</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store, T&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |V|(bool, T)): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_filter_map">filter_map</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>, T&gt;(
    set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;,
    f: |V| (bool, T)
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;[];
    <a href="enumerable_map.md#0x1_enumerable_map_for_each_value">for_each_value</a>(set, |v| {
        <b>let</b> (should_include, transformed_value) = f(v);
        <b>if</b> (should_include) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, transformed_value);
        }
    });
    result
}
</code></pre>



</details>

<a id="0x1_enumerable_map_filter_map_ref"></a>

## Function `filter_map_ref`

Applies a filter and transformation function to values in an EnumerableMap, returning a vector of results.


<pre><code><b>public</b> <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_filter_map_ref">filter_map_ref</a>&lt;K: <b>copy</b>, drop, V: <b>copy</b>, drop, store, T&gt;(set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">enumerable_map::EnumerableMap</a>&lt;K, V&gt;, f: |&V|(bool, T)): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="enumerable_map.md#0x1_enumerable_map_filter_map_ref">filter_map_ref</a>&lt;K: <b>copy</b>+drop, V: store+drop+<b>copy</b>, T&gt;(
    set: &<a href="enumerable_map.md#0x1_enumerable_map_EnumerableMap">EnumerableMap</a>&lt;K, V&gt;,
    f: |&V| (bool, T)
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;[];
    <a href="enumerable_map.md#0x1_enumerable_map_for_each_value_ref">for_each_value_ref</a>(set, |v| {
        <b>let</b> (should_include, transformed_value) = f(v);
        <b>if</b> (should_include) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, transformed_value);
        }
    });
    result
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
