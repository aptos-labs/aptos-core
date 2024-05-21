
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


<pre><code>use 0x1::string;<br/>use 0x1::string_utils;<br/></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_debug_MSG_1"></a>



<pre><code>const MSG_1: vector&lt;u8&gt; &#61; [97, 98, 99, 100, 101, 102];<br/></code></pre>



<a id="0x1_debug_MSG_2"></a>



<pre><code>const MSG_2: vector&lt;u8&gt; &#61; [49, 50, 51, 52, 53, 54];<br/></code></pre>



<a id="0x1_debug_print"></a>

## Function `print`



<pre><code>public fun print&lt;T&gt;(x: &amp;T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun print&lt;T&gt;(x: &amp;T) &#123;<br/>    native_print(format(x));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_debug_print_stack_trace"></a>

## Function `print_stack_trace`



<pre><code>public fun print_stack_trace()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun print_stack_trace() &#123;<br/>    native_print(native_stack_trace());<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_debug_format"></a>

## Function `format`



<pre><code>fun format&lt;T&gt;(x: &amp;T): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun format&lt;T&gt;(x: &amp;T): String &#123;<br/>    aptos_std::string_utils::debug_string(x)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_debug_native_print"></a>

## Function `native_print`



<pre><code>fun native_print(x: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun native_print(x: String);<br/></code></pre>



</details>

<a id="0x1_debug_native_stack_trace"></a>

## Function `native_stack_trace`



<pre><code>fun native_stack_trace(): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun native_stack_trace(): String;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_print"></a>

### Function `print`


<pre><code>public fun print&lt;T&gt;(x: &amp;T)<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>



<a id="@Specification_1_print_stack_trace"></a>

### Function `print_stack_trace`


<pre><code>public fun print_stack_trace()<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>



<a id="@Specification_1_native_print"></a>

### Function `native_print`


<pre><code>fun native_print(x: string::String)<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_native_stack_trace"></a>

### Function `native_stack_trace`


<pre><code>fun native_stack_trace(): string::String<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
