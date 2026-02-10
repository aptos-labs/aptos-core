
<a id="0x7_public_statement"></a>

# Module `0x7::public_statement`

TODO: make more functions public(friend)


-  [Struct `PublicStatement`](#0x7_public_statement_PublicStatement)
-  [Function `new_public_statement`](#0x7_public_statement_new_public_statement)
-  [Function `get_point`](#0x7_public_statement_get_point)
-  [Function `get_scalars`](#0x7_public_statement_get_scalars)
-  [Function `get_points`](#0x7_public_statement_get_points)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a id="0x7_public_statement_PublicStatement"></a>

## Struct `PublicStatement`

A *public statement* consists of:
- a <code>points</code> vector of $n_1$ group elements
- a <code>scalars</code> vector of $n_2$ scalars


<pre><code><b>struct</b> <a href="public_statement.md#0x7_public_statement_PublicStatement">PublicStatement</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>points: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scalars: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_public_statement_new_public_statement"></a>

## Function `new_public_statement`



<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_new_public_statement">new_public_statement</a>(points: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, scalars: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_new_public_statement">new_public_statement</a>(points: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, scalars: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="public_statement.md#0x7_public_statement_PublicStatement">PublicStatement</a> {
    <a href="public_statement.md#0x7_public_statement_PublicStatement">PublicStatement</a> { points, scalars }
}
</code></pre>



</details>

<a id="0x7_public_statement_get_point"></a>

## Function `get_point`

Returns the $i$th elliptic curve point in the public statement.


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_get_point">get_point</a>(self: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>, i: u64): &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_get_point">get_point</a>(self: &<a href="public_statement.md#0x7_public_statement_PublicStatement">PublicStatement</a>, i: u64): &RistrettoPoint {
    &self.points[i]
}
</code></pre>



</details>

<a id="0x7_public_statement_get_scalars"></a>

## Function `get_scalars`

Returns all the scalars in the statement.
(Needed to feed in the statement in the Fiat-Shamir transform.)


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_get_scalars">get_scalars</a>(self: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_get_scalars">get_scalars</a>(self: &<a href="public_statement.md#0x7_public_statement_PublicStatement">PublicStatement</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    &self.scalars
}
</code></pre>



</details>

<a id="0x7_public_statement_get_points"></a>

## Function `get_points`

Returns all the elliptic curve points in the statement.
(Needed to feed in the statement in the Fiat-Shamir transform.)


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_get_points">get_points</a>(self: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="public_statement.md#0x7_public_statement_get_points">get_points</a>(self: &<a href="public_statement.md#0x7_public_statement_PublicStatement">PublicStatement</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    &self.points
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
