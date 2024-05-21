
<a id="0x1_bcs"></a>

# Module `0x1::bcs`

Utility for converting a Move value to its binary representation in BCS (Binary Canonical
Serialization). BCS is the binary encoding for Move resources and other non-module values
published on-chain. See https://github.com/aptos-labs/bcs#binary-canonical-serialization-bcs for more
details on BCS.


-  [Function `to_bytes`](#0x1_bcs_to_bytes)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<a id="0x1_bcs_to_bytes"></a>

## Function `to_bytes`

Return the binary representation of <code>v</code> in BCS (Binary Canonical Serialization) format


<pre><code>public fun to_bytes&lt;MoveValue&gt;(v: &amp;MoveValue): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun to_bytes&lt;MoveValue&gt;(v: &amp;MoveValue): vector&lt;u8&gt;;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



Native function which is defined in the prover's prelude.


<a id="0x1_bcs_serialize"></a>


<pre><code>native fun serialize&lt;MoveValue&gt;(v: &amp;MoveValue): vector&lt;u8&gt;;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
