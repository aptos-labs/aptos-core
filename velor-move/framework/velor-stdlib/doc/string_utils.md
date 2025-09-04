
<a id="0x1_string_utils"></a>

# Module `0x1::string_utils`

A module for formatting move values as strings.


-  [Struct `Cons`](#0x1_string_utils_Cons)
-  [Struct `NIL`](#0x1_string_utils_NIL)
-  [Struct `FakeCons`](#0x1_string_utils_FakeCons)
    -  [[test_only]](#@[test_only]_0)
-  [Constants](#@Constants_1)
-  [Function `to_string`](#0x1_string_utils_to_string)
-  [Function `to_string_with_canonical_addresses`](#0x1_string_utils_to_string_with_canonical_addresses)
-  [Function `to_string_with_integer_types`](#0x1_string_utils_to_string_with_integer_types)
-  [Function `debug_string`](#0x1_string_utils_debug_string)
-  [Function `format1`](#0x1_string_utils_format1)
-  [Function `format2`](#0x1_string_utils_format2)
-  [Function `format3`](#0x1_string_utils_format3)
-  [Function `format4`](#0x1_string_utils_format4)
-  [Function `cons`](#0x1_string_utils_cons)
-  [Function `nil`](#0x1_string_utils_nil)
-  [Function `list1`](#0x1_string_utils_list1)
-  [Function `list2`](#0x1_string_utils_list2)
-  [Function `list3`](#0x1_string_utils_list3)
-  [Function `list4`](#0x1_string_utils_list4)
-  [Function `native_format`](#0x1_string_utils_native_format)
-  [Function `native_format_list`](#0x1_string_utils_native_format_list)
-  [Specification](#@Specification_2)
    -  [Function `to_string`](#@Specification_2_to_string)
    -  [Function `to_string_with_canonical_addresses`](#@Specification_2_to_string_with_canonical_addresses)
    -  [Function `to_string_with_integer_types`](#@Specification_2_to_string_with_integer_types)
    -  [Function `debug_string`](#@Specification_2_debug_string)
    -  [Function `format1`](#@Specification_2_format1)
    -  [Function `format2`](#@Specification_2_format2)
    -  [Function `format3`](#@Specification_2_format3)
    -  [Function `format4`](#@Specification_2_format4)
    -  [Function `native_format`](#@Specification_2_native_format)
    -  [Function `native_format_list`](#@Specification_2_native_format_list)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_string_utils_Cons"></a>

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

<a id="0x1_string_utils_NIL"></a>

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

<a id="0x1_string_utils_FakeCons"></a>

## Struct `FakeCons`


<a id="@[test_only]_0"></a>

### [test_only]



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

<a id="@Constants_1"></a>

## Constants


<a id="0x1_string_utils_EARGS_MISMATCH"></a>

The number of values in the list does not match the number of "{}" in the format string.


<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EARGS_MISMATCH">EARGS_MISMATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_string_utils_EINVALID_FORMAT"></a>

The format string is not valid.


<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EINVALID_FORMAT">EINVALID_FORMAT</a>: u64 = 2;
</code></pre>



<a id="0x1_string_utils_EUNABLE_TO_FORMAT_DELAYED_FIELD"></a>

Formatting is not possible because the value contains delayed fields such as aggregators.


<pre><code><b>const</b> <a href="string_utils.md#0x1_string_utils_EUNABLE_TO_FORMAT_DELAYED_FIELD">EUNABLE_TO_FORMAT_DELAYED_FIELD</a>: u64 = 3;
</code></pre>



<a id="0x1_string_utils_to_string"></a>

## Function `to_string`

Format a move value as a human readable string,
eg. <code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&1u64) == "1"</code>, <code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&<b>false</b>) == "<b>false</b>"</code>, <code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&@0x1) == "@0x1"</code>.
For vectors and structs the format is similar to rust, eg.
<code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&<a href="string_utils.md#0x1_string_utils_cons">cons</a>(1,2)) == "<a href="string_utils.md#0x1_string_utils_Cons">Cons</a> { car: 1, cdr: 2 }"</code> and <code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 2, 3]) == "[ 1, 2, 3 ]"</code>
For vectors of u8 the output is hex encoded, eg. <code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1u8, 2u8, 3u8]) == "0x010203"</code>
For std::string::String the output is the string itself including quotes, eg.
<code><a href="string_utils.md#0x1_string_utils_to_string">to_string</a>(&std::string::utf8(b"My <a href="../../move-stdlib/doc/string.md#0x1_string">string</a>")) == "\"My <a href="../../move-stdlib/doc/string.md#0x1_string">string</a>\""</code>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string">to_string</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string">to_string</a>&lt;T&gt;(s: &T): String {
    <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>(s, <b>false</b>, <b>false</b>, <b>true</b>, <b>false</b>)
}
</code></pre>



</details>

<a id="0x1_string_utils_to_string_with_canonical_addresses"></a>

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

<a id="0x1_string_utils_to_string_with_integer_types"></a>

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

<a id="0x1_string_utils_debug_string"></a>

## Function `debug_string`

Format vectors and structs with newlines and indentation.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_debug_string">debug_string</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_debug_string">debug_string</a>&lt;T&gt;(s: &T): String {
    <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>(s, <b>true</b>, <b>false</b>, <b>false</b>, <b>false</b>)
}
</code></pre>



</details>

<a id="0x1_string_utils_format1"></a>

## Function `format1`

Formatting with a rust-like format string, eg. <code><a href="string_utils.md#0x1_string_utils_format2">format2</a>(&b"a = {}, b = {}", 1, 2) == "a = 1, b = 2"</code>.


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format1">format1</a>&lt;T0: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format1">format1</a>&lt;T0: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &<a href="string_utils.md#0x1_string_utils_list1">list1</a>(a))
}
</code></pre>



</details>

<a id="0x1_string_utils_format2"></a>

## Function `format2`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format2">format2</a>&lt;T0: drop, T1: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format2">format2</a>&lt;T0: drop, T1: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &<a href="string_utils.md#0x1_string_utils_list2">list2</a>(a, b))
}
</code></pre>



</details>

<a id="0x1_string_utils_format3"></a>

## Function `format3`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format3">format3</a>&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format3">format3</a>&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &<a href="string_utils.md#0x1_string_utils_list3">list3</a>(a, b, c))
}
</code></pre>



</details>

<a id="0x1_string_utils_format4"></a>

## Function `format4`



<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format4">format4</a>&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format4">format4</a>&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): String {
    <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>(fmt, &<a href="string_utils.md#0x1_string_utils_list4">list4</a>(a, b, c, d))
}
</code></pre>



</details>

<a id="0x1_string_utils_cons"></a>

## Function `cons`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_cons">cons</a>&lt;T, N&gt;(car: T, cdr: N): <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T, N&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_cons">cons</a>&lt;T, N&gt;(car: T, cdr: N): <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T, N&gt; { <a href="string_utils.md#0x1_string_utils_Cons">Cons</a> { car, cdr } }
</code></pre>



</details>

<a id="0x1_string_utils_nil"></a>

## Function `nil`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_nil">nil</a>(): <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_nil">nil</a>(): <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> { <a href="string_utils.md#0x1_string_utils_NIL">NIL</a> {} }
</code></pre>



</details>

<a id="0x1_string_utils_list1"></a>

## Function `list1`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_list1">list1</a>&lt;T0&gt;(a: T0): <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="string_utils.md#0x1_string_utils_list1">list1</a>&lt;T0&gt;(a: T0): <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_NIL">NIL</a>&gt; { <a href="string_utils.md#0x1_string_utils_cons">cons</a>(a, <a href="string_utils.md#0x1_string_utils_nil">nil</a>()) }
</code></pre>



</details>

<a id="0x1_string_utils_list2"></a>

## Function `list2`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_list2">list2</a>&lt;T0, T1&gt;(a: T0, b: T1): <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T1, <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="string_utils.md#0x1_string_utils_list2">list2</a>&lt;T0, T1&gt;(a: T0, b: T1): <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T1, <a href="string_utils.md#0x1_string_utils_NIL">NIL</a>&gt;&gt; { <a href="string_utils.md#0x1_string_utils_cons">cons</a>(a, <a href="string_utils.md#0x1_string_utils_list1">list1</a>(b)) }
</code></pre>



</details>

<a id="0x1_string_utils_list3"></a>

## Function `list3`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_list3">list3</a>&lt;T0, T1, T2&gt;(a: T0, b: T1, c: T2): <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T1, <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T2, <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="string_utils.md#0x1_string_utils_list3">list3</a>&lt;T0, T1, T2&gt;(a: T0, b: T1, c: T2): <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T1, <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T2, <a href="string_utils.md#0x1_string_utils_NIL">NIL</a>&gt;&gt;&gt; { <a href="string_utils.md#0x1_string_utils_cons">cons</a>(a, <a href="string_utils.md#0x1_string_utils_list2">list2</a>(b, c)) }
</code></pre>



</details>

<a id="0x1_string_utils_list4"></a>

## Function `list4`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_list4">list4</a>&lt;T0, T1, T2, T3&gt;(a: T0, b: T1, c: T2, d: T3): <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T1, <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T2, <a href="string_utils.md#0x1_string_utils_Cons">string_utils::Cons</a>&lt;T3, <a href="string_utils.md#0x1_string_utils_NIL">string_utils::NIL</a>&gt;&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="string_utils.md#0x1_string_utils_list4">list4</a>&lt;T0, T1, T2, T3&gt;(a: T0, b: T1, c: T2, d: T3): <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T0, <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T1, <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T2, <a href="string_utils.md#0x1_string_utils_Cons">Cons</a>&lt;T3, <a href="string_utils.md#0x1_string_utils_NIL">NIL</a>&gt;&gt;&gt;&gt; { <a href="string_utils.md#0x1_string_utils_cons">cons</a>(a, <a href="string_utils.md#0x1_string_utils_list3">list3</a>(b, c, d)) }
</code></pre>



</details>

<a id="0x1_string_utils_native_format"></a>

## Function `native_format`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>&lt;T&gt;(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>&lt;T&gt;(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;
</code></pre>



</details>

<a id="0x1_string_utils_native_format_list"></a>

## Function `native_format_list`



<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>&lt;T&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>&lt;T&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: &T): String;
</code></pre>



</details>

<a id="@Specification_2"></a>

## Specification


<a id="@Specification_2_to_string"></a>

### Function `to_string`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string">to_string</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format">spec_native_format</a>(s, <b>false</b>, <b>false</b>, <b>true</b>, <b>false</b>);
</code></pre>



<a id="@Specification_2_to_string_with_canonical_addresses"></a>

### Function `to_string_with_canonical_addresses`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string_with_canonical_addresses">to_string_with_canonical_addresses</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format">spec_native_format</a>(s, <b>false</b>, <b>true</b>, <b>true</b>, <b>false</b>);
</code></pre>



<a id="@Specification_2_to_string_with_integer_types"></a>

### Function `to_string_with_integer_types`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_to_string_with_integer_types">to_string_with_integer_types</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format">spec_native_format</a>(s, <b>false</b>, <b>true</b>, <b>true</b>, <b>false</b>);
</code></pre>



<a id="@Specification_2_debug_string"></a>

### Function `debug_string`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_debug_string">debug_string</a>&lt;T&gt;(s: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format">spec_native_format</a>(s, <b>true</b>, <b>false</b>, <b>false</b>, <b>false</b>);
</code></pre>



<a id="@Specification_2_format1"></a>

### Function `format1`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format1">format1</a>&lt;T0: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <a href="string_utils.md#0x1_string_utils_args_mismatch_or_invalid_format">args_mismatch_or_invalid_format</a>(fmt, <a href="string_utils.md#0x1_string_utils_list1">list1</a>(a));
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format_list">spec_native_format_list</a>(fmt, <a href="string_utils.md#0x1_string_utils_list1">list1</a>(a));
</code></pre>



<a id="@Specification_2_format2"></a>

### Function `format2`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format2">format2</a>&lt;T0: drop, T1: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <a href="string_utils.md#0x1_string_utils_args_mismatch_or_invalid_format">args_mismatch_or_invalid_format</a>(fmt, <a href="string_utils.md#0x1_string_utils_list2">list2</a>(a, b));
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format_list">spec_native_format_list</a>(fmt, <a href="string_utils.md#0x1_string_utils_list2">list2</a>(a, b));
</code></pre>



<a id="@Specification_2_format3"></a>

### Function `format3`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format3">format3</a>&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <a href="string_utils.md#0x1_string_utils_args_mismatch_or_invalid_format">args_mismatch_or_invalid_format</a>(fmt, <a href="string_utils.md#0x1_string_utils_list3">list3</a>(a, b, c));
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format_list">spec_native_format_list</a>(fmt, <a href="string_utils.md#0x1_string_utils_list3">list3</a>(a, b, c));
</code></pre>



<a id="@Specification_2_format4"></a>

### Function `format4`


<pre><code><b>public</b> <b>fun</b> <a href="string_utils.md#0x1_string_utils_format4">format4</a>&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> <a href="string_utils.md#0x1_string_utils_args_mismatch_or_invalid_format">args_mismatch_or_invalid_format</a>(fmt, <a href="string_utils.md#0x1_string_utils_list4">list4</a>(a, b, c, d));
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format_list">spec_native_format_list</a>(fmt, <a href="string_utils.md#0x1_string_utils_list4">list4</a>(a, b, c, d));
</code></pre>



<a id="@Specification_2_native_format"></a>

### Function `native_format`


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format">native_format</a>&lt;T&gt;(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format">spec_native_format</a>(s, type_tag, canonicalize, single_line, include_int_types);
</code></pre>



<a id="@Specification_2_native_format_list"></a>

### Function `native_format_list`


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_native_format_list">native_format_list</a>&lt;T&gt;(fmt: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: &T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <a href="string_utils.md#0x1_string_utils_args_mismatch_or_invalid_format">args_mismatch_or_invalid_format</a>(fmt, val);
<b>ensures</b> result == <a href="string_utils.md#0x1_string_utils_spec_native_format_list">spec_native_format_list</a>(fmt, val);
</code></pre>




<a id="0x1_string_utils_spec_native_format"></a>


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_spec_native_format">spec_native_format</a>&lt;T&gt;(s: T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;
</code></pre>




<a id="0x1_string_utils_spec_native_format_list"></a>


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_spec_native_format_list">spec_native_format_list</a>&lt;T&gt;(fmt: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: T): String;
</code></pre>




<a id="0x1_string_utils_args_mismatch_or_invalid_format"></a>


<pre><code><b>fun</b> <a href="string_utils.md#0x1_string_utils_args_mismatch_or_invalid_format">args_mismatch_or_invalid_format</a>&lt;T&gt;(fmt: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, val: T): bool;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
