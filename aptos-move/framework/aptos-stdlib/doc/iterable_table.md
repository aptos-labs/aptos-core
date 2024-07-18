
<a id="0x1_iterable_table"></a>

# Module `0x1::iterable_table`



-  [Struct `IterableValue`](#0x1_iterable_table_IterableValue)
-  [Struct `IterableTable`](#0x1_iterable_table_IterableTable)
-  [Function `new`](#0x1_iterable_table_new)
-  [Function `destroy_empty`](#0x1_iterable_table_destroy_empty)
-  [Function `add`](#0x1_iterable_table_add)
-  [Function `remove`](#0x1_iterable_table_remove)
-  [Function `borrow`](#0x1_iterable_table_borrow)
-  [Function `borrow_mut`](#0x1_iterable_table_borrow_mut)
-  [Function `borrow_mut_with_default`](#0x1_iterable_table_borrow_mut_with_default)
-  [Function `length`](#0x1_iterable_table_length)
-  [Function `empty`](#0x1_iterable_table_empty)
-  [Function `contains`](#0x1_iterable_table_contains)
-  [Function `head_key`](#0x1_iterable_table_head_key)
-  [Function `tail_key`](#0x1_iterable_table_tail_key)
-  [Function `borrow_iter`](#0x1_iterable_table_borrow_iter)
-  [Function `borrow_iter_mut`](#0x1_iterable_table_borrow_iter_mut)
-  [Function `remove_iter`](#0x1_iterable_table_remove_iter)
-  [Function `append`](#0x1_iterable_table_append)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
</code></pre>



<a id="0x1_iterable_table_IterableValue"></a>

## Struct `IterableValue`

The iterable wrapper around value, points to previous and next key if any.


<pre><code><b>struct</b> <a href="iterable_table.md#0x1_iterable_table_IterableValue">IterableValue</a>&lt;K: <b>copy</b>, drop, store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>val: V</code>
</dt>
<dd>

</dd>
<dt>
<code>prev: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>next: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_iterable_table_IterableTable"></a>

## Struct `IterableTable`

An iterable table implementation based on double linked list.


<pre><code><b>struct</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K: <b>copy</b>, drop, store, V: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, <a href="iterable_table.md#0x1_iterable_table_IterableValue">iterable_table::IterableValue</a>&lt;K, V&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>head: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>tail: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_iterable_table_new"></a>

## Function `new`

Regular table API.
Create an empty table.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_new">new</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(): <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_new">new</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(): <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt; {
    <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a> {
        inner: <a href="table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),
        head: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        tail: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_iterable_table_destroy_empty"></a>

## Function `destroy_empty`

Destroy a table. The table must be empty to succeed.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;) {
    <b>assert</b>!(<a href="iterable_table.md#0x1_iterable_table_empty">empty</a>(&<a href="table.md#0x1_table">table</a>), 0);
    <b>assert</b>!(<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&<a href="table.md#0x1_table">table</a>.head), 0);
    <b>assert</b>!(<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&<a href="table.md#0x1_table">table</a>.tail), 0);
    <b>let</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a> { inner, head: _, tail: _ } = <a href="table.md#0x1_table">table</a>;
    <a href="table_with_length.md#0x1_table_with_length_destroy_empty">table_with_length::destroy_empty</a>(inner);
}
</code></pre>



</details>

<a id="0x1_iterable_table_add"></a>

## Function `add`

Add a new entry to the table. Aborts if an entry for this
key already exists.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_add">add</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K, val: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_add">add</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;, key: K, val: V) {
    <b>let</b> wrapped_value = <a href="iterable_table.md#0x1_iterable_table_IterableValue">IterableValue</a> {
        val,
        prev: <a href="table.md#0x1_table">table</a>.tail,
        next: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    };
    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key, wrapped_value);
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="table.md#0x1_table">table</a>.tail)) {
        <b>let</b> k = <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<a href="table.md#0x1_table">table</a>.tail);
        <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, *k).next = <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(key);
    } <b>else</b> {
        <a href="table.md#0x1_table">table</a>.head = <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(key);
    };
    <a href="table.md#0x1_table">table</a>.tail = <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(key);
}
</code></pre>



</details>

<a id="0x1_iterable_table_remove"></a>

## Function `remove`

Remove from <code><a href="table.md#0x1_table">table</a></code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_remove">remove</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_remove">remove</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;, key: K): V {
    <b>let</b> (val, _, _) = <a href="iterable_table.md#0x1_iterable_table_remove_iter">remove_iter</a>(<a href="table.md#0x1_table">table</a>, key);
    val
}
</code></pre>



</details>

<a id="0x1_iterable_table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow">borrow</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow">borrow</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;, key: K): &V {
    &<a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&<a href="table.md#0x1_table">table</a>.inner, key).val
}
</code></pre>



</details>

<a id="0x1_iterable_table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;, key: K): &<b>mut</b> V {
    &<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key).val
}
</code></pre>



</details>

<a id="0x1_iterable_table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b>, drop, store, V: drop, store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K, default: V): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b> + store + drop, V: store + drop&gt;(
    <a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;,
    key: K,
    default: V
): &<b>mut</b> V {
    <b>if</b> (!<a href="iterable_table.md#0x1_iterable_table_contains">contains</a>(<a href="table.md#0x1_table">table</a>, key)) {
        <a href="iterable_table.md#0x1_iterable_table_add">add</a>(<a href="table.md#0x1_table">table</a>, key, default)
    };
    <a href="iterable_table.md#0x1_iterable_table_borrow_mut">borrow_mut</a>(<a href="table.md#0x1_table">table</a>, key)
}
</code></pre>



</details>

<a id="0x1_iterable_table_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_length">length</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_length">length</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;): u64 {
    <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&<a href="table.md#0x1_table">table</a>.inner)
}
</code></pre>



</details>

<a id="0x1_iterable_table_empty"></a>

## Function `empty`

Returns true if this table is empty.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_empty">empty</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_empty">empty</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;): bool {
    <a href="table_with_length.md#0x1_table_with_length_empty">table_with_length::empty</a>(&<a href="table.md#0x1_table">table</a>.inner)
}
</code></pre>



</details>

<a id="0x1_iterable_table_contains"></a>

## Function `contains`

Returns true iff <code><a href="table.md#0x1_table">table</a></code> contains an entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_contains">contains</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_contains">contains</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;, key: K): bool {
    <a href="table_with_length.md#0x1_table_with_length_contains">table_with_length::contains</a>(&<a href="table.md#0x1_table">table</a>.inner, key)
}
</code></pre>



</details>

<a id="0x1_iterable_table_head_key"></a>

## Function `head_key`

Iterable API.
Returns the key of the head for iteration.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_head_key">head_key</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_head_key">head_key</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;): Option&lt;K&gt; {
    <a href="table.md#0x1_table">table</a>.head
}
</code></pre>



