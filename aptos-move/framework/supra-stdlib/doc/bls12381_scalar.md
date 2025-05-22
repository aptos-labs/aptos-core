
<a id="0x1_bls12381_scalar"></a>

# Module `0x1::bls12381_scalar`



-  [Constants](#@Constants_0)
-  [Function `bls12381_hash_to_scalar`](#0x1_bls12381_scalar_bls12381_hash_to_scalar)
-  [Function `native_hash_to_scalar`](#0x1_bls12381_scalar_native_hash_to_scalar)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_bls12381_scalar_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native functions have not been rolled out yet.


<pre><code><b>const</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 1;
</code></pre>



<a id="0x1_bls12381_scalar_bls12381_hash_to_scalar"></a>

## Function `bls12381_hash_to_scalar`



<pre><code><b>public</b> <b>fun</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_bls12381_hash_to_scalar">bls12381_hash_to_scalar</a>(dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_Fr">bls12381_algebra::Fr</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_scalar.md#0x1_bls12381_scalar_bls12381_hash_to_scalar">bls12381_hash_to_scalar</a>(
    dst: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): Option&lt;Element&lt;Fr&gt;&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_private_poll_enabled">features::supra_private_poll_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="bls12381_scalar.md#0x1_bls12381_scalar_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>));
    <b>let</b> scalar_bytes = <a href="bls12381_scalar.md#0x1_bls12381_scalar_native_hash_to_scalar">native_hash_to_scalar</a>(dst, msg);
    deserialize&lt;Fr, FormatFrLsb&gt;(&scalar_bytes)
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
