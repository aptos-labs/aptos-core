
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="table.md#0x1_table">0x1::table</a>;<br /></code></pre>



<a id="0x1_table_with_length_TableWithLength"></a>

## Struct `TableWithLength`

Type of tables


<pre><code><b>struct</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K: <b>copy</b>, drop, V&gt; <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;</code>
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



<pre><code><b>const</b> <a href="table_with_length.md#0x1_table_with_length_EALREADY_EXISTS">EALREADY_EXISTS</a>: u64 &#61; 100;<br /></code></pre>



<a id="0x1_table_with_length_ENOT_EMPTY"></a>



<pre><code><b>const</b> <a href="table_with_length.md#0x1_table_with_length_ENOT_EMPTY">ENOT_EMPTY</a>: u64 &#61; 102;<br /></code></pre>



<a id="0x1_table_with_length_ENOT_FOUND"></a>



<pre><code><b>const</b> <a href="table_with_length.md#0x1_table_with_length_ENOT_FOUND">ENOT_FOUND</a>: u64 &#61; 101;<br /></code></pre>



<a id="0x1_table_with_length_new"></a>

## Function `new`

Create a new Table.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_new">new</a>&lt;K: <b>copy</b>, drop, V: store&gt;(): <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_new">new</a>&lt;K: <b>copy</b> &#43; drop, V: store&gt;(): <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt; &#123;<br />    <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a> &#123;<br />        inner: <a href="table.md#0x1_table_new">table::new</a>&lt;K, V&gt;(),<br />        length: 0,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_destroy_empty"></a>

## Function `destroy_empty`

Destroy a table. The table must be empty to succeed.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;) &#123;<br />    <b>assert</b>!(<a href="table.md#0x1_table">table</a>.length &#61;&#61; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="table_with_length.md#0x1_table_with_length_ENOT_EMPTY">ENOT_EMPTY</a>));<br />    <b>let</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a> &#123; inner, length: _ &#125; &#61; <a href="table.md#0x1_table">table</a>;<br />    <a href="table.md#0x1_table_destroy">table::destroy</a>(inner)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_add"></a>

## Function `add`

Add a new entry to the table. Aborts if an entry for this
key already exists. The entry itself is not stored in the
table, and cannot be discovered from it.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_add">add</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K, val: V)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_add">add</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K, val: V) &#123;<br />    <a href="table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key, val);<br />    <a href="table.md#0x1_table">table</a>.length &#61; <a href="table.md#0x1_table">table</a>.length &#43; 1;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow">borrow</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): &amp;V<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow">borrow</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K): &amp;V &#123;<br />    <a href="table.md#0x1_table_borrow">table::borrow</a>(&amp;<a href="table.md#0x1_table">table</a>.inner, key)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): &amp;<b>mut</b> V<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K): &amp;<b>mut</b> V &#123;<br />    <a href="table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_length">length</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_length">length</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;): u64 &#123;<br />    <a href="table.md#0x1_table">table</a>.length<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_empty"></a>

## Function `empty`

Returns true if this table is empty.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_empty">empty</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_empty">empty</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;): bool &#123;<br />    <a href="table.md#0x1_table">table</a>.length &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K, default: V): &amp;<b>mut</b> V<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b> &#43; drop, V: drop&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K, default: V): &amp;<b>mut</b> V &#123;<br />    <b>if</b> (<a href="table.md#0x1_table_contains">table::contains</a>(&amp;<a href="table.md#0x1_table">table</a>.inner, key)) &#123;<br />        <a href="table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key)<br />    &#125; <b>else</b> &#123;<br />        <a href="table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key, default);<br />        <a href="table.md#0x1_table">table</a>.length &#61; <a href="table.md#0x1_table">table</a>.length &#43; 1;<br />        <a href="table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.
update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_upsert">upsert</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K, value: V)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_upsert">upsert</a>&lt;K: <b>copy</b> &#43; drop, V: drop&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K, value: V) &#123;<br />    <b>if</b> (!<a href="table.md#0x1_table_contains">table::contains</a>(&amp;<a href="table.md#0x1_table">table</a>.inner, key)) &#123;<br />        <a href="table_with_length.md#0x1_table_with_length_add">add</a>(<a href="table.md#0x1_table">table</a>, <b>copy</b> key, value)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> ref &#61; <a href="table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key);<br />        &#42;ref &#61; value;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_remove"></a>

