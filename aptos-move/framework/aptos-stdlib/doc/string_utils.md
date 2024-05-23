
<a id="0x1_string_utils"></a>

# Module `0x1::string_utils`

A module for formatting move values as strings.


-  [Struct `Cons`](#0x1_string_utils_Cons)
-  [Struct `NIL`](#0x1_string_utils_NIL)
-  [Struct `FakeCons`](#0x1_string_utils_FakeCons)
-  [Constants](#@Constants_0)
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
-  [Specification](#@Specification_1)
    -  [Function `to_string`](#@Specification_1_to_string)
    -  [Function `to_string_with_canonical_addresses`](#@Specification_1_to_string_with_canonical_addresses)
    -  [Function `to_string_with_integer_types`](#@Specification_1_to_string_with_integer_types)
    -  [Function `debug_string`](#@Specification_1_debug_string)
    -  [Function `format1`](#@Specification_1_format1)
    -  [Function `format2`](#@Specification_1_format2)
    -  [Function `format3`](#@Specification_1_format3)
    -  [Function `format4`](#@Specification_1_format4)
    -  [Function `native_format`](#@Specification_1_native_format)
    -  [Function `native_format_list`](#@Specification_1_native_format_list)


<pre><code>use 0x1::string;<br/></code></pre>



<a id="0x1_string_utils_Cons"></a>

## Struct `Cons`



<pre><code>struct Cons&lt;T, N&gt; has copy, drop, store<br/></code></pre>



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



<pre><code>struct NIL has copy, drop, store<br/></code></pre>



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

&#35;[test_only]


<pre><code>struct FakeCons&lt;T, N&gt; has copy, drop, store<br/></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_string_utils_EARGS_MISMATCH"></a>

The number of values in the list does not match the number of &quot;&#123;&#125;&quot; in the format string.


<pre><code>const EARGS_MISMATCH: u64 &#61; 1;<br/></code></pre>



<a id="0x1_string_utils_EINVALID_FORMAT"></a>

The format string is not valid.


<pre><code>const EINVALID_FORMAT: u64 &#61; 2;<br/></code></pre>



<a id="0x1_string_utils_EUNABLE_TO_FORMAT_DELAYED_FIELD"></a>

Formatting is not possible because the value contains delayed fields such as aggregators.


<pre><code>const EUNABLE_TO_FORMAT_DELAYED_FIELD: u64 &#61; 3;<br/></code></pre>



<a id="0x1_string_utils_to_string"></a>

## Function `to_string`

Format a move value as a human readable string,<br/> eg. <code>to_string(&amp;1u64) &#61;&#61; &quot;1&quot;</code>, <code>to_string(&amp;false) &#61;&#61; &quot;false&quot;</code>, <code>to_string(&amp;@0x1) &#61;&#61; &quot;@0x1&quot;</code>.<br/> For vectors and structs the format is similar to rust, eg.<br/> <code>to_string(&amp;cons(1,2)) &#61;&#61; &quot;Cons &#123; car: 1, cdr: 2 &#125;&quot;</code> and <code>to_string(&amp;vector[1, 2, 3]) &#61;&#61; &quot;[ 1, 2, 3 ]&quot;</code><br/> For vectors of u8 the output is hex encoded, eg. <code>to_string(&amp;vector[1u8, 2u8, 3u8]) &#61;&#61; &quot;0x010203&quot;</code><br/> For std::string::String the output is the string itself including quotes, eg.<br/> <code>to_string(&amp;std::string::utf8(b&quot;My string&quot;)) &#61;&#61; &quot;\&quot;My string\&quot;&quot;</code>


<pre><code>public fun to_string&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string&lt;T&gt;(s: &amp;T): String &#123;<br/>    native_format(s, false, false, true, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_to_string_with_canonical_addresses"></a>

## Function `to_string_with_canonical_addresses`

Format addresses as 64 zero&#45;padded hexadecimals.


<pre><code>public fun to_string_with_canonical_addresses&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string_with_canonical_addresses&lt;T&gt;(s: &amp;T): String &#123;<br/>    native_format(s, false, true, true, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_to_string_with_integer_types"></a>

## Function `to_string_with_integer_types`

Format emitting integers with types ie. 6u8 or 128u32.


<pre><code>public fun to_string_with_integer_types&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string_with_integer_types&lt;T&gt;(s: &amp;T): String &#123;<br/>    native_format(s, false, true, true, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_debug_string"></a>

## Function `debug_string`

Format vectors and structs with newlines and indentation.


<pre><code>public fun debug_string&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun debug_string&lt;T&gt;(s: &amp;T): String &#123;<br/>    native_format(s, true, false, false, false)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_format1"></a>

## Function `format1`

Formatting with a rust&#45;like format string, eg. <code>format2(&amp;b&quot;a &#61; &#123;&#125;, b &#61; &#123;&#125;&quot;, 1, 2) &#61;&#61; &quot;a &#61; 1, b &#61; 2&quot;</code>.


<pre><code>public fun format1&lt;T0: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format1&lt;T0: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0): String &#123;<br/>    native_format_list(fmt, &amp;list1(a))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_format2"></a>

## Function `format2`



<pre><code>public fun format2&lt;T0: drop, T1: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format2&lt;T0: drop, T1: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1): String &#123;<br/>    native_format_list(fmt, &amp;list2(a, b))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_format3"></a>

## Function `format3`



<pre><code>public fun format3&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format3&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2): String &#123;<br/>    native_format_list(fmt, &amp;list3(a, b, c))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_format4"></a>

## Function `format4`



<pre><code>public fun format4&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format4&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): String &#123;<br/>    native_format_list(fmt, &amp;list4(a, b, c, d))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_cons"></a>

## Function `cons`



<pre><code>fun cons&lt;T, N&gt;(car: T, cdr: N): string_utils::Cons&lt;T, N&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun cons&lt;T, N&gt;(car: T, cdr: N): Cons&lt;T, N&gt; &#123; Cons &#123; car, cdr &#125; &#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_nil"></a>

## Function `nil`



<pre><code>fun nil(): string_utils::NIL<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun nil(): NIL &#123; NIL &#123;&#125; &#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_list1"></a>

## Function `list1`



<pre><code>fun list1&lt;T0&gt;(a: T0): string_utils::Cons&lt;T0, string_utils::NIL&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list1&lt;T0&gt;(a: T0): Cons&lt;T0, NIL&gt; &#123; cons(a, nil()) &#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_list2"></a>

## Function `list2`



<pre><code>fun list2&lt;T0, T1&gt;(a: T0, b: T1): string_utils::Cons&lt;T0, string_utils::Cons&lt;T1, string_utils::NIL&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list2&lt;T0, T1&gt;(a: T0, b: T1): Cons&lt;T0, Cons&lt;T1, NIL&gt;&gt; &#123; cons(a, list1(b)) &#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_list3"></a>

## Function `list3`



<pre><code>fun list3&lt;T0, T1, T2&gt;(a: T0, b: T1, c: T2): string_utils::Cons&lt;T0, string_utils::Cons&lt;T1, string_utils::Cons&lt;T2, string_utils::NIL&gt;&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list3&lt;T0, T1, T2&gt;(a: T0, b: T1, c: T2): Cons&lt;T0, Cons&lt;T1, Cons&lt;T2, NIL&gt;&gt;&gt; &#123; cons(a, list2(b, c)) &#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_list4"></a>

## Function `list4`



<pre><code>fun list4&lt;T0, T1, T2, T3&gt;(a: T0, b: T1, c: T2, d: T3): string_utils::Cons&lt;T0, string_utils::Cons&lt;T1, string_utils::Cons&lt;T2, string_utils::Cons&lt;T3, string_utils::NIL&gt;&gt;&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list4&lt;T0, T1, T2, T3&gt;(a: T0, b: T1, c: T2, d: T3): Cons&lt;T0, Cons&lt;T1, Cons&lt;T2, Cons&lt;T3, NIL&gt;&gt;&gt;&gt; &#123; cons(a, list3(b, c, d)) &#125;<br/></code></pre>



</details>

<a id="0x1_string_utils_native_format"></a>

## Function `native_format`



<pre><code>fun native_format&lt;T&gt;(s: &amp;T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun native_format&lt;T&gt;(s: &amp;T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;<br/></code></pre>



</details>

<a id="0x1_string_utils_native_format_list"></a>

## Function `native_format_list`



<pre><code>fun native_format_list&lt;T&gt;(fmt: &amp;vector&lt;u8&gt;, val: &amp;T): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun native_format_list&lt;T&gt;(fmt: &amp;vector&lt;u8&gt;, val: &amp;T): String;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_to_string"></a>

### Function `to_string`


<pre><code>public fun to_string&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_native_format(s, false, false, true, false);<br/></code></pre>



<a id="@Specification_1_to_string_with_canonical_addresses"></a>

### Function `to_string_with_canonical_addresses`


<pre><code>public fun to_string_with_canonical_addresses&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_native_format(s, false, true, true, false);<br/></code></pre>



<a id="@Specification_1_to_string_with_integer_types"></a>

### Function `to_string_with_integer_types`


<pre><code>public fun to_string_with_integer_types&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_native_format(s, false, true, true, false);<br/></code></pre>



<a id="@Specification_1_debug_string"></a>

### Function `debug_string`


<pre><code>public fun debug_string&lt;T&gt;(s: &amp;T): string::String<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_native_format(s, true, false, false, false);<br/></code></pre>



<a id="@Specification_1_format1"></a>

### Function `format1`


<pre><code>public fun format1&lt;T0: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0): string::String<br/></code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list1(a));<br/>ensures result &#61;&#61; spec_native_format_list(fmt, list1(a));<br/></code></pre>



<a id="@Specification_1_format2"></a>

### Function `format2`


<pre><code>public fun format2&lt;T0: drop, T1: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1): string::String<br/></code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list2(a, b));<br/>ensures result &#61;&#61; spec_native_format_list(fmt, list2(a, b));<br/></code></pre>



<a id="@Specification_1_format3"></a>

### Function `format3`


<pre><code>public fun format3&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2): string::String<br/></code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list3(a, b, c));<br/>ensures result &#61;&#61; spec_native_format_list(fmt, list3(a, b, c));<br/></code></pre>



<a id="@Specification_1_format4"></a>

### Function `format4`


<pre><code>public fun format4&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): string::String<br/></code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list4(a, b, c, d));<br/>ensures result &#61;&#61; spec_native_format_list(fmt, list4(a, b, c, d));<br/></code></pre>



<a id="@Specification_1_native_format"></a>

### Function `native_format`


<pre><code>fun native_format&lt;T&gt;(s: &amp;T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): string::String<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_native_format(s, type_tag, canonicalize, single_line, include_int_types);<br/></code></pre>



<a id="@Specification_1_native_format_list"></a>

### Function `native_format_list`


<pre><code>fun native_format_list&lt;T&gt;(fmt: &amp;vector&lt;u8&gt;, val: &amp;T): string::String<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if args_mismatch_or_invalid_format(fmt, val);<br/>ensures result &#61;&#61; spec_native_format_list(fmt, val);<br/></code></pre>




<a id="0x1_string_utils_spec_native_format"></a>


<pre><code>fun spec_native_format&lt;T&gt;(s: T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;<br/></code></pre>




<a id="0x1_string_utils_spec_native_format_list"></a>


<pre><code>fun spec_native_format_list&lt;T&gt;(fmt: vector&lt;u8&gt;, val: T): String;<br/></code></pre>




<a id="0x1_string_utils_args_mismatch_or_invalid_format"></a>


<pre><code>fun args_mismatch_or_invalid_format&lt;T&gt;(fmt: vector&lt;u8&gt;, val: T): bool;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
