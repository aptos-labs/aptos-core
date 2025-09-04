
<a id="0x1_table"></a>

# Module `0x1::table`

Type of large-scale storage tables.
source: https://github.com/move-language/move/blob/1b6b7513dcc1a5c866f178ca5c1e74beb2ce181e/language/extensions/move-table-extension/sources/Table.move#L1

It implements the Table type which supports individual table items to be represented by
separate global state items. The number of items and a unique handle are tracked on the table
struct itself, while the operations are implemented as native functions. No traversal is provided.


-  [Struct `Table`](#0x1_table_Table)
-  [Resource `Box`](#0x1_table_Box)
-  [Function `new`](#0x1_table_new)
-  [Function `add`](#0x1_table_add)
-  [Function `borrow`](#0x1_table_borrow)
-  [Function `borrow_with_default`](#0x1_table_borrow_with_default)
-  [Function `borrow_mut`](#0x1_table_borrow_mut)
-  [Function `borrow_mut_with_default`](#0x1_table_borrow_mut_with_default)
-  [Function `upsert`](#0x1_table_upsert)
-  [Function `remove`](#0x1_table_remove)
-  [Function `contains`](#0x1_table_contains)
-  [Function `destroy_known_empty_unsafe`](#0x1_table_destroy_known_empty_unsafe)
-  [Function `new_table_handle`](#0x1_table_new_table_handle)
-  [Function `add_box`](#0x1_table_add_box)
-  [Function `borrow_box`](#0x1_table_borrow_box)
-  [Function `borrow_box_mut`](#0x1_table_borrow_box_mut)
-  [Function `contains_box`](#0x1_table_contains_box)
-  [Function `remove_box`](#0x1_table_remove_box)
-  [Function `destroy_empty_box`](#0x1_table_destroy_empty_box)
-  [Function `drop_unchecked_box`](#0x1_table_drop_unchecked_box)
-  [Specification](#@Specification_0)
    -  [Struct `Table`](#@Specification_0_Table)
    -  [Function `new`](#@Specification_0_new)
    -  [Function `add`](#@Specification_0_add)
    -  [Function `borrow`](#@Specification_0_borrow)
    -  [Function `borrow_with_default`](#@Specification_0_borrow_with_default)
    -  [Function `borrow_mut`](#@Specification_0_borrow_mut)
    -  [Function `borrow_mut_with_default`](#@Specification_0_borrow_mut_with_default)
    -  [Function `upsert`](#@Specification_0_upsert)
    -  [Function `remove`](#@Specification_0_remove)
    -  [Function `contains`](#@Specification_0_contains)
    -  [Function `destroy_known_empty_unsafe`](#@Specification_0_destroy_known_empty_unsafe)


<pre><code></code></pre>



<a id="0x1_table_Table"></a>

## Struct `Table`

Type of tables


<pre><code><b>struct</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K: <b>copy</b>, drop, V&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_table_Box"></a>

## Resource `Box`

Wrapper for values. Required for making values appear as resources in the implementation.


<pre><code><b>struct</b> <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt; <b>has</b> drop, store, key
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

<a id="0x1_table_new"></a>

## Function `new`

Create a new Table.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_new">new</a>&lt;K: <b>copy</b>, drop, V: store&gt;(): <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_new">new</a>&lt;K: <b>copy</b> + drop, V: store&gt;(): <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt; {
    <a href="table.md#0x1_table_Table">Table</a> {
        handle: <a href="table.md#0x1_table_new_table_handle">new_table_handle</a>&lt;K, V&gt;(),
    }
}
</code></pre>



</details>

<a id="0x1_table_add"></a>

## Function `add`

Add a new entry to the table. Aborts if an entry for this
key already exists. The entry itself is not stored in the
table, and cannot be discovered from it.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_add">add</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, val: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_add">add</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K, val: V) {
    <a href="table.md#0x1_table_add_box">add_box</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(self, key, <a href="table.md#0x1_table_Box">Box</a> { val })
}
</code></pre>



</details>

<a id="0x1_table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow">borrow</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow">borrow</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): &V {
    &<a href="table.md#0x1_table_borrow_box">borrow_box</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(self, key).val
}
</code></pre>



</details>

<a id="0x1_table_borrow_with_default"></a>

## Function `borrow_with_default`

Acquire an immutable reference to the value which <code>key</code> maps to.
Returns specified default value if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_with_default">borrow_with_default</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, default: &V): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_with_default">borrow_with_default</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K, default: &V): &V {
    <b>if</b> (!self.<a href="table.md#0x1_table_contains">contains</a>(<b>copy</b> key)) {
        default
    } <b>else</b> {
        self.<a href="table.md#0x1_table_borrow">borrow</a>(<b>copy</b> key)
    }
}
</code></pre>



</details>

<a id="0x1_table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): &<b>mut</b> V {
    &<b>mut</b> <a href="table.md#0x1_table_borrow_box_mut">borrow_box_mut</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(self, key).val
}
</code></pre>



</details>

<a id="0x1_table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, default: V): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b> + drop, V: drop&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K, default: V): &<b>mut</b> V {
    <b>if</b> (!self.<a href="table.md#0x1_table_contains">contains</a>(<b>copy</b> key)) {
        self.<a href="table.md#0x1_table_add">add</a>(<b>copy</b> key, default)
    };
    self.<a href="table.md#0x1_table_borrow_mut">borrow_mut</a>(key)
}
</code></pre>



</details>

<a id="0x1_table_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.
update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_upsert">upsert</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_upsert">upsert</a>&lt;K: <b>copy</b> + drop, V: drop&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K, value: V) {
    <b>if</b> (!self.<a href="table.md#0x1_table_contains">contains</a>(<b>copy</b> key)) {
        self.<a href="table.md#0x1_table_add">add</a>(<b>copy</b> key, value)
    } <b>else</b> {
        <b>let</b> ref = self.<a href="table.md#0x1_table_borrow_mut">borrow_mut</a>(key);
        *ref = value;
    };
}
</code></pre>



</details>

<a id="0x1_table_remove"></a>

## Function `remove`

Remove from <code>self</code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_remove">remove</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_remove">remove</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): V {
    <b>let</b> <a href="table.md#0x1_table_Box">Box</a> { val } = <a href="table.md#0x1_table_remove_box">remove_box</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(self, key);
    val
}
</code></pre>



</details>

<a id="0x1_table_contains"></a>

## Function `contains`

Returns true iff <code>self</code> contains an entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_contains">contains</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_contains">contains</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): bool {
    <a href="table.md#0x1_table_contains_box">contains_box</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(self, key)
}
</code></pre>



</details>

<a id="0x1_table_destroy_known_empty_unsafe"></a>

## Function `destroy_known_empty_unsafe`

Table cannot know if it is empty or not, so this method is not public,
and can be used only in modules that know by themselves that table is empty.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="table.md#0x1_table_destroy_known_empty_unsafe">destroy_known_empty_unsafe</a>&lt;K: <b>copy</b>, drop, V&gt;(self: <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="table.md#0x1_table_destroy_known_empty_unsafe">destroy_known_empty_unsafe</a>&lt;K: <b>copy</b> + drop, V&gt;(self: <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;) {
    <a href="table.md#0x1_table_destroy_empty_box">destroy_empty_box</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(&self);
    <a href="table.md#0x1_table_drop_unchecked_box">drop_unchecked_box</a>&lt;K, V, <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;&gt;(self)
}
</code></pre>



</details>

<a id="0x1_table_new_table_handle"></a>

## Function `new_table_handle`



<pre><code><b>fun</b> <a href="table.md#0x1_table_new_table_handle">new_table_handle</a>&lt;K, V&gt;(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_new_table_handle">new_table_handle</a>&lt;K, V&gt;(): <b>address</b>;
</code></pre>



</details>

<a id="0x1_table_add_box"></a>

## Function `add_box`



<pre><code><b>fun</b> <a href="table.md#0x1_table_add_box">add_box</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, val: <a href="table.md#0x1_table_Box">table::Box</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_add_box">add_box</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K, val: <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;);
</code></pre>



</details>

<a id="0x1_table_borrow_box"></a>

## Function `borrow_box`



<pre><code><b>fun</b> <a href="table.md#0x1_table_borrow_box">borrow_box</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): &<a href="table.md#0x1_table_Box">table::Box</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_borrow_box">borrow_box</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): &<a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;;
</code></pre>



</details>

<a id="0x1_table_borrow_box_mut"></a>

## Function `borrow_box_mut`



<pre><code><b>fun</b> <a href="table.md#0x1_table_borrow_box_mut">borrow_box_mut</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): &<b>mut</b> <a href="table.md#0x1_table_Box">table::Box</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_borrow_box_mut">borrow_box_mut</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): &<b>mut</b> <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;;
</code></pre>



</details>

<a id="0x1_table_contains_box"></a>

## Function `contains_box`



<pre><code><b>fun</b> <a href="table.md#0x1_table_contains_box">contains_box</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_contains_box">contains_box</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): bool;
</code></pre>



</details>

<a id="0x1_table_remove_box"></a>

## Function `remove_box`



<pre><code><b>fun</b> <a href="table.md#0x1_table_remove_box">remove_box</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): <a href="table.md#0x1_table_Box">table::Box</a>&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_remove_box">remove_box</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<b>mut</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, key: K): <a href="table.md#0x1_table_Box">Box</a>&lt;V&gt;;
</code></pre>



</details>

<a id="0x1_table_destroy_empty_box"></a>

## Function `destroy_empty_box`



<pre><code><b>fun</b> <a href="table.md#0x1_table_destroy_empty_box">destroy_empty_box</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_destroy_empty_box">destroy_empty_box</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: &<a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;);
</code></pre>



</details>

<a id="0x1_table_drop_unchecked_box"></a>

## Function `drop_unchecked_box`



<pre><code><b>fun</b> <a href="table.md#0x1_table_drop_unchecked_box">drop_unchecked_box</a>&lt;K: <b>copy</b>, drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_drop_unchecked_box">drop_unchecked_box</a>&lt;K: <b>copy</b> + drop, V, B&gt;(<a href="table.md#0x1_table">table</a>: <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;);
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_Table"></a>

### Struct `Table`


<pre><code><b>struct</b> <a href="table.md#0x1_table_Table">Table</a>&lt;K: <b>copy</b>, drop, V&gt; <b>has</b> store
</code></pre>



<dl>
<dt>
<code>handle: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>pragma</b> intrinsic = map,
    map_new = new,
    map_destroy_empty = destroy_known_empty_unsafe,
    map_has_key = contains,
    map_add_no_override = add,
    map_add_override_if_exists = upsert,
    map_del_must_exist = remove,
    map_borrow = borrow,
    map_borrow_mut = borrow_mut,
    map_borrow_mut_with_default = borrow_mut_with_default,
    map_borrow_with_default = borrow_with_default,
    map_spec_get = spec_get,
    map_spec_set = spec_set,
    map_spec_del = spec_remove,
    map_spec_has_key = spec_contains;
</code></pre>



<a id="@Specification_0_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_new">new</a>&lt;K: <b>copy</b>, drop, V: store&gt;(): <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_add">add</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, val: V)
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow">borrow</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): &V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_borrow_with_default"></a>

### Function `borrow_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_with_default">borrow_with_default</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, default: &V): &V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_borrow_mut_with_default"></a>

### Function `borrow_mut_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, default: V): &<b>mut</b> V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_upsert"></a>

### Function `upsert`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_upsert">upsert</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_remove">remove</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): V
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="table.md#0x1_table_contains">contains</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;, key: K): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>




<a id="0x1_table_spec_contains"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_spec_contains">spec_contains</a>&lt;K, V&gt;(t: <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, k: K): bool;
</code></pre>




<a id="0x1_table_spec_remove"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_spec_remove">spec_remove</a>&lt;K, V&gt;(t: <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, k: K): <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;;
</code></pre>




<a id="0x1_table_spec_set"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_spec_set">spec_set</a>&lt;K, V&gt;(t: <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, k: K, v: V): <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;;
</code></pre>




<a id="0x1_table_spec_get"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table.md#0x1_table_spec_get">spec_get</a>&lt;K, V&gt;(t: <a href="table.md#0x1_table_Table">Table</a>&lt;K, V&gt;, k: K): V;
</code></pre>



<a id="@Specification_0_destroy_known_empty_unsafe"></a>

### Function `destroy_known_empty_unsafe`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="table.md#0x1_table_destroy_known_empty_unsafe">destroy_known_empty_unsafe</a>&lt;K: <b>copy</b>, drop, V&gt;(self: <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
