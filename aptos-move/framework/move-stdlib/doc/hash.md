
<a id="0x1_hash"></a>

# Module `0x1::hash`

Module which defines SHA hashes for byte vectors.

The functions in this module are natively declared both in the Move runtime
as in the Move prover's prelude.


-  [Function `sha2_256`](#0x1_hash_sha2_256)
-  [Function `sha3_256`](#0x1_hash_sha3_256)


<pre><code></code></pre>



<a id="0x1_hash_sha2_256"></a>

## Function `sha2_256`



<pre><code>public fun sha2_256(data: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun sha2_256(data: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_hash_sha3_256"></a>

## Function `sha3_256`



<pre><code>public fun sha3_256(data: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun sha3_256(data: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
