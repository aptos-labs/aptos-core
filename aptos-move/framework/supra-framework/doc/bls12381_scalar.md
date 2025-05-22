
<a id="0x1_bls12381_scalar"></a>

# Module `0x1::bls12381_scalar`



-  [Function `bls12381_hash_to_scalar`](#0x1_bls12381_scalar_bls12381_hash_to_scalar)
-  [Function `native_hash_to_scalar`](#0x1_bls12381_scalar_native_hash_to_scalar)


<pre><code></code></pre>



<a id="0x1_bls12381_scalar_bls12381_hash_to_scalar"></a>

## Function `bls12381_hash_to_scalar`



<pre><code><b>public</b> <b>fun</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_bls12381_hash_to_scalar">bls12381_hash_to_scalar</a>(dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_bls12381_hash_to_scalar">bls12381_hash_to_scalar</a>(
    dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="bls12381_scalar.md#0x1_bls12381_scalar_native_hash_to_scalar">native_hash_to_scalar</a>(dst, msg)
}
</code></pre>



</details>

<a id="0x1_bls12381_scalar_native_hash_to_scalar"></a>

## Function `native_hash_to_scalar`



<pre><code><b>fun</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_native_hash_to_scalar">native_hash_to_scalar</a>(dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_native_hash_to_scalar">native_hash_to_scalar</a>(
    dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
