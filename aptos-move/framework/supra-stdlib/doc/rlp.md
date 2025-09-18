
<a id="0x1_rlp"></a>

# Module `0x1::rlp`



-  [Constants](#@Constants_0)
-  [Function `encode`](#0x1_rlp_encode)
-  [Function `decode`](#0x1_rlp_decode)
-  [Function `encode_list_scalar`](#0x1_rlp_encode_list_scalar)
-  [Function `decode_list_scalar`](#0x1_rlp_decode_list_scalar)
-  [Function `encode_list_byte_array`](#0x1_rlp_encode_list_byte_array)
-  [Function `decode_list_byte_array`](#0x1_rlp_decode_list_byte_array)
-  [Function `native_rlp_encode`](#0x1_rlp_native_rlp_encode)
-  [Function `native_rlp_decode`](#0x1_rlp_native_rlp_decode)
-  [Function `native_rlp_encode_list_scalar`](#0x1_rlp_native_rlp_encode_list_scalar)
-  [Function `native_rlp_decode_list_scalar`](#0x1_rlp_native_rlp_decode_list_scalar)
-  [Function `native_rlp_encode_list_byte_array`](#0x1_rlp_native_rlp_encode_list_byte_array)
-  [Function `native_rlp_decode_list_byte_array`](#0x1_rlp_native_rlp_decode_list_byte_array)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED"></a>

SUPRA_RLP_ENCODE feature APIs are disabled.


<pre><code><b>const</b> <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>: u64 = 1;
</code></pre>



<a id="0x1_rlp_encode"></a>

## Function `encode`



<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_encode">encode</a>&lt;T&gt;(x: T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_encode">encode</a>&lt;T&gt;(x: T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_rlp_enabled">features::supra_rlp_enabled</a>(), <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>);
    <a href="rlp.md#0x1_rlp_native_rlp_encode">native_rlp_encode</a>(x)
}
</code></pre>



</details>

<a id="0x1_rlp_decode"></a>

## Function `decode`



<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_decode">decode</a>&lt;T&gt;(encoded_rlp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_decode">decode</a>&lt;T&gt;(encoded_rlp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_rlp_enabled">features::supra_rlp_enabled</a>(), <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>);
    <a href="rlp.md#0x1_rlp_native_rlp_decode">native_rlp_decode</a>(encoded_rlp)
}
</code></pre>



</details>

<a id="0x1_rlp_encode_list_scalar"></a>

## Function `encode_list_scalar`



<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_encode_list_scalar">encode_list_scalar</a>&lt;T: drop&gt;(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_encode_list_scalar">encode_list_scalar</a>&lt;T: drop&gt;(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_rlp_enabled">features::supra_rlp_enabled</a>(), <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>);
    <a href="rlp.md#0x1_rlp_native_rlp_encode_list_scalar">native_rlp_encode_list_scalar</a>&lt;T&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&data))
}
</code></pre>



</details>

<a id="0x1_rlp_decode_list_scalar"></a>

## Function `decode_list_scalar`



<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_decode_list_scalar">decode_list_scalar</a>&lt;T&gt;(encoded_rlp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_decode_list_scalar">decode_list_scalar</a>&lt;T&gt;(encoded_rlp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_rlp_enabled">features::supra_rlp_enabled</a>(), <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>);
    <a href="rlp.md#0x1_rlp_native_rlp_decode_list_scalar">native_rlp_decode_list_scalar</a>&lt;T&gt;(encoded_rlp)
}
</code></pre>



</details>

<a id="0x1_rlp_encode_list_byte_array"></a>

## Function `encode_list_byte_array`



<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_encode_list_byte_array">encode_list_byte_array</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_encode_list_byte_array">encode_list_byte_array</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_rlp_enabled">features::supra_rlp_enabled</a>(), <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>);
    <a href="rlp.md#0x1_rlp_native_rlp_encode_list_byte_array">native_rlp_encode_list_byte_array</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&data))
}
</code></pre>



</details>

<a id="0x1_rlp_decode_list_byte_array"></a>

## Function `decode_list_byte_array`



<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_decode_list_byte_array">decode_list_byte_array</a>(encoded_rlp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rlp.md#0x1_rlp_decode_list_byte_array">decode_list_byte_array</a>(encoded_rlp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_supra_rlp_enabled">features::supra_rlp_enabled</a>(), <a href="rlp.md#0x1_rlp_ERLP_ENCODE_FEATURE_DISABLED">ERLP_ENCODE_FEATURE_DISABLED</a>);
    <b>let</b> ser_result = <a href="rlp.md#0x1_rlp_native_rlp_decode_list_byte_array">native_rlp_decode_list_byte_array</a>(encoded_rlp);
    <b>let</b> any_ser = <a href="../../aptos-stdlib/doc/any.md#0x1_any_new">any::new</a>(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;(), ser_result);
    <a href="../../aptos-stdlib/doc/any.md#0x1_any_unpack">any::unpack</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;(any_ser)
}
</code></pre>



</details>

<a id="0x1_rlp_native_rlp_encode"></a>

## Function `native_rlp_encode`



<pre><code><b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_encode">native_rlp_encode</a>&lt;T&gt;(x: T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_encode">native_rlp_encode</a>&lt;T&gt;(x: T): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_rlp_native_rlp_decode"></a>

## Function `native_rlp_decode`



<pre><code><b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_decode">native_rlp_decode</a>&lt;T&gt;(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_decode">native_rlp_decode</a>&lt;T&gt;(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T;
</code></pre>



</details>

<a id="0x1_rlp_native_rlp_encode_list_scalar"></a>

## Function `native_rlp_encode_list_scalar`



<pre><code><b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_encode_list_scalar">native_rlp_encode_list_scalar</a>&lt;T&gt;(x: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_encode_list_scalar">native_rlp_encode_list_scalar</a>&lt;T&gt;(x: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_rlp_native_rlp_decode_list_scalar"></a>

## Function `native_rlp_decode_list_scalar`



<pre><code><b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_decode_list_scalar">native_rlp_decode_list_scalar</a>&lt;T&gt;(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_decode_list_scalar">native_rlp_decode_list_scalar</a>&lt;T&gt;(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;;
</code></pre>



</details>

<a id="0x1_rlp_native_rlp_encode_list_byte_array"></a>

## Function `native_rlp_encode_list_byte_array`



<pre><code><b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_encode_list_byte_array">native_rlp_encode_list_byte_array</a>(x: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_encode_list_byte_array">native_rlp_encode_list_byte_array</a>(x: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_rlp_native_rlp_decode_list_byte_array"></a>

## Function `native_rlp_decode_list_byte_array`



<pre><code><b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_decode_list_byte_array">native_rlp_decode_list_byte_array</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="rlp.md#0x1_rlp_native_rlp_decode_list_byte_array">native_rlp_decode_list_byte_array</a>(data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