## Function `remove`

Remove from <code><a href="table.md#0x1_table">table</a></code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_remove">remove</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): V<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_remove">remove</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K): V &#123;<br />    <b>let</b> val &#61; <a href="table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> <a href="table.md#0x1_table">table</a>.inner, key);<br />    <a href="table.md#0x1_table">table</a>.length &#61; <a href="table.md#0x1_table">table</a>.length &#45; 1;<br />    val<br />&#125;<br /></code></pre>



</details>

<a id="0x1_table_with_length_contains"></a>

## Function `contains`

Returns true iff <code><a href="table.md#0x1_table">table</a></code> contains an entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_contains">contains</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_contains">contains</a>&lt;K: <b>copy</b> &#43; drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, key: K): bool &#123;<br />    <a href="table.md#0x1_table_contains">table::contains</a>(&amp;<a href="table.md#0x1_table">table</a>.inner, key)<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_TableWithLength"></a>

### Struct `TableWithLength`


<pre><code><b>struct</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K: <b>copy</b>, drop, V&gt; <b>has</b> store<br /></code></pre>



<dl>
<dt>
<code>inner: <a href="table.md#0x1_table_Table">table::Table</a>&lt;K, V&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>pragma</b> intrinsic &#61; map,<br />    map_new &#61; new,<br />    map_destroy_empty &#61; destroy_empty,<br />    map_len &#61; length,<br />    map_is_empty &#61; empty,<br />    map_has_key &#61; contains,<br />    map_add_no_override &#61; add,<br />    map_add_override_if_exists &#61; upsert,<br />    map_del_must_exist &#61; remove,<br />    map_borrow &#61; borrow,<br />    map_borrow_mut &#61; borrow_mut,<br />    map_borrow_mut_with_default &#61; borrow_mut_with_default,<br />    map_spec_get &#61; spec_get,<br />    map_spec_set &#61; spec_set,<br />    map_spec_del &#61; spec_remove,<br />    map_spec_len &#61; spec_len,<br />    map_spec_has_key &#61; spec_contains;<br /></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_new">new</a>&lt;K: <b>copy</b>, drop, V: store&gt;(): <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_destroy_empty">destroy_empty</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_add">add</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K, val: V)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow">borrow</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): &amp;V<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut">borrow_mut</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): &amp;<b>mut</b> V<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_length">length</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;): u64<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_empty">empty</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_borrow_mut_with_default"></a>

### Function `borrow_mut_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K, default: V): &amp;<b>mut</b> V<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_upsert">upsert</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K, value: V)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_remove">remove</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): V<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_contains">contains</a>&lt;K: <b>copy</b>, drop, V&gt;(<a href="table.md#0x1_table">table</a>: &amp;<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;K, V&gt;, key: K): bool<br /></code></pre>




<pre><code><b>pragma</b> intrinsic;<br /></code></pre>




<a id="0x1_table_with_length_spec_len"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_spec_len">spec_len</a>&lt;K, V&gt;(t: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;): num;<br /></code></pre>




<a id="0x1_table_with_length_spec_contains"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_spec_contains">spec_contains</a>&lt;K, V&gt;(t: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, k: K): bool;<br /></code></pre>




<a id="0x1_table_with_length_spec_set"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_spec_set">spec_set</a>&lt;K, V&gt;(t: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, k: K, v: V): <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;;<br /></code></pre>




<a id="0x1_table_with_length_spec_remove"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_spec_remove">spec_remove</a>&lt;K, V&gt;(t: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, k: K): <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;;<br /></code></pre>




<a id="0x1_table_with_length_spec_get"></a>


<pre><code><b>native</b> <b>fun</b> <a href="table_with_length.md#0x1_table_with_length_spec_get">spec_get</a>&lt;K, V&gt;(t: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">TableWithLength</a>&lt;K, V&gt;, k: K): V;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
