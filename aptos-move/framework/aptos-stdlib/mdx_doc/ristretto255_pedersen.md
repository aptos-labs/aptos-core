
<a id="0x1_ristretto255_pedersen"></a>

# Module `0x1::ristretto255_pedersen`

This module implements a Pedersen commitment API, over the Ristretto255 curve, that can be used with the
Bulletproofs module.

A Pedersen commitment to a value <code>v</code> under _commitment key_ <code>(g, h)</code> is <code>v &#42; g &#43; r &#42; h</code>, for a random scalar <code>r</code>.


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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;<br /></code></pre>



<a id="0x1_ristretto255_pedersen_Commitment"></a>

## Struct `Commitment`

A Pedersen commitment to some value with some randomness.


<pre><code><b>struct</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> <b>has</b> drop<br /></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ristretto255_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE"></a>

The default Pedersen randomness base <code>h</code> used in our underlying Bulletproofs library.
This is obtained by hashing the compressed Ristretto255 basepoint using SHA3&#45;512 (not SHA2&#45;512).


<pre><code><b>const</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [140, 146, 64, 180, 86, 169, 230, 220, 101, 195, 119, 161, 4, 141, 116, 95, 148, 160, 140, 219, 127, 68, 203, 205, 123, 70, 243, 64, 72, 135, 17, 52];<br /></code></pre>



<a id="0x1_ristretto255_pedersen_new_commitment_from_bytes"></a>

## Function `new_commitment_from_bytes`

Creates a new public key from a serialized Ristretto255 point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment_from_bytes">new_commitment_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment_from_bytes">new_commitment_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>&gt; &#123;<br />    <b>let</b> point &#61; <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes);<br />    <b>if</b> (std::option::is_some(&amp;<b>mut</b> point)) &#123;<br />        <b>let</b> comm &#61; <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />            point: std::option::extract(&amp;<b>mut</b> point)<br />        &#125;;<br />        std::option::some(comm)<br />    &#125; <b>else</b> &#123;<br />        std::option::none&lt;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_to_bytes"></a>

## Function `commitment_to_bytes`

Returns a commitment as a serialized byte array


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_to_bytes">commitment_to_bytes</a>(comm: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_to_bytes">commitment_to_bytes</a>(comm: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&amp;<a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&amp;comm.point))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_from_point"></a>

## Function `commitment_from_point`

Moves a Ristretto point into a Pedersen commitment.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_from_point">commitment_from_point</a>(point: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_from_point">commitment_from_point</a>(point: RistrettoPoint): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_from_compressed"></a>

## Function `commitment_from_compressed`

Deserializes a commitment from a compressed Ristretto point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_from_compressed">commitment_from_compressed</a>(point: &amp;<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_from_compressed">commitment_from_compressed</a>(point: &amp;CompressedRistretto): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(point)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_new_commitment"></a>

## Function `new_commitment`

Returns a commitment <code>v &#42; val_base &#43; r &#42; rand_base</code> where <code>(val_base, rand_base)</code> is the commitment key.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment">new_commitment</a>(v: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, val_base: &amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, r: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand_base: &amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment">new_commitment</a>(v: &amp;Scalar, val_base: &amp;RistrettoPoint, r: &amp;Scalar, rand_base: &amp;RistrettoPoint): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul">ristretto255::double_scalar_mul</a>(v, val_base, r, rand_base)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_new_commitment_with_basepoint"></a>

## Function `new_commitment_with_basepoint`

Returns a commitment <code>v &#42; G &#43; r &#42; rand_base</code> where <code>G</code> is the Ristretto255 basepoint.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment_with_basepoint">new_commitment_with_basepoint</a>(v: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, r: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand_base: &amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment_with_basepoint">new_commitment_with_basepoint</a>(v: &amp;Scalar, r: &amp;Scalar, rand_base: &amp;RistrettoPoint): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">ristretto255::basepoint_double_mul</a>(r, rand_base, v)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_new_commitment_for_bulletproof"></a>

## Function `new_commitment_for_bulletproof`

Returns a commitment <code>v &#42; G &#43; r &#42; H</code> where <code>G</code> is the Ristretto255 basepoint and <code>H</code> is the default randomness
base used in the Bulletproofs library (i.e., <code><a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a></code>).


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment_for_bulletproof">new_commitment_for_bulletproof</a>(v: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, r: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_new_commitment_for_bulletproof">new_commitment_for_bulletproof</a>(v: &amp;Scalar, r: &amp;Scalar): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <b>let</b> rand_base &#61; <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>);<br />    <b>let</b> rand_base &#61; std::option::extract(&amp;<b>mut</b> rand_base);<br /><br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">ristretto255::basepoint_double_mul</a>(r, &amp;rand_base, v)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_add"></a>

