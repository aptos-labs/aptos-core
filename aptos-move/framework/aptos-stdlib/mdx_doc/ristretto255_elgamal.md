
<a id="0x1_ristretto255_elgamal"></a>

# Module `0x1::ristretto255_elgamal`

This module implements an ElGamal encryption API, over the Ristretto255 curve, that can be used with the
Bulletproofs module.

An ElGamal &#42;ciphertext&#42; is an encryption of a value <code>v</code> under a basepoint <code>G</code> and public key <code>Y &#61; sk &#42; G</code>, where <code>sk</code>
is the corresponding secret key, is <code>(v &#42; G &#43; r &#42; Y, r &#42; G)</code>, for a random scalar <code>r</code>.

Note that we place the value <code>v</code> &quot;in the exponent&quot; of <code>G</code> so that ciphertexts are additively homomorphic: i.e., so
that <code>Enc_Y(v, r) &#43; Enc_Y(v&apos;, r&apos;) &#61; Enc_Y(v &#43; v&apos;, r &#43; r&apos;)</code> where <code>v, v&apos;</code> are plaintext messages, <code>Y</code> is a public key and <code>r, r&apos;</code>
are the randomness of the ciphertexts.


-  [Struct `Ciphertext`](#0x1_ristretto255_elgamal_Ciphertext)
-  [Struct `CompressedCiphertext`](#0x1_ristretto255_elgamal_CompressedCiphertext)
-  [Struct `CompressedPubkey`](#0x1_ristretto255_elgamal_CompressedPubkey)
-  [Function `new_pubkey_from_bytes`](#0x1_ristretto255_elgamal_new_pubkey_from_bytes)
-  [Function `pubkey_to_bytes`](#0x1_ristretto255_elgamal_pubkey_to_bytes)
-  [Function `pubkey_to_point`](#0x1_ristretto255_elgamal_pubkey_to_point)
-  [Function `pubkey_to_compressed_point`](#0x1_ristretto255_elgamal_pubkey_to_compressed_point)
-  [Function `new_ciphertext_from_bytes`](#0x1_ristretto255_elgamal_new_ciphertext_from_bytes)
-  [Function `new_ciphertext_no_randomness`](#0x1_ristretto255_elgamal_new_ciphertext_no_randomness)
-  [Function `ciphertext_from_points`](#0x1_ristretto255_elgamal_ciphertext_from_points)
-  [Function `ciphertext_from_compressed_points`](#0x1_ristretto255_elgamal_ciphertext_from_compressed_points)
-  [Function `ciphertext_to_bytes`](#0x1_ristretto255_elgamal_ciphertext_to_bytes)
-  [Function `ciphertext_into_points`](#0x1_ristretto255_elgamal_ciphertext_into_points)
-  [Function `ciphertext_as_points`](#0x1_ristretto255_elgamal_ciphertext_as_points)
-  [Function `compress_ciphertext`](#0x1_ristretto255_elgamal_compress_ciphertext)
-  [Function `decompress_ciphertext`](#0x1_ristretto255_elgamal_decompress_ciphertext)
-  [Function `ciphertext_add`](#0x1_ristretto255_elgamal_ciphertext_add)
-  [Function `ciphertext_add_assign`](#0x1_ristretto255_elgamal_ciphertext_add_assign)
-  [Function `ciphertext_sub`](#0x1_ristretto255_elgamal_ciphertext_sub)
-  [Function `ciphertext_sub_assign`](#0x1_ristretto255_elgamal_ciphertext_sub_assign)
-  [Function `ciphertext_clone`](#0x1_ristretto255_elgamal_ciphertext_clone)
-  [Function `ciphertext_equals`](#0x1_ristretto255_elgamal_ciphertext_equals)
-  [Function `get_value_component`](#0x1_ristretto255_elgamal_get_value_component)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_ristretto255_elgamal_Ciphertext"></a>

## Struct `Ciphertext`

An ElGamal ciphertext.


<pre><code><b>struct</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> <b>has</b> drop<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>left: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>right: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_elgamal_CompressedCiphertext"></a>

## Struct `CompressedCiphertext`

A compressed ElGamal ciphertext.


<pre><code><b>struct</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">CompressedCiphertext</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>left: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>right: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_elgamal_CompressedPubkey"></a>

## Struct `CompressedPubkey`

An ElGamal public key.


<pre><code><b>struct</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_elgamal_new_pubkey_from_bytes"></a>

## Function `new_pubkey_from_bytes`

Creates a new public key from a serialized Ristretto255 point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_new_pubkey_from_bytes">new_pubkey_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_new_pubkey_from_bytes">new_pubkey_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a>&gt; &#123;<br />    <b>let</b> point &#61; <a href="ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(bytes);<br />    <b>if</b> (std::option::is_some(&amp;<b>mut</b> point)) &#123;<br />        <b>let</b> pk &#61; <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a> &#123;<br />            point: std::option::extract(&amp;<b>mut</b> point)<br />        &#125;;<br />        std::option::some(pk)<br />    &#125; <b>else</b> &#123;<br />        std::option::none&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a>&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_pubkey_to_bytes"></a>

## Function `pubkey_to_bytes`

Given an ElGamal public key <code>pubkey</code>, returns the byte representation of that public key.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_pubkey_to_bytes">pubkey_to_bytes</a>(pubkey: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_pubkey_to_bytes">pubkey_to_bytes</a>(pubkey: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(pubkey.point)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_pubkey_to_point"></a>

## Function `pubkey_to_point`

Given a public key <code>pubkey</code>, returns the underlying <code>RistrettoPoint</code> representing that key.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_pubkey_to_point">pubkey_to_point</a>(pubkey: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_pubkey_to_point">pubkey_to_point</a>(pubkey: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a>): RistrettoPoint &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&amp;pubkey.point)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_pubkey_to_compressed_point"></a>

## Function `pubkey_to_compressed_point`

Given a public key, returns the underlying <code>CompressedRistretto</code> point representing that key.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_pubkey_to_compressed_point">pubkey_to_compressed_point</a>(pubkey: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_pubkey_to_compressed_point">pubkey_to_compressed_point</a>(pubkey: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">CompressedPubkey</a>): CompressedRistretto &#123;<br />    pubkey.point<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_new_ciphertext_from_bytes"></a>

## Function `new_ciphertext_from_bytes`

Creates a new ciphertext from two serialized Ristretto255 points: the first 32 bytes store <code>r &#42; G</code> while the
next 32 bytes store <code>v &#42; G &#43; r &#42; Y</code>, where <code>Y</code> is the public key.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_new_ciphertext_from_bytes">new_ciphertext_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_new_ciphertext_from_bytes">new_ciphertext_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>&gt; &#123;<br />    <b>if</b>(<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bytes) !&#61; 64) &#123;<br />        <b>return</b> std::option::none&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>&gt;()<br />    &#125;;<br /><br />    <b>let</b> bytes_right &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_trim">vector::trim</a>(&amp;<b>mut</b> bytes, 32);<br /><br />    <b>let</b> left_point &#61; <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes);<br />    <b>let</b> right_point &#61; <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes_right);<br /><br />    <b>if</b> (std::option::is_some&lt;RistrettoPoint&gt;(&amp;<b>mut</b> left_point) &amp;&amp; std::option::is_some&lt;RistrettoPoint&gt;(&amp;<b>mut</b> right_point)) &#123;<br />        std::option::some&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>&gt;(<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />            left: std::option::extract&lt;RistrettoPoint&gt;(&amp;<b>mut</b> left_point),<br />            right: std::option::extract&lt;RistrettoPoint&gt;(&amp;<b>mut</b> right_point)<br />        &#125;)<br />    &#125; <b>else</b> &#123;<br />        std::option::none&lt;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_new_ciphertext_no_randomness"></a>

## Function `new_ciphertext_no_randomness`

Creates a new ciphertext <code>(val &#42; G &#43; 0 &#42; Y, 0 &#42; G) &#61; (val &#42; G, 0 &#42; G)</code> where <code>G</code> is the Ristretto255 basepoint
and the randomness is set to zero.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_new_ciphertext_no_randomness">new_ciphertext_no_randomness</a>(val: &amp;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_new_ciphertext_no_randomness">new_ciphertext_no_randomness</a>(val: &amp;Scalar): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />        left: <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(val),<br />        right: <a href="ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>(),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_from_points"></a>

## Function `ciphertext_from_points`

Moves a pair of Ristretto points into an ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_from_points">ciphertext_from_points</a>(left: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, right: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_from_points">ciphertext_from_points</a>(left: RistrettoPoint, right: RistrettoPoint): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />        left,<br />        right,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_from_compressed_points"></a>

## Function `ciphertext_from_compressed_points`

Moves a pair of <code>CompressedRistretto</code> points into an ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_from_compressed_points">ciphertext_from_compressed_points</a>(left: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, right: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_from_compressed_points">ciphertext_from_compressed_points</a>(left: CompressedRistretto, right: CompressedRistretto): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">CompressedCiphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">CompressedCiphertext</a> &#123;<br />        left,<br />        right,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_to_bytes"></a>

## Function `ciphertext_to_bytes`

Given a ciphertext <code>ct</code>, serializes that ciphertext into bytes.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_to_bytes">ciphertext_to_bytes</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_to_bytes">ciphertext_to_bytes</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>let</b> bytes_left &#61; <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&amp;<a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&amp;ct.left));<br />    <b>let</b> bytes_right &#61; <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&amp;<a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&amp;ct.right));<br />    <b>let</b> bytes &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>&lt;u8&gt;(&amp;<b>mut</b> bytes, bytes_left);<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>&lt;u8&gt;(&amp;<b>mut</b> bytes, bytes_right);<br />    bytes<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_into_points"></a>

## Function `ciphertext_into_points`

Moves the ciphertext into a pair of <code>RistrettoPoint</code>&apos;s.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_into_points">ciphertext_into_points</a>(c: <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): (<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_into_points">ciphertext_into_points</a>(c: <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): (RistrettoPoint, RistrettoPoint) &#123;<br />    <b>let</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123; left, right &#125; &#61; c;<br />    (left, right)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_as_points"></a>

## Function `ciphertext_as_points`

Returns the pair of <code>RistrettoPoint</code>&apos;s representing the ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_as_points">ciphertext_as_points</a>(c: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): (&amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, &amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_as_points">ciphertext_as_points</a>(c: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): (&amp;RistrettoPoint, &amp;RistrettoPoint) &#123;<br />    (&amp;c.left, &amp;c.right)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_compress_ciphertext"></a>

## Function `compress_ciphertext`

Creates a new compressed ciphertext from a decompressed ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_compress_ciphertext">compress_ciphertext</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_compress_ciphertext">compress_ciphertext</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">CompressedCiphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">CompressedCiphertext</a> &#123;<br />        left: point_compress(&amp;ct.left),<br />        right: point_compress(&amp;ct.right),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_decompress_ciphertext"></a>

## Function `decompress_ciphertext`

Creates a new decompressed ciphertext from a compressed ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_decompress_ciphertext">decompress_ciphertext</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_decompress_ciphertext">decompress_ciphertext</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">CompressedCiphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />        left: <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&amp;ct.left),<br />        right: <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&amp;ct.right),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_add"></a>

## Function `ciphertext_add`

Homomorphically combines two ciphertexts <code>lhs</code> and <code>rhs</code> as <code>lhs &#43; rhs</code>.
Useful for re&#45;randomizing the ciphertext or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_add">ciphertext_add</a>(lhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_add">ciphertext_add</a>(lhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />        left: <a href="ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&amp;lhs.left, &amp;rhs.left),<br />        right: <a href="ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&amp;lhs.right, &amp;rhs.right),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_add_assign"></a>

## Function `ciphertext_add_assign`

Like <code>ciphertext_add</code> but assigns <code>lhs &#61; lhs &#43; rhs</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_add_assign">ciphertext_add_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_add_assign">ciphertext_add_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>) &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&amp;<b>mut</b> lhs.left, &amp;rhs.left);<br />    <a href="ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&amp;<b>mut</b> lhs.right, &amp;rhs.right);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_sub"></a>

## Function `ciphertext_sub`

Homomorphically combines two ciphertexts <code>lhs</code> and <code>rhs</code> as <code>lhs &#45; rhs</code>.
Useful for re&#45;randomizing the ciphertext or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_sub">ciphertext_sub</a>(lhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_sub">ciphertext_sub</a>(lhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />        left: <a href="ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&amp;lhs.left, &amp;rhs.left),<br />        right: <a href="ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&amp;lhs.right, &amp;rhs.right),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_sub_assign"></a>

## Function `ciphertext_sub_assign`

Like <code>ciphertext_add</code> but assigns <code>lhs &#61; lhs &#45; rhs</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_sub_assign">ciphertext_sub_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_sub_assign">ciphertext_sub_assign</a>(lhs: &amp;<b>mut</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>) &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&amp;<b>mut</b> lhs.left, &amp;rhs.left);<br />    <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&amp;<b>mut</b> lhs.right, &amp;rhs.right);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_clone"></a>

## Function `ciphertext_clone`

Creates a copy of this ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_clone">ciphertext_clone</a>(c: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_clone">ciphertext_clone</a>(c: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />    <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a> &#123;<br />        left: <a href="ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&amp;c.left),<br />        right: <a href="ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&amp;c.right),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_equals"></a>

## Function `ciphertext_equals`

Returns true if the two ciphertexts are identical: i.e., same value and same randomness.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_equals">ciphertext_equals</a>(lhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_ciphertext_equals">ciphertext_equals</a>(lhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>, rhs: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): bool &#123;<br />    <a href="ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&amp;lhs.left, &amp;rhs.left) &amp;&amp;<br />    <a href="ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&amp;lhs.right, &amp;rhs.right)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_get_value_component"></a>

## Function `get_value_component`

Returns the <code>RistrettoPoint</code> in the ciphertext which contains the encrypted value in the exponent.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_get_value_component">get_value_component</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>): &amp;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_get_value_component">get_value_component</a>(ct: &amp;<a href="ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">Ciphertext</a>): &amp;RistrettoPoint &#123;<br />    &amp;ct.left<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