</details>

<a id="0x1_iterable_table_tail_key"></a>

## Function `tail_key`

Returns the key of the tail for iteration.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_tail_key">tail_key</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_tail_key">tail_key</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;): Option&lt;K&gt; {
    <a href="table.md#0x1_table">table</a>.tail
}
</code></pre>



</details>

<a id="0x1_iterable_table_borrow_iter"></a>

## Function `borrow_iter`

Acquire an immutable reference to the IterableValue which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_iter">borrow_iter</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): (&V, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_iter">borrow_iter</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(
    <a href="table.md#0x1_table">table</a>: &<a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;,
    key: K
): (&V, Option&lt;K&gt;, Option&lt;K&gt;) {
    <b>let</b> v = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&<a href="table.md#0x1_table">table</a>.inner, key);
    (&v.val, v.prev, v.next)
}
</code></pre>



</details>

<a id="0x1_iterable_table_borrow_iter_mut"></a>

## Function `borrow_iter_mut`

Acquire a mutable reference to the value and previous/next key which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_iter_mut">borrow_iter_mut</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): (&<b>mut</b> V, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_borrow_iter_mut">borrow_iter_mut</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(
    <a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;,
    key: K
): (&<b>mut</b> V, Option&lt;K&gt;, Option&lt;K&gt;) {
    <b>let</b> v = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key);
    (&<b>mut</b> v.val, v.prev, v.next)
}
</code></pre>



</details>

<a id="0x1_iterable_table_remove_iter"></a>

## Function `remove_iter`

Remove from <code><a href="table.md#0x1_table">table</a></code> and return the value and previous/next key which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_remove_iter">remove_iter</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, key: K): (V, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;K&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_remove_iter">remove_iter</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(
    <a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;,
    key: K
): (V, Option&lt;K&gt;, Option&lt;K&gt;) {
    <b>let</b> val = <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, <b>copy</b> key);
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_contains">option::contains</a>(&<a href="table.md#0x1_table">table</a>.tail, &key)) {
        <a href="table.md#0x1_table">table</a>.tail = val.prev;
    };
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_contains">option::contains</a>(&<a href="table.md#0x1_table">table</a>.head, &key)) {
        <a href="table.md#0x1_table">table</a>.head = val.next;
    };
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&val.prev)) {
        <b>let</b> key = <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&val.prev);
        <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, *key).next = val.next;
    };
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&val.next)) {
        <b>let</b> key = <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&val.next);
        <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, *key).prev = val.prev;
    };
    <b>let</b> <a href="iterable_table.md#0x1_iterable_table_IterableValue">IterableValue</a> { val, prev, next } = val;
    (val, prev, next)
}
</code></pre>



</details>

<a id="0x1_iterable_table_append"></a>

## Function `append`

Remove all items from v2 and append to v1.


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_append">append</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(v1: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;, v2: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">iterable_table::IterableTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="iterable_table.md#0x1_iterable_table_append">append</a>&lt;K: <b>copy</b> + store + drop, V: store&gt;(v1: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;, v2: &<b>mut</b> <a href="iterable_table.md#0x1_iterable_table_IterableTable">IterableTable</a>&lt;K, V&gt;) {
    <b>let</b> key = <a href="iterable_table.md#0x1_iterable_table_head_key">head_key</a>(v2);
    <b>while</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&key)) {
        <b>let</b> (val, _, next) = <a href="iterable_table.md#0x1_iterable_table_remove_iter">remove_iter</a>(v2, *<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&key));
        <a href="iterable_table.md#0x1_iterable_table_add">add</a>(v1, *<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&key), val);
        key = next;
    };
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
