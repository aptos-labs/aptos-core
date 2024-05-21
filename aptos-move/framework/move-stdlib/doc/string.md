
<a id="0x1_string"></a>

# Module `0x1::string`

The <code>string</code> module defines the <code>String</code> type which represents UTF8 encoded strings.


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


<pre><code>use 0x1::option;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_string_String"></a>

## Struct `String`

A <code>String</code> holds a sequence of bytes which is guaranteed to be in utf8 format.


<pre><code>struct String has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_string_EINVALID_INDEX"></a>

Index out of range.


<pre><code>const EINVALID_INDEX: u64 &#61; 2;<br/></code></pre>



<a id="0x1_string_EINVALID_UTF8"></a>

An invalid UTF8 encoding.


<pre><code>const EINVALID_UTF8: u64 &#61; 1;<br/></code></pre>



<a id="0x1_string_utf8"></a>

## Function `utf8`

Creates a new string from a sequence of bytes. Aborts if the bytes do not represent valid utf8.


<pre><code>public fun utf8(bytes: vector&lt;u8&gt;): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun utf8(bytes: vector&lt;u8&gt;): String &#123;<br/>    assert!(internal_check_utf8(&amp;bytes), EINVALID_UTF8);<br/>    String&#123;bytes&#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_try_utf8"></a>

## Function `try_utf8`

Tries to create a new string from a sequence of bytes.


<pre><code>public fun try_utf8(bytes: vector&lt;u8&gt;): option::Option&lt;string::String&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun try_utf8(bytes: vector&lt;u8&gt;): Option&lt;String&gt; &#123;<br/>    if (internal_check_utf8(&amp;bytes)) &#123;<br/>        option::some(String&#123;bytes&#125;)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_bytes"></a>

## Function `bytes`

Returns a reference to the underlying byte vector.


<pre><code>public fun bytes(s: &amp;string::String): &amp;vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun bytes(s: &amp;String): &amp;vector&lt;u8&gt; &#123;<br/>    &amp;s.bytes<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_is_empty"></a>

## Function `is_empty`

Checks whether this string is empty.


<pre><code>public fun is_empty(s: &amp;string::String): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_empty(s: &amp;String): bool &#123;<br/>    vector::is_empty(&amp;s.bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_length"></a>

## Function `length`

Returns the length of this string, in bytes.


<pre><code>public fun length(s: &amp;string::String): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length(s: &amp;String): u64 &#123;<br/>    vector::length(&amp;s.bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_append"></a>

## Function `append`

Appends a string.


