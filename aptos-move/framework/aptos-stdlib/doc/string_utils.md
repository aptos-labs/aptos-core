
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


<pre><code>use 0x1::string;
</code></pre>



<a id="0x1_string_utils_Cons"></a>

## Struct `Cons`



<pre><code>struct Cons&lt;T, N&gt; has copy, drop, store
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



<pre><code>struct NIL has copy, drop, store
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



<pre><code>struct FakeCons&lt;T, N&gt; has copy, drop, store
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


<pre><code>const EARGS_MISMATCH: u64 &#61; 1;
</code></pre>



<a id="0x1_string_utils_EINVALID_FORMAT"></a>

The format string is not valid.


<pre><code>const EINVALID_FORMAT: u64 &#61; 2;
</code></pre>



<a id="0x1_string_utils_EUNABLE_TO_FORMAT_DELAYED_FIELD"></a>

Formatting is not possible because the value contains delayed fields such as aggregators.


<pre><code>const EUNABLE_TO_FORMAT_DELAYED_FIELD: u64 &#61; 3;
</code></pre>



<a id="0x1_string_utils_to_string"></a>

## Function `to_string`

Format a move value as a human readable string,
eg. <code>to_string(&amp;1u64) &#61;&#61; &quot;1&quot;</code>, <code>to_string(&amp;false) &#61;&#61; &quot;false&quot;</code>, <code>to_string(&amp;@0x1) &#61;&#61; &quot;@0x1&quot;</code>.
For vectors and structs the format is similar to rust, eg.
<code>to_string(&amp;cons(1,2)) &#61;&#61; &quot;Cons &#123; car: 1, cdr: 2 &#125;&quot;</code> and <code>to_string(&amp;vector[1, 2, 3]) &#61;&#61; &quot;[ 1, 2, 3 ]&quot;</code>
For vectors of u8 the output is hex encoded, eg. <code>to_string(&amp;vector[1u8, 2u8, 3u8]) &#61;&#61; &quot;0x010203&quot;</code>
For std::string::String the output is the string itself including quotes, eg.
<code>to_string(&amp;std::string::utf8(b&quot;My string&quot;)) &#61;&#61; &quot;\&quot;My string\&quot;&quot;</code>


<pre><code>public fun to_string&lt;T&gt;(s: &amp;T): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string&lt;T&gt;(s: &amp;T): String &#123;
    native_format(s, false, false, true, false)
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_to_string_with_canonical_addresses"></a>

## Function `to_string_with_canonical_addresses`

Format addresses as 64 zero-padded hexadecimals.


<pre><code>public fun to_string_with_canonical_addresses&lt;T&gt;(s: &amp;T): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string_with_canonical_addresses&lt;T&gt;(s: &amp;T): String &#123;
    native_format(s, false, true, true, false)
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_to_string_with_integer_types"></a>

## Function `to_string_with_integer_types`

Format emitting integers with types ie. 6u8 or 128u32.


<pre><code>public fun to_string_with_integer_types&lt;T&gt;(s: &amp;T): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_string_with_integer_types&lt;T&gt;(s: &amp;T): String &#123;
    native_format(s, false, true, true, false)
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_debug_string"></a>

## Function `debug_string`

Format vectors and structs with newlines and indentation.


<pre><code>public fun debug_string&lt;T&gt;(s: &amp;T): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun debug_string&lt;T&gt;(s: &amp;T): String &#123;
    native_format(s, true, false, false, false)
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_format1"></a>

## Function `format1`

Formatting with a rust-like format string, eg. <code>format2(&amp;b&quot;a &#61; &#123;&#125;, b &#61; &#123;&#125;&quot;, 1, 2) &#61;&#61; &quot;a &#61; 1, b &#61; 2&quot;</code>.


<pre><code>public fun format1&lt;T0: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format1&lt;T0: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0): String &#123;
    native_format_list(fmt, &amp;list1(a))
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_format2"></a>

## Function `format2`



<pre><code>public fun format2&lt;T0: drop, T1: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format2&lt;T0: drop, T1: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1): String &#123;
    native_format_list(fmt, &amp;list2(a, b))
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_format3"></a>

## Function `format3`



<pre><code>public fun format3&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format3&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2): String &#123;
    native_format_list(fmt, &amp;list3(a, b, c))
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_format4"></a>

## Function `format4`



<pre><code>public fun format4&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun format4&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): String &#123;
    native_format_list(fmt, &amp;list4(a, b, c, d))
&#125;
</code></pre>



</details>

<a id="0x1_string_utils_cons"></a>

## Function `cons`



<pre><code>fun cons&lt;T, N&gt;(car: T, cdr: N): string_utils::Cons&lt;T, N&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun cons&lt;T, N&gt;(car: T, cdr: N): Cons&lt;T, N&gt; &#123; Cons &#123; car, cdr &#125; &#125;
</code></pre>



</details>

<a id="0x1_string_utils_nil"></a>

## Function `nil`



<pre><code>fun nil(): string_utils::NIL
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun nil(): NIL &#123; NIL &#123;&#125; &#125;
</code></pre>



</details>

<a id="0x1_string_utils_list1"></a>

## Function `list1`



<pre><code>fun list1&lt;T0&gt;(a: T0): string_utils::Cons&lt;T0, string_utils::NIL&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list1&lt;T0&gt;(a: T0): Cons&lt;T0, NIL&gt; &#123; cons(a, nil()) &#125;
</code></pre>



</details>

<a id="0x1_string_utils_list2"></a>

## Function `list2`



<pre><code>fun list2&lt;T0, T1&gt;(a: T0, b: T1): string_utils::Cons&lt;T0, string_utils::Cons&lt;T1, string_utils::NIL&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list2&lt;T0, T1&gt;(a: T0, b: T1): Cons&lt;T0, Cons&lt;T1, NIL&gt;&gt; &#123; cons(a, list1(b)) &#125;
</code></pre>



