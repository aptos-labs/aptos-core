
<a name="0x1_string_utils"></a>

# Module `0x1::string_utils`



-  [Struct `List`](#0x1_string_utils_List)
-  [Struct `NIL`](#0x1_string_utils_NIL)
-  [Struct `FakeList`](#0x1_string_utils_FakeList)
-  [Constants](#@Constants_0)
-  [Function `cons`](#0x1_string_utils_cons)
-  [Function `nil`](#0x1_string_utils_nil)
-  [Function `format`](#0x1_string_utils_format)
-  [Function `format_list`](#0x1_string_utils_format_list)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_string_utils_List"></a>

## Struct `List`



<pre><code><b>struct</b> <a href="string_utils.md#0x1_string_utils_List">List</a>&lt;T, N&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>car: T</code>
</dt>
<dd>

</dd>
<dt>
<code>cdr: N</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_string_utils_NIL"></a>

## Struct `NIL`



<pre><code><b>struct</b> <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_string_utils_FakeList"></a>

## Struct `FakeList`



<pre><code><b>struct</b> <a href="string_utils.md#0x1_string_utils_FakeList">FakeList</a>&lt;T, N&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>car: T</code>
</dt>
<dd>

</dd>
<dt>
<code>cdr: N</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_string_utils_EARGS_MISMATCH"></a>



<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EARGS_MISMATCH">EARGS_MISMATCH</a>: u64 = 1;
</code></pre>



<a name="0x1_string_utils_EINVALID_FORMAT"></a>



<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EINVALID_FORMAT">EINVALID_FORMAT</a>: u64 = 2;
</code></pre>



<a name="0x1_string_utils_cons"></a>

## Function `cons`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_cons">cons</a>&lt;T, N&gt;(car: T, cdr: N): <a href="string_utils.md#0x1_string_utils_List">string_utils::List</a>&lt;T, N&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_cons">cons</a>&lt;T, N&gt;(car: T, cdr: N): <a href="string_utils.md#0x1_string_utils_List">List</a>&lt;T, N&gt; {
    <a href="string_utils.md#0x1_string_utils_List">List</a> { car, cdr }
}
</code></pre>



</details>

<a name="0x1_string_utils_nil"></a>

## Function `nil`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_nil">nil</a>(): <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_nil">nil</a>(): <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> { <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> {} }
</code></pre>



</details>

<a name="0x1_string_utils_format"></a>

## Function `format`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_format">format</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format">format</a>&lt;T&gt;(s: &T): String;
</code></pre>



</details>

<a name="0x1_string_utils_format_list"></a>

## Function `format_list`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_format_list">format_list</a>&lt;T&gt;(fmt: &<a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, val: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format_list">format_list</a>&lt;T&gt;(fmt: &String, val: &T): String;
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
