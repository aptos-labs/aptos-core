
<a id="0x1_string"></a>

# Module `0x1::string`

The <code><a href="string.md#0x1_string">string</a></code> module defines the <code><a href="string.md#0x1_string_String">String</a></code> type which represents UTF8 encoded strings.


-  [Struct `String`](#0x1_string_String)
-  [Constants](#@Constants_0)
-  [Function `utf8`](#0x1_string_utf8)
-  [Function `try_utf8`](#0x1_string_try_utf8)
-  [Function `bytes`](#0x1_string_bytes)
-  [Function `is_empty`](#0x1_string_is_empty)
-  [Function `length`](#0x1_string_length)
-  [Function `append`](#0x1_string_append)
-  [Function `append_utf8`](#0x1_string_append_utf8)
-  [Function `insert`](#0x1_string_insert)
-  [Function `sub_string`](#0x1_string_sub_string)
-  [Function `index_of`](#0x1_string_index_of)
-  [Function `internal_check_utf8`](#0x1_string_internal_check_utf8)
-  [Function `internal_is_char_boundary`](#0x1_string_internal_is_char_boundary)
-  [Function `internal_sub_string`](#0x1_string_internal_sub_string)
-  [Function `internal_index_of`](#0x1_string_internal_index_of)
-  [Specification](#@Specification_1)
    -  [Function `internal_check_utf8`](#@Specification_1_internal_check_utf8)
    -  [Function `internal_is_char_boundary`](#@Specification_1_internal_is_char_boundary)
    -  [Function `internal_sub_string`](#@Specification_1_internal_sub_string)
    -  [Function `internal_index_of`](#@Specification_1_internal_index_of)


<pre><code><b>use</b> <a href="option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_string_String"></a>

## Struct `String`

A <code><a href="string.md#0x1_string_String">String</a></code> holds a sequence of bytes which is guaranteed to be in utf8 format.


<pre><code><b>struct</b> <a href="string.md#0x1_string_String">String</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_string_EINVALID_INDEX"></a>

Index out of range.


<pre><code><b>const</b> <a href="string.md#0x1_string_EINVALID_INDEX">EINVALID_INDEX</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_string_EINVALID_UTF8"></a>

An invalid UTF8 encoding.


<pre><code><b>const</b> <a href="string.md#0x1_string_EINVALID_UTF8">EINVALID_UTF8</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_string_utf8"></a>

## Function `utf8`

Creates a new string from a sequence of bytes. Aborts if the bytes do not represent valid utf8.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_utf8">utf8</a>(bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_utf8">utf8</a>(bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="string.md#0x1_string_String">String</a> &#123;<br />    <b>assert</b>!(<a href="string.md#0x1_string_internal_check_utf8">internal_check_utf8</a>(&amp;bytes), <a href="string.md#0x1_string_EINVALID_UTF8">EINVALID_UTF8</a>);<br />    <a href="string.md#0x1_string_String">String</a>&#123;bytes&#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_try_utf8"></a>

## Function `try_utf8`

Tries to create a new string from a sequence of bytes.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_try_utf8">try_utf8</a>(bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="option.md#0x1_option_Option">option::Option</a>&lt;<a href="string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_try_utf8">try_utf8</a>(bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="string.md#0x1_string_String">String</a>&gt; &#123;<br />    <b>if</b> (<a href="string.md#0x1_string_internal_check_utf8">internal_check_utf8</a>(&amp;bytes)) &#123;<br />        <a href="option.md#0x1_option_some">option::some</a>(<a href="string.md#0x1_string_String">String</a>&#123;bytes&#125;)<br />    &#125; <b>else</b> &#123;<br />        <a href="option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_bytes"></a>

## Function `bytes`

Returns a reference to the underlying byte vector.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_bytes">bytes</a>(s: &amp;<a href="string.md#0x1_string_String">string::String</a>): &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_bytes">bytes</a>(s: &amp;<a href="string.md#0x1_string_String">String</a>): &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    &amp;s.bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_is_empty"></a>

## Function `is_empty`

Checks whether this string is empty.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_is_empty">is_empty</a>(s: &amp;<a href="string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_is_empty">is_empty</a>(s: &amp;<a href="string.md#0x1_string_String">String</a>): bool &#123;<br />    <a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;s.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_length"></a>

## Function `length`

Returns the length of this string, in bytes.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_length">length</a>(s: &amp;<a href="string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_length">length</a>(s: &amp;<a href="string.md#0x1_string_String">String</a>): u64 &#123;<br />    <a href="vector.md#0x1_vector_length">vector::length</a>(&amp;s.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_append"></a>

## Function `append`

Appends a string.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_append">append</a>(s: &amp;<b>mut</b> <a href="string.md#0x1_string_String">string::String</a>, r: <a href="string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_append">append</a>(s: &amp;<b>mut</b> <a href="string.md#0x1_string_String">String</a>, r: <a href="string.md#0x1_string_String">String</a>) &#123;<br />    <a href="vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> s.bytes, r.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_append_utf8"></a>

## Function `append_utf8`

Appends bytes which must be in valid utf8 format.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_append_utf8">append_utf8</a>(s: &amp;<b>mut</b> <a href="string.md#0x1_string_String">string::String</a>, bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_append_utf8">append_utf8</a>(s: &amp;<b>mut</b> <a href="string.md#0x1_string_String">String</a>, bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;) &#123;<br />    <a href="string.md#0x1_string_append">append</a>(s, <a href="string.md#0x1_string_utf8">utf8</a>(bytes))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_insert"></a>

## Function `insert`

Insert the other string at the byte index in given string. The index must be at a valid utf8 char
boundary.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_insert">insert</a>(s: &amp;<b>mut</b> <a href="string.md#0x1_string_String">string::String</a>, at: u64, o: <a href="string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_insert">insert</a>(s: &amp;<b>mut</b> <a href="string.md#0x1_string_String">String</a>, at: u64, o: <a href="string.md#0x1_string_String">String</a>) &#123;<br />    <b>let</b> bytes &#61; &amp;s.bytes;<br />    <b>assert</b>!(at &lt;&#61; <a href="vector.md#0x1_vector_length">vector::length</a>(bytes) &amp;&amp; <a href="string.md#0x1_string_internal_is_char_boundary">internal_is_char_boundary</a>(bytes, at), <a href="string.md#0x1_string_EINVALID_INDEX">EINVALID_INDEX</a>);<br />    <b>let</b> l &#61; <a href="string.md#0x1_string_length">length</a>(s);<br />    <b>let</b> front &#61; <a href="string.md#0x1_string_sub_string">sub_string</a>(s, 0, at);<br />    <b>let</b> end &#61; <a href="string.md#0x1_string_sub_string">sub_string</a>(s, at, l);<br />    <a href="string.md#0x1_string_append">append</a>(&amp;<b>mut</b> front, o);<br />    <a href="string.md#0x1_string_append">append</a>(&amp;<b>mut</b> front, end);<br />    &#42;s &#61; front;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_sub_string"></a>

## Function `sub_string`

Returns a sub&#45;string using the given byte indices, where <code>i</code> is the first byte position and <code>j</code> is the start
of the first byte not included (or the length of the string). The indices must be at valid utf8 char boundaries,
guaranteeing that the result is valid utf8.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_sub_string">sub_string</a>(s: &amp;<a href="string.md#0x1_string_String">string::String</a>, i: u64, j: u64): <a href="string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_sub_string">sub_string</a>(s: &amp;<a href="string.md#0x1_string_String">String</a>, i: u64, j: u64): <a href="string.md#0x1_string_String">String</a> &#123;<br />    <b>let</b> bytes &#61; &amp;s.bytes;<br />    <b>let</b> l &#61; <a href="vector.md#0x1_vector_length">vector::length</a>(bytes);<br />    <b>assert</b>!(<br />        j &lt;&#61; l &amp;&amp; i &lt;&#61; j &amp;&amp; <a href="string.md#0x1_string_internal_is_char_boundary">internal_is_char_boundary</a>(bytes, i) &amp;&amp; <a href="string.md#0x1_string_internal_is_char_boundary">internal_is_char_boundary</a>(bytes, j),<br />        <a href="string.md#0x1_string_EINVALID_INDEX">EINVALID_INDEX</a><br />    );<br />    <a href="string.md#0x1_string_String">String</a> &#123; bytes: <a href="string.md#0x1_string_internal_sub_string">internal_sub_string</a>(bytes, i, j) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_index_of"></a>

## Function `index_of`

Computes the index of the first occurrence of a string. Returns <code><a href="string.md#0x1_string_length">length</a>(s)</code> if no occurrence found.


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_index_of">index_of</a>(s: &amp;<a href="string.md#0x1_string_String">string::String</a>, r: &amp;<a href="string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_index_of">index_of</a>(s: &amp;<a href="string.md#0x1_string_String">String</a>, r: &amp;<a href="string.md#0x1_string_String">String</a>): u64 &#123;<br />    <a href="string.md#0x1_string_internal_index_of">internal_index_of</a>(&amp;s.bytes, &amp;r.bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_string_internal_check_utf8"></a>

## Function `internal_check_utf8`



<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_internal_check_utf8">internal_check_utf8</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="string.md#0x1_string_internal_check_utf8">internal_check_utf8</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /></code></pre>



</details>

<a id="0x1_string_internal_is_char_boundary"></a>

## Function `internal_is_char_boundary`



<pre><code><b>fun</b> <a href="string.md#0x1_string_internal_is_char_boundary">internal_is_char_boundary</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string.md#0x1_string_internal_is_char_boundary">internal_is_char_boundary</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64): bool;<br /></code></pre>



</details>

<a id="0x1_string_internal_sub_string"></a>

## Function `internal_sub_string`



<pre><code><b>fun</b> <a href="string.md#0x1_string_internal_sub_string">internal_sub_string</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64, j: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string.md#0x1_string_internal_sub_string">internal_sub_string</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64, j: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /></code></pre>



</details>

<a id="0x1_string_internal_index_of"></a>

## Function `internal_index_of`



<pre><code><b>fun</b> <a href="string.md#0x1_string_internal_index_of">internal_index_of</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, r: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string.md#0x1_string_internal_index_of">internal_index_of</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, r: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_internal_check_utf8"></a>

### Function `internal_check_utf8`


<pre><code><b>public</b> <b>fun</b> <a href="string.md#0x1_string_internal_check_utf8">internal_check_utf8</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="string.md#0x1_string_spec_internal_check_utf8">spec_internal_check_utf8</a>(v);<br /></code></pre>



<a id="@Specification_1_internal_is_char_boundary"></a>

### Function `internal_is_char_boundary`


<pre><code><b>fun</b> <a href="string.md#0x1_string_internal_is_char_boundary">internal_is_char_boundary</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="string.md#0x1_string_spec_internal_is_char_boundary">spec_internal_is_char_boundary</a>(v, i);<br /></code></pre>



<a id="@Specification_1_internal_sub_string"></a>

### Function `internal_sub_string`


<pre><code><b>fun</b> <a href="string.md#0x1_string_internal_sub_string">internal_sub_string</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64, j: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="string.md#0x1_string_spec_internal_sub_string">spec_internal_sub_string</a>(v, i, j);<br /></code></pre>



<a id="@Specification_1_internal_index_of"></a>

### Function `internal_index_of`


<pre><code><b>fun</b> <a href="string.md#0x1_string_internal_index_of">internal_index_of</a>(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, r: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="string.md#0x1_string_spec_internal_index_of">spec_internal_index_of</a>(v, r);<br /></code></pre>




<a id="0x1_string_spec_utf8"></a>


<pre><code><b>fun</b> <a href="string.md#0x1_string_spec_utf8">spec_utf8</a>(bytes: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="string.md#0x1_string_String">String</a> &#123;<br />   <a href="string.md#0x1_string_String">String</a>&#123;bytes&#125;<br />&#125;<br /></code></pre>




<a id="0x1_string_spec_internal_check_utf8"></a>


<pre><code><b>fun</b> <a href="string.md#0x1_string_spec_internal_check_utf8">spec_internal_check_utf8</a>(v: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;<br /><a id="0x1_string_spec_internal_is_char_boundary"></a>
<b>fun</b> <a href="string.md#0x1_string_spec_internal_is_char_boundary">spec_internal_is_char_boundary</a>(v: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64): bool;<br /><a id="0x1_string_spec_internal_sub_string"></a>
<b>fun</b> <a href="string.md#0x1_string_spec_internal_sub_string">spec_internal_sub_string</a>(v: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, i: u64, j: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><a id="0x1_string_spec_internal_index_of"></a>
<b>fun</b> <a href="string.md#0x1_string_spec_internal_index_of">spec_internal_index_of</a>(v: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, r: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
