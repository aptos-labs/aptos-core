
<a id="0x1_debug"></a>

# Module `0x1::debug`

Module providing debug functionality.


-  [Constants](#@Constants_0)
-  [Function `print`](#0x1_debug_print)
-  [Function `print_stack_trace`](#0x1_debug_print_stack_trace)
-  [Function `format`](#0x1_debug_format)
-  [Function `native_print`](#0x1_debug_native_print)
-  [Function `native_stack_trace`](#0x1_debug_native_stack_trace)
-  [Specification](#@Specification_1)
    -  [Function `print`](#@Specification_1_print)
    -  [Function `print_stack_trace`](#@Specification_1_print_stack_trace)
    -  [Function `native_print`](#@Specification_1_native_print)
    -  [Function `native_stack_trace`](#@Specification_1_native_stack_trace)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="string_utils.md#0x1_string_utils">0x1::string_utils</a>;<br /></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_debug_MSG_1"></a>



<pre><code><b>const</b> <a href="debug.md#0x1_debug_MSG_1">MSG_1</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [97, 98, 99, 100, 101, 102];<br /></code></pre>



<a id="0x1_debug_MSG_2"></a>



<pre><code><b>const</b> <a href="debug.md#0x1_debug_MSG_2">MSG_2</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [49, 50, 51, 52, 53, 54];<br /></code></pre>



<a id="0x1_debug_print"></a>

## Function `print`



<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print">print</a>&lt;T&gt;(x: &amp;T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print">print</a>&lt;T&gt;(x: &amp;T) &#123;<br />    <a href="debug.md#0x1_debug_native_print">native_print</a>(<a href="debug.md#0x1_debug_format">format</a>(x));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_debug_print_stack_trace"></a>

## Function `print_stack_trace`



<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print_stack_trace">print_stack_trace</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print_stack_trace">print_stack_trace</a>() &#123;<br />    <a href="debug.md#0x1_debug_native_print">native_print</a>(<a href="debug.md#0x1_debug_native_stack_trace">native_stack_trace</a>());<br />&#125;<br /></code></pre>



</details>

<a id="0x1_debug_format"></a>

## Function `format`



<pre><code><b>fun</b> <a href="debug.md#0x1_debug_format">format</a>&lt;T&gt;(x: &amp;T): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="debug.md#0x1_debug_format">format</a>&lt;T&gt;(x: &amp;T): String &#123;<br />    aptos_std::string_utils::debug_string(x)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_debug_native_print"></a>

## Function `native_print`



<pre><code><b>fun</b> <a href="debug.md#0x1_debug_native_print">native_print</a>(x: <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="debug.md#0x1_debug_native_print">native_print</a>(x: String);<br /></code></pre>



</details>

<a id="0x1_debug_native_stack_trace"></a>

## Function `native_stack_trace`



<pre><code><b>fun</b> <a href="debug.md#0x1_debug_native_stack_trace">native_stack_trace</a>(): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="debug.md#0x1_debug_native_stack_trace">native_stack_trace</a>(): String;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_print"></a>

### Function `print`


<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print">print</a>&lt;T&gt;(x: &amp;T)<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_print_stack_trace"></a>

### Function `print_stack_trace`


<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print_stack_trace">print_stack_trace</a>()<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_native_print"></a>

### Function `native_print`


<pre><code><b>fun</b> <a href="debug.md#0x1_debug_native_print">native_print</a>(x: <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_native_stack_trace"></a>

### Function `native_stack_trace`


<pre><code><b>fun</b> <a href="debug.md#0x1_debug_native_stack_trace">native_stack_trace</a>(): <a href="../../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
