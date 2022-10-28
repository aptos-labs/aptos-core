
<a name="0x1_transaction_context"></a>

# Module `0x1::transaction_context`



-  [Function `get_script_hash`](#0x1_transaction_context_get_script_hash)
-  [Specification](#@Specification_0)
    -  [Function `get_script_hash`](#@Specification_0_get_script_hash)


<pre><code></code></pre>



<details>
<summary>Show all the modules that "transaction_context" depends on directly or indirectly</summary>


![](img/transaction_context_forward_dep.svg)


</details>

<details>
<summary>Show all the modules that depend on "transaction_context" directly or indirectly</summary>


![](img/transaction_context_backward_dep.svg)


</details>

<a name="0x1_transaction_context_get_script_hash"></a>

## Function `get_script_hash`

Return the script hash of the current entry function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<details>
<summary>Show all the functions that "get_script_hash" calls</summary>


![](img/transaction_context_get_script_hash_forward_call_graph.svg)


</details>

<details>
<summary>Show all the functions that call "get_script_hash"</summary>


![](img/transaction_context_get_script_hash_backward_call_graph.svg)


</details>

<a name="@Specification_0"></a>

## Specification


<a name="@Specification_0_get_script_hash"></a>

### Function `get_script_hash`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
