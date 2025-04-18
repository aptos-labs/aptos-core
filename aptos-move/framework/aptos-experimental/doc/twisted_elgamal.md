
<a id="0x7_twisted_elgamal"></a>

# Module `0x7::twisted_elgamal`

This module implements a Twisted ElGamal encryption API, over the Ristretto255 curve, designed to work with
additional cryptographic constructs such as Bulletproofs.

A Twisted ElGamal *ciphertext* encrypts a value <code>v</code> under a basepoint <code>G</code> and a secondary point <code>H</code>,
alongside a public key <code>Y = sk^(-1) * H</code>, where <code>sk</code> is the corresponding secret key. The ciphertext is of the form:
<code>(v * G + r * H, r * Y)</code>, where <code>r</code> is a random scalar.

The Twisted ElGamal scheme differs from standard ElGamal by introducing a secondary point <code>H</code> to enhance
flexibility and functionality in cryptographic protocols. This design still maintains the homomorphic property:
<code>Enc_Y(v, r) + Enc_Y(v', r') = Enc_Y(v + v', r + r')</code>, where <code>v, v'</code> are plaintexts, <code>Y</code> is the public key,
and <code>r, r'</code> are random scalars.


-  [Struct `Ciphertext`](#0x7_twisted_elgamal_Ciphertext)
-  [Struct `CompressedCiphertext`](#0x7_twisted_elgamal_CompressedCiphertext)
-  [Struct `CompressedPubkey`](#0x7_twisted_elgamal_CompressedPubkey)
-  [Function `new_pubkey_from_bytes`](#0x7_twisted_elgamal_new_pubkey_from_bytes)
-  [Function `pubkey_to_bytes`](#0x7_twisted_elgamal_pubkey_to_bytes)
-  [Function `pubkey_to_point`](#0x7_twisted_elgamal_pubkey_to_point)
-  [Function `pubkey_to_compressed_point`](#0x7_twisted_elgamal_pubkey_to_compressed_point)
-  [Function `new_ciphertext_from_bytes`](#0x7_twisted_elgamal_new_ciphertext_from_bytes)
-  [Function `new_ciphertext_no_randomness`](#0x7_twisted_elgamal_new_ciphertext_no_randomness)
-  [Function `ciphertext_from_points`](#0x7_twisted_elgamal_ciphertext_from_points)
-  [Function `ciphertext_from_compressed_points`](#0x7_twisted_elgamal_ciphertext_from_compressed_points)
-  [Function `ciphertext_to_bytes`](#0x7_twisted_elgamal_ciphertext_to_bytes)
-  [Function `ciphertext_into_points`](#0x7_twisted_elgamal_ciphertext_into_points)
-  [Function `ciphertext_as_points`](#0x7_twisted_elgamal_ciphertext_as_points)
-  [Function `compress_ciphertext`](#0x7_twisted_elgamal_compress_ciphertext)
-  [Function `decompress_ciphertext`](#0x7_twisted_elgamal_decompress_ciphertext)
-  [Function `ciphertext_add`](#0x7_twisted_elgamal_ciphertext_add)
-  [Function `ciphertext_add_assign`](#0x7_twisted_elgamal_ciphertext_add_assign)
-  [Function `ciphertext_sub`](#0x7_twisted_elgamal_ciphertext_sub)
-  [Function `ciphertext_sub_assign`](#0x7_twisted_elgamal_ciphertext_sub_assign)
-  [Function `ciphertext_clone`](#0x7_twisted_elgamal_ciphertext_clone)
-  [Function `ciphertext_equals`](#0x7_twisted_elgamal_ciphertext_equals)
-  [Function `get_value_component`](#0x7_twisted_elgamal_get_value_component)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x7_twisted_elgamal_Ciphertext"></a>

## Struct `Ciphertext`

A Twisted ElGamal ciphertext, consisting of two Ristretto255 points.


<pre><code><b>struct</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_twisted_elgamal_CompressedCiphertext"></a>

## Struct `CompressedCiphertext`

A compressed Twisted ElGamal ciphertext, consisting of two compressed Ristretto255 points.


<pre><code><b>struct</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_twisted_elgamal_CompressedPubkey"></a>

## Struct `CompressedPubkey`

A Twisted ElGamal public key, represented as a compressed Ristretto255 point.


<pre><code><b>struct</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>point: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_twisted_elgamal_new_pubkey_from_bytes"></a>

## Function `new_pubkey_from_bytes`

Creates a new public key from a serialized Ristretto255 point.
Returns <code>Some(<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a>)</code> if the deserialization is successful, otherwise <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_new_pubkey_from_bytes">new_pubkey_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">twisted_elgamal::CompressedPubkey</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_new_pubkey_from_bytes">new_pubkey_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a>&gt; {
    <b>let</b> point = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(bytes);
    <b>if</b> (point.is_some()) {
        <b>let</b> pk = <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a> {
            point: point.extract()
        };
        std::option::some(pk)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_pubkey_to_bytes"></a>

## Function `pubkey_to_bytes`

Serializes a Twisted ElGamal public key into its byte representation.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_pubkey_to_bytes">pubkey_to_bytes</a>(pubkey: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">twisted_elgamal::CompressedPubkey</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_pubkey_to_bytes">pubkey_to_bytes</a>(pubkey: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(pubkey.point)
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_pubkey_to_point"></a>

## Function `pubkey_to_point`

Converts a public key into its corresponding <code>RistrettoPoint</code>.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_pubkey_to_point">pubkey_to_point</a>(pubkey: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">twisted_elgamal::CompressedPubkey</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_pubkey_to_point">pubkey_to_point</a>(pubkey: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a>): RistrettoPoint {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&pubkey.point)
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_pubkey_to_compressed_point"></a>

## Function `pubkey_to_compressed_point`

Converts a public key into its corresponding <code>CompressedRistretto</code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_pubkey_to_compressed_point">pubkey_to_compressed_point</a>(pubkey: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">twisted_elgamal::CompressedPubkey</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_pubkey_to_compressed_point">pubkey_to_compressed_point</a>(pubkey: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedPubkey">CompressedPubkey</a>): CompressedRistretto {
    pubkey.point
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_new_ciphertext_from_bytes"></a>

## Function `new_ciphertext_from_bytes`

Creates a new ciphertext from a serialized representation, consisting of two 32-byte Ristretto255 points.
Returns <code>Some(<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>)</code> if the deserialization succeeds, otherwise <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_new_ciphertext_from_bytes">new_ciphertext_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_new_ciphertext_from_bytes">new_ciphertext_from_bytes</a>(bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>&gt; {
    <b>if</b> (bytes.length() != 64) {
        <b>return</b> std::option::none()
    };

    <b>let</b> bytes_right = bytes.trim(32);

    <b>let</b> left_point = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes);
    <b>let</b> right_point = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes_right);

    <b>if</b> (left_point.is_some() && right_point.is_some()) {
        std::option::some(<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
            left: left_point.extract(),
            right: right_point.extract()
        })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_new_ciphertext_no_randomness"></a>

## Function `new_ciphertext_no_randomness`

Creates a ciphertext <code>(val * G, 0 * G)</code> where <code>val</code> is the plaintext, and the randomness is set to zero.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_new_ciphertext_no_randomness">new_ciphertext_no_randomness</a>(val: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_new_ciphertext_no_randomness">new_ciphertext_no_randomness</a>(val: &Scalar): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(val),
        right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>(),
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_from_points"></a>

## Function `ciphertext_from_points`

Constructs a Twisted ElGamal ciphertext from two <code>RistrettoPoint</code>s.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_from_points">ciphertext_from_points</a>(left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_from_points">ciphertext_from_points</a>(left: RistrettoPoint, right: RistrettoPoint): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
        left,
        right,
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_from_compressed_points"></a>

## Function `ciphertext_from_compressed_points`

Constructs a Twisted ElGamal ciphertext from two compressed Ristretto255 points.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_from_compressed_points">ciphertext_from_compressed_points</a>(left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">twisted_elgamal::CompressedCiphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_from_compressed_points">ciphertext_from_compressed_points</a>(
    left: CompressedRistretto,
    right: CompressedRistretto
): <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
        left,
        right,
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_to_bytes"></a>

## Function `ciphertext_to_bytes`

Serializes a Twisted ElGamal ciphertext into its byte representation.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_to_bytes">ciphertext_to_bytes</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_to_bytes">ciphertext_to_bytes</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.left));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.right)));
    bytes
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_into_points"></a>

## Function `ciphertext_into_points`

Converts a ciphertext into a pair of <code>RistrettoPoint</code>s.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_into_points">ciphertext_into_points</a>(c: <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): (<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_into_points">ciphertext_into_points</a>(c: <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): (RistrettoPoint, RistrettoPoint) {
    <b>let</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> { left, right } = c;
    (left, right)
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_as_points"></a>

## Function `ciphertext_as_points`

Returns the two <code>RistrettoPoint</code>s representing the ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_as_points">ciphertext_as_points</a>(c: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): (&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_as_points">ciphertext_as_points</a>(c: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): (&RistrettoPoint, &RistrettoPoint) {
    (&c.left, &c.right)
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_compress_ciphertext"></a>

## Function `compress_ciphertext`

Compresses a Twisted ElGamal ciphertext into its <code><a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_compress_ciphertext">compress_ciphertext</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">twisted_elgamal::CompressedCiphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_compress_ciphertext">compress_ciphertext</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
        left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.left),
        right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.right),
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_decompress_ciphertext"></a>

## Function `decompress_ciphertext`

Decompresses a <code><a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a></code> back into its <code><a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a></code> representation.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_decompress_ciphertext">decompress_ciphertext</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">twisted_elgamal::CompressedCiphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_decompress_ciphertext">decompress_ciphertext</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_CompressedCiphertext">CompressedCiphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&ct.left),
        right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&ct.right),
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_add"></a>

## Function `ciphertext_add`

Adds two ciphertexts homomorphically, producing a new ciphertext representing the sum of the two.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_add">ciphertext_add</a>(lhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_add">ciphertext_add</a>(lhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&lhs.left, &rhs.left),
        right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&lhs.right, &rhs.right),
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_add_assign"></a>

## Function `ciphertext_add_assign`

Adds two ciphertexts homomorphically, updating the first ciphertext in place.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_add_assign">ciphertext_add_assign</a>(lhs: &<b>mut</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_add_assign">ciphertext_add_assign</a>(lhs: &<b>mut</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>) {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.left, &rhs.left);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.right, &rhs.right);
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_sub"></a>

## Function `ciphertext_sub`

Subtracts one ciphertext from another homomorphically, producing a new ciphertext representing the difference.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_sub">ciphertext_sub</a>(lhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_sub">ciphertext_sub</a>(lhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&lhs.left, &rhs.left),
        right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&lhs.right, &rhs.right),
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_sub_assign"></a>

## Function `ciphertext_sub_assign`

Subtracts one ciphertext from another homomorphically, updating the first ciphertext in place.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_sub_assign">ciphertext_sub_assign</a>(lhs: &<b>mut</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_sub_assign">ciphertext_sub_assign</a>(lhs: &<b>mut</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>) {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&<b>mut</b> lhs.left, &rhs.left);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&<b>mut</b> lhs.right, &rhs.right);
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_clone"></a>

## Function `ciphertext_clone`

Creates a copy of the provided ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_clone">ciphertext_clone</a>(c: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_clone">ciphertext_clone</a>(c: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
    <a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&c.left),
        right: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&c.right),
    }
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_ciphertext_equals"></a>

## Function `ciphertext_equals`

Compares two ciphertexts for equality, returning <code><b>true</b></code> if they encrypt the same value and randomness.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_equals">ciphertext_equals</a>(lhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_ciphertext_equals">ciphertext_equals</a>(lhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): bool {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs.left, &rhs.left) &&
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs.right, &rhs.right)
}
</code></pre>



</details>

<a id="0x7_twisted_elgamal_get_value_component"></a>

## Function `get_value_component`

Returns the <code>RistrettoPoint</code> in the ciphertext that contains the encrypted value in the exponent.


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_get_value_component">get_value_component</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">twisted_elgamal::Ciphertext</a>): &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="twisted_elgamal.md#0x7_twisted_elgamal_get_value_component">get_value_component</a>(ct: &<a href="twisted_elgamal.md#0x7_twisted_elgamal_Ciphertext">Ciphertext</a>): &RistrettoPoint {
    &ct.left
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