<pre><code>public fun append(s: &amp;mut string::String, r: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun append(s: &amp;mut String, r: String) &#123;<br/>    vector::append(&amp;mut s.bytes, r.bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_append_utf8"></a>

## Function `append_utf8`

Appends bytes which must be in valid utf8 format.


<pre><code>public fun append_utf8(s: &amp;mut string::String, bytes: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun append_utf8(s: &amp;mut String, bytes: vector&lt;u8&gt;) &#123;<br/>    append(s, utf8(bytes))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_insert"></a>

## Function `insert`

Insert the other string at the byte index in given string. The index must be at a valid utf8 char
boundary.


<pre><code>public fun insert(s: &amp;mut string::String, at: u64, o: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun insert(s: &amp;mut String, at: u64, o: String) &#123;<br/>    let bytes &#61; &amp;s.bytes;<br/>    assert!(at &lt;&#61; vector::length(bytes) &amp;&amp; internal_is_char_boundary(bytes, at), EINVALID_INDEX);<br/>    let l &#61; length(s);<br/>    let front &#61; sub_string(s, 0, at);<br/>    let end &#61; sub_string(s, at, l);<br/>    append(&amp;mut front, o);<br/>    append(&amp;mut front, end);<br/>    &#42;s &#61; front;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_sub_string"></a>

## Function `sub_string`

Returns a sub-string using the given byte indices, where <code>i</code> is the first byte position and <code>j</code> is the start
of the first byte not included (or the length of the string). The indices must be at valid utf8 char boundaries,
guaranteeing that the result is valid utf8.


<pre><code>public fun sub_string(s: &amp;string::String, i: u64, j: u64): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sub_string(s: &amp;String, i: u64, j: u64): String &#123;<br/>    let bytes &#61; &amp;s.bytes;<br/>    let l &#61; vector::length(bytes);<br/>    assert!(<br/>        j &lt;&#61; l &amp;&amp; i &lt;&#61; j &amp;&amp; internal_is_char_boundary(bytes, i) &amp;&amp; internal_is_char_boundary(bytes, j),<br/>        EINVALID_INDEX<br/>    );<br/>    String &#123; bytes: internal_sub_string(bytes, i, j) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_index_of"></a>

## Function `index_of`

Computes the index of the first occurrence of a string. Returns <code>length(s)</code> if no occurrence found.


<pre><code>public fun index_of(s: &amp;string::String, r: &amp;string::String): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun index_of(s: &amp;String, r: &amp;String): u64 &#123;<br/>    internal_index_of(&amp;s.bytes, &amp;r.bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_internal_check_utf8"></a>

## Function `internal_check_utf8`



<pre><code>public fun internal_check_utf8(v: &amp;vector&lt;u8&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun internal_check_utf8(v: &amp;vector&lt;u8&gt;): bool;<br/></code></pre>



</details>

<a id="0x1_string_internal_is_char_boundary"></a>

## Function `internal_is_char_boundary`



<pre><code>fun internal_is_char_boundary(v: &amp;vector&lt;u8&gt;, i: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun internal_is_char_boundary(v: &amp;vector&lt;u8&gt;, i: u64): bool;<br/></code></pre>



</details>

<a id="0x1_string_internal_sub_string"></a>

## Function `internal_sub_string`



<pre><code>fun internal_sub_string(v: &amp;vector&lt;u8&gt;, i: u64, j: u64): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun internal_sub_string(v: &amp;vector&lt;u8&gt;, i: u64, j: u64): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_string_internal_index_of"></a>

## Function `internal_index_of`



<pre><code>fun internal_index_of(v: &amp;vector&lt;u8&gt;, r: &amp;vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun internal_index_of(v: &amp;vector&lt;u8&gt;, r: &amp;vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_internal_check_utf8"></a>

### Function `internal_check_utf8`


<pre><code>public fun internal_check_utf8(v: &amp;vector&lt;u8&gt;): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_internal_check_utf8(v);<br/></code></pre>



<a id="@Specification_1_internal_is_char_boundary"></a>

### Function `internal_is_char_boundary`


<pre><code>fun internal_is_char_boundary(v: &amp;vector&lt;u8&gt;, i: u64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_internal_is_char_boundary(v, i);<br/></code></pre>



<a id="@Specification_1_internal_sub_string"></a>

### Function `internal_sub_string`


<pre><code>fun internal_sub_string(v: &amp;vector&lt;u8&gt;, i: u64, j: u64): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_internal_sub_string(v, i, j);<br/></code></pre>



<a id="@Specification_1_internal_index_of"></a>

### Function `internal_index_of`


<pre><code>fun internal_index_of(v: &amp;vector&lt;u8&gt;, r: &amp;vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_internal_index_of(v, r);<br/></code></pre>




<a id="0x1_string_spec_utf8"></a>


<pre><code>fun spec_utf8(bytes: vector&lt;u8&gt;): String &#123;<br/>   String&#123;bytes&#125;<br/>&#125;<br/></code></pre>




<a id="0x1_string_spec_internal_check_utf8"></a>


<pre><code>fun spec_internal_check_utf8(v: vector&lt;u8&gt;): bool;<br/><a id="0x1_string_spec_internal_is_char_boundary"></a>
fun spec_internal_is_char_boundary(v: vector&lt;u8&gt;, i: u64): bool;<br/><a id="0x1_string_spec_internal_sub_string"></a>
fun spec_internal_sub_string(v: vector&lt;u8&gt;, i: u64, j: u64): vector&lt;u8&gt;;<br/><a id="0x1_string_spec_internal_index_of"></a>
fun spec_internal_index_of(v: vector&lt;u8&gt;, r: vector&lt;u8&gt;): u64;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
