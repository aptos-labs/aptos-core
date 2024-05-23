
<a id="0x1_table_with_length"></a>

# Module `0x1::table_with_length`

Extends Table and provides functions such as length and the ability to be destroyed


-  [Struct `TableWithLength`](#0x1_table_with_length_TableWithLength)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_table_with_length_new)
-  [Function `destroy_empty`](#0x1_table_with_length_destroy_empty)
-  [Function `add`](#0x1_table_with_length_add)
-  [Function `borrow`](#0x1_table_with_length_borrow)
-  [Function `borrow_mut`](#0x1_table_with_length_borrow_mut)
-  [Function `length`](#0x1_table_with_length_length)
-  [Function `empty`](#0x1_table_with_length_empty)
-  [Function `borrow_mut_with_default`](#0x1_table_with_length_borrow_mut_with_default)
-  [Function `upsert`](#0x1_table_with_length_upsert)
-  [Function `remove`](#0x1_table_with_length_remove)
-  [Function `contains`](#0x1_table_with_length_contains)
-  [Specification](#@Specification_1)
    -  [Struct `TableWithLength`](#@Specification_1_TableWithLength)
    -  [Function `new`](#@Specification_1_new)
    -  [Function `destroy_empty`](#@Specification_1_destroy_empty)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `borrow_mut`](#@Specification_1_borrow_mut)
    -  [Function `length`](#@Specification_1_length)
    -  [Function `empty`](#@Specification_1_empty)
    -  [Function `borrow_mut_with_default`](#@Specification_1_borrow_mut_with_default)
    -  [Function `upsert`](#@Specification_1_upsert)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `contains`](#@Specification_1_contains)


<pre><code>use 0x1::error;<br/>use 0x1::table;<br/></code></pre>



<a id="0x1_table_with_length_TableWithLength"></a>

## Struct `TableWithLength`

Type of tables


<pre><code>struct TableWithLength&lt;K: copy, drop, V&gt; has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: table::Table&lt;K, V&gt;</code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_table_with_length_EALREADY_EXISTS"></a>



<pre><code>const EALREADY_EXISTS: u64 &#61; 100;<br/></code></pre>



<a id="0x1_table_with_length_ENOT_EMPTY"></a>



<pre><code>const ENOT_EMPTY: u64 &#61; 102;<br/></code></pre>



<a id="0x1_table_with_length_ENOT_FOUND"></a>



<pre><code>const ENOT_FOUND: u64 &#61; 101;<br/></code></pre>



<a id="0x1_table_with_length_new"></a>

## Function `new`

Create a new Table.


<pre><code>public fun new&lt;K: copy, drop, V: store&gt;(): table_with_length::TableWithLength&lt;K, V&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;K: copy &#43; drop, V: store&gt;(): TableWithLength&lt;K, V&gt; &#123;<br/>    TableWithLength &#123;<br/>        inner: table::new&lt;K, V&gt;(),<br/>        length: 0,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_destroy_empty"></a>

## Function `destroy_empty`

Destroy a table. The table must be empty to succeed.


<pre><code>public fun destroy_empty&lt;K: copy, drop, V&gt;(table: table_with_length::TableWithLength&lt;K, V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;K: copy &#43; drop, V&gt;(table: TableWithLength&lt;K, V&gt;) &#123;<br/>    assert!(table.length &#61;&#61; 0, error::invalid_state(ENOT_EMPTY));<br/>    let TableWithLength &#123; inner, length: _ &#125; &#61; table;<br/>    table::destroy(inner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_add"></a>

## Function `add`

Add a new entry to the table. Aborts if an entry for this<br/> key already exists. The entry itself is not stored in the<br/> table, and cannot be discovered from it.


<pre><code>public fun add&lt;K: copy, drop, V&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K, val: V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;K: copy &#43; drop, V&gt;(table: &amp;mut TableWithLength&lt;K, V&gt;, key: K, val: V) &#123;<br/>    table::add(&amp;mut table.inner, key, val);<br/>    table.length &#61; table.length &#43; 1;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.<br/> Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;, key: K): &amp;V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;K: copy &#43; drop, V&gt;(table: &amp;TableWithLength&lt;K, V&gt;, key: K): &amp;V &#123;<br/>    table::borrow(&amp;table.inner, key)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.<br/> Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut&lt;K: copy, drop, V&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K): &amp;mut V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;K: copy &#43; drop, V&gt;(table: &amp;mut TableWithLength&lt;K, V&gt;, key: K): &amp;mut V &#123;<br/>    table::borrow_mut(&amp;mut table.inner, key)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code>public fun length&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;K: copy &#43; drop, V&gt;(table: &amp;TableWithLength&lt;K, V&gt;): u64 &#123;<br/>    table.length<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_empty"></a>

## Function `empty`

Returns true if this table is empty.


<pre><code>public fun empty&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun empty&lt;K: copy &#43; drop, V&gt;(table: &amp;TableWithLength&lt;K, V&gt;): bool &#123;<br/>    table.length &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.<br/> Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut_with_default&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K, default: V): &amp;mut V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut_with_default&lt;K: copy &#43; drop, V: drop&gt;(table: &amp;mut TableWithLength&lt;K, V&gt;, key: K, default: V): &amp;mut V &#123;<br/>    if (table::contains(&amp;table.inner, key)) &#123;<br/>        table::borrow_mut(&amp;mut table.inner, key)<br/>    &#125; else &#123;<br/>        table::add(&amp;mut table.inner, key, default);<br/>        table.length &#61; table.length &#43; 1;<br/>        table::borrow_mut(&amp;mut table.inner, key)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.<br/> update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code>public fun upsert&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K, value: V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert&lt;K: copy &#43; drop, V: drop&gt;(table: &amp;mut TableWithLength&lt;K, V&gt;, key: K, value: V) &#123;<br/>    if (!table::contains(&amp;table.inner, key)) &#123;<br/>        add(table, copy key, value)<br/>    &#125; else &#123;<br/>        let ref &#61; table::borrow_mut(&amp;mut table.inner, key);<br/>        &#42;ref &#61; value;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_remove"></a>

## Function `remove`

Remove from <code>table</code> and return the value which <code>key</code> maps to.<br/> Aborts if there is no entry for <code>key</code>.


<pre><code>public fun remove&lt;K: copy, drop, V&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K): V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;K: copy &#43; drop, V&gt;(table: &amp;mut TableWithLength&lt;K, V&gt;, key: K): V &#123;<br/>    let val &#61; table::remove(&amp;mut table.inner, key);<br/>    table.length &#61; table.length &#45; 1;<br/>    val<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_table_with_length_contains"></a>

## Function `contains`

Returns true iff <code>table</code> contains an entry for <code>key</code>.


<pre><code>public fun contains&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;, key: K): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;K: copy &#43; drop, V&gt;(table: &amp;TableWithLength&lt;K, V&gt;, key: K): bool &#123;<br/>    table::contains(&amp;table.inner, key)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_TableWithLength"></a>

### Struct `TableWithLength`


<pre><code>struct TableWithLength&lt;K: copy, drop, V&gt; has store<br/></code></pre>



<dl>
<dt>
<code>inner: table::Table&lt;K, V&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma intrinsic &#61; map,<br/>    map_new &#61; new,<br/>    map_destroy_empty &#61; destroy_empty,<br/>    map_len &#61; length,<br/>    map_is_empty &#61; empty,<br/>    map_has_key &#61; contains,<br/>    map_add_no_override &#61; add,<br/>    map_add_override_if_exists &#61; upsert,<br/>    map_del_must_exist &#61; remove,<br/>    map_borrow &#61; borrow,<br/>    map_borrow_mut &#61; borrow_mut,<br/>    map_borrow_mut_with_default &#61; borrow_mut_with_default,<br/>    map_spec_get &#61; spec_get,<br/>    map_spec_set &#61; spec_set,<br/>    map_spec_del &#61; spec_remove,<br/>    map_spec_len &#61; spec_len,<br/>    map_spec_has_key &#61; spec_contains;<br/></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public fun new&lt;K: copy, drop, V: store&gt;(): table_with_length::TableWithLength&lt;K, V&gt;<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code>public fun destroy_empty&lt;K: copy, drop, V&gt;(table: table_with_length::TableWithLength&lt;K, V&gt;)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add&lt;K: copy, drop, V&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K, val: V)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;, key: K): &amp;V<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;K: copy, drop, V&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K): &amp;mut V<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code>public fun length&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;): u64<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>public fun empty&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;): bool<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_borrow_mut_with_default"></a>

### Function `borrow_mut_with_default`


<pre><code>public fun borrow_mut_with_default&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K, default: V): &amp;mut V<br/></code></pre>




<pre><code>aborts_if false;<br/>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code>public fun upsert&lt;K: copy, drop, V: drop&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K, value: V)<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;K: copy, drop, V&gt;(table: &amp;mut table_with_length::TableWithLength&lt;K, V&gt;, key: K): V<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code>public fun contains&lt;K: copy, drop, V&gt;(table: &amp;table_with_length::TableWithLength&lt;K, V&gt;, key: K): bool<br/></code></pre>




<pre><code>pragma intrinsic;<br/></code></pre>




<a id="0x1_table_with_length_spec_len"></a>


<pre><code>native fun spec_len&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;): num;<br/></code></pre>




<a id="0x1_table_with_length_spec_contains"></a>


<pre><code>native fun spec_contains&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): bool;<br/></code></pre>




<a id="0x1_table_with_length_spec_set"></a>


<pre><code>native fun spec_set&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K, v: V): TableWithLength&lt;K, V&gt;;<br/></code></pre>




<a id="0x1_table_with_length_spec_remove"></a>


<pre><code>native fun spec_remove&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): TableWithLength&lt;K, V&gt;;<br/></code></pre>




<a id="0x1_table_with_length_spec_get"></a>


<pre><code>native fun spec_get&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): V;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
