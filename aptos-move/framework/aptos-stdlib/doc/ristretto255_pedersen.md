
<a id="0x1_ristretto255_pedersen"></a>

# Module `0x1::ristretto255_pedersen`

This module implements a Pedersen commitment API, over the Ristretto255 curve, that can be used with the<br/> Bulletproofs module.<br/><br/> A Pedersen commitment to a value <code>v</code> under _commitment key_ <code>(g, h)</code> is <code>v &#42; g &#43; r &#42; h</code>, for a random scalar <code>r</code>.


-  [Struct `Commitment`](#0x1_ristretto255_pedersen_Commitment)
-  [Constants](#@Constants_0)
-  [Function `new_commitment_from_bytes`](#0x1_ristretto255_pedersen_new_commitment_from_bytes)
-  [Function `commitment_to_bytes`](#0x1_ristretto255_pedersen_commitment_to_bytes)
-  [Function `commitment_from_point`](#0x1_ristretto255_pedersen_commitment_from_point)
-  [Function `commitment_from_compressed`](#0x1_ristretto255_pedersen_commitment_from_compressed)
-  [Function `new_commitment`](#0x1_ristretto255_pedersen_new_commitment)
-  [Function `new_commitment_with_basepoint`](#0x1_ristretto255_pedersen_new_commitment_with_basepoint)
-  [Function `new_commitment_for_bulletproof`](#0x1_ristretto255_pedersen_new_commitment_for_bulletproof)
-  [Function `commitment_add`](#0x1_ristretto255_pedersen_commitment_add)
-  [Function `commitment_add_assign`](#0x1_ristretto255_pedersen_commitment_add_assign)
-  [Function `commitment_sub`](#0x1_ristretto255_pedersen_commitment_sub)
-  [Function `commitment_sub_assign`](#0x1_ristretto255_pedersen_commitment_sub_assign)
-  [Function `commitment_clone`](#0x1_ristretto255_pedersen_commitment_clone)
-  [Function `commitment_equals`](#0x1_ristretto255_pedersen_commitment_equals)
-  [Function `commitment_as_point`](#0x1_ristretto255_pedersen_commitment_as_point)
-  [Function `commitment_as_compressed_point`](#0x1_ristretto255_pedersen_commitment_as_compressed_point)
-  [Function `commitment_into_point`](#0x1_ristretto255_pedersen_commitment_into_point)
-  [Function `commitment_into_compressed_point`](#0x1_ristretto255_pedersen_commitment_into_compressed_point)
-  [Function `randomness_base_for_bulletproof`](#0x1_ristretto255_pedersen_randomness_base_for_bulletproof)


<pre><code>use 0x1::option;<br/>use 0x1::ristretto255;<br/></code></pre>



<a id="0x1_ristretto255_pedersen_Commitment"></a>

## Struct `Commitment`

A Pedersen commitment to some value with some randomness.


<pre><code>struct Commitment has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point: ristretto255::RistrettoPoint</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ristretto255_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE"></a>

The default Pedersen randomness base <code>h</code> used in our underlying Bulletproofs library.<br/> This is obtained by hashing the compressed Ristretto255 basepoint using SHA3&#45;512 (not SHA2&#45;512).


<pre><code>const BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE: vector&lt;u8&gt; &#61; [140, 146, 64, 180, 86, 169, 230, 220, 101, 195, 119, 161, 4, 141, 116, 95, 148, 160, 140, 219, 127, 68, 203, 205, 123, 70, 243, 64, 72, 135, 17, 52];<br/></code></pre>



<a id="0x1_ristretto255_pedersen_new_commitment_from_bytes"></a>

## Function `new_commitment_from_bytes`

Creates a new public key from a serialized Ristretto255 point.


<pre><code>public fun new_commitment_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255_pedersen::Commitment&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_commitment_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;Commitment&gt; &#123;<br/>    let point &#61; ristretto255::new_point_from_bytes(bytes);<br/>    if (std::option::is_some(&amp;mut point)) &#123;<br/>        let comm &#61; Commitment &#123;<br/>            point: std::option::extract(&amp;mut point)<br/>        &#125;;<br/>        std::option::some(comm)<br/>    &#125; else &#123;<br/>        std::option::none&lt;Commitment&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_to_bytes"></a>

## Function `commitment_to_bytes`

Returns a commitment as a serialized byte array


<pre><code>public fun commitment_to_bytes(comm: &amp;ristretto255_pedersen::Commitment): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_to_bytes(comm: &amp;Commitment): vector&lt;u8&gt; &#123;<br/>    ristretto255::point_to_bytes(&amp;ristretto255::point_compress(&amp;comm.point))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_from_point"></a>

## Function `commitment_from_point`

Moves a Ristretto point into a Pedersen commitment.


<pre><code>public fun commitment_from_point(point: ristretto255::RistrettoPoint): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_from_point(point: RistrettoPoint): Commitment &#123;<br/>    Commitment &#123;<br/>        point<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_from_compressed"></a>

## Function `commitment_from_compressed`

Deserializes a commitment from a compressed Ristretto point.


<pre><code>public fun commitment_from_compressed(point: &amp;ristretto255::CompressedRistretto): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_from_compressed(point: &amp;CompressedRistretto): Commitment &#123;<br/>    Commitment &#123;<br/>        point: ristretto255::point_decompress(point)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_new_commitment"></a>

## Function `new_commitment`

Returns a commitment <code>v &#42; val_base &#43; r &#42; rand_base</code> where <code>(val_base, rand_base)</code> is the commitment key.


<pre><code>public fun new_commitment(v: &amp;ristretto255::Scalar, val_base: &amp;ristretto255::RistrettoPoint, r: &amp;ristretto255::Scalar, rand_base: &amp;ristretto255::RistrettoPoint): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_commitment(v: &amp;Scalar, val_base: &amp;RistrettoPoint, r: &amp;Scalar, rand_base: &amp;RistrettoPoint): Commitment &#123;<br/>    Commitment &#123;<br/>        point: ristretto255::double_scalar_mul(v, val_base, r, rand_base)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_new_commitment_with_basepoint"></a>

## Function `new_commitment_with_basepoint`

Returns a commitment <code>v &#42; G &#43; r &#42; rand_base</code> where <code>G</code> is the Ristretto255 basepoint.


<pre><code>public fun new_commitment_with_basepoint(v: &amp;ristretto255::Scalar, r: &amp;ristretto255::Scalar, rand_base: &amp;ristretto255::RistrettoPoint): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_commitment_with_basepoint(v: &amp;Scalar, r: &amp;Scalar, rand_base: &amp;RistrettoPoint): Commitment &#123;<br/>    Commitment &#123;<br/>        point: ristretto255::basepoint_double_mul(r, rand_base, v)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_new_commitment_for_bulletproof"></a>

## Function `new_commitment_for_bulletproof`

Returns a commitment <code>v &#42; G &#43; r &#42; H</code> where <code>G</code> is the Ristretto255 basepoint and <code>H</code> is the default randomness<br/> base used in the Bulletproofs library (i.e., <code>BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</code>).


<pre><code>public fun new_commitment_for_bulletproof(v: &amp;ristretto255::Scalar, r: &amp;ristretto255::Scalar): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_commitment_for_bulletproof(v: &amp;Scalar, r: &amp;Scalar): Commitment &#123;<br/>    let rand_base &#61; ristretto255::new_point_from_bytes(BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE);<br/>    let rand_base &#61; std::option::extract(&amp;mut rand_base);<br/><br/>    Commitment &#123;<br/>        point: ristretto255::basepoint_double_mul(r, &amp;rand_base, v)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_add"></a>

## Function `commitment_add`

Homomorphically combines two commitments <code>lhs</code> and <code>rhs</code> as <code>lhs &#43; rhs</code>.<br/> Useful for re&#45;randomizing the commitment or updating the committed value.


<pre><code>public fun commitment_add(lhs: &amp;ristretto255_pedersen::Commitment, rhs: &amp;ristretto255_pedersen::Commitment): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_add(lhs: &amp;Commitment, rhs: &amp;Commitment): Commitment &#123;<br/>    Commitment &#123;<br/>        point: ristretto255::point_add(&amp;lhs.point, &amp;rhs.point)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_add_assign"></a>

## Function `commitment_add_assign`

Like <code>commitment_add</code> but assigns <code>lhs &#61; lhs &#43; rhs</code>.


<pre><code>public fun commitment_add_assign(lhs: &amp;mut ristretto255_pedersen::Commitment, rhs: &amp;ristretto255_pedersen::Commitment)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_add_assign(lhs: &amp;mut Commitment, rhs: &amp;Commitment) &#123;<br/>    ristretto255::point_add_assign(&amp;mut lhs.point, &amp;rhs.point);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_sub"></a>

## Function `commitment_sub`

Homomorphically combines two commitments <code>lhs</code> and <code>rhs</code> as <code>lhs &#45; rhs</code>.<br/> Useful for re&#45;randomizing the commitment or updating the committed value.


<pre><code>public fun commitment_sub(lhs: &amp;ristretto255_pedersen::Commitment, rhs: &amp;ristretto255_pedersen::Commitment): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_sub(lhs: &amp;Commitment, rhs: &amp;Commitment): Commitment &#123;<br/>    Commitment &#123;<br/>        point: ristretto255::point_sub(&amp;lhs.point, &amp;rhs.point)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_sub_assign"></a>

## Function `commitment_sub_assign`

Like <code>commitment_add</code> but assigns <code>lhs &#61; lhs &#45; rhs</code>.


<pre><code>public fun commitment_sub_assign(lhs: &amp;mut ristretto255_pedersen::Commitment, rhs: &amp;ristretto255_pedersen::Commitment)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_sub_assign(lhs: &amp;mut Commitment, rhs: &amp;Commitment) &#123;<br/>    ristretto255::point_sub_assign(&amp;mut lhs.point, &amp;rhs.point);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_clone"></a>

## Function `commitment_clone`

Creates a copy of this commitment.


<pre><code>public fun commitment_clone(c: &amp;ristretto255_pedersen::Commitment): ristretto255_pedersen::Commitment<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_clone(c: &amp;Commitment): Commitment &#123;<br/>    Commitment &#123;<br/>        point: ristretto255::point_clone(&amp;c.point)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_equals"></a>

## Function `commitment_equals`

Returns true if the two commitments are identical: i.e., same value and same randomness.


<pre><code>public fun commitment_equals(lhs: &amp;ristretto255_pedersen::Commitment, rhs: &amp;ristretto255_pedersen::Commitment): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_equals(lhs: &amp;Commitment, rhs: &amp;Commitment): bool &#123;<br/>    ristretto255::point_equals(&amp;lhs.point, &amp;rhs.point)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_as_point"></a>

## Function `commitment_as_point`

Returns the underlying elliptic curve point representing the commitment as an in&#45;memory <code>RistrettoPoint</code>.


<pre><code>public fun commitment_as_point(c: &amp;ristretto255_pedersen::Commitment): &amp;ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_as_point(c: &amp;Commitment): &amp;RistrettoPoint &#123;<br/>    &amp;c.point<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_as_compressed_point"></a>

## Function `commitment_as_compressed_point`

Returns the Pedersen commitment as a <code>CompressedRistretto</code> point.


<pre><code>public fun commitment_as_compressed_point(c: &amp;ristretto255_pedersen::Commitment): ristretto255::CompressedRistretto<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_as_compressed_point(c: &amp;Commitment): CompressedRistretto &#123;<br/>    point_compress(&amp;c.point)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_into_point"></a>

## Function `commitment_into_point`

Moves the Commitment into a CompressedRistretto point.


<pre><code>public fun commitment_into_point(c: ristretto255_pedersen::Commitment): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_into_point(c: Commitment): RistrettoPoint &#123;<br/>    let Commitment &#123; point &#125; &#61; c;<br/>    point<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_into_compressed_point"></a>

## Function `commitment_into_compressed_point`

Moves the Commitment into a <code>CompressedRistretto</code> point.


<pre><code>public fun commitment_into_compressed_point(c: ristretto255_pedersen::Commitment): ristretto255::CompressedRistretto<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commitment_into_compressed_point(c: Commitment): CompressedRistretto &#123;<br/>    point_compress(&amp;c.point)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_randomness_base_for_bulletproof"></a>

## Function `randomness_base_for_bulletproof`

Returns the randomness base compatible with the Bulletproofs module.<br/><br/> Recal that a Bulletproof range proof attests, in zero&#45;knowledge, that a value <code>v</code> inside a Pedersen commitment<br/> <code>v &#42; g &#43; r &#42; h</code> is sufficiently &quot;small&quot; (e.g., is 32&#45;bits wide). Here, <code>h</code> is referred to as the<br/> &quot;randomness base&quot; of the commitment scheme.<br/><br/> Bulletproof has a default choice for <code>g</code> and <code>h</code> and this function returns the default <code>h</code> as used in the<br/> Bulletproofs Move module.


<pre><code>public fun randomness_base_for_bulletproof(): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun randomness_base_for_bulletproof(): RistrettoPoint &#123;<br/>    std::option::extract(&amp;mut ristretto255::new_point_from_bytes(BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE))<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
