
<a id="0x7_sigma_protocol_utils"></a>

# Module `0x7::sigma_protocol_utils`



-  [Function `points_clone`](#0x7_sigma_protocol_utils_points_clone)
-  [Function `deserialize_points`](#0x7_sigma_protocol_utils_deserialize_points)
-  [Function `deserialize_compressed_points`](#0x7_sigma_protocol_utils_deserialize_compressed_points)
-  [Function `deserialize_scalars`](#0x7_sigma_protocol_utils_deserialize_scalars)
-  [Function `e_wrong_num_points`](#0x7_sigma_protocol_utils_e_wrong_num_points)
-  [Function `e_wrong_num_scalars`](#0x7_sigma_protocol_utils_e_wrong_num_scalars)
-  [Function `e_wrong_witness_len`](#0x7_sigma_protocol_utils_e_wrong_witness_len)
-  [Function `e_wrong_output_len`](#0x7_sigma_protocol_utils_e_wrong_output_len)
-  [Function `neg_scalars`](#0x7_sigma_protocol_utils_neg_scalars)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x7_sigma_protocol_utils_points_clone"></a>

## Function `points_clone`

Clones a vector of Ristretto255 points


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_points_clone">points_clone</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_points_clone">points_clone</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    a.map_ref(|p| p.point_clone())
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_deserialize_points"></a>

## Function `deserialize_points`

Deserializes a vector of point bytes to a vector of RistrettoPoints and a vector of their compressed counterparts.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_points">deserialize_points</a>(points_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_points">deserialize_points</a>(points_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    <b>let</b> points = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> compressed_points = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    points_bytes.for_each(|point_bytes| {
        <b>let</b> (point, compressed_point) = new_point_and_compressed_from_bytes(point_bytes);
        points.push_back(point);
        compressed_points.push_back(compressed_point);
    });

    (points, compressed_points)
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_deserialize_compressed_points"></a>

## Function `deserialize_compressed_points`

Deserializes a vector of point bytes to a vector of CompressedRistretto's (without decompressing to RistrettoPoint).


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_compressed_points">deserialize_compressed_points</a>(points_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_compressed_points">deserialize_compressed_points</a>(points_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    points_bytes.map(|bytes| new_compressed_point_from_bytes(bytes).extract())
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_deserialize_scalars"></a>

## Function `deserialize_scalars`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_scalars">deserialize_scalars</a>(scalars_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_scalars">deserialize_scalars</a>(scalars_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    scalars_bytes.map(|scalar_bytes| new_scalar_from_bytes(scalar_bytes).extract())
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_e_wrong_num_points"></a>

## Function `e_wrong_num_points`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_num_points">e_wrong_num_points</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_num_points">e_wrong_num_points</a>(): u64 { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(1) }
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_e_wrong_num_scalars"></a>

## Function `e_wrong_num_scalars`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_num_scalars">e_wrong_num_scalars</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_num_scalars">e_wrong_num_scalars</a>(): u64 { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(2) }
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_e_wrong_witness_len"></a>

## Function `e_wrong_witness_len`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_witness_len">e_wrong_witness_len</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_witness_len">e_wrong_witness_len</a>(): u64 { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(3) }
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_e_wrong_output_len"></a>

## Function `e_wrong_output_len`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_output_len">e_wrong_output_len</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_e_wrong_output_len">e_wrong_output_len</a>(): u64 { <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(4) }
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_neg_scalars"></a>

## Function `neg_scalars`

Negates a vector of scalars <code>a</code>, returns a new vector <code>c</code> where <code>c[i] = -a[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_neg_scalars">neg_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_neg_scalars">neg_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    a.map_ref(|s| s.scalar_neg())
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
