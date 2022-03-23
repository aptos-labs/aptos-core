
<a name="0x1_Table"></a>

# Module `0x1::Table`

This module provides a temporary solution for tables by providing a layer on top of Vector


-  [Struct `Table`](#0x1_Table_Table)
-  [Struct `TableElement`](#0x1_Table_TableElement)
-  [Constants](#@Constants_0)
-  [Function `count`](#0x1_Table_count)
-  [Function `create`](#0x1_Table_create)
-  [Function `borrow`](#0x1_Table_borrow)
-  [Function `borrow_mut`](#0x1_Table_borrow_mut)
-  [Function `contains_key`](#0x1_Table_contains_key)
-  [Function `destroy_empty`](#0x1_Table_destroy_empty)
-  [Function `insert`](#0x1_Table_insert)
-  [Function `remove`](#0x1_Table_remove)
-  [Function `find`](#0x1_Table_find)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_Table_Table"></a>

## Struct `Table`



<pre><code><b>struct</b> <a href="Table.md#0x1_Table">Table</a>&lt;Key: store, Value: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: vector&lt;<a href="Table.md#0x1_Table_TableElement">Table::TableElement</a>&lt;Key, Value&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Table_TableElement"></a>

## Struct `TableElement`



<pre><code><b>struct</b> <a href="Table.md#0x1_Table_TableElement">TableElement</a>&lt;Key: store, Value: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: Key</code>
</dt>
<dd>

</dd>
<dt>
<code>value: Value</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Table_EKEY_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="Table.md#0x1_Table_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>: u64 = 0;
</code></pre>



<a name="0x1_Table_EKEY_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="Table.md#0x1_Table_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a name="0x1_Table_count"></a>

## Function `count`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_count">count</a>&lt;Key: store, Value: store&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_count">count</a>&lt;Key: store, Value: store&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;): u64 {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&table.data)
}
</code></pre>



</details>

<a name="0x1_Table_create"></a>

## Function `create`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_create">create</a>&lt;Key: store, Value: store&gt;(): <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_create">create</a>&lt;Key: store, Value: store&gt;(): <a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt; {
    <a href="Table.md#0x1_Table">Table</a> {
        data: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>(),
    }
}
</code></pre>



</details>

<a name="0x1_Table_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow">borrow</a>&lt;Key: store, Value: store&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;, key: &Key): &Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow">borrow</a>&lt;Key: store, Value: store&gt;(
    table: &<a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;,
    key: &Key,
): &Value {
    <b>let</b> maybe_idx = <a href="Table.md#0x1_Table_find">find</a>(table, key);
    <b>assert</b>!(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&maybe_idx), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Table.md#0x1_Table_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> idx = *<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&maybe_idx);
    &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&table.data, idx).value
}
</code></pre>



</details>

<a name="0x1_Table_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_mut">borrow_mut</a>&lt;Key: store, Value: store&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;, key: &Key): &<b>mut</b> Value
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_mut">borrow_mut</a>&lt;Key: store, Value: store&gt;(
    table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;,
    key: &Key,
): &<b>mut</b> Value {
    <b>let</b> maybe_idx = <a href="Table.md#0x1_Table_find">find</a>(table, key);
    <b>assert</b>!(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&maybe_idx), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Table.md#0x1_Table_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> idx = *<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&maybe_idx);
    &<b>mut</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow_mut">Vector::borrow_mut</a>(&<b>mut</b> table.data, idx).value
}
</code></pre>



</details>

<a name="0x1_Table_contains_key"></a>

## Function `contains_key`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_contains_key">contains_key</a>&lt;Key: store, Value: store&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;, key: &Key): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_contains_key">contains_key</a>&lt;Key: store, Value: store&gt;(
    table: &<a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;,
    key: &Key,
): bool {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&<a href="Table.md#0x1_Table_find">find</a>(table, key))
}
</code></pre>



</details>

<a name="0x1_Table_destroy_empty"></a>

## Function `destroy_empty`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_destroy_empty">destroy_empty</a>&lt;Key: store, Value: store&gt;(table: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_destroy_empty">destroy_empty</a>&lt;Key: store, Value: store&gt;(table: <a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;) {
    <b>let</b> <a href="Table.md#0x1_Table">Table</a> { data } = table;
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_destroy_empty">Vector::destroy_empty</a>(data);
}
</code></pre>



</details>

<a name="0x1_Table_insert"></a>

## Function `insert`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_insert">insert</a>&lt;Key: store, Value: store&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;, key: Key, value: Value)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_insert">insert</a>&lt;Key: store, Value: store&gt;(
    table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;,
    key: Key,
    value: Value,
) {
    <b>let</b> maybe_idx = <a href="Table.md#0x1_Table_find">find</a>(table, &key);
    <b>assert</b>!(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_none">Option::is_none</a>(&maybe_idx), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Table.md#0x1_Table_EKEY_ALREADY_EXISTS">EKEY_ALREADY_EXISTS</a>));
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(&<b>mut</b> table.data, <a href="Table.md#0x1_Table_TableElement">TableElement</a> { key, value });
}
</code></pre>



</details>

<a name="0x1_Table_remove"></a>

## Function `remove`



<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_remove">remove</a>&lt;Key: store, Value: store&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;, key: &Key): (Key, Value)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_remove">remove</a>&lt;Key: store, Value: store&gt;(
    table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;,
    key: &Key,
): (Key, Value) {
    <b>let</b> maybe_idx = <a href="Table.md#0x1_Table_find">find</a>(table, key);
    <b>assert</b>!(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&maybe_idx), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Table.md#0x1_Table_EKEY_NOT_FOUND">EKEY_NOT_FOUND</a>));
    <b>let</b> idx = *<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&maybe_idx);
    <b>let</b> <a href="Table.md#0x1_Table_TableElement">TableElement</a> { key, value } = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_swap_remove">Vector::swap_remove</a>(&<b>mut</b> table.data, idx);
    (key, value)
}
</code></pre>



</details>

<a name="0x1_Table_find"></a>

## Function `find`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_find">find</a>&lt;Key: store, Value: store&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;Key, Value&gt;, key: &Key): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Table.md#0x1_Table_find">find</a>&lt;Key: store, Value: store&gt;(
    table: &<a href="Table.md#0x1_Table">Table</a>&lt;Key, Value&gt;,
    key: &Key,
): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt; {
    <b>let</b> size = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&table.data);
    <b>let</b> idx = 0;
    <b>while</b> (idx &lt; size) {
        <b>if</b> (&<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&table.data, idx).key == key) {
            <b>return</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(idx)
        };
        idx = idx + 1
    };

    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>()
}
</code></pre>



</details>
