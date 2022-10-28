
<a name="0x1_debug"></a>

# Module `0x1::debug`

Module providing debug functionality.


-  [Function `print`](#0x1_debug_print)
-  [Function `print_stack_trace`](#0x1_debug_print_stack_trace)
-  [Specification](#@Specification_0)
    -  [Function `print`](#@Specification_0_print)
    -  [Function `print_stack_trace`](#@Specification_0_print_stack_trace)


<pre><code></code></pre>



<details>
<summary>Show all the modules that "debug" depends on directly or indirectly</summary>


![](img/debug_forward_dep.svg)


</details>

<details>
<summary>Show all the modules that depend on "debug" directly or indirectly</summary>


![](img/debug_backward_dep.svg)


</details>

<a name="0x1_debug_print"></a>

## Function `print`



<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print">print</a>&lt;T&gt;(x: &T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print">print</a>&lt;T&gt;(x: &T);
</code></pre>



</details>

<details>
<summary>Show all the functions that "print" calls</summary>


![](img/debug_print_forward_call_graph.svg)


</details>

<details>
<summary>Show all the functions that call "print"</summary>


![](img/debug_print_backward_call_graph.svg)


</details>

<a name="0x1_debug_print_stack_trace"></a>

## Function `print_stack_trace`



<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print_stack_trace">print_stack_trace</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print_stack_trace">print_stack_trace</a>();
</code></pre>



</details>

<details>
<summary>Show all the functions that "print_stack_trace" calls</summary>


![](img/debug_print_stack_trace_forward_call_graph.svg)


</details>

<details>
<summary>Show all the functions that call "print_stack_trace"</summary>


![](img/debug_print_stack_trace_backward_call_graph.svg)


</details>

<a name="@Specification_0"></a>

## Specification


<a name="@Specification_0_print"></a>

### Function `print`


<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print">print</a>&lt;T&gt;(x: &T)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_0_print_stack_trace"></a>

### Function `print_stack_trace`


<pre><code><b>public</b> <b>fun</b> <a href="debug.md#0x1_debug_print_stack_trace">print_stack_trace</a>()
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
