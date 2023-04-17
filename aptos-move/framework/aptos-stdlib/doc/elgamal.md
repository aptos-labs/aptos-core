
<a name="0x1_elgamal"></a>

# Module `0x1::elgamal`

This module implements an ElGamal encryption API that can be used with the Bulletproofs module.

An ElGamal encryption of a value v under a generator g and public key y is (v * g + r * y, r * g), for a random scalar r.
Note we place the value v in the exponent of g so that ciphertexts are additively homomorphic, so that Enc(v,y) + Enc(v',y) = Enc(v+v', y) where v,v' are encrypted messages, y is a public key, and the same randomness is used across both encryptions.


-  [Struct `Ciphertext`](#0x1_elgamal_Ciphertext)
-  [Struct `CompressedCiphertext`](#0x1_elgamal_CompressedCiphertext)
-  [Struct `Pubkey`](#0x1_elgamal_Pubkey)
-  [Constants](#@Constants_0)
-  [Function `get_point_from_pubkey`](#0x1_elgamal_get_point_from_pubkey)
-  [Function `get_pubkey_from_scalar`](#0x1_elgamal_get_pubkey_from_scalar)
-  [Function `get_compressed_point_from_pubkey`](#0x1_elgamal_get_compressed_point_from_pubkey)
-  [Function `new_pubkey_from_bytes`](#0x1_elgamal_new_pubkey_from_bytes)
-  [Function `pubkey_to_bytes`](#0x1_elgamal_pubkey_to_bytes)
-  [Function `new_ciphertext_from_bytes`](#0x1_elgamal_new_ciphertext_from_bytes)
-  [Function `ciphertext_to_bytes`](#0x1_elgamal_ciphertext_to_bytes)
-  [Function `new_ciphertext_from_points`](#0x1_elgamal_new_ciphertext_from_points)
-  [Function `new_ciphertext_from_compressed`](#0x1_elgamal_new_ciphertext_from_compressed)
-  [Function `new_ciphertext_no_randomness`](#0x1_elgamal_new_ciphertext_no_randomness)
-  [Function `compress_ciphertext`](#0x1_elgamal_compress_ciphertext)
-  [Function `decompress_ciphertext`](#0x1_elgamal_decompress_ciphertext)
-  [Function `new_ciphertext`](#0x1_elgamal_new_ciphertext)
-  [Function `new_ciphertext_with_basepoint`](#0x1_elgamal_new_ciphertext_with_basepoint)
-  [Function `ciphertext_add`](#0x1_elgamal_ciphertext_add)
-  [Function `ciphertext_add_assign`](#0x1_elgamal_ciphertext_add_assign)
-  [Function `ciphertext_sub`](#0x1_elgamal_ciphertext_sub)
-  [Function `ciphertext_sub_assign`](#0x1_elgamal_ciphertext_sub_assign)
-  [Function `ciphertext_clone`](#0x1_elgamal_ciphertext_clone)
-  [Function `ciphertext_equals`](#0x1_elgamal_ciphertext_equals)
-  [Function `ciphertext_as_points`](#0x1_elgamal_ciphertext_as_points)
-  [Function `ciphertext_as_compressed_points`](#0x1_elgamal_ciphertext_as_compressed_points)
-  [Function `ciphertext_into_points`](#0x1_elgamal_ciphertext_into_points)
-  [Function `ciphertext_into_compressed_points`](#0x1_elgamal_ciphertext_into_compressed_points)
-  [Function `get_value_component`](#0x1_elgamal_get_value_component)
-  [Function `get_value_component_compressed`](#0x1_elgamal_get_value_component_compressed)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_elgamal_Ciphertext"></a>

## Struct `Ciphertext`

An ElGamal ciphertext to some value.


<pre><code><b>struct</b> <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> <b>has</b> drop
</code></pre>



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

<a name="0x1_elgamal_CompressedCiphertext"></a>

## Struct `CompressedCiphertext`

A compressed ElGamal ciphertext to some value.


<pre><code><b>struct</b> <a href="elgamal.md#0x1_elgamal_CompressedCiphertext">CompressedCiphertext</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



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

<a name="0x1_elgamal_Pubkey"></a>

## Struct `Pubkey`

An ElGamal public key.


<pre><code><b>struct</b> <a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_elgamal_EWRONG_BYTE_LENGTH"></a>

The wrong number of bytes was passed in for deserialization


<pre><code><b>const</b> <a href="elgamal.md#0x1_elgamal_EWRONG_BYTE_LENGTH">EWRONG_BYTE_LENGTH</a>: u64 = 1;
</code></pre>



<a name="0x1_elgamal_get_point_from_pubkey"></a>

## Function `get_point_from_pubkey`

Given a public key <code>pubkey</code>, returns the underlying RistrettoPoint representing that key


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_point_from_pubkey">get_point_from_pubkey</a>(pubkey: &<a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_point_from_pubkey">get_point_from_pubkey</a>(pubkey: &<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>): RistrettoPoint {
	<a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&pubkey.point)
}
</code></pre>



</details>

<a name="0x1_elgamal_get_pubkey_from_scalar"></a>

## Function `get_pubkey_from_scalar`

Given a ristretto255 <code>scalar</code>, returns as an ElGamal public key the ristretto255 basepoint multiplied
by <code>scalar</code>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_pubkey_from_scalar">get_pubkey_from_scalar</a>(scalar: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_pubkey_from_scalar">get_pubkey_from_scalar</a>(scalar: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a> {
    <b>let</b> point = <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(scalar);
    <a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a> {
        point: point_compress(&point)
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_get_compressed_point_from_pubkey"></a>

## Function `get_compressed_point_from_pubkey`

Given a public key, returns the underlying CompressedRistretto representing that key


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_compressed_point_from_pubkey">get_compressed_point_from_pubkey</a>(pubkey: &<a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_compressed_point_from_pubkey">get_compressed_point_from_pubkey</a>(pubkey: &<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>): CompressedRistretto {
	pubkey.point
}
</code></pre>



</details>

<a name="0x1_elgamal_new_pubkey_from_bytes"></a>

## Function `new_pubkey_from_bytes`

Creates a new public key from a serialized RistrettoPoint


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_pubkey_from_bytes">new_pubkey_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_pubkey_from_bytes">new_pubkey_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>&gt; {
	<b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&bytes) == 32, <a href="elgamal.md#0x1_elgamal_EWRONG_BYTE_LENGTH">EWRONG_BYTE_LENGTH</a>);
	<b>let</b> point = <a href="ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(bytes);
	<b>if</b> (std::option::is_some(&<b>mut</b> point)) {
	    <b>let</b> pk = <a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a> {
	        point: std::option::extract(&<b>mut</b> point)
	    };
	    std::option::some(pk)
	} <b>else</b> {
	    std::option::none&lt;<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>&gt;()
	}
}
</code></pre>



</details>

<a name="0x1_elgamal_pubkey_to_bytes"></a>

## Function `pubkey_to_bytes`

Given an ElGamal public key <code>pubkey</code>, returns the byte representation of that public key


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_pubkey_to_bytes">pubkey_to_bytes</a>(pubkey: &<a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_pubkey_to_bytes">pubkey_to_bytes</a>(pubkey: &<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(pubkey.point)
}
</code></pre>



</details>

<a name="0x1_elgamal_new_ciphertext_from_bytes"></a>

## Function `new_ciphertext_from_bytes`

Creates a new ciphertext from two serialized Ristretto points


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_from_bytes">new_ciphertext_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_from_bytes">new_ciphertext_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>&gt; {
	<b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&bytes) == 64, <a href="elgamal.md#0x1_elgamal_EWRONG_BYTE_LENGTH">EWRONG_BYTE_LENGTH</a>);
	<b>let</b> bytes_right = <a href="../../move-stdlib/doc/vector.md#0x1_vector_trim">vector::trim</a>(&<b>mut</b> bytes, 32);
	<b>let</b> left_point = <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes);
	<b>let</b> right_point = <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(bytes_right);
	<b>if</b> (std::option::is_some&lt;RistrettoPoint&gt;(&<b>mut</b> left_point) && std::option::is_some&lt;RistrettoPoint&gt;(&<b>mut</b> right_point)) {
		std::option::some&lt;<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>&gt;(<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> { left: std::option::extract&lt;RistrettoPoint&gt;(&<b>mut</b> left_point), right: std::option::extract&lt;RistrettoPoint&gt;(&<b>mut</b> right_point) })
	} <b>else</b> {
		std::option::none&lt;<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>&gt;()
	}
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_to_bytes"></a>

## Function `ciphertext_to_bytes`

Given a ciphertext <code>ct</code>, returns that ciphertext in serialzied byte form


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_to_bytes">ciphertext_to_bytes</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_to_bytes">ciphertext_to_bytes</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes_left = <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.left));
    <b>let</b> bytes_right = <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.right));
    <b>let</b> bytes = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>&lt;u8&gt;(&<b>mut</b> bytes, bytes_left);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>&lt;u8&gt;(&<b>mut</b> bytes, bytes_right);
    bytes
}
</code></pre>



</details>

<a name="0x1_elgamal_new_ciphertext_from_points"></a>

## Function `new_ciphertext_from_points`

Moves a pair of Ristretto points into an ElGamal ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_from_points">new_ciphertext_from_points</a>(left: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, right: <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_from_points">new_ciphertext_from_points</a>(left: RistrettoPoint, right: RistrettoPoint): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
    <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
        left,
	    right,
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_new_ciphertext_from_compressed"></a>

## Function `new_ciphertext_from_compressed`

Deserializes a ciphertext from compressed Ristretto points.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_from_compressed">new_ciphertext_from_compressed</a>(left: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, right: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="elgamal.md#0x1_elgamal_CompressedCiphertext">elgamal::CompressedCiphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_from_compressed">new_ciphertext_from_compressed</a>(left: CompressedRistretto, right: CompressedRistretto): <a href="elgamal.md#0x1_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
    <a href="elgamal.md#0x1_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
        left,
	    right,
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_new_ciphertext_no_randomness"></a>

## Function `new_ciphertext_no_randomness`

Creates a new ciphertext (val * basepoint, id) where <code>basepoint</code> is the Ristretto255 basepoint and id is the identity point.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_no_randomness">new_ciphertext_no_randomness</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_no_randomness">new_ciphertext_no_randomness</a>(val: &Scalar): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
	<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
	    left: <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(val),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_identity">ristretto255::point_identity</a>(),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_compress_ciphertext"></a>

## Function `compress_ciphertext`

Creates a new compressed ciphertext from a decompressed ciphertext


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_compress_ciphertext">compress_ciphertext</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_CompressedCiphertext">elgamal::CompressedCiphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_compress_ciphertext">compress_ciphertext</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
	<a href="elgamal.md#0x1_elgamal_CompressedCiphertext">CompressedCiphertext</a> {
	    left: <a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.left),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&ct.right),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_decompress_ciphertext"></a>

## Function `decompress_ciphertext`

Creates a new decompressed ciphertext from a compressed ciphertext


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_decompress_ciphertext">decompress_ciphertext</a>(ct: &<a href="elgamal.md#0x1_elgamal_CompressedCiphertext">elgamal::CompressedCiphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_decompress_ciphertext">decompress_ciphertext</a>(ct: &<a href="elgamal.md#0x1_elgamal_CompressedCiphertext">CompressedCiphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
	<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
	    left: <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&ct.left),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&ct.right),
	}
}
</code></pre>



</details>

<a name="0x1_elgamal_new_ciphertext"></a>

## Function `new_ciphertext`

Returns a ciphertext (val * val_base + r * pub_key, r * val_base) where val_base is the generator.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext">new_ciphertext</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, val_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, rand: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, pub_key: &<a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext">new_ciphertext</a>(val: &Scalar, val_base: &RistrettoPoint, rand: &Scalar, pub_key: &<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
    <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul">ristretto255::double_scalar_mul</a>(val, val_base, rand, &<a href="elgamal.md#0x1_elgamal_get_point_from_pubkey">get_point_from_pubkey</a>(pub_key)),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(val_base, rand),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_new_ciphertext_with_basepoint"></a>

## Function `new_ciphertext_with_basepoint`

Returns a ciphertext (val * basepoint + r * pub_key, rand * basepoint) where <code>basepoint</code> is the Ristretto255 basepoint.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_with_basepoint">new_ciphertext_with_basepoint</a>(val: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rand: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, pub_key: &<a href="elgamal.md#0x1_elgamal_Pubkey">elgamal::Pubkey</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_new_ciphertext_with_basepoint">new_ciphertext_with_basepoint</a>(val: &Scalar, rand: &Scalar, pub_key: &<a href="elgamal.md#0x1_elgamal_Pubkey">Pubkey</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
    <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">ristretto255::basepoint_double_mul</a>(rand, &<a href="elgamal.md#0x1_elgamal_get_point_from_pubkey">get_point_from_pubkey</a>(pub_key), val),
	    right: <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(rand),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_add"></a>

## Function `ciphertext_add`

Returns lhs + rhs. Useful for re-randomizing the ciphertext or updating the committed value.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_add">ciphertext_add</a>(lhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_add">ciphertext_add</a>(lhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
    <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&lhs.left, &rhs.left),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_add">ristretto255::point_add</a>(&lhs.right, &rhs.right),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_add_assign"></a>

## Function `ciphertext_add_assign`

Sets lhs = lhs + rhs. Useful for re-randomizing the ciphertext or updating the encrypted value.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_add_assign">ciphertext_add_assign</a>(lhs: &<b>mut</b> <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_add_assign">ciphertext_add_assign</a>(lhs: &<b>mut</b> <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>) {
    <a href="ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.left, &rhs.left);
	<a href="ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> lhs.right, &rhs.right);
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_sub"></a>

## Function `ciphertext_sub`

Returns lhs - rhs. Useful for re-randomizing the ciphertext or updating the encrypted value.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_sub">ciphertext_sub</a>(lhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_sub">ciphertext_sub</a>(lhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
    <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&lhs.left, &rhs.left),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(&lhs.right, &rhs.right),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_sub_assign"></a>

## Function `ciphertext_sub_assign`

Sets lhs = lhs - rhs. Useful for re-randomizing the ciphertext or updating the encrypted value.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_sub_assign">ciphertext_sub_assign</a>(lhs: &<b>mut</b> <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_sub_assign">ciphertext_sub_assign</a>(lhs: &<b>mut</b> <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>) {
    <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&<b>mut</b> lhs.left, &rhs.left);
	<a href="ristretto255.md#0x1_ristretto255_point_sub_assign">ristretto255::point_sub_assign</a>(&<b>mut</b> lhs.right, &rhs.right);
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_clone"></a>

## Function `ciphertext_clone`

Creates a copy of this ciphertext.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_clone">ciphertext_clone</a>(c: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_clone">ciphertext_clone</a>(c: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
    <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> {
        left: <a href="ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&c.left),
	    right: <a href="ristretto255.md#0x1_ristretto255_point_clone">ristretto255::point_clone</a>(&c.right),
    }
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_equals"></a>

## Function `ciphertext_equals`

Returns true if the two ciphertexts are identical: i.e., same value and same randomness.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_equals">ciphertext_equals</a>(lhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_equals">ciphertext_equals</a>(lhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>, rhs: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): bool {
    <a href="ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs.left, &rhs.left) &&
	<a href="ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs.right, &rhs.right)
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_as_points"></a>

## Function `ciphertext_as_points`

Returns the underlying elliptic curve point representing the ciphertext as a pair of in-memory RistrettoPoints.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_as_points">ciphertext_as_points</a>(c: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): (&<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_as_points">ciphertext_as_points</a>(c: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): (&RistrettoPoint, &RistrettoPoint) {
    (&c.left, &c.right)
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_as_compressed_points"></a>

## Function `ciphertext_as_compressed_points`

Returns the ciphertext as a pair of CompressedRistretto points.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_as_compressed_points">ciphertext_as_compressed_points</a>(c: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): (<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_as_compressed_points">ciphertext_as_compressed_points</a>(c: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): (CompressedRistretto, CompressedRistretto)   {
    (point_compress(&c.left), point_compress(&c.right))
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_into_points"></a>

## Function `ciphertext_into_points`

Moves the ciphertext into a pair of RistrettoPoints.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_into_points">ciphertext_into_points</a>(c: <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): (<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_into_points">ciphertext_into_points</a>(c: <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): (RistrettoPoint, RistrettoPoint) {
    <b>let</b> <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a> { left, right } = c;
    (left, right)
}
</code></pre>



</details>

<a name="0x1_elgamal_ciphertext_into_compressed_points"></a>

## Function `ciphertext_into_compressed_points`

Moves the ciphertext into a pair of CompressedRistretto points.


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_into_compressed_points">ciphertext_into_compressed_points</a>(c: <a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): (<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_ciphertext_into_compressed_points">ciphertext_into_compressed_points</a>(c: <a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): (CompressedRistretto, CompressedRistretto) {
    (point_compress(&c.left), point_compress(&c.right))
}
</code></pre>



</details>

<a name="0x1_elgamal_get_value_component"></a>

## Function `get_value_component`

Returns the RistrettoPoint in the ciphertext which contains the encrypted value in the exponent


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_value_component">get_value_component</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_value_component">get_value_component</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): &RistrettoPoint {
    &ct.left
}
</code></pre>



</details>

<a name="0x1_elgamal_get_value_component_compressed"></a>

## Function `get_value_component_compressed`

Returns the RistrettoPoint in the ciphertext which contains the encrypted value in the exponent


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_value_component_compressed">get_value_component_compressed</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="elgamal.md#0x1_elgamal_get_value_component_compressed">get_value_component_compressed</a>(ct: &<a href="elgamal.md#0x1_elgamal_Ciphertext">Ciphertext</a>): CompressedRistretto {
    point_compress(&ct.left)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
