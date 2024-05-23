
<a id="0x1_bcs"></a>

# Module `0x1::bcs`

Utility for converting a Move value to its binary representation in BCS (Binary Canonical<br/> Serialization). BCS is the binary encoding for Move resources and other non&#45;module values<br/> published on&#45;chain. See https://github.com/aptos&#45;labs/bcs&#35;binary&#45;canonical&#45;serialization&#45;bcs for more<br/> details on BCS.


-  [Function `to_bytes`](#0x1_bcs_to_bytes)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<a id="0x1_bcs_to_bytes"></a>

## Function `to_bytes`

Return the binary representation of <code>v</code> in BCS (Binary Canonical Serialization) format


<pre><code>public fun to_bytes&lt;MoveValue&gt;(v: &amp;MoveValue): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun to_bytes&lt;MoveValue&gt;(v: &amp;MoveValue): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



Native function which is defined in the prover&apos;s prelude.


<a id="0x1_bcs_serialize"></a>


<pre><code>native fun serialize&lt;MoveValue&gt;(v: &amp;MoveValue): vector&lt;u8&gt;;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
