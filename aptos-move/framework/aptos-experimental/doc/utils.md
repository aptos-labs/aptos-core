
<a id="0x7_utils"></a>

# Module `0x7::utils`



-  [Constants](#@Constants_0)
-  [Function `add_vec_points`](#0x7_utils_add_vec_points)
-  [Function `mul_points`](#0x7_utils_mul_points)
-  [Function `equal_vec_points`](#0x7_utils_equal_vec_points)
-  [Function `points_clone`](#0x7_utils_points_clone)
-  [Function `compress_points`](#0x7_utils_compress_points)
-  [Function `add_vec_scalars`](#0x7_utils_add_vec_scalars)
-  [Function `mul_scalars`](#0x7_utils_mul_scalars)
-  [Function `neg_scalars`](#0x7_utils_neg_scalars)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_utils_E_INTERNAL_INVARIANT_FAILED"></a>

One of our internal invariants was broken. There is likely a logical error in the code.


<pre><code><b>const</b> <a href="utils.md#0x7_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>: u64 = 0;
</code></pre>



<a id="0x7_utils_add_vec_points"></a>

## Function `add_vec_points`

Adds up two vectors of Ristretto255 points <code>a</code> and <code>b</code> returning a new vector <code>c</code> where <code>c[i] = a[i] + b[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_add_vec_points">add_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_add_vec_points">add_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    <b>assert</b>!(a.length() == b.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="utils.md#0x7_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.enumerate_ref(|i, pt| {
        r.push_back(point_add(pt, &b[i]));
    });

    r
}
</code></pre>



</details>

<a id="0x7_utils_mul_points"></a>

## Function `mul_points`

Given a vector of Ristretto255 points <code>a</code> and a scalar <code>e</code>, returns a new vector <code>c</code> where <code>c[i] = e * a[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_mul_points">mul_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, e: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_mul_points">mul_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, e: &Scalar): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.for_each_ref(|pt| {
        r.push_back(point_mul(pt, e));
    });

    r
}
</code></pre>



</details>

<a id="0x7_utils_equal_vec_points"></a>

## Function `equal_vec_points`

Ensures two vectors of Ristretto255 points are equal.


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_equal_vec_points">equal_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_equal_vec_points">equal_vec_points</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): bool {
    <b>let</b> m = a.length();
    <b>assert</b>!(m == b.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="utils.md#0x7_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    <b>let</b> i = 0;
    <b>while</b> (i &lt; m) {
        <b>if</b> (!point_equals(&a[i], &b[i])) {
            <b>return</b> <b>false</b>
        };

        i += 1;
    };

    <b>true</b>
}
</code></pre>



</details>

<a id="0x7_utils_points_clone"></a>

## Function `points_clone`

Clones a vector of Ristretto255 points


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_points_clone">points_clone</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_points_clone">points_clone</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    <b>let</b> cloned = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    a.for_each_ref(|p| {
        // TODO(Perf): Annoying limitation of our Ristretto255 <b>module</b>. (Should we "fix" it <b>as</b> per `<a href="../../aptos-framework/../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">crypto_algebra</a>`?)
        cloned.push_back(point_clone(p));
    });

    cloned
}
</code></pre>



</details>

<a id="0x7_utils_compress_points"></a>

## Function `compress_points`

Needed for Fiat-Shamir hashing


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="utils.md#0x7_utils_compress_points">compress_points</a>(points: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="utils.md#0x7_utils_compress_points">compress_points</a>(points: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    <b>let</b> compressed = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <b>let</b> i = 0;
    <b>let</b> len = points.length();
    <b>while</b> (i &lt; len) {
        compressed.push_back(point_compress(&points[i]));
        i += 1;
    };

    compressed
}
</code></pre>



</details>

<a id="0x7_utils_add_vec_scalars"></a>

## Function `add_vec_scalars`

Adds up two vectors of scalar points <code>a</code> and <code>b</code> returning a new vector <code>c</code> where <code>c[i] = a[i] + b[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_add_vec_scalars">add_vec_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_add_vec_scalars">add_vec_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <b>assert</b>!(a.length() == b.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="utils.md#0x7_utils_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.enumerate_ref(|i, a_i| {
        r.push_back(scalar_add(a_i, &b[i]));
    });

    r
}
</code></pre>



</details>

<a id="0x7_utils_mul_scalars"></a>

## Function `mul_scalars`

Given a vector of scalars <code>a</code> and a scalar <code>e</code>, returns a new vector <code>c</code> where <code>c[i] = e * a[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_mul_scalars">mul_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;, e: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_mul_scalars">mul_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;, e: &Scalar): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.for_each_ref(|s| {
        r.push_back(scalar_mul(s, e));
    });

    r
}
</code></pre>



</details>

<a id="0x7_utils_neg_scalars"></a>

## Function `neg_scalars`

Given a vector of scalars <code>a</code> and a scalar <code>e</code>, returns a new vector <code>c</code> where <code>c[i] = e * a[i]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_neg_scalars">neg_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="utils.md#0x7_utils_neg_scalars">neg_scalars</a>(a: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    <b>let</b> r = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    a.for_each_ref(|s| {
        r.push_back(scalar_neg(s));
    });

    r
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
