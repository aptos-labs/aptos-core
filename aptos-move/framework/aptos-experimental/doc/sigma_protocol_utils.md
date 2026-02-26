
<a id="0x7_sigma_protocol_utils"></a>

# Module `0x7::sigma_protocol_utils`



-  [Constants](#@Constants_0)
-  [Function `add_vec_points`](#0x7_sigma_protocol_utils_add_vec_points)
-  [Function `mul_points`](#0x7_sigma_protocol_utils_mul_points)
-  [Function `equal_vec_points`](#0x7_sigma_protocol_utils_equal_vec_points)
-  [Function `points_clone`](#0x7_sigma_protocol_utils_points_clone)
-  [Function `deserialize_points`](#0x7_sigma_protocol_utils_deserialize_points)
-  [Function `deserialize_scalars`](#0x7_sigma_protocol_utils_deserialize_scalars)
-  [Function `decompress_points`](#0x7_sigma_protocol_utils_decompress_points)
-  [Function `compress_points`](#0x7_sigma_protocol_utils_compress_points)
-  [Function `add_vec_scalars`](#0x7_sigma_protocol_utils_add_vec_scalars)
-  [Function `mul_scalars`](#0x7_sigma_protocol_utils_mul_scalars)
-  [Function `neg_scalars`](#0x7_sigma_protocol_utils_neg_scalars)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED"></a>

One of our internal invariants was broken. There is likely a logical error in the code.


<pre><code><b>const</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_utils_add_vec_points"></a>

## Function `add_vec_points`

Adds up two vectors of Ristretto255 points <code>a</code> and <code>b</code> returning a new vector <code>c</code> where <code>c[i] = a[i] + b[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_add_vec_points">add_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_add_vec_points">add_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    <b>assert</b>!(a.length() == b.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.enumerate_ref(|i, pt| {
        r.push_back(pt.point_add(&b[i]));
    });

    r
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_mul_points"></a>

## Function `mul_points`

Given a vector of Ristretto255 points <code>a</code> and a scalar <code>e</code>, returns a new vector <code>c</code> where <code>c[i] = e * a[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_mul_points">mul_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, e: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_mul_points">mul_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, e: &Scalar): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    a.map_ref(|pt| pt.point_mul(e))
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_equal_vec_points"></a>

## Function `equal_vec_points`

Ensures two vectors of Ristretto255 points are equal.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_equal_vec_points">equal_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_equal_vec_points">equal_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): bool {
    <b>let</b> m = a.length();
    <b>assert</b>!(m == b.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    <b>let</b> i = 0;
    <b>while</b> (i &lt; m) {
        <b>if</b> (!a[i].point_equals(&b[i])) {
            <b>return</b> <b>false</b>
        };

        i += 1;
    };

    <b>true</b>
}
</code></pre>



</details>

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


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_points">deserialize_points</a>(points_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_points">deserialize_points</a>(points_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    <b>let</b> points = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> compressed_points = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    points_bytes.for_each(|point_bytes| {
        <b>let</b> (point, compressed_point) = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_and_compressed_from_bytes">ristretto255::new_point_and_compressed_from_bytes</a>(point_bytes);

        points.push_back(point);
        compressed_points.push_back(compressed_point);
    });

    <b>assert</b>!(points.length() == points_bytes.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));
    <b>assert</b>!(points.length() == compressed_points.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    (points, compressed_points)
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_deserialize_scalars"></a>

## Function `deserialize_scalars`

Deserializes a vector of scalar bytes to a vector of Scalar's


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_scalars">deserialize_scalars</a>(scalars_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_scalars">deserialize_scalars</a>(scalars_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    scalars_bytes.map(|scalar_bytes| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(scalar_bytes).extract()

    })
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_decompress_points"></a>

## Function `decompress_points`

Decmpresses a vector of CompressedRistretto's


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_decompress_points">decompress_points</a>(compressed: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_decompress_points">decompress_points</a>(compressed: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    compressed.map_ref(|p| {
        p.point_decompress()
    })
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_compress_points"></a>

## Function `compress_points`

Compresses a vector of Ristretto255 points.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_compress_points">compress_points</a>(points: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_compress_points">compress_points</a>(points: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    points.map_ref(|p| p.point_compress())
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_add_vec_scalars"></a>

## Function `add_vec_scalars`

Adds up two vectors of scalar points <code>a</code> and <code>b</code> returning a new vector <code>c</code> where <code>c[i] = a[i] + b[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_add_vec_scalars">add_vec_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_add_vec_scalars">add_vec_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <b>assert</b>!(a.length() == b.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.enumerate_ref(|i, a_i| {
        r.push_back(a_i.scalar_add(&b[i]));
    });

    r
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_utils_mul_scalars"></a>

## Function `mul_scalars`

Given a vector of scalars <code>a</code> and a scalar <code>e</code>, returns a new vector <code>c</code> where <code>c[i] = e * a[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_mul_scalars">mul_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;, e: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_mul_scalars">mul_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;, e: &Scalar): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    a.map_ref(|s| s.scalar_mul(e))
}
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