## Function `commitment_add`

Homomorphically combines two commitments <code>lhs</code> and <code>rhs</code> as <code>lhs &#43; rhs</code>.
Useful for re&#45;randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_add">commitment_add</a>(lhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_add">commitment_add</a>(lhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&amp;lhs.point, &amp;rhs.point)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_add_assign"></a>

## Function `commitment_add_assign`

Like <code>commitment_add</code> but assigns <code>lhs &#61; lhs &#43; rhs</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_add_assign">commitment_add_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_add_assign">commitment_add_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>) &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&amp;<b>mut</b> lhs.point, &amp;rhs.point);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_sub"></a>

## Function `commitment_sub`

Homomorphically combines two commitments <code>lhs</code> and <code>rhs</code> as <code>lhs &#45; rhs</code>.
Useful for re&#45;randomizing the commitment or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_sub">commitment_sub</a>(lhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_sub">commitment_sub</a>(lhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&amp;lhs.point, &amp;rhs.point)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_sub_assign"></a>

## Function `commitment_sub_assign`

Like <code>commitment_add</code> but assigns <code>lhs &#61; lhs &#45; rhs</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_sub_assign">commitment_sub_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_sub_assign">commitment_sub_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>) &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&amp;<b>mut</b> lhs.point, &amp;rhs.point);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_clone"></a>

## Function `commitment_clone`

Creates a copy of this commitment.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_clone">commitment_clone</a>(c: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_clone">commitment_clone</a>(c: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />    <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123;<br />        point: <a href="ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&amp;c.point)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_equals"></a>

## Function `commitment_equals`

Returns true if the two commitments are identical: i.e., same value and same randomness.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_equals">commitment_equals</a>(lhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_equals">commitment_equals</a>(lhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>, rhs: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): bool &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&amp;lhs.point, &amp;rhs.point)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_as_point"></a>

## Function `commitment_as_point`

Returns the underlying elliptic curve point representing the commitment as an in&#45;memory <code>RistrettoPoint</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_as_point">commitment_as_point</a>(c: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): &amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_as_point">commitment_as_point</a>(c: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): &amp;RistrettoPoint &#123;<br />    &amp;c.point<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_as_compressed_point"></a>

## Function `commitment_as_compressed_point`

Returns the Pedersen commitment as a <code>CompressedRistretto</code> point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_as_compressed_point">commitment_as_compressed_point</a>(c: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_as_compressed_point">commitment_as_compressed_point</a>(c: &amp;<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): CompressedRistretto &#123;<br />    point_compress(&amp;c.point)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_into_point"></a>

## Function `commitment_into_point`

Moves the Commitment into a CompressedRistretto point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_into_point">commitment_into_point</a>(c: <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_into_point">commitment_into_point</a>(c: <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): RistrettoPoint &#123;<br />    <b>let</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a> &#123; point &#125; &#61; c;<br />    point<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_commitment_into_compressed_point"></a>

## Function `commitment_into_compressed_point`

Moves the Commitment into a <code>CompressedRistretto</code> point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_into_compressed_point">commitment_into_compressed_point</a>(c: <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_commitment_into_compressed_point">commitment_into_compressed_point</a>(c: <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">Commitment</a>): CompressedRistretto &#123;<br />    point_compress(&amp;c.point)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_pedersen_randomness_base_for_bulletproof"></a>

## Function `randomness_base_for_bulletproof`

Returns the randomness base compatible with the Bulletproofs module.

Recal that a Bulletproof range proof attests, in zero&#45;knowledge, that a value <code>v</code> inside a Pedersen commitment
<code>v &#42; g &#43; r &#42; h</code> is sufficiently &quot;small&quot; (e.g., is 32&#45;bits wide). Here, <code>h</code> is referred to as the
&quot;randomness base&quot; of the commitment scheme.

Bulletproof has a default choice for <code>g</code> and <code>h</code> and this function returns the default <code>h</code> as used in the
Bulletproofs Move module.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_randomness_base_for_bulletproof">randomness_base_for_bulletproof</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_randomness_base_for_bulletproof">randomness_base_for_bulletproof</a>(): RistrettoPoint &#123;<br />    std::option::extract(&amp;<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(<a href="ristretto255_pedersen.md#0x1_ristretto255_pedersen_BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE">BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE</a>))<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
