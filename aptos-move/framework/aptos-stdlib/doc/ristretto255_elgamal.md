
<a id="0x1_ristretto255_elgamal"></a>

# Module `0x1::ristretto255_elgamal`

This module implements an ElGamal encryption API, over the Ristretto255 curve, that can be used with the
Bulletproofs module.

An ElGamal *ciphertext* is an encryption of a value <code>v</code> under a basepoint <code>G</code> and public key <code>Y &#61; sk &#42; G</code>, where <code>sk</code>
is the corresponding secret key, is <code>(v &#42; G &#43; r &#42; Y, r &#42; G)</code>, for a random scalar <code>r</code>.

Note that we place the value <code>v</code> "in the exponent" of <code>G</code> so that ciphertexts are additively homomorphic: i.e., so
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


<pre><code>use 0x1::option;<br/>use 0x1::ristretto255;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_ristretto255_elgamal_Ciphertext"></a>

## Struct `Ciphertext`

An ElGamal ciphertext.


<pre><code>struct Ciphertext has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>left: ristretto255::RistrettoPoint</code>
</dt>
<dd>

</dd>
<dt>
<code>right: ristretto255::RistrettoPoint</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_elgamal_CompressedCiphertext"></a>

## Struct `CompressedCiphertext`

A compressed ElGamal ciphertext.


<pre><code>struct CompressedCiphertext has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>left: ristretto255::CompressedRistretto</code>
</dt>
<dd>

</dd>
<dt>
<code>right: ristretto255::CompressedRistretto</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_elgamal_CompressedPubkey"></a>

## Struct `CompressedPubkey`

An ElGamal public key.


<pre><code>struct CompressedPubkey has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point: ristretto255::CompressedRistretto</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_elgamal_new_pubkey_from_bytes"></a>

## Function `new_pubkey_from_bytes`

Creates a new public key from a serialized Ristretto255 point.


<pre><code>public fun new_pubkey_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255_elgamal::CompressedPubkey&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_pubkey_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;CompressedPubkey&gt; &#123;<br/>    let point &#61; ristretto255::new_compressed_point_from_bytes(bytes);<br/>    if (std::option::is_some(&amp;mut point)) &#123;<br/>        let pk &#61; CompressedPubkey &#123;<br/>            point: std::option::extract(&amp;mut point)<br/>        &#125;;<br/>        std::option::some(pk)<br/>    &#125; else &#123;<br/>        std::option::none&lt;CompressedPubkey&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_pubkey_to_bytes"></a>

## Function `pubkey_to_bytes`

Given an ElGamal public key <code>pubkey</code>, returns the byte representation of that public key.


<pre><code>public fun pubkey_to_bytes(pubkey: &amp;ristretto255_elgamal::CompressedPubkey): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pubkey_to_bytes(pubkey: &amp;CompressedPubkey): vector&lt;u8&gt; &#123;<br/>    ristretto255::compressed_point_to_bytes(pubkey.point)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_pubkey_to_point"></a>

## Function `pubkey_to_point`

Given a public key <code>pubkey</code>, returns the underlying <code>RistrettoPoint</code> representing that key.


<pre><code>public fun pubkey_to_point(pubkey: &amp;ristretto255_elgamal::CompressedPubkey): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pubkey_to_point(pubkey: &amp;CompressedPubkey): RistrettoPoint &#123;<br/>    ristretto255::point_decompress(&amp;pubkey.point)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_pubkey_to_compressed_point"></a>

## Function `pubkey_to_compressed_point`

Given a public key, returns the underlying <code>CompressedRistretto</code> point representing that key.


<pre><code>public fun pubkey_to_compressed_point(pubkey: &amp;ristretto255_elgamal::CompressedPubkey): ristretto255::CompressedRistretto<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pubkey_to_compressed_point(pubkey: &amp;CompressedPubkey): CompressedRistretto &#123;<br/>    pubkey.point<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_new_ciphertext_from_bytes"></a>

## Function `new_ciphertext_from_bytes`

Creates a new ciphertext from two serialized Ristretto255 points: the first 32 bytes store <code>r &#42; G</code> while the
next 32 bytes store <code>v &#42; G &#43; r &#42; Y</code>, where <code>Y</code> is the public key.


<pre><code>public fun new_ciphertext_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255_elgamal::Ciphertext&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_ciphertext_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;Ciphertext&gt; &#123;<br/>    if(vector::length(&amp;bytes) !&#61; 64) &#123;<br/>        return std::option::none&lt;Ciphertext&gt;()<br/>    &#125;;<br/><br/>    let bytes_right &#61; vector::trim(&amp;mut bytes, 32);<br/><br/>    let left_point &#61; ristretto255::new_point_from_bytes(bytes);<br/>    let right_point &#61; ristretto255::new_point_from_bytes(bytes_right);<br/><br/>    if (std::option::is_some&lt;RistrettoPoint&gt;(&amp;mut left_point) &amp;&amp; std::option::is_some&lt;RistrettoPoint&gt;(&amp;mut right_point)) &#123;<br/>        std::option::some&lt;Ciphertext&gt;(Ciphertext &#123;<br/>            left: std::option::extract&lt;RistrettoPoint&gt;(&amp;mut left_point),<br/>            right: std::option::extract&lt;RistrettoPoint&gt;(&amp;mut right_point)<br/>        &#125;)<br/>    &#125; else &#123;<br/>        std::option::none&lt;Ciphertext&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_new_ciphertext_no_randomness"></a>

## Function `new_ciphertext_no_randomness`

Creates a new ciphertext <code>(val &#42; G &#43; 0 &#42; Y, 0 &#42; G) &#61; (val &#42; G, 0 &#42; G)</code> where <code>G</code> is the Ristretto255 basepoint
and the randomness is set to zero.


<pre><code>public fun new_ciphertext_no_randomness(val: &amp;ristretto255::Scalar): ristretto255_elgamal::Ciphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_ciphertext_no_randomness(val: &amp;Scalar): Ciphertext &#123;<br/>    Ciphertext &#123;<br/>        left: ristretto255::basepoint_mul(val),<br/>        right: ristretto255::point_identity(),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_from_points"></a>

## Function `ciphertext_from_points`

Moves a pair of Ristretto points into an ElGamal ciphertext.


<pre><code>public fun ciphertext_from_points(left: ristretto255::RistrettoPoint, right: ristretto255::RistrettoPoint): ristretto255_elgamal::Ciphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_from_points(left: RistrettoPoint, right: RistrettoPoint): Ciphertext &#123;<br/>    Ciphertext &#123;<br/>        left,<br/>        right,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_from_compressed_points"></a>

## Function `ciphertext_from_compressed_points`

Moves a pair of <code>CompressedRistretto</code> points into an ElGamal ciphertext.


<pre><code>public fun ciphertext_from_compressed_points(left: ristretto255::CompressedRistretto, right: ristretto255::CompressedRistretto): ristretto255_elgamal::CompressedCiphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_from_compressed_points(left: CompressedRistretto, right: CompressedRistretto): CompressedCiphertext &#123;<br/>    CompressedCiphertext &#123;<br/>        left,<br/>        right,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_to_bytes"></a>

## Function `ciphertext_to_bytes`

Given a ciphertext <code>ct</code>, serializes that ciphertext into bytes.


<pre><code>public fun ciphertext_to_bytes(ct: &amp;ristretto255_elgamal::Ciphertext): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_to_bytes(ct: &amp;Ciphertext): vector&lt;u8&gt; &#123;<br/>    let bytes_left &#61; ristretto255::point_to_bytes(&amp;ristretto255::point_compress(&amp;ct.left));<br/>    let bytes_right &#61; ristretto255::point_to_bytes(&amp;ristretto255::point_compress(&amp;ct.right));<br/>    let bytes &#61; vector::empty&lt;u8&gt;();<br/>    vector::append&lt;u8&gt;(&amp;mut bytes, bytes_left);<br/>    vector::append&lt;u8&gt;(&amp;mut bytes, bytes_right);<br/>    bytes<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_into_points"></a>

## Function `ciphertext_into_points`

Moves the ciphertext into a pair of <code>RistrettoPoint</code>'s.


<pre><code>public fun ciphertext_into_points(c: ristretto255_elgamal::Ciphertext): (ristretto255::RistrettoPoint, ristretto255::RistrettoPoint)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_into_points(c: Ciphertext): (RistrettoPoint, RistrettoPoint) &#123;<br/>    let Ciphertext &#123; left, right &#125; &#61; c;<br/>    (left, right)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_as_points"></a>

## Function `ciphertext_as_points`

Returns the pair of <code>RistrettoPoint</code>'s representing the ciphertext.


<pre><code>public fun ciphertext_as_points(c: &amp;ristretto255_elgamal::Ciphertext): (&amp;ristretto255::RistrettoPoint, &amp;ristretto255::RistrettoPoint)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_as_points(c: &amp;Ciphertext): (&amp;RistrettoPoint, &amp;RistrettoPoint) &#123;<br/>    (&amp;c.left, &amp;c.right)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_compress_ciphertext"></a>

## Function `compress_ciphertext`

Creates a new compressed ciphertext from a decompressed ciphertext.


<pre><code>public fun compress_ciphertext(ct: &amp;ristretto255_elgamal::Ciphertext): ristretto255_elgamal::CompressedCiphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun compress_ciphertext(ct: &amp;Ciphertext): CompressedCiphertext &#123;<br/>    CompressedCiphertext &#123;<br/>        left: point_compress(&amp;ct.left),<br/>        right: point_compress(&amp;ct.right),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_decompress_ciphertext"></a>

## Function `decompress_ciphertext`

Creates a new decompressed ciphertext from a compressed ciphertext.


<pre><code>public fun decompress_ciphertext(ct: &amp;ristretto255_elgamal::CompressedCiphertext): ristretto255_elgamal::Ciphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun decompress_ciphertext(ct: &amp;CompressedCiphertext): Ciphertext &#123;<br/>    Ciphertext &#123;<br/>        left: ristretto255::point_decompress(&amp;ct.left),<br/>        right: ristretto255::point_decompress(&amp;ct.right),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_add"></a>

## Function `ciphertext_add`

Homomorphically combines two ciphertexts <code>lhs</code> and <code>rhs</code> as <code>lhs &#43; rhs</code>.
Useful for re-randomizing the ciphertext or updating the committed value.


<pre><code>public fun ciphertext_add(lhs: &amp;ristretto255_elgamal::Ciphertext, rhs: &amp;ristretto255_elgamal::Ciphertext): ristretto255_elgamal::Ciphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_add(lhs: &amp;Ciphertext, rhs: &amp;Ciphertext): Ciphertext &#123;<br/>    Ciphertext &#123;<br/>        left: ristretto255::point_add(&amp;lhs.left, &amp;rhs.left),<br/>        right: ristretto255::point_add(&amp;lhs.right, &amp;rhs.right),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_add_assign"></a>

## Function `ciphertext_add_assign`

Like <code>ciphertext_add</code> but assigns <code>lhs &#61; lhs &#43; rhs</code>.


<pre><code>public fun ciphertext_add_assign(lhs: &amp;mut ristretto255_elgamal::Ciphertext, rhs: &amp;ristretto255_elgamal::Ciphertext)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_add_assign(lhs: &amp;mut Ciphertext, rhs: &amp;Ciphertext) &#123;<br/>    ristretto255::point_add_assign(&amp;mut lhs.left, &amp;rhs.left);<br/>    ristretto255::point_add_assign(&amp;mut lhs.right, &amp;rhs.right);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_sub"></a>

## Function `ciphertext_sub`

Homomorphically combines two ciphertexts <code>lhs</code> and <code>rhs</code> as <code>lhs &#45; rhs</code>.
Useful for re-randomizing the ciphertext or updating the committed value.


<pre><code>public fun ciphertext_sub(lhs: &amp;ristretto255_elgamal::Ciphertext, rhs: &amp;ristretto255_elgamal::Ciphertext): ristretto255_elgamal::Ciphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_sub(lhs: &amp;Ciphertext, rhs: &amp;Ciphertext): Ciphertext &#123;<br/>    Ciphertext &#123;<br/>        left: ristretto255::point_sub(&amp;lhs.left, &amp;rhs.left),<br/>        right: ristretto255::point_sub(&amp;lhs.right, &amp;rhs.right),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_sub_assign"></a>

## Function `ciphertext_sub_assign`

Like <code>ciphertext_add</code> but assigns <code>lhs &#61; lhs &#45; rhs</code>.


<pre><code>public fun ciphertext_sub_assign(lhs: &amp;mut ristretto255_elgamal::Ciphertext, rhs: &amp;ristretto255_elgamal::Ciphertext)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_sub_assign(lhs: &amp;mut Ciphertext, rhs: &amp;Ciphertext) &#123;<br/>    ristretto255::point_sub_assign(&amp;mut lhs.left, &amp;rhs.left);<br/>    ristretto255::point_sub_assign(&amp;mut lhs.right, &amp;rhs.right);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_clone"></a>

## Function `ciphertext_clone`

Creates a copy of this ciphertext.


<pre><code>public fun ciphertext_clone(c: &amp;ristretto255_elgamal::Ciphertext): ristretto255_elgamal::Ciphertext<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_clone(c: &amp;Ciphertext): Ciphertext &#123;<br/>    Ciphertext &#123;<br/>        left: ristretto255::point_clone(&amp;c.left),<br/>        right: ristretto255::point_clone(&amp;c.right),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_ciphertext_equals"></a>

## Function `ciphertext_equals`

Returns true if the two ciphertexts are identical: i.e., same value and same randomness.


<pre><code>public fun ciphertext_equals(lhs: &amp;ristretto255_elgamal::Ciphertext, rhs: &amp;ristretto255_elgamal::Ciphertext): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ciphertext_equals(lhs: &amp;Ciphertext, rhs: &amp;Ciphertext): bool &#123;<br/>    ristretto255::point_equals(&amp;lhs.left, &amp;rhs.left) &amp;&amp;<br/>    ristretto255::point_equals(&amp;lhs.right, &amp;rhs.right)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_elgamal_get_value_component"></a>

## Function `get_value_component`

Returns the <code>RistrettoPoint</code> in the ciphertext which contains the encrypted value in the exponent.


<pre><code>public fun get_value_component(ct: &amp;ristretto255_elgamal::Ciphertext): &amp;ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_value_component(ct: &amp;Ciphertext): &amp;RistrettoPoint &#123;<br/>    &amp;ct.left<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
