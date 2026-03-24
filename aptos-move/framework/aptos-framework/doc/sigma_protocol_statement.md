
<a id="0x1_sigma_protocol_statement"></a>

# Module `0x1::sigma_protocol_statement`



-  [Struct `Statement`](#0x1_sigma_protocol_statement_Statement)
-  [Constants](#@Constants_0)
-  [Function `new_statement`](#0x1_sigma_protocol_statement_new_statement)
-  [Function `get_point`](#0x1_sigma_protocol_statement_get_point)
-  [Function `get_scalars`](#0x1_sigma_protocol_statement_get_scalars)
-  [Function `get_points`](#0x1_sigma_protocol_statement_get_points)
-  [Function `get_compressed_points`](#0x1_sigma_protocol_statement_get_compressed_points)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a id="0x1_sigma_protocol_statement_Statement"></a>

## Struct `Statement`

A *public statement* consists of:
- a <code>points</code> vector of $n_1$ group elements
- a <code>compressed_points</code> vector of $n_1$ compressed group elements (redundant, for faster Fiat-Shamir)
- a <code>scalars</code> vector of $n_2$ scalars

The phantom type parameter <code>P</code> tags the statement with a specific protocol marker type
(e.g., <code>Registration</code>, <code>KeyRotation</code>, etc.) for compile-time safety.


<pre><code><b>struct</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a>&lt;P&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scalars: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_sigma_protocol_statement_E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS"></a>

When creating a <code><a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a></code>, the # of points must match the # of compressed points.


<pre><code><b>const</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS">E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_statement_new_statement"></a>

## Function `new_statement`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_new_statement">new_statement</a>&lt;P&gt;(points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, compressed_points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, scalars: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_new_statement">new_statement</a>&lt;P&gt;(
    points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    compressed_points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    scalars: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;
): <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a>&lt;P&gt; {
    <b>assert</b>!(points.length() == compressed_points.length(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS">E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS</a>));
    <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a> { points, compressed_points, scalars }
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_get_point"></a>

## Function `get_point`

Returns the $i$th elliptic curve point in the public statement.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_point">get_point</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;, i: u64): &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_point">get_point</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a>&lt;P&gt;, i: u64): &RistrettoPoint {
    &self.points[i]
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_get_scalars"></a>

## Function `get_scalars`

Returns all the scalars in the statement.
(Needed to feed in the statement in the Fiat-Shamir transform.)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_scalars">get_scalars</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_scalars">get_scalars</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a>&lt;P&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    &self.scalars
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_get_points"></a>

## Function `get_points`

Returns all the elliptic curve points in the statement.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_points">get_points</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_points">get_points</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a>&lt;P&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.points
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_get_compressed_points"></a>

## Function `get_compressed_points`

Returns all the compressed elliptic curve points in the statement.
(Needed to feed in the statement in the Fiat-Shamir transform.)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_compressed_points">get_compressed_points</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_get_compressed_points">get_compressed_points</a>&lt;P&gt;(self: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">Statement</a>&lt;P&gt;): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; {
    &self.compressed_points
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
