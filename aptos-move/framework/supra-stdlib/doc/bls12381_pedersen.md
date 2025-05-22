
<a id="0x1_bls12381_pedersen"></a>

# Module `0x1::bls12381_pedersen`

This module implements a Pedersen commitment API, over the Ristretto255 curve, that can be used with the
Bulletproofs module.

A Pedersen commitment to a value <code>v</code> under _commitment key_ <code>(g, h)</code> is <code>v * g + r * h</code>, for a random scalar <code>r</code>.


-  [Struct `Commitment`](#0x1_bls12381_pedersen_Commitment)
-  [Constants](#@Constants_0)
-  [Function `new_commitment_from_bytes`](#0x1_bls12381_pedersen_new_commitment_from_bytes)
-  [Function `commitment_to_bytes`](#0x1_bls12381_pedersen_commitment_to_bytes)
-  [Function `commitment_from_point`](#0x1_bls12381_pedersen_commitment_from_point)
-  [Function `new_commitment`](#0x1_bls12381_pedersen_new_commitment)
-  [Function `new_commitment_for_bulletproof`](#0x1_bls12381_pedersen_new_commitment_for_bulletproof)
-  [Function `commitment_add`](#0x1_bls12381_pedersen_commitment_add)
-  [Function `commitment_add_assign`](#0x1_bls12381_pedersen_commitment_add_assign)
-  [Function `commitment_sub`](#0x1_bls12381_pedersen_commitment_sub)
-  [Function `commitment_sub_assign`](#0x1_bls12381_pedersen_commitment_sub_assign)
-  [Function `commitment_equals`](#0x1_bls12381_pedersen_commitment_equals)
-  [Function `commitment_as_point`](#0x1_bls12381_pedersen_commitment_as_point)
-  [Function `commitment_into_point`](#0x1_bls12381_pedersen_commitment_into_point)
-  [Function `randomness_base_for_bulletproof`](#0x1_bls12381_pedersen_randomness_base_for_bulletproof)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x1_bls12381_pedersen_Commitment"></a>

## Struct `Commitment`

A Pedersen commitment to some value with some randomness.


<pre><code><b>struct</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point: <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_bls12381_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE"></a>

The default Pedersen randomness base <code>h</code> used in our underlying Bulletproofs library.
This is obtained by hashing the compressed Bls12381 basepoint using SHA3-512 (not SHA2-512).


<pre><code><b>const</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [173, 57, 199, 200, 109, 183, 123, 97, 177, 96, 166, 117, 104, 239, 88, 18, 95, 178, 232, 67, 248, 58, 50, 123, 161, 248, 34, 70, 43, 90, 44, 19, 7, 49, 216, 102, 225, 25, 188, 193, 118, 105, 16, 227, 247, 103, 40, 120];
</code></pre>



<a id="0x1_bls12381_pedersen_new_commitment_from_bytes"></a>

## Function `new_commitment_from_bytes`

Creates a new public key from a serialized Bls12381 point.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_new_commitment_from_bytes">new_commitment_from_bytes</a>(bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_new_commitment_from_bytes">new_commitment_from_bytes</a>(bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>&gt; {
    <b>let</b> point = deserialize&lt;G1, FormatG1Compr&gt;(&bytes);
    <b>if</b> (std::option::is_some(&<b>mut</b> point)) {
        <b>let</b> comm = <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
            point: std::option::extract(&<b>mut</b> point)
        };
        std::option::some(comm)
    } <b>else</b> {
        std::option::none&lt;<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>&gt;()
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_to_bytes"></a>

## Function `commitment_to_bytes`

Returns a commitment as a serialized byte array


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_to_bytes">commitment_to_bytes</a>(comm: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_to_bytes">commitment_to_bytes</a>(comm: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    serialize&lt;G1,FormatG1Compr&gt;(&comm.point)
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_from_point"></a>

## Function `commitment_from_point`

Moves a Ristretto point into a Pedersen commitment.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_from_point">commitment_from_point</a>(point: <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_from_point">commitment_from_point</a>(point: Element&lt;G1&gt;): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
    <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
        point
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_new_commitment"></a>

## Function `new_commitment`

Returns a commitment <code>v * val_base + r * rand_base</code> where <code>(val_base, rand_base)</code> is the commitment key.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_new_commitment">new_commitment</a>(v: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_Fr">bls12381_algebra::Fr</a>&gt;, val_base: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;, r: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_Fr">bls12381_algebra::Fr</a>&gt;, rand_base: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_new_commitment">new_commitment</a>(v: &Element&lt;Fr&gt;, val_base: &Element&lt;G1&gt;, r: &Element&lt;Fr&gt;, rand_base: &Element&lt;G1&gt;): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
    <b>let</b> a = scalar_mul(val_base, v);
    <b>let</b> b = scalar_mul(rand_base, r);
    <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
        point: add(&a, &b)
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_new_commitment_for_bulletproof"></a>

## Function `new_commitment_for_bulletproof`

Returns a commitment <code>v * G + r * H</code> where <code>G</code> is the Ristretto255 basepoint and <code>H</code> is the default randomness
base used in the Bulletproofs library (i.e., <code><a href="bls12381_pedersen.md#0x1_bls12381_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a></code>).


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_new_commitment_for_bulletproof">new_commitment_for_bulletproof</a>(v: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_Fr">bls12381_algebra::Fr</a>&gt;, r: &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_Fr">bls12381_algebra::Fr</a>&gt;): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_new_commitment_for_bulletproof">new_commitment_for_bulletproof</a>(v: &Element&lt;Fr&gt;, r: &Element&lt;Fr&gt;): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
    <b>let</b> rand_base = deserialize&lt;G1, FormatG1Compr&gt;(&<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>);
    <b>let</b> rand_base = std::option::extract(&<b>mut</b> rand_base);

    <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
        point: multi_scalar_mul(&<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[one&lt;G1&gt;(), rand_base], &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[*v, *r])
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_add"></a>

## Function `commitment_add`

Homomorphically combines two commitments <code>lhs</code> and <code>rhs</code> as <code>lhs + rhs</code>.
Useful for re-randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_add">commitment_add</a>(lhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_add">commitment_add</a>(lhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
    <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
        point: add(&lhs.point, &rhs.point)
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_add_assign"></a>

## Function `commitment_add_assign`

Like <code>commitment_add</code> but assigns <code>lhs = lhs + rhs</code>.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_add_assign">commitment_add_assign</a>(lhs: &<b>mut</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_add_assign">commitment_add_assign</a>(lhs: &<b>mut</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>) {
    lhs.point = add(&lhs.point, &rhs.point);
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_sub"></a>

## Function `commitment_sub`

Homomorphically combines two commitments <code>lhs</code> and <code>rhs</code> as <code>lhs - rhs</code>.
Useful for re-randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_sub">commitment_sub</a>(lhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_sub">commitment_sub</a>(lhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>): <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
    <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> {
        point: sub(&lhs.point, &rhs.point)
    }
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_sub_assign"></a>

## Function `commitment_sub_assign`

Like <code>commitment_add</code> but assigns <code>lhs = lhs - rhs</code>.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_sub_assign">commitment_sub_assign</a>(lhs: &<b>mut</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_sub_assign">commitment_sub_assign</a>(lhs: &<b>mut</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>) {
    lhs.point = sub(&lhs.point, &rhs.point);
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_equals"></a>

## Function `commitment_equals`

Returns true if the two commitments are identical: i.e., same value and same randomness.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_equals">commitment_equals</a>(lhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_equals">commitment_equals</a>(lhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>, rhs: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>): bool {
    eq(&lhs.point, &rhs.point)
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_as_point"></a>

## Function `commitment_as_point`

Returns the underlying elliptic curve point representing the commitment as an in-memory <code>RistrettoPoint</code>.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_as_point">commitment_as_point</a>(c: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>): &<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_as_point">commitment_as_point</a>(c: &<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>): &Element&lt;G1&gt; {
    &c.point
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_commitment_into_point"></a>

## Function `commitment_into_point`

Moves the Commitment into a CompressedRistretto point.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_into_point">commitment_into_point</a>(c: <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">bls12381_pedersen::Commitment</a>): <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_commitment_into_point">commitment_into_point</a>(c: <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a>): Element&lt;G1&gt; {
    <b>let</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_Commitment">Commitment</a> { point } = c;
    point
}
</code></pre>



</details>

<a id="0x1_bls12381_pedersen_randomness_base_for_bulletproof"></a>

## Function `randomness_base_for_bulletproof`

Returns the randomness base compatible with the Bulletproofs module.

Recal that a Bulletproof range proof attests, in zero-knowledge, that a value <code>v</code> inside a Pedersen commitment
<code>v * g + r * h</code> is sufficiently "small" (e.g., is 32-bits wide). Here, <code>h</code> is referred to as the
"randomness base" of the commitment scheme.

Bulletproof has a default choice for <code>g</code> and <code>h</code> and this function returns the default <code>h</code> as used in the
Bulletproofs Move module.


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_randomness_base_for_bulletproof">randomness_base_for_bulletproof</a>(): <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bls12381_pedersen.md#0x1_bls12381_pedersen_randomness_base_for_bulletproof">randomness_base_for_bulletproof</a>(): Element&lt;G1&gt; {
    std::option::extract(&<b>mut</b> deserialize&lt;G1, FormatG1Compr&gt;(&<a href="bls12381_pedersen.md#0x1_bls12381_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
