
<a name="0x1_pedersen"></a>

# Module `0x1::pedersen`

This module implements a Pedersen commitment API that can be used with the Bulletproofs module.

A Pedersen commitment to a value v under a _commitment key_ (g, h) is v * g + r * h, for a random scalar r.


-  [Struct `Commitment`](#0x1_pedersen_Commitment)
-  [Constants](#@Constants_0)
-  [Function `new_commitment_from_point`](#0x1_pedersen_new_commitment_from_point)
-  [Function `new_commitment_from_compressed`](#0x1_pedersen_new_commitment_from_compressed)
-  [Function `new_commitment`](#0x1_pedersen_new_commitment)
-  [Function `new_commitment_with_basepoint`](#0x1_pedersen_new_commitment_with_basepoint)
-  [Function `new_commitment_for_bulletproof`](#0x1_pedersen_new_commitment_for_bulletproof)
-  [Function `new_non_hiding_commitment_for_bulletproof`](#0x1_pedersen_new_non_hiding_commitment_for_bulletproof)
-  [Function `commitment_add`](#0x1_pedersen_commitment_add)
-  [Function `commitment_add_assign`](#0x1_pedersen_commitment_add_assign)
-  [Function `commitment_sub`](#0x1_pedersen_commitment_sub)
-  [Function `commitment_sub_assign`](#0x1_pedersen_commitment_sub_assign)
-  [Function `commitment_clone`](#0x1_pedersen_commitment_clone)
-  [Function `commitment_equals`](#0x1_pedersen_commitment_equals)
-  [Function `commitment_as_point`](#0x1_pedersen_commitment_as_point)
-  [Function `commitment_as_compressed_point`](#0x1_pedersen_commitment_as_compressed_point)
-  [Function `commitment_into_point`](#0x1_pedersen_commitment_into_point)
-  [Function `commitment_into_compressed_point`](#0x1_pedersen_commitment_into_compressed_point)
-  [Function `randomness_base_for_bulletproof`](#0x1_pedersen_randomness_base_for_bulletproof)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a name="0x1_pedersen_Commitment"></a>

## Struct `Commitment`

A Pedersen commitment to some value with some randomness.


<pre><code><b>struct</b> <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE"></a>

The default Pedersen randomness base used in our underlying Bulletproofs library.
This is obtained by hashing the compressed Ristretto255 basepoint using SHA3-512 (not SHA2-512).


<pre><code><b>const</b> <a href="pedersen.md#0x1_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [140, 146, 64, 180, 86, 169, 230, 220, 101, 195, 119, 161, 4, 141, 116, 95, 148, 160, 140, 219, 127, 68, 203, 205, 123, 70, 243, 64, 72, 135, 17, 52];
</code></pre>



<a name="0x1_pedersen_new_commitment_from_point"></a>

## Function `new_commitment_from_point`

Moves a Ristretto point into a Pedersen commitment.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_from_point">new_commitment_from_point</a>(point: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_from_point">new_commitment_from_point</a>(point: RistrettoPoint): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_new_commitment_from_compressed"></a>

## Function `new_commitment_from_compressed`

Deserializes a commitment from a compressed Ristretto point.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_from_compressed">new_commitment_from_compressed</a>(point: &<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_from_compressed">new_commitment_from_compressed</a>(point: &CompressedRistretto): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(point)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_new_commitment"></a>

## Function `new_commitment`

Returns a commitment val * val_base + r * rand_base where (val_base, rand_base) is the commitment key.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment">new_commitment</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment">new_commitment</a>(val: &Scalar, val_base: &RistrettoPoint, rand: &Scalar, rand_base: &RistrettoPoint): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul">ristretto255::double_scalar_mul</a>(val, val_base, rand, rand_base)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_new_commitment_with_basepoint"></a>

## Function `new_commitment_with_basepoint`

Returns a commitment val * basepoint + r * rand_base where <code>basepoint</code> is the Ristretto255 basepoint.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_with_basepoint">new_commitment_with_basepoint</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_with_basepoint">new_commitment_with_basepoint</a>(val: &Scalar, rand: &Scalar, rand_base: &RistrettoPoint): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">ristretto255::basepoint_double_mul</a>(rand, rand_base, val)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_new_commitment_for_bulletproof"></a>

## Function `new_commitment_for_bulletproof`

Returns a commitment val * basepoint + r * rand_base where <code>basepoint</code> is the Ristretto255 basepoint and <code>rand_base</code>
is the default randomness based used in the Bulletproof library (i.e., BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE).


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_for_bulletproof">new_commitment_for_bulletproof</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_commitment_for_bulletproof">new_commitment_for_bulletproof</a>(val: &Scalar, rand: &Scalar): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <b>let</b> rand_base = <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(<a href="pedersen.md#0x1_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>);
    <b>let</b> rand_base = std::option::extract(&<b>mut</b> rand_base);

    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">ristretto255::basepoint_double_mul</a>(rand, &rand_base, val)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_new_non_hiding_commitment_for_bulletproof"></a>

## Function `new_non_hiding_commitment_for_bulletproof`

Returns a non-hiding commitment val * basepoint where <code>basepoint</code> is the Ristretto255 basepoint.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_non_hiding_commitment_for_bulletproof">new_non_hiding_commitment_for_bulletproof</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_new_non_hiding_commitment_for_bulletproof">new_non_hiding_commitment_for_bulletproof</a>(val: &Scalar): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(val)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_add"></a>

## Function `commitment_add`

Returns lhs + rhs. Useful for re-randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_add">commitment_add</a>(lhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_add">commitment_add</a>(lhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&lhs.point, &rhs.point)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_add_assign"></a>

## Function `commitment_add_assign`

Sets lhs = lhs + rhs. Useful for re-randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_add_assign">commitment_add_assign</a>(lhs: &<b>mut</b> <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_add_assign">commitment_add_assign</a>(lhs: &<b>mut</b> <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>) {
    <a href="ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.point, &rhs.point);
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_sub"></a>

## Function `commitment_sub`

Returns lhs - rhs. Useful for re-randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_sub">commitment_sub</a>(lhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_sub">commitment_sub</a>(lhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&lhs.point, &rhs.point)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_sub_assign"></a>

## Function `commitment_sub_assign`

Sets lhs = lhs - rhs. Useful for re-randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_sub_assign">commitment_sub_assign</a>(lhs: &<b>mut</b> <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_sub_assign">commitment_sub_assign</a>(lhs: &<b>mut</b> <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>) {
    <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&<b>mut</b> lhs.point, &rhs.point);
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_clone"></a>

## Function `commitment_clone`

Creates a copy of this commitment.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_clone">commitment_clone</a>(c: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_clone">commitment_clone</a>(c: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
    <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> {
        point: <a href="ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&c.point)
    }
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_equals"></a>

## Function `commitment_equals`

Returns true if the two commitments are identical: i.e., same value and same randomness.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_equals">commitment_equals</a>(lhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_equals">commitment_equals</a>(lhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>, rhs: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): bool {
    <a href="ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs.point, &rhs.point)
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_as_point"></a>

## Function `commitment_as_point`

Returns the underlying elliptic curve point representing the commitment as an in-memory RistrettoPoint.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_as_point">commitment_as_point</a>(c: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_as_point">commitment_as_point</a>(c: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): &RistrettoPoint {
    &c.point
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_as_compressed_point"></a>

## Function `commitment_as_compressed_point`

Returns the Pedersen commitment as a CompressedRistretto point.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_as_compressed_point">commitment_as_compressed_point</a>(c: &<a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_as_compressed_point">commitment_as_compressed_point</a>(c: &<a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): CompressedRistretto {
    point_compress(&c.point)
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_into_point"></a>

## Function `commitment_into_point`

Moves the Commitment into a CompressedRistretto point.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_into_point">commitment_into_point</a>(c: <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_into_point">commitment_into_point</a>(c: <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): RistrettoPoint {
    <b>let</b> <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a> { point } = c;
    point
}
</code></pre>



</details>

<a name="0x1_pedersen_commitment_into_compressed_point"></a>

## Function `commitment_into_compressed_point`

Moves the Commitment into a CompressedRistretto point.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_into_compressed_point">commitment_into_compressed_point</a>(c: <a href="pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_commitment_into_compressed_point">commitment_into_compressed_point</a>(c: <a href="pedersen.md#0x1_pedersen_Commitment">Commitment</a>): CompressedRistretto {
    point_compress(&c.point)
}
</code></pre>



</details>

<a name="0x1_pedersen_randomness_base_for_bulletproof"></a>

## Function `randomness_base_for_bulletproof`

Returns the randomness base compatible with the Bulletproofs module.


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_randomness_base_for_bulletproof">randomness_base_for_bulletproof</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pedersen.md#0x1_pedersen_randomness_base_for_bulletproof">randomness_base_for_bulletproof</a>(): RistrettoPoint {
    std::option::extract(&<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(<a href="pedersen.md#0x1_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>))
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
