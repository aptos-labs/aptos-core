
<a name="0x1_string_utils"></a>

# Module `0x1::string_utils`



-  [Struct `Cons`](#0x1_string_utils_Cons)
-  [Struct `NIL`](#0x1_string_utils_NIL)
-  [Struct `FakeCons`](#0x1_string_utils_FakeCons)
-  [Constants](#@Constants_0)
-  [Function `cons`](#0x1_string_utils_cons)
-  [Function `nil`](#0x1_string_utils_nil)
-  [Function `format1`](#0x1_string_utils_format1)
-  [Function `format2`](#0x1_string_utils_format2)
-  [Function `format3`](#0x1_string_utils_format3)
-  [Function `format4`](#0x1_string_utils_format4)
-  [Function `to_string`](#0x1_string_utils_to_string)
-  [Function `to_string_with_canonical_addresses`](#0x1_string_utils_to_string_with_canonical_addresses)
-  [Function `to_string_with_integer_types`](#0x1_string_utils_to_string_with_integer_types)
-  [Function `debug_string`](#0x1_string_utils_debug_string)
-  [Function `native_format`](#0x1_string_utils_native_format)
-  [Function `native_format_list`](#0x1_string_utils_native_format_list)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_string_utils_Cons"></a>

## Struct `Cons`



<pre><code><b>struct</b> <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T, N&gt; <b>has</b> <b>copy</b>, drop, store
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

<a name="0x1_string_utils_FakeCons"></a>

## Struct `FakeCons`



<pre><code><b>struct</b> <a href="string_utils.md#0x1_string_utils_FakeCons">FakeCons</a>&lt;T, N&gt; <b>has</b> <b>copy</b>, drop, store
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

The number of values in the list does not match the number of "{}" in the format string.


<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EARGS_MISMATCH">EARGS_MISMATCH</a>: u64 = 1;
</code></pre>



<a name="0x1_string_utils_EINVALID_FORMAT"></a>

The format string is not valid.


<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EINVALID_FORMAT">EINVALID_FORMAT</a>: u64 = 2;
</code></pre>



<a name="0x1_string_utils_cons"></a>

## Function `cons`

Create a pair of values.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_cons">cons</a>&lt;T, N&gt;(car: T, cdr: N): <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T, N&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_cons">cons</a>&lt;T, N&gt;(car: T, cdr: N): <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T, N&gt; { <a href="string_utils.md#0x1_string_utils_Cons">Cons</a> { car, cdr } }
</code></pre>



</details>

<a name="0x1_string_utils_nil"></a>

## Function `nil`

Create a nil value.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_nil">nil</a>(): <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_nil">nil</a>(): <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> { <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> {} }
</code></pre>



</details>

<a name="0x1_string_utils_format1"></a>

## Function `format1`

Specialized versions of format_list for 1, 2, 3 and 4 values as a convenience.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format1">format1</a>&lt;T0: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format1">format1</a>&lt;T0: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &list1(a))
}
</code></pre>



</details>

<a name="0x1_string_utils_format2"></a>

## Function `format2`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format2">format2</a>&lt;T0: drop, T1: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format2">format2</a>&lt;T0: drop, T1: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &list2(a, b))
}
</code></pre>



</details>

<a name="0x1_string_utils_format3"></a>

## Function `format3`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format3">format3</a>&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format3">format3</a>&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &list3(a, b, c))
}
</code></pre>



</details>

<a name="0x1_string_utils_format4"></a>

## Function `format4`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format4">format4</a>&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format4">format4</a>&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &list4(a, b, c, d))
}
</code></pre>



</details>

<a name="0x1_string_utils_to_string"></a>

## Function `to_string`

Format a move value as a human readable string.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string">to_string</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string">to_string</a>&lt;T&gt;(s: &T): String {
    <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>(s, <b>false</b>, <b>false</b>, <b>true</b>, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_string_utils_to_string_with_canonical_addresses"></a>

## Function `to_string_with_canonical_addresses`

Format addresses as 64 zero-padded hexadecimals.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string_with_canonical_addresses">to_string_with_canonical_addresses</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string_with_canonical_addresses">to_string_with_canonical_addresses</a>&lt;T&gt;(s: &T): String {
    <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>(s, <b>false</b>, <b>true</b>, <b>true</b>, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_string_utils_to_string_with_integer_types"></a>

## Function `to_string_with_integer_types`

Format emitting integers with types ie. 6u8 or 128u32.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string_with_integer_types">to_string_with_integer_types</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string_with_integer_types">to_string_with_integer_types</a>&lt;T&gt;(s: &T): String {
    <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>(s, <b>false</b>, <b>true</b>, <b>true</b>, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_string_utils_debug_string"></a>

## Function `debug_string`

Format vectors and structs with newlines


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_debug_string">debug_string</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_debug_string">debug_string</a>&lt;T&gt;(s: &T): String {
    <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>(s, <b>true</b>, <b>false</b>, <b>false</b>, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_string_utils_native_format"></a>

## Function `native_format`

Format a move value as a human readable string.
eg. <code>format(&1u64) == "1"</code>, <code>format(&<b>false</b>) == "<b>false</b>"</code> and <code>format(&<a href="string_utils.md#0x1_string_utils_cons">cons</a>(1,2)) == "<a href="string_utils.md#0x1_string_utils_Cons">Cons</a> { car: 1, cdr: 2 }"</code>


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>&lt;T&gt;(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>&lt;T&gt;(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;
</code></pre>



</details>

<a name="0x1_string_utils_native_format_list"></a>

## Function `native_format_list`

Format a list of move values as a human readable string with rust-like format string.
eg. <code>format_list(&b"a = {} b = {} c = {}", &<a href="string_utils.md#0x1_string_utils_cons">cons</a>(1, <a href="string_utils.md#0x1_string_utils_cons">cons</a>(2, <a href="string_utils.md#0x1_string_utils_cons">cons</a>(3, <a href="string_utils.md#0x1_string_utils_nil">nil</a>())))) == "a = 1 b = 2 c = 3"</code>
fmt must be utf8 encoded and must contain the same number of "{}" as the number of values in the list.


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>&lt;T&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>&lt;T&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: &T): String;
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
