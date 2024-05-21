
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
-  [Function `destroy`](#0x1_table_destroy)
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
    -  [Function `borrow_mut`](#@Specification_0_borrow_mut)
    -  [Function `borrow_mut_with_default`](#@Specification_0_borrow_mut_with_default)
    -  [Function `upsert`](#@Specification_0_upsert)
    -  [Function `remove`](#@Specification_0_remove)
    -  [Function `contains`](#@Specification_0_contains)
    -  [Function `destroy`](#@Specification_0_destroy)


<pre><code></code></pre>



<a id="0x1_table_Table"></a>

## Struct `Table`

Type of tables


<pre><code>struct Table&lt;K: copy, drop, V&gt; has store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_table_Box"></a>

## Resource `Box`

Wrapper for values. Required for making values appear as resources in the implementation.


<pre><code>struct Box&lt;V&gt; has drop, store, key
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


<pre><code>public fun new&lt;K: copy, drop, V: store&gt;(): table::Table&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;K: copy &#43; drop, V: store&gt;(): Table&lt;K, V&gt; &#123;
    Table &#123;
        handle: new_table_handle&lt;K, V&gt;(),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_table_add"></a>

## Function `add`

Add a new entry to the table. Aborts if an entry for this
key already exists. The entry itself is not stored in the
table, and cannot be discovered from it.


<pre><code>public fun add&lt;K: copy, drop, V&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, val: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;K: copy &#43; drop, V&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K, val: V) &#123;
    add_box&lt;K, V, Box&lt;V&gt;&gt;(table, key, Box &#123; val &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow&lt;K: copy, drop, V&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K): &amp;V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;K: copy &#43; drop, V&gt;(table: &amp;Table&lt;K, V&gt;, key: K): &amp;V &#123;
    &amp;borrow_box&lt;K, V, Box&lt;V&gt;&gt;(table, key).val
&#125;
</code></pre>



</details>

<a id="0x1_table_borrow_with_default"></a>

## Function `borrow_with_default`

Acquire an immutable reference to the value which <code>key</code> maps to.
Returns specified default value if there is no entry for <code>key</code>.


<pre><code>public fun borrow_with_default&lt;K: copy, drop, V&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K, default: &amp;V): &amp;V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_with_default&lt;K: copy &#43; drop, V&gt;(table: &amp;Table&lt;K, V&gt;, key: K, default: &amp;V): &amp;V &#123;
    if (!contains(table, copy key)) &#123;
        default
    &#125; else &#123;
        borrow(table, copy key)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut&lt;K: copy, drop, V&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K): &amp;mut V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;K: copy &#43; drop, V&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K): &amp;mut V &#123;
    &amp;mut borrow_box_mut&lt;K, V, Box&lt;V&gt;&gt;(table, key).val
&#125;
</code></pre>



</details>

<a id="0x1_table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut_with_default&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, default: V): &amp;mut V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut_with_default&lt;K: copy &#43; drop, V: drop&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K, default: V): &amp;mut V &#123;
    if (!contains(table, copy key)) &#123;
        add(table, copy key, default)
    &#125;;
    borrow_mut(table, key)
&#125;
</code></pre>



</details>

<a id="0x1_table_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.
update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code>public fun upsert&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert&lt;K: copy &#43; drop, V: drop&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K, value: V) &#123;
    if (!contains(table, copy key)) &#123;
        add(table, copy key, value)
    &#125; else &#123;
        let ref &#61; borrow_mut(table, key);
        &#42;ref &#61; value;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_table_remove"></a>

## Function `remove`

Remove from <code>table</code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code>public fun remove&lt;K: copy, drop, V&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;K: copy &#43; drop, V&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K): V &#123;
    let Box &#123; val &#125; &#61; remove_box&lt;K, V, Box&lt;V&gt;&gt;(table, key);
    val
&#125;
</code></pre>



</details>

<a id="0x1_table_contains"></a>

## Function `contains`

Returns true iff <code>table</code> contains an entry for <code>key</code>.


<pre><code>public fun contains&lt;K: copy, drop, V&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;K: copy &#43; drop, V&gt;(table: &amp;Table&lt;K, V&gt;, key: K): bool &#123;
    contains_box&lt;K, V, Box&lt;V&gt;&gt;(table, key)
&#125;
</code></pre>



</details>

<a id="0x1_table_destroy"></a>

## Function `destroy`



<pre><code>public(friend) fun destroy&lt;K: copy, drop, V&gt;(table: table::Table&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun destroy&lt;K: copy &#43; drop, V&gt;(table: Table&lt;K, V&gt;) &#123;
    destroy_empty_box&lt;K, V, Box&lt;V&gt;&gt;(&amp;table);
    drop_unchecked_box&lt;K, V, Box&lt;V&gt;&gt;(table)
&#125;
</code></pre>



</details>

<a id="0x1_table_new_table_handle"></a>

## Function `new_table_handle`



<pre><code>fun new_table_handle&lt;K, V&gt;(): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun new_table_handle&lt;K, V&gt;(): address;
</code></pre>



</details>

<a id="0x1_table_add_box"></a>

## Function `add_box`



<pre><code>fun add_box&lt;K: copy, drop, V, B&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, val: table::Box&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun add_box&lt;K: copy &#43; drop, V, B&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K, val: Box&lt;V&gt;);
</code></pre>



</details>

<a id="0x1_table_borrow_box"></a>

## Function `borrow_box`



<pre><code>fun borrow_box&lt;K: copy, drop, V, B&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K): &amp;table::Box&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun borrow_box&lt;K: copy &#43; drop, V, B&gt;(table: &amp;Table&lt;K, V&gt;, key: K): &amp;Box&lt;V&gt;;
</code></pre>



</details>

<a id="0x1_table_borrow_box_mut"></a>

## Function `borrow_box_mut`



<pre><code>fun borrow_box_mut&lt;K: copy, drop, V, B&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K): &amp;mut table::Box&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun borrow_box_mut&lt;K: copy &#43; drop, V, B&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K): &amp;mut Box&lt;V&gt;;
</code></pre>



</details>

<a id="0x1_table_contains_box"></a>

## Function `contains_box`



<pre><code>fun contains_box&lt;K: copy, drop, V, B&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun contains_box&lt;K: copy &#43; drop, V, B&gt;(table: &amp;Table&lt;K, V&gt;, key: K): bool;
</code></pre>



</details>

<a id="0x1_table_remove_box"></a>

## Function `remove_box`



<pre><code>fun remove_box&lt;K: copy, drop, V, B&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K): table::Box&lt;V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun remove_box&lt;K: copy &#43; drop, V, B&gt;(table: &amp;mut Table&lt;K, V&gt;, key: K): Box&lt;V&gt;;
</code></pre>



</details>

<a id="0x1_table_destroy_empty_box"></a>

## Function `destroy_empty_box`



<pre><code>fun destroy_empty_box&lt;K: copy, drop, V, B&gt;(table: &amp;table::Table&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun destroy_empty_box&lt;K: copy &#43; drop, V, B&gt;(table: &amp;Table&lt;K, V&gt;);
</code></pre>



</details>

<a id="0x1_table_drop_unchecked_box"></a>

## Function `drop_unchecked_box`



<pre><code>fun drop_unchecked_box&lt;K: copy, drop, V, B&gt;(table: table::Table&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun drop_unchecked_box&lt;K: copy &#43; drop, V, B&gt;(table: Table&lt;K, V&gt;);
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_Table"></a>

### Struct `Table`


<pre><code>struct Table&lt;K: copy, drop, V&gt; has store
</code></pre>



<dl>
<dt>
<code>handle: address</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma intrinsic &#61; map,
    map_new &#61; new,
    map_destroy_empty &#61; destroy,
    map_has_key &#61; contains,
    map_add_no_override &#61; add,
    map_add_override_if_exists &#61; upsert,
    map_del_must_exist &#61; remove,
    map_borrow &#61; borrow,
    map_borrow_mut &#61; borrow_mut,
    map_borrow_mut_with_default &#61; borrow_mut_with_default,
    map_spec_get &#61; spec_get,
    map_spec_set &#61; spec_set,
    map_spec_del &#61; spec_remove,
    map_spec_has_key &#61; spec_contains;
</code></pre>



<a id="@Specification_0_new"></a>

### Function `new`


<pre><code>public fun new&lt;K: copy, drop, V: store&gt;(): table::Table&lt;K, V&gt;
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_add"></a>

### Function `add`


<pre><code>public fun add&lt;K: copy, drop, V&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, val: V)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;K: copy, drop, V&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K): &amp;V
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;K: copy, drop, V&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K): &amp;mut V
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_borrow_mut_with_default"></a>

### Function `borrow_mut_with_default`


<pre><code>public fun borrow_mut_with_default&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, default: V): &amp;mut V
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_upsert"></a>

### Function `upsert`


<pre><code>public fun upsert&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K, value: V)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;K: copy, drop, V&gt;(table: &amp;mut table::Table&lt;K, V&gt;, key: K): V
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>



<a id="@Specification_0_contains"></a>

### Function `contains`


<pre><code>public fun contains&lt;K: copy, drop, V&gt;(table: &amp;table::Table&lt;K, V&gt;, key: K): bool
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>




<a id="0x1_table_spec_contains"></a>


<pre><code>native fun spec_contains&lt;K, V&gt;(t: Table&lt;K, V&gt;, k: K): bool;
</code></pre>




<a id="0x1_table_spec_remove"></a>


<pre><code>native fun spec_remove&lt;K, V&gt;(t: Table&lt;K, V&gt;, k: K): Table&lt;K, V&gt;;
</code></pre>




<a id="0x1_table_spec_set"></a>


<pre><code>native fun spec_set&lt;K, V&gt;(t: Table&lt;K, V&gt;, k: K, v: V): Table&lt;K, V&gt;;
</code></pre>




<a id="0x1_table_spec_get"></a>


<pre><code>native fun spec_get&lt;K, V&gt;(t: Table&lt;K, V&gt;, k: K): V;
</code></pre>



<a id="@Specification_0_destroy"></a>

### Function `destroy`


<pre><code>public(friend) fun destroy&lt;K: copy, drop, V&gt;(table: table::Table&lt;K, V&gt;)
</code></pre>




<pre><code>pragma intrinsic;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