</details>

<a id="0x1_string_utils_list3"></a>

## Function `list3`



<pre><code>fun list3&lt;T0, T1, T2&gt;(a: T0, b: T1, c: T2): string_utils::Cons&lt;T0, string_utils::Cons&lt;T1, string_utils::Cons&lt;T2, string_utils::NIL&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list3&lt;T0, T1, T2&gt;(a: T0, b: T1, c: T2): Cons&lt;T0, Cons&lt;T1, Cons&lt;T2, NIL&gt;&gt;&gt; &#123; cons(a, list2(b, c)) &#125;
</code></pre>



</details>

<a id="0x1_string_utils_list4"></a>

## Function `list4`



<pre><code>fun list4&lt;T0, T1, T2, T3&gt;(a: T0, b: T1, c: T2, d: T3): string_utils::Cons&lt;T0, string_utils::Cons&lt;T1, string_utils::Cons&lt;T2, string_utils::Cons&lt;T3, string_utils::NIL&gt;&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun list4&lt;T0, T1, T2, T3&gt;(a: T0, b: T1, c: T2, d: T3): Cons&lt;T0, Cons&lt;T1, Cons&lt;T2, Cons&lt;T3, NIL&gt;&gt;&gt;&gt; &#123; cons(a, list3(b, c, d)) &#125;
</code></pre>



</details>

<a id="0x1_string_utils_native_format"></a>

## Function `native_format`



<pre><code>fun native_format&lt;T&gt;(s: &amp;T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun native_format&lt;T&gt;(s: &amp;T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;
</code></pre>



</details>

<a id="0x1_string_utils_native_format_list"></a>

## Function `native_format_list`



<pre><code>fun native_format_list&lt;T&gt;(fmt: &amp;vector&lt;u8&gt;, val: &amp;T): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun native_format_list&lt;T&gt;(fmt: &amp;vector&lt;u8&gt;, val: &amp;T): String;
</code></pre>



</details>

<a id="@Specification_2"></a>

## Specification


<a id="@Specification_2_to_string"></a>

### Function `to_string`


<pre><code>public fun to_string&lt;T&gt;(s: &amp;T): string::String
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_native_format(s, false, false, true, false);
</code></pre>



<a id="@Specification_2_to_string_with_canonical_addresses"></a>

### Function `to_string_with_canonical_addresses`


<pre><code>public fun to_string_with_canonical_addresses&lt;T&gt;(s: &amp;T): string::String
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_native_format(s, false, true, true, false);
</code></pre>



<a id="@Specification_2_to_string_with_integer_types"></a>

### Function `to_string_with_integer_types`


<pre><code>public fun to_string_with_integer_types&lt;T&gt;(s: &amp;T): string::String
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_native_format(s, false, true, true, false);
</code></pre>



<a id="@Specification_2_debug_string"></a>

### Function `debug_string`


<pre><code>public fun debug_string&lt;T&gt;(s: &amp;T): string::String
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_native_format(s, true, false, false, false);
</code></pre>



<a id="@Specification_2_format1"></a>

### Function `format1`


<pre><code>public fun format1&lt;T0: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0): string::String
</code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list1(a));
ensures result &#61;&#61; spec_native_format_list(fmt, list1(a));
</code></pre>



<a id="@Specification_2_format2"></a>

### Function `format2`


<pre><code>public fun format2&lt;T0: drop, T1: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1): string::String
</code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list2(a, b));
ensures result &#61;&#61; spec_native_format_list(fmt, list2(a, b));
</code></pre>



<a id="@Specification_2_format3"></a>

### Function `format3`


<pre><code>public fun format3&lt;T0: drop, T1: drop, T2: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2): string::String
</code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list3(a, b, c));
ensures result &#61;&#61; spec_native_format_list(fmt, list3(a, b, c));
</code></pre>



<a id="@Specification_2_format4"></a>

### Function `format4`


<pre><code>public fun format4&lt;T0: drop, T1: drop, T2: drop, T3: drop&gt;(fmt: &amp;vector&lt;u8&gt;, a: T0, b: T1, c: T2, d: T3): string::String
</code></pre>




<pre><code>aborts_if args_mismatch_or_invalid_format(fmt, list4(a, b, c, d));
ensures result &#61;&#61; spec_native_format_list(fmt, list4(a, b, c, d));
</code></pre>



<a id="@Specification_2_native_format"></a>

### Function `native_format`


<pre><code>fun native_format&lt;T&gt;(s: &amp;T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): string::String
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_native_format(s, type_tag, canonicalize, single_line, include_int_types);
</code></pre>



<a id="@Specification_2_native_format_list"></a>

### Function `native_format_list`


<pre><code>fun native_format_list&lt;T&gt;(fmt: &amp;vector&lt;u8&gt;, val: &amp;T): string::String
</code></pre>




<pre><code>pragma opaque;
aborts_if args_mismatch_or_invalid_format(fmt, val);
ensures result &#61;&#61; spec_native_format_list(fmt, val);
</code></pre>




<a id="0x1_string_utils_spec_native_format"></a>


<pre><code>fun spec_native_format&lt;T&gt;(s: T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;
</code></pre>




<a id="0x1_string_utils_spec_native_format_list"></a>


<pre><code>fun spec_native_format_list&lt;T&gt;(fmt: vector&lt;u8&gt;, val: T): String;
</code></pre>




<a id="0x1_string_utils_args_mismatch_or_invalid_format"></a>


<pre><code>fun args_mismatch_or_invalid_format&lt;T&gt;(fmt: vector&lt;u8&gt;, val: T): bool;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
