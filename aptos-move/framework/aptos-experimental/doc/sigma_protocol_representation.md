
<a id="0x7_sigma_protocol_representation"></a>

# Module `0x7::sigma_protocol_representation`



-  [Struct `Representation`](#0x7_sigma_protocol_representation_Representation)
-  [Constants](#@Constants_0)
-  [Function `new_representation`](#0x7_sigma_protocol_representation_new_representation)
-  [Function `to_points`](#0x7_sigma_protocol_representation_to_points)
-  [Function `get_scalars`](#0x7_sigma_protocol_representation_get_scalars)
-  [Function `scale`](#0x7_sigma_protocol_representation_scale)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
</code></pre>



<a id="0x7_sigma_protocol_representation_Representation"></a>

## Struct `Representation`

A *representation* of a group element $G$ is a list of group elements $G_i$ and scalars $a_i$ such that:
$G = \sum_{i \in [n_1]} a_i G_i$
The actual group elements are large, so to indicate that $G_i$ is the $j$th entry from the
<code>Statement::points</code> vector, we set <code>Representation::points_idxs[i]</code> to $j$. (Note that $j \in [0, n_1)$.)

Note: Instead of returning $m$ group elements, the Move implementation of a transformation function $f$ (and/or
a homomorphism $\psi$) will return $m$ representations. This makes it possible to implement a faster verifier
(and prover too) that uses multi-scalar multiplications!


<pre><code><b>struct</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">Representation</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point_idxs: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
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

<a id="@Constants_0"></a>

## Constants


<a id="0x7_sigma_protocol_representation_E_MISMATCHED_LENGTHS"></a>

The number of points and scalars in a Representation needs to be the same.


<pre><code><b>const</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_E_MISMATCHED_LENGTHS">E_MISMATCHED_LENGTHS</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_representation_new_representation"></a>

## Function `new_representation`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_new_representation">new_representation</a>(points: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalars: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_new_representation">new_representation</a>(points: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalars: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">Representation</a> {
    <b>assert</b>!(points.length() == scalars.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_E_MISMATCHED_LENGTHS">E_MISMATCHED_LENGTHS</a>));
    <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">Representation</a> {
        point_idxs: points, scalars
    }
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_to_points"></a>

## Function `to_points`

Given a representation, which only stores locations of group elements within a public statement, returns the
actual vector of group elements by "looking up" these elements in the public statement.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_to_points">to_points</a>(self: &<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_to_points">to_points</a>(self: &<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">Representation</a>, stmt: &Statement): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt; {
    self.point_idxs.map(|idx| stmt.get_point(idx).point_clone())
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_get_scalars"></a>

## Function `get_scalars`

Returns the scalars in the representation.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_get_scalars">get_scalars</a>(self: &<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_get_scalars">get_scalars</a>(self: &<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">Representation</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    &self.scalars
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_scale"></a>

## Function `scale`

Multiplies all the scalars in the representation by $e$.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_scale">scale</a>(self: &<b>mut</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>, e: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_scale">scale</a>(self: &<b>mut</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">Representation</a>, e: &Scalar) {
    self.scalars.for_each_mut(|scalar| {
        scalar.scalar_mul_assign(e);
    });
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
