
<a name="0x1_Table"></a>

# Module `0x1::Table`

Type of large-scale storage tables.
source: https://github.com/move-language/move/blob/1b6b7513dcc1a5c866f178ca5c1e74beb2ce181e/language/extensions/move-table-extension/sources/Table.move#L1

This is a exact copy from the Move repo. It implements the Table type which supports individual table items to
be represented by separate global state items. The number of items and a unique handle are tracked on the table
struct itself, while the operations are implemented as native functions. No traversal is provided.


-  [Struct `Table`](#0x1_Table_Table)
-  [Resource `Box`](#0x1_Table_Box)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_Table_new)
-  [Function `destroy_empty`](#0x1_Table_destroy_empty)
-  [Function `add`](#0x1_Table_add)
-  [Function `borrow`](#0x1_Table_borrow)
-  [Function `borrow_mut`](#0x1_Table_borrow_mut)
-  [Function `length`](#0x1_Table_length)
-  [Function `empty`](#0x1_Table_empty)
-  [Function `borrow_mut_with_default`](#0x1_Table_borrow_mut_with_default)
-  [Function `remove`](#0x1_Table_remove)
-  [Function `contains`](#0x1_Table_contains)
-  [Function `new_table_handle`](#0x1_Table_new_table_handle)
-  [Function `add_box`](#0x1_Table_add_box)
-  [Function `borrow_box`](#0x1_Table_borrow_box)
-  [Function `borrow_box_mut`](#0x1_Table_borrow_box_mut)
-  [Function `contains_box`](#0x1_Table_contains_box)
-  [Function `remove_box`](#0x1_Table_remove_box)
-  [Function `destroy_empty_box`](#0x1_Table_destroy_empty_box)
-  [Function `drop_unchecked_box`](#0x1_Table_drop_unchecked_box)


<pre><code><b>use</b> <a href="../MoveStdlib/Errors.md#0x1_Errors">0x1::Errors</a>;
</code></pre>



<a name="0x1_Table_Table"></a>

## Struct `Table`

Type of tables


<pre><code><b>struct</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Table_Box"></a>

## Resource `Box`

Wrapper for values. Required for making values appear as resources in the implementation.


<pre><code><b>struct</b> <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt; <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>val: V</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Table_EALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="Table.md#0x1_Table_EALREADY_EXISTS">EALREADY_EXISTS</a>: u64 = 100;
</code></pre>



<a name="0x1_Table_ENOT_EMPTY"></a>



<pre><code><b>const</b> <a href="Table.md#0x1_Table_ENOT_EMPTY">ENOT_EMPTY</a>: u64 = 102;
</code></pre>



<a name="0x1_Table_ENOT_FOUND"></a>



<pre><code><b>const</b> <a href="Table.md#0x1_Table_ENOT_FOUND">ENOT_FOUND</a>: u64 = 101;
</code></pre>



<a name="0x1_Table_new"></a>

## Function `new`

Create a new Table.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_new">new</a>&lt;K, V: store&gt;(): <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_new">new</a>&lt;K, V: store&gt;(): <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt; {
    <a href="Table.md#0x1_Table">Table</a>{
        handle: <a href="Table.md#0x1_Table_new_table_handle">new_table_handle</a>(),
        length: 0,
    }
}
</code></pre>



</details>

<a name="0x1_Table_destroy_empty"></a>

## Function `destroy_empty`

Destroy a table. The table must be empty to succeed.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_destroy_empty">destroy_empty</a>&lt;K, V&gt;(table: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_destroy_empty">destroy_empty</a>&lt;K, V&gt;(table: <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;) {
    <b>assert</b>!(table.length == 0, <a href="../MoveStdlib/Errors.md#0x1_Errors_invalid_state">Errors::invalid_state</a>(<a href="Table.md#0x1_Table_ENOT_EMPTY">ENOT_EMPTY</a>));
    <a href="Table.md#0x1_Table_destroy_empty_box">destroy_empty_box</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(&table);
    <a href="Table.md#0x1_Table_drop_unchecked_box">drop_unchecked_box</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(table)
}
</code></pre>



</details>

<a name="0x1_Table_add"></a>

## Function `add`

Add a new entry to the table. Aborts if an entry for this
key already exists. The entry itself is not stored in the
table, and cannot be discovered from it.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_add">add</a>&lt;K, V&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K, val: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_add">add</a>&lt;K, V&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K, val: V) {
    <a href="Table.md#0x1_Table_add_box">add_box</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(table, key, <a href="Table.md#0x1_Table_Box">Box</a>{val});
    table.length = table.length + 1
}
</code></pre>



</details>

<a name="0x1_Table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow">borrow</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow">borrow</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): &V {
    &<a href="Table.md#0x1_Table_borrow_box">borrow_box</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(table, key).val
}
</code></pre>



</details>

<a name="0x1_Table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_mut">borrow_mut</a>&lt;K, V&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_mut">borrow_mut</a>&lt;K, V&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): &<b>mut</b> V {
    &<b>mut</b> <a href="Table.md#0x1_Table_borrow_box_mut">borrow_box_mut</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(table, key).val
}
</code></pre>



</details>

<a name="0x1_Table_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_length">length</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_length">length</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;): u64 {
    table.length
}
</code></pre>



</details>

<a name="0x1_Table_empty"></a>

## Function `empty`

Returns true if this table is empty.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_empty">empty</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_empty">empty</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;): bool {
    table.length == 0
}
</code></pre>



</details>

<a name="0x1_Table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K, V: drop&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K, default: V): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K, V: drop&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K, default: V): &<b>mut</b> V {
    <b>if</b> (!<a href="Table.md#0x1_Table_contains">contains</a>(table, key)) {
        <a href="Table.md#0x1_Table_add">add</a>(table, key, default)
    };
    <a href="Table.md#0x1_Table_borrow_mut">borrow_mut</a>(table, key)
}
</code></pre>



</details>

<a name="0x1_Table_remove"></a>

## Function `remove`

Remove from <code>table</code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_remove">remove</a>&lt;K, V&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_remove">remove</a>&lt;K, V&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): V {
    <b>let</b> <a href="Table.md#0x1_Table_Box">Box</a>{val} = <a href="Table.md#0x1_Table_remove_box">remove_box</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(table, key);
    table.length = table.length - 1;
    val
}
</code></pre>



</details>

<a name="0x1_Table_contains"></a>

## Function `contains`

Returns true iff <code>table</code> contains an entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_contains">contains</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Table.md#0x1_Table_contains">contains</a>&lt;K, V&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): bool {
    <a href="Table.md#0x1_Table_contains_box">contains_box</a>&lt;K, V, <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;&gt;(table, key)
}
</code></pre>



</details>

<a name="0x1_Table_new_table_handle"></a>

## Function `new_table_handle`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_new_table_handle">new_table_handle</a>(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_new_table_handle">new_table_handle</a>(): u128;
</code></pre>



</details>

<a name="0x1_Table_add_box"></a>

## Function `add_box`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_add_box">add_box</a>&lt;K, V, B&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K, val: <a href="Table.md#0x1_Table_Box">Table::Box</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_add_box">add_box</a>&lt;K, V, B&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K, val: <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;);
</code></pre>



</details>

<a name="0x1_Table_borrow_box"></a>

## Function `borrow_box`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_borrow_box">borrow_box</a>&lt;K, V, B&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): &<a href="Table.md#0x1_Table_Box">Table::Box</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_box">borrow_box</a>&lt;K, V, B&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): &<a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;;
</code></pre>



</details>

<a name="0x1_Table_borrow_box_mut"></a>

## Function `borrow_box_mut`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_borrow_box_mut">borrow_box_mut</a>&lt;K, V, B&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): &<b>mut</b> <a href="Table.md#0x1_Table_Box">Table::Box</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_borrow_box_mut">borrow_box_mut</a>&lt;K, V, B&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): &<b>mut</b> <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;;
</code></pre>



</details>

<a name="0x1_Table_contains_box"></a>

## Function `contains_box`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_contains_box">contains_box</a>&lt;K, V, B&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_contains_box">contains_box</a>&lt;K, V, B&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): bool;
</code></pre>



</details>

<a name="0x1_Table_remove_box"></a>

## Function `remove_box`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_remove_box">remove_box</a>&lt;K, V, B&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;, key: &K): <a href="Table.md#0x1_Table_Box">Table::Box</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_remove_box">remove_box</a>&lt;K, V, B&gt;(table: &<b>mut</b> <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;, key: &K): <a href="Table.md#0x1_Table_Box">Box</a>&lt;V&gt;;
</code></pre>



</details>

<a name="0x1_Table_destroy_empty_box"></a>

## Function `destroy_empty_box`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_destroy_empty_box">destroy_empty_box</a>&lt;K, V, B&gt;(table: &<a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_destroy_empty_box">destroy_empty_box</a>&lt;K, V, B&gt;(table: &<a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;);
</code></pre>



</details>

<a name="0x1_Table_drop_unchecked_box"></a>

## Function `drop_unchecked_box`



<pre><code><b>fun</b> <a href="Table.md#0x1_Table_drop_unchecked_box">drop_unchecked_box</a>&lt;K, V, B&gt;(table: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="Table.md#0x1_Table_drop_unchecked_box">drop_unchecked_box</a>&lt;K, V, B&gt;(table: <a href="Table.md#0x1_Table">Table</a>&lt;K, V&gt;);
</code></pre>



</details>
