
<a id="0x1_bcs"></a>

# Module `0x1::bcs`

Utility for converting a Move value to its binary representation in BCS (Binary Canonical
Serialization). BCS is the binary encoding for Move resources and other non-module values
published on-chain. See https://github.com/diem/bcs#binary-canonical-serialization-bcs for more
details on BCS.


-  [Function `to_bytes`](#0x1_bcs_to_bytes)
-  [Function `native_load_layout`](#0x1_bcs_native_load_layout)
-  [Function `native_to_bytes`](#0x1_bcs_native_to_bytes)
-  [Module Specification](#@Module_Specification_0)


<pre><code></code></pre>



<a id="0x1_bcs_to_bytes"></a>

## Function `to_bytes`

Return the binary representation of <code>v</code> in BCS (Binary Canonical Serialization) format


<pre><code><b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_to_bytes">to_bytes</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_to_bytes">to_bytes</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="bcs.md#0x1_bcs_native_load_layout">native_load_layout</a>&lt;MoveValue&gt;();
    <a href="bcs.md#0x1_bcs_native_to_bytes">native_to_bytes</a>&lt;MoveValue&gt;(v)
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="bcs.md#0x1_bcs_serialize">serialize</a>(v);
</code></pre>



</details>

<a id="0x1_bcs_native_load_layout"></a>

## Function `native_load_layout`



<pre><code><b>fun</b> <a href="bcs.md#0x1_bcs_native_load_layout">native_load_layout</a>&lt;MoveValue&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="bcs.md#0x1_bcs_native_load_layout">native_load_layout</a>&lt;MoveValue&gt;();
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
</code></pre>



</details>

<a id="0x1_bcs_native_to_bytes"></a>

## Function `native_to_bytes`



<pre><code><b>fun</b> <a href="bcs.md#0x1_bcs_native_to_bytes">native_to_bytes</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="bcs.md#0x1_bcs_native_to_bytes">native_to_bytes</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="@Module_Specification_0"></a>

## Module Specification



Native function which is defined in the prover's prelude.


<a id="0x1_bcs_serialize"></a>


<pre><code><b>native</b> <b>fun</b> <a href="bcs.md#0x1_bcs_serialize">serialize</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>


[//]: # ("File containing references which can be used from documentation")
