
<a id="0x1_ristretto255"></a>

# Module `0x1::ristretto255`

This module contains functions for Ristretto255 curve arithmetic, assuming addition as the group operation.

The order of the Ristretto255 elliptic curve group is $\ell = 2^252 + 27742317777372353535851937790883648493$, same
as the order of the prime-order subgroup of Curve25519.

This module provides two structs for encoding Ristretto elliptic curves to the developer:

- First, a 32-byte-sized CompressedRistretto struct, which is used to persist points in storage.

- Second, a larger, in-memory, RistrettoPoint struct, which is decompressable from a CompressedRistretto struct. This
larger struct can be used for fast arithmetic operations (additions, multiplications, etc.). The results can be saved
back into storage by compressing RistrettoPoint structs back to CompressedRistretto structs.

This module also provides a Scalar struct for persisting scalars in storage and doing fast arithmetic on them.

One invariant maintained by this module is that all CompressedRistretto structs store a canonically-encoded point,
which can always be decompressed into a valid point on the curve as a RistrettoPoint struct. Unfortunately, due to
limitations in our underlying curve25519-dalek elliptic curve library, this decompression will unnecessarily verify
the validity of the point and thus slightly decrease performance.

Similarly, all Scalar structs store a canonically-encoded scalar, which can always be safely operated on using
arithmetic operations.

In the future, we might support additional features:

* For scalars:
- batch_invert()

* For points:
- double()
+ The challenge is that curve25519-dalek does NOT export double for Ristretto points (nor for Edwards)

- double_and_compress_batch()

- fixed-base, variable-time via optional_mixed_multiscalar_mul() in VartimePrecomputedMultiscalarMul
+ This would require a storage-friendly RistrettoBasepointTable and an in-memory variant of it too
+ Similar to the CompressedRistretto and RistrettoPoint structs in this module
+ The challenge is that curve25519-dalek's RistrettoBasepointTable is not serializable


-  [Struct `Scalar`](#0x1_ristretto255_Scalar)
-  [Struct `CompressedRistretto`](#0x1_ristretto255_CompressedRistretto)
-  [Struct `RistrettoPoint`](#0x1_ristretto255_RistrettoPoint)
-  [Constants](#@Constants_0)
-  [Function `point_identity_compressed`](#0x1_ristretto255_point_identity_compressed)
-  [Function `point_identity`](#0x1_ristretto255_point_identity)
-  [Function `basepoint_compressed`](#0x1_ristretto255_basepoint_compressed)
-  [Function `hash_to_point_base`](#0x1_ristretto255_hash_to_point_base)
-  [Function `basepoint`](#0x1_ristretto255_basepoint)
-  [Function `basepoint_mul`](#0x1_ristretto255_basepoint_mul)
-  [Function `new_compressed_point_from_bytes`](#0x1_ristretto255_new_compressed_point_from_bytes)
-  [Function `new_point_from_bytes`](#0x1_ristretto255_new_point_from_bytes)
-  [Function `compressed_point_to_bytes`](#0x1_ristretto255_compressed_point_to_bytes)
-  [Function `new_point_from_sha512`](#0x1_ristretto255_new_point_from_sha512)
-  [Function `new_point_from_sha2_512`](#0x1_ristretto255_new_point_from_sha2_512)
-  [Function `new_point_from_64_uniform_bytes`](#0x1_ristretto255_new_point_from_64_uniform_bytes)
-  [Function `point_decompress`](#0x1_ristretto255_point_decompress)
-  [Function `point_clone`](#0x1_ristretto255_point_clone)
-  [Function `point_compress`](#0x1_ristretto255_point_compress)
-  [Function `point_to_bytes`](#0x1_ristretto255_point_to_bytes)
-  [Function `point_mul`](#0x1_ristretto255_point_mul)
-  [Function `point_mul_assign`](#0x1_ristretto255_point_mul_assign)
-  [Function `basepoint_double_mul`](#0x1_ristretto255_basepoint_double_mul)
-  [Function `point_add`](#0x1_ristretto255_point_add)
-  [Function `point_add_assign`](#0x1_ristretto255_point_add_assign)
-  [Function `point_sub`](#0x1_ristretto255_point_sub)
-  [Function `point_sub_assign`](#0x1_ristretto255_point_sub_assign)
-  [Function `point_neg`](#0x1_ristretto255_point_neg)
-  [Function `point_neg_assign`](#0x1_ristretto255_point_neg_assign)
-  [Function `point_equals`](#0x1_ristretto255_point_equals)
-  [Function `double_scalar_mul`](#0x1_ristretto255_double_scalar_mul)
-  [Function `multi_scalar_mul`](#0x1_ristretto255_multi_scalar_mul)
-  [Function `new_scalar_from_bytes`](#0x1_ristretto255_new_scalar_from_bytes)
-  [Function `new_scalar_from_sha512`](#0x1_ristretto255_new_scalar_from_sha512)
-  [Function `new_scalar_from_sha2_512`](#0x1_ristretto255_new_scalar_from_sha2_512)
-  [Function `new_scalar_from_u8`](#0x1_ristretto255_new_scalar_from_u8)
-  [Function `new_scalar_from_u32`](#0x1_ristretto255_new_scalar_from_u32)
-  [Function `new_scalar_from_u64`](#0x1_ristretto255_new_scalar_from_u64)
-  [Function `new_scalar_from_u128`](#0x1_ristretto255_new_scalar_from_u128)
-  [Function `new_scalar_reduced_from_32_bytes`](#0x1_ristretto255_new_scalar_reduced_from_32_bytes)
-  [Function `new_scalar_uniform_from_64_bytes`](#0x1_ristretto255_new_scalar_uniform_from_64_bytes)
-  [Function `scalar_zero`](#0x1_ristretto255_scalar_zero)
-  [Function `scalar_is_zero`](#0x1_ristretto255_scalar_is_zero)
-  [Function `scalar_one`](#0x1_ristretto255_scalar_one)
-  [Function `scalar_is_one`](#0x1_ristretto255_scalar_is_one)
-  [Function `scalar_equals`](#0x1_ristretto255_scalar_equals)
-  [Function `scalar_invert`](#0x1_ristretto255_scalar_invert)
-  [Function `scalar_mul`](#0x1_ristretto255_scalar_mul)
-  [Function `scalar_mul_assign`](#0x1_ristretto255_scalar_mul_assign)
-  [Function `scalar_add`](#0x1_ristretto255_scalar_add)
-  [Function `scalar_add_assign`](#0x1_ristretto255_scalar_add_assign)
-  [Function `scalar_sub`](#0x1_ristretto255_scalar_sub)
-  [Function `scalar_sub_assign`](#0x1_ristretto255_scalar_sub_assign)
-  [Function `scalar_neg`](#0x1_ristretto255_scalar_neg)
-  [Function `scalar_neg_assign`](#0x1_ristretto255_scalar_neg_assign)
-  [Function `scalar_to_bytes`](#0x1_ristretto255_scalar_to_bytes)
-  [Function `new_point_from_sha512_internal`](#0x1_ristretto255_new_point_from_sha512_internal)
-  [Function `new_point_from_64_uniform_bytes_internal`](#0x1_ristretto255_new_point_from_64_uniform_bytes_internal)
-  [Function `point_is_canonical_internal`](#0x1_ristretto255_point_is_canonical_internal)
-  [Function `point_identity_internal`](#0x1_ristretto255_point_identity_internal)
-  [Function `point_decompress_internal`](#0x1_ristretto255_point_decompress_internal)
-  [Function `point_clone_internal`](#0x1_ristretto255_point_clone_internal)
-  [Function `point_compress_internal`](#0x1_ristretto255_point_compress_internal)
-  [Function `point_mul_internal`](#0x1_ristretto255_point_mul_internal)
-  [Function `basepoint_mul_internal`](#0x1_ristretto255_basepoint_mul_internal)
-  [Function `basepoint_double_mul_internal`](#0x1_ristretto255_basepoint_double_mul_internal)
-  [Function `point_add_internal`](#0x1_ristretto255_point_add_internal)
-  [Function `point_sub_internal`](#0x1_ristretto255_point_sub_internal)
-  [Function `point_neg_internal`](#0x1_ristretto255_point_neg_internal)
-  [Function `double_scalar_mul_internal`](#0x1_ristretto255_double_scalar_mul_internal)
-  [Function `multi_scalar_mul_internal`](#0x1_ristretto255_multi_scalar_mul_internal)
-  [Function `scalar_is_canonical_internal`](#0x1_ristretto255_scalar_is_canonical_internal)
-  [Function `scalar_from_u64_internal`](#0x1_ristretto255_scalar_from_u64_internal)
-  [Function `scalar_from_u128_internal`](#0x1_ristretto255_scalar_from_u128_internal)
-  [Function `scalar_reduced_from_32_bytes_internal`](#0x1_ristretto255_scalar_reduced_from_32_bytes_internal)
-  [Function `scalar_uniform_from_64_bytes_internal`](#0x1_ristretto255_scalar_uniform_from_64_bytes_internal)
-  [Function `scalar_invert_internal`](#0x1_ristretto255_scalar_invert_internal)
-  [Function `scalar_from_sha512_internal`](#0x1_ristretto255_scalar_from_sha512_internal)
-  [Function `scalar_mul_internal`](#0x1_ristretto255_scalar_mul_internal)
-  [Function `scalar_add_internal`](#0x1_ristretto255_scalar_add_internal)
-  [Function `scalar_sub_internal`](#0x1_ristretto255_scalar_sub_internal)
-  [Function `scalar_neg_internal`](#0x1_ristretto255_scalar_neg_internal)
-  [Specification](#@Specification_1)
    -  [Function `point_equals`](#@Specification_1_point_equals)
    -  [Function `double_scalar_mul`](#@Specification_1_double_scalar_mul)
    -  [Function `multi_scalar_mul`](#@Specification_1_multi_scalar_mul)
    -  [Function `new_scalar_from_bytes`](#@Specification_1_new_scalar_from_bytes)
    -  [Function `new_scalar_from_sha2_512`](#@Specification_1_new_scalar_from_sha2_512)
    -  [Function `new_scalar_from_u8`](#@Specification_1_new_scalar_from_u8)
    -  [Function `new_scalar_from_u32`](#@Specification_1_new_scalar_from_u32)
    -  [Function `new_scalar_from_u64`](#@Specification_1_new_scalar_from_u64)
    -  [Function `new_scalar_from_u128`](#@Specification_1_new_scalar_from_u128)
    -  [Function `new_scalar_reduced_from_32_bytes`](#@Specification_1_new_scalar_reduced_from_32_bytes)
    -  [Function `new_scalar_uniform_from_64_bytes`](#@Specification_1_new_scalar_uniform_from_64_bytes)
    -  [Function `scalar_zero`](#@Specification_1_scalar_zero)
    -  [Function `scalar_is_zero`](#@Specification_1_scalar_is_zero)
    -  [Function `scalar_one`](#@Specification_1_scalar_one)
    -  [Function `scalar_is_one`](#@Specification_1_scalar_is_one)
    -  [Function `scalar_equals`](#@Specification_1_scalar_equals)
    -  [Function `scalar_invert`](#@Specification_1_scalar_invert)
    -  [Function `scalar_mul`](#@Specification_1_scalar_mul)
    -  [Function `scalar_mul_assign`](#@Specification_1_scalar_mul_assign)
    -  [Function `scalar_add`](#@Specification_1_scalar_add)
    -  [Function `scalar_add_assign`](#@Specification_1_scalar_add_assign)
    -  [Function `scalar_sub`](#@Specification_1_scalar_sub)
    -  [Function `scalar_sub_assign`](#@Specification_1_scalar_sub_assign)
    -  [Function `scalar_neg`](#@Specification_1_scalar_neg)
    -  [Function `scalar_neg_assign`](#@Specification_1_scalar_neg_assign)
    -  [Function `scalar_to_bytes`](#@Specification_1_scalar_to_bytes)
    -  [Helper functions](#@Helper_functions_2)
    -  [Function `new_point_from_sha512_internal`](#@Specification_1_new_point_from_sha512_internal)
    -  [Function `new_point_from_64_uniform_bytes_internal`](#@Specification_1_new_point_from_64_uniform_bytes_internal)
    -  [Function `point_is_canonical_internal`](#@Specification_1_point_is_canonical_internal)
    -  [Function `point_identity_internal`](#@Specification_1_point_identity_internal)
    -  [Function `point_decompress_internal`](#@Specification_1_point_decompress_internal)
    -  [Function `point_clone_internal`](#@Specification_1_point_clone_internal)
    -  [Function `point_compress_internal`](#@Specification_1_point_compress_internal)
    -  [Function `point_mul_internal`](#@Specification_1_point_mul_internal)
    -  [Function `basepoint_mul_internal`](#@Specification_1_basepoint_mul_internal)
    -  [Function `basepoint_double_mul_internal`](#@Specification_1_basepoint_double_mul_internal)
    -  [Function `point_add_internal`](#@Specification_1_point_add_internal)
    -  [Function `point_sub_internal`](#@Specification_1_point_sub_internal)
    -  [Function `point_neg_internal`](#@Specification_1_point_neg_internal)
    -  [Function `double_scalar_mul_internal`](#@Specification_1_double_scalar_mul_internal)
    -  [Function `multi_scalar_mul_internal`](#@Specification_1_multi_scalar_mul_internal)
    -  [Function `scalar_is_canonical_internal`](#@Specification_1_scalar_is_canonical_internal)
    -  [Function `scalar_from_u64_internal`](#@Specification_1_scalar_from_u64_internal)
    -  [Function `scalar_from_u128_internal`](#@Specification_1_scalar_from_u128_internal)
    -  [Function `scalar_reduced_from_32_bytes_internal`](#@Specification_1_scalar_reduced_from_32_bytes_internal)
    -  [Function `scalar_uniform_from_64_bytes_internal`](#@Specification_1_scalar_uniform_from_64_bytes_internal)
    -  [Function `scalar_invert_internal`](#@Specification_1_scalar_invert_internal)
    -  [Function `scalar_from_sha512_internal`](#@Specification_1_scalar_from_sha512_internal)
    -  [Function `scalar_mul_internal`](#@Specification_1_scalar_mul_internal)
    -  [Function `scalar_add_internal`](#@Specification_1_scalar_add_internal)
    -  [Function `scalar_sub_internal`](#@Specification_1_scalar_sub_internal)
    -  [Function `scalar_neg_internal`](#@Specification_1_scalar_neg_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x1_ristretto255_Scalar"></a>

## Struct `Scalar`

This struct represents a scalar as a little-endian byte encoding of an integer in $\mathbb{Z}_\ell$, which is
stored in <code>data</code>. Here, \ell denotes the order of the scalar field (and the underlying elliptic curve group).


<pre><code><b>struct</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_CompressedRistretto"></a>

## Struct `CompressedRistretto`

This struct represents a serialized point on the Ristretto255 curve, in 32 bytes.
This struct can be decompressed from storage into an in-memory RistrettoPoint, on which fast curve arithmetic
can be performed.


<pre><code><b>struct</b> <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ristretto255_RistrettoPoint"></a>

## Struct `RistrettoPoint`

This struct represents an in-memory Ristretto255 point and supports fast curve arithmetic.

An important invariant: There will never be two RistrettoPoint's constructed with the same handle. One can have
immutable references to the same RistrettoPoint, of course.


<pre><code><b>struct</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ristretto255_E_NATIVE_FUN_NOT_AVAILABLE"></a>

The native function has not been deployed yet.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 5;
</code></pre>



<a id="0x1_ristretto255_BASE_POINT"></a>

The basepoint (generator) of the Ristretto255 group


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_BASE_POINT">BASE_POINT</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [226, 242, 174, 10, 106, 188, 78, 113, 168, 132, 169, 97, 197, 0, 81, 95, 88, 227, 11, 106, 165, 130, 221, 141, 182, 166, 89, 69, 224, 141, 45, 118];
</code></pre>



<a id="0x1_ristretto255_E_DIFFERENT_NUM_POINTS_AND_SCALARS"></a>

The number of scalars does not match the number of points.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_E_DIFFERENT_NUM_POINTS_AND_SCALARS">E_DIFFERENT_NUM_POINTS_AND_SCALARS</a>: u64 = 1;
</code></pre>



<a id="0x1_ristretto255_E_TOO_MANY_POINTS_CREATED"></a>

Too many points have been created in the current transaction execution.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_E_TOO_MANY_POINTS_CREATED">E_TOO_MANY_POINTS_CREATED</a>: u64 = 4;
</code></pre>



<a id="0x1_ristretto255_E_ZERO_POINTS"></a>

Expected more than zero points as input.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_E_ZERO_POINTS">E_ZERO_POINTS</a>: u64 = 2;
</code></pre>



<a id="0x1_ristretto255_E_ZERO_SCALARS"></a>

Expected more than zero scalars as input.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_E_ZERO_SCALARS">E_ZERO_SCALARS</a>: u64 = 3;
</code></pre>



<a id="0x1_ristretto255_HASH_BASE_POINT"></a>

The hash of the basepoint of the Ristretto255 group using SHA3_512


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_HASH_BASE_POINT">HASH_BASE_POINT</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [140, 146, 64, 180, 86, 169, 230, 220, 101, 195, 119, 161, 4, 141, 116, 95, 148, 160, 140, 219, 127, 68, 203, 205, 123, 70, 243, 64, 72, 135, 17, 52];
</code></pre>



<a id="0x1_ristretto255_L_MINUS_ONE"></a>

<code><a href="ristretto255.md#0x1_ristretto255_ORDER_ELL">ORDER_ELL</a></code> - 1: i.e., the "largest", reduced scalar in the field


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_L_MINUS_ONE">L_MINUS_ONE</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [236, 211, 245, 92, 26, 99, 18, 88, 214, 156, 247, 162, 222, 249, 222, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16];
</code></pre>



<a id="0x1_ristretto255_MAX_POINT_NUM_BYTES"></a>

The maximum size in bytes of a canonically-encoded Ristretto255 point is 32 bytes.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_MAX_POINT_NUM_BYTES">MAX_POINT_NUM_BYTES</a>: u64 = 32;
</code></pre>



<a id="0x1_ristretto255_MAX_SCALAR_NUM_BITS"></a>

The maximum size in bits of a canonically-encoded Scalar is 256 bits.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_MAX_SCALAR_NUM_BITS">MAX_SCALAR_NUM_BITS</a>: u64 = 256;
</code></pre>



<a id="0x1_ristretto255_MAX_SCALAR_NUM_BYTES"></a>

The maximum size in bytes of a canonically-encoded Scalar is 32 bytes.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_MAX_SCALAR_NUM_BYTES">MAX_SCALAR_NUM_BYTES</a>: u64 = 32;
</code></pre>



<a id="0x1_ristretto255_ORDER_ELL"></a>

The order of the Ristretto255 group and its scalar field, in little-endian.


<pre><code><b>const</b> <a href="ristretto255.md#0x1_ristretto255_ORDER_ELL">ORDER_ELL</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [237, 211, 245, 92, 26, 99, 18, 88, 214, 156, 247, 162, 222, 249, 222, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16];
</code></pre>



<a id="0x1_ristretto255_point_identity_compressed"></a>

## Function `point_identity_compressed`

Returns the identity point as a CompressedRistretto.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity_compressed">point_identity_compressed</a>(): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity_compressed">point_identity_compressed</a>(): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
    <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
        data: x"0000000000000000000000000000000000000000000000000000000000000000"
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_identity"></a>

## Function `point_identity`

Returns the identity point as a CompressedRistretto.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity">point_identity</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity">point_identity</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_point_identity_internal">point_identity_internal</a>()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_basepoint_compressed"></a>

## Function `basepoint_compressed`

Returns the basepoint (generator) of the Ristretto255 group as a compressed point


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_compressed">basepoint_compressed</a>(): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_compressed">basepoint_compressed</a>(): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
    <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_BASE_POINT">BASE_POINT</a>
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_hash_to_point_base"></a>

## Function `hash_to_point_base`

Returns the hash-to-point result of serializing the basepoint of the Ristretto255 group.
For use as the random value basepoint in Pedersen commitments


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_hash_to_point_base">hash_to_point_base</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_hash_to_point_base">hash_to_point_base</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <b>let</b> comp_res = <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> { data: <a href="ristretto255.md#0x1_ristretto255_HASH_BASE_POINT">HASH_BASE_POINT</a> };
    <a href="ristretto255.md#0x1_ristretto255_point_decompress">point_decompress</a>(&comp_res)
}
</code></pre>



</details>

<a id="0x1_ristretto255_basepoint"></a>

## Function `basepoint`

Returns the basepoint (generator) of the Ristretto255 group


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint">basepoint</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint">basepoint</a>(): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <b>let</b> (handle, _) = <a href="ristretto255.md#0x1_ristretto255_point_decompress_internal">point_decompress_internal</a>(<a href="ristretto255.md#0x1_ristretto255_BASE_POINT">BASE_POINT</a>);

    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_basepoint_mul"></a>

## Function `basepoint_mul`

Multiplies the basepoint (generator) of the Ristretto255 group by a scalar and returns the result.
This call is much faster than <code><a href="ristretto255.md#0x1_ristretto255_point_mul">point_mul</a>(&<a href="ristretto255.md#0x1_ristretto255_basepoint">basepoint</a>(), &some_scalar)</code> because of precomputation tables.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">basepoint_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_mul">basepoint_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_basepoint_mul_internal">basepoint_mul_internal</a>(a.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_compressed_point_from_bytes"></a>

## Function `new_compressed_point_from_bytes`

Creates a new CompressedRistretto point from a sequence of 32 bytes. If those bytes do not represent a valid
point, returns None.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">new_compressed_point_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">new_compressed_point_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a>&gt; {
    <b>if</b> (<a href="ristretto255.md#0x1_ristretto255_point_is_canonical_internal">point_is_canonical_internal</a>(bytes)) {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
            data: bytes
        })
    } <b>else</b> {
        std::option::none&lt;<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a>&gt;()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_bytes"></a>

## Function `new_point_from_bytes`

Creates a new RistrettoPoint from a sequence of 32 bytes. If those bytes do not represent a valid point,
returns None.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">new_point_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_bytes">new_point_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>&gt; {
    <b>let</b> (handle, is_canonical) = <a href="ristretto255.md#0x1_ristretto255_point_decompress_internal">point_decompress_internal</a>(bytes);
    <b>if</b> (is_canonical) {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> { handle })
    } <b>else</b> {
        std::option::none&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>&gt;()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_compressed_point_to_bytes"></a>

## Function `compressed_point_to_bytes`

Given a compressed ristretto point <code>point</code>, returns the byte representation of that point


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">compressed_point_to_bytes</a>(point: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">compressed_point_to_bytes</a>(point: <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    point.data
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_sha512"></a>

## Function `new_point_from_sha512`

DEPRECATED: Use the more clearly-named <code>new_point_from_sha2_512</code>

Hashes the input to a uniformly-at-random RistrettoPoint via SHA512.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha512">new_point_from_sha512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha512">new_point_from_sha512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha2_512">new_point_from_sha2_512</a>(sha2_512_input)
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_sha2_512"></a>

## Function `new_point_from_sha2_512`

Hashes the input to a uniformly-at-random RistrettoPoint via SHA2-512.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha2_512">new_point_from_sha2_512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha2_512">new_point_from_sha2_512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha512_internal">new_point_from_sha512_internal</a>(sha2_512_input)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_64_uniform_bytes"></a>

## Function `new_point_from_64_uniform_bytes`

Samples a uniformly-at-random RistrettoPoint given a sequence of 64 uniformly-at-random bytes. This function
can be used to build a collision-resistant hash function that maps 64-byte messages to RistrettoPoint's.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_64_uniform_bytes">new_point_from_64_uniform_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_64_uniform_bytes">new_point_from_64_uniform_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>&gt; {
    <b>if</b> (bytes.length() == 64) {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
            handle: <a href="ristretto255.md#0x1_ristretto255_new_point_from_64_uniform_bytes_internal">new_point_from_64_uniform_bytes_internal</a>(bytes)
        })
    } <b>else</b> {
        std::option::none&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>&gt;()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_decompress"></a>

## Function `point_decompress`

Decompresses a CompressedRistretto from storage into a RistrettoPoint which can be used for fast arithmetic.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_decompress">point_decompress</a>(point: &<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_decompress">point_decompress</a>(point: &<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    // NOTE: Our <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> <b>invariant</b> assures us that every <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> in storage is a valid
    // <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>
    <b>let</b> (handle, _) = <a href="ristretto255.md#0x1_ristretto255_point_decompress_internal">point_decompress_internal</a>(point.data);
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> { handle }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_clone"></a>

## Function `point_clone`

Clones a RistrettoPoint.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_clone">point_clone</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_clone">point_clone</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <b>if</b>(!<a href="../../move-stdlib/doc/features.md#0x1_features_bulletproofs_enabled">features::bulletproofs_enabled</a>()) {
        <b>abort</b>(std::error::invalid_state(<a href="ristretto255.md#0x1_ristretto255_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>))
    };

    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_point_clone_internal">point_clone_internal</a>(point.handle)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_compress"></a>

## Function `point_compress`

Compresses a RistrettoPoint to a CompressedRistretto which can be put in storage.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_compress">point_compress</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_compress">point_compress</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
    <a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_point_compress_internal">point_compress_internal</a>(point)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_to_bytes"></a>

## Function `point_to_bytes`

Returns the sequence of bytes representin this Ristretto point.
To convert a RistrettoPoint 'p' to bytes, first compress it via <code>c = <a href="ristretto255.md#0x1_ristretto255_point_compress">point_compress</a>(&p)</code>, and then call this
function on <code>c</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">point_to_bytes</a>(point: &<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_to_bytes">point_to_bytes</a>(point: &<a href="ristretto255.md#0x1_ristretto255_CompressedRistretto">CompressedRistretto</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    point.data
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_mul"></a>

## Function `point_mul`

Returns a * point.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul">point_mul</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul">point_mul</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_point_mul_internal">point_mul_internal</a>(point, a.data, <b>false</b>)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_mul_assign"></a>

## Function `point_mul_assign`

Sets a *= point and returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul_assign">point_mul_assign</a>(point: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul_assign">point_mul_assign</a>(point: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_point_mul_internal">point_mul_internal</a>(point, a.data, <b>true</b>);
    point
}
</code></pre>



</details>

<a id="0x1_ristretto255_basepoint_double_mul"></a>

## Function `basepoint_double_mul`

Returns (a * a_base + b * base_point), where base_point is the Ristretto basepoint encoded in <code><a href="ristretto255.md#0x1_ristretto255_BASE_POINT">BASE_POINT</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">basepoint_double_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, a_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul">basepoint_double_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, a_base: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul_internal">basepoint_double_mul_internal</a>(a.data, a_base, b.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_add"></a>

## Function `point_add`

Returns a + b


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add">point_add</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add">point_add</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_point_add_internal">point_add_internal</a>(a, b, <b>false</b>)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_add_assign"></a>

## Function `point_add_assign`

Sets a += b and returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add_assign">point_add_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add_assign">point_add_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_point_add_internal">point_add_internal</a>(a, b, <b>true</b>);
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_sub"></a>

## Function `point_sub`

Returns a - b


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub">point_sub</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub">point_sub</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_point_sub_internal">point_sub_internal</a>(a, b, <b>false</b>)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_sub_assign"></a>

## Function `point_sub_assign`

Sets a -= b and returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">point_sub_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub_assign">point_sub_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_point_sub_internal">point_sub_internal</a>(a, b, <b>true</b>);
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_neg"></a>

## Function `point_neg`

Returns -a


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg">point_neg</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg">point_neg</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_point_neg_internal">point_neg_internal</a>(a, <b>false</b>)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_neg_assign"></a>

## Function `point_neg_assign`

Sets a = -a, and returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg_assign">point_neg_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg_assign">point_neg_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <a href="ristretto255.md#0x1_ristretto255_point_neg_internal">point_neg_internal</a>(a, <b>true</b>);
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_point_equals"></a>

## Function `point_equals`

Returns true if the two RistrettoPoints are the same points on the elliptic curve.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_equals">point_equals</a>(g: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, h: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_equals">point_equals</a>(g: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, h: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): bool;
</code></pre>



</details>

<a id="0x1_ristretto255_double_scalar_mul"></a>

## Function `double_scalar_mul`

Computes a double-scalar multiplication, returning a_1 p_1 + a_2 p_2
This function is much faster than computing each a_i p_i using <code>point_mul</code> and adding up the results using <code>point_add</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul">double_scalar_mul</a>(scalar1: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, point1: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, scalar2: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, point2: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul">double_scalar_mul</a>(scalar1: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, point1: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, scalar2: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, point2: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <b>if</b>(!<a href="../../move-stdlib/doc/features.md#0x1_features_bulletproofs_enabled">features::bulletproofs_enabled</a>()) {
        <b>abort</b>(std::error::invalid_state(<a href="ristretto255.md#0x1_ristretto255_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>))
    };

    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul_internal">double_scalar_mul_internal</a>(point1.handle, point2.handle, scalar1.data, scalar2.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_multi_scalar_mul"></a>

## Function `multi_scalar_mul`

Computes a multi-scalar multiplication, returning a_1 p_1 + a_2 p_2 + ... + a_n p_n.
This function is much faster than computing each a_i p_i using <code>point_mul</code> and adding up the results using <code>point_add</code>.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul">multi_scalar_mul</a>(points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul">multi_scalar_mul</a>(points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
    <b>assert</b>!(!points.is_empty(), std::error::invalid_argument(<a href="ristretto255.md#0x1_ristretto255_E_ZERO_POINTS">E_ZERO_POINTS</a>));
    <b>assert</b>!(!scalars.is_empty(), std::error::invalid_argument(<a href="ristretto255.md#0x1_ristretto255_E_ZERO_SCALARS">E_ZERO_SCALARS</a>));
    <b>assert</b>!(
        points.length() == scalars.length(), std::error::invalid_argument(<a href="ristretto255.md#0x1_ristretto255_E_DIFFERENT_NUM_POINTS_AND_SCALARS">E_DIFFERENT_NUM_POINTS_AND_SCALARS</a>));

    <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a> {
        handle: <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt;(points, scalars)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_bytes"></a>

## Function `new_scalar_from_bytes`

Given a sequence of 32 bytes, checks if they canonically-encode a Scalar and return it.
Otherwise, returns None.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt; {
    <b>if</b> (<a href="ristretto255.md#0x1_ristretto255_scalar_is_canonical_internal">scalar_is_canonical_internal</a>(bytes)) {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
            data: bytes
        })
    } <b>else</b> {
        std::option::none&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt;()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_sha512"></a>

## Function `new_scalar_from_sha512`

DEPRECATED: Use the more clearly-named <code>new_scalar_from_sha2_512</code>

Hashes the input to a uniformly-at-random Scalar via SHA2-512


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_sha512">new_scalar_from_sha512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_sha512">new_scalar_from_sha512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">new_scalar_from_sha2_512</a>(sha2_512_input)
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_sha2_512"></a>

## Function `new_scalar_from_sha2_512`

Hashes the input to a uniformly-at-random Scalar via SHA2-512


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">new_scalar_from_sha2_512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">new_scalar_from_sha2_512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_from_sha512_internal">scalar_from_sha512_internal</a>(sha2_512_input)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u8"></a>

## Function `new_scalar_from_u8`

Creates a Scalar from an u8.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u8">new_scalar_from_u8</a>(byte: u8): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u8">new_scalar_from_u8</a>(byte: u8): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <b>let</b> s = <a href="ristretto255.md#0x1_ristretto255_scalar_zero">scalar_zero</a>();
    s.data[0] = byte;
    s
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u32"></a>

## Function `new_scalar_from_u32`

Creates a Scalar from an u32.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u32">new_scalar_from_u32</a>(four_bytes: u32): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u32">new_scalar_from_u32</a>(four_bytes: u32): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_from_u64_internal">scalar_from_u64_internal</a>((four_bytes <b>as</b> u64))
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u64"></a>

## Function `new_scalar_from_u64`

Creates a Scalar from an u64.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u64">new_scalar_from_u64</a>(eight_bytes: u64): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u64">new_scalar_from_u64</a>(eight_bytes: u64): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_from_u64_internal">scalar_from_u64_internal</a>(eight_bytes)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u128"></a>

## Function `new_scalar_from_u128`

Creates a Scalar from an u128.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u128">new_scalar_from_u128</a>(sixteen_bytes: u128): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u128">new_scalar_from_u128</a>(sixteen_bytes: u128): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_from_u128_internal">scalar_from_u128_internal</a>(sixteen_bytes)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_reduced_from_32_bytes"></a>

## Function `new_scalar_reduced_from_32_bytes`

Creates a Scalar from 32 bytes by reducing the little-endian-encoded number in those bytes modulo $\ell$.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_reduced_from_32_bytes">new_scalar_reduced_from_32_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_reduced_from_32_bytes">new_scalar_reduced_from_32_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt; {
    <b>if</b> (bytes.length() == 32) {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
            data: <a href="ristretto255.md#0x1_ristretto255_scalar_reduced_from_32_bytes_internal">scalar_reduced_from_32_bytes_internal</a>(bytes)
        })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_uniform_from_64_bytes"></a>

## Function `new_scalar_uniform_from_64_bytes`

Samples a scalar uniformly-at-random given 64 uniform-at-random bytes as input by reducing the little-endian-encoded number
in those bytes modulo $\ell$.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_uniform_from_64_bytes">new_scalar_uniform_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_uniform_from_64_bytes">new_scalar_uniform_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt; {
    <b>if</b> (bytes.length() == 64) {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
            data: <a href="ristretto255.md#0x1_ristretto255_scalar_uniform_from_64_bytes_internal">scalar_uniform_from_64_bytes_internal</a>(bytes)
        })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_zero"></a>

## Function `scalar_zero`

Returns 0 as a Scalar.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_zero">scalar_zero</a>(): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_zero">scalar_zero</a>(): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: x"0000000000000000000000000000000000000000000000000000000000000000"
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_is_zero"></a>

## Function `scalar_is_zero`

Returns true if the given Scalar equals 0.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_zero">scalar_is_zero</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_zero">scalar_is_zero</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): bool {
    s.data == x"0000000000000000000000000000000000000000000000000000000000000000"
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_one"></a>

## Function `scalar_one`

Returns 1 as a Scalar.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_one">scalar_one</a>(): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_one">scalar_one</a>(): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: x"0100000000000000000000000000000000000000000000000000000000000000"
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_is_one"></a>

## Function `scalar_is_one`

Returns true if the given Scalar equals 1.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_one">scalar_is_one</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_one">scalar_is_one</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): bool {
    s.data == x"0100000000000000000000000000000000000000000000000000000000000000"
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_equals"></a>

## Function `scalar_equals`

Returns true if the two scalars are equal.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_equals">scalar_equals</a>(lhs: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rhs: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_equals">scalar_equals</a>(lhs: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, rhs: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): bool {
    lhs.data == rhs.data
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_invert"></a>

## Function `scalar_invert`

Returns the inverse s^{-1} mod \ell of a scalar s.
Returns None if s is zero.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_invert">scalar_invert</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_invert">scalar_invert</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): Option&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt; {
    <b>if</b> (<a href="ristretto255.md#0x1_ristretto255_scalar_is_zero">scalar_is_zero</a>(s)) {
        std::option::none&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>&gt;()
    } <b>else</b> {
        std::option::some(<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
            data: <a href="ristretto255.md#0x1_ristretto255_scalar_invert_internal">scalar_invert_internal</a>(s.data)
        })
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_mul"></a>

## Function `scalar_mul`

Returns the product of the two scalars.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul">scalar_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul">scalar_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_mul_internal">scalar_mul_internal</a>(a.data, b.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_mul_assign"></a>

## Function `scalar_mul_assign`

Computes the product of 'a' and 'b' and assigns the result to 'a'.
Returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul_assign">scalar_mul_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul_assign">scalar_mul_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    a.data = <a href="ristretto255.md#0x1_ristretto255_scalar_mul">scalar_mul</a>(a, b).data;
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_add"></a>

## Function `scalar_add`

Returns the sum of the two scalars.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add">scalar_add</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add">scalar_add</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_add_internal">scalar_add_internal</a>(a.data, b.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_add_assign"></a>

## Function `scalar_add_assign`

Computes the sum of 'a' and 'b' and assigns the result to 'a'
Returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add_assign">scalar_add_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add_assign">scalar_add_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    a.data = <a href="ristretto255.md#0x1_ristretto255_scalar_add">scalar_add</a>(a, b).data;
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_sub"></a>

## Function `scalar_sub`

Returns the difference of the two scalars.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub">scalar_sub</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub">scalar_sub</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_sub_internal">scalar_sub_internal</a>(a.data, b.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_sub_assign"></a>

## Function `scalar_sub_assign`

Subtracts 'b' from 'a' and assigns the result to 'a'.
Returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub_assign">scalar_sub_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub_assign">scalar_sub_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    a.data = <a href="ristretto255.md#0x1_ristretto255_scalar_sub">scalar_sub</a>(a, b).data;
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_neg"></a>

## Function `scalar_neg`

Returns the negation of 'a': i.e., $(0 - a) \mod \ell$.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg">scalar_neg</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg">scalar_neg</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
        data: <a href="ristretto255.md#0x1_ristretto255_scalar_neg_internal">scalar_neg_internal</a>(a.data)
    }
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_neg_assign"></a>

## Function `scalar_neg_assign`

Replaces 'a' by its negation.
Returns 'a'.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg_assign">scalar_neg_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg_assign">scalar_neg_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a> {
    a.data = <a href="ristretto255.md#0x1_ristretto255_scalar_neg">scalar_neg</a>(a).data;
    a
}
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_to_bytes"></a>

## Function `scalar_to_bytes`

Returns the byte-representation of the scalar.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_to_bytes">scalar_to_bytes</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_to_bytes">scalar_to_bytes</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    s.data
}
</code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_sha512_internal"></a>

## Function `new_point_from_sha512_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha512_internal">new_point_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha512_internal">new_point_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_64_uniform_bytes_internal"></a>

## Function `new_point_from_64_uniform_bytes_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_64_uniform_bytes_internal">new_point_from_64_uniform_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_64_uniform_bytes_internal">new_point_from_64_uniform_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_point_is_canonical_internal"></a>

## Function `point_is_canonical_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_is_canonical_internal">point_is_canonical_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_is_canonical_internal">point_is_canonical_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="0x1_ristretto255_point_identity_internal"></a>

## Function `point_identity_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity_internal">point_identity_internal</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity_internal">point_identity_internal</a>(): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_point_decompress_internal"></a>

## Function `point_decompress_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_decompress_internal">point_decompress_internal</a>(maybe_non_canonical_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (u64, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_decompress_internal">point_decompress_internal</a>(maybe_non_canonical_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (u64, bool);
</code></pre>



</details>

<a id="0x1_ristretto255_point_clone_internal"></a>

## Function `point_clone_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_clone_internal">point_clone_internal</a>(point_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_clone_internal">point_clone_internal</a>(point_handle: u64): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_point_compress_internal"></a>

## Function `point_compress_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_compress_internal">point_compress_internal</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_compress_internal">point_compress_internal</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_point_mul_internal"></a>

## Function `point_mul_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul_internal">point_mul_internal</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, in_place: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul_internal">point_mul_internal</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, in_place: bool): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_basepoint_mul_internal"></a>

## Function `basepoint_mul_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_mul_internal">basepoint_mul_internal</a>(a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_mul_internal">basepoint_mul_internal</a>(a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_basepoint_double_mul_internal"></a>

## Function `basepoint_double_mul_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul_internal">basepoint_double_mul_internal</a>(a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, some_point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul_internal">basepoint_double_mul_internal</a>(a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, some_point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_point_add_internal"></a>

## Function `point_add_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add_internal">point_add_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, in_place: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add_internal">point_add_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, in_place: bool): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_point_sub_internal"></a>

## Function `point_sub_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub_internal">point_sub_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, in_place: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub_internal">point_sub_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, in_place: bool): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_point_neg_internal"></a>

## Function `point_neg_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg_internal">point_neg_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, in_place: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg_internal">point_neg_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">RistrettoPoint</a>, in_place: bool): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_double_scalar_mul_internal"></a>

## Function `double_scalar_mul_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul_internal">double_scalar_mul_internal</a>(point1: u64, point2: u64, scalar1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar2: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul_internal">double_scalar_mul_internal</a>(point1: u64, point2: u64, scalar1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar2: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_multi_scalar_mul_internal"></a>

## Function `multi_scalar_mul_internal`

The generic arguments are needed to deal with some Move VM peculiarities which prevent us from borrowing the
points (or scalars) inside a &vector in Rust.

WARNING: This function can only be called with P = RistrettoPoint and S = Scalar.


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;P, S&gt;(points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;P&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;S&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;P, S&gt;(points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;P&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;S&gt;): u64;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_is_canonical_internal"></a>

## Function `scalar_is_canonical_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_canonical_internal">scalar_is_canonical_internal</a>(s: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_canonical_internal">scalar_is_canonical_internal</a>(s: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_from_u64_internal"></a>

## Function `scalar_from_u64_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_u64_internal">scalar_from_u64_internal</a>(num: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_u64_internal">scalar_from_u64_internal</a>(num: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_from_u128_internal"></a>

## Function `scalar_from_u128_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_u128_internal">scalar_from_u128_internal</a>(num: u128): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_u128_internal">scalar_from_u128_internal</a>(num: u128): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_reduced_from_32_bytes_internal"></a>

## Function `scalar_reduced_from_32_bytes_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_reduced_from_32_bytes_internal">scalar_reduced_from_32_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_reduced_from_32_bytes_internal">scalar_reduced_from_32_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_uniform_from_64_bytes_internal"></a>

## Function `scalar_uniform_from_64_bytes_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_uniform_from_64_bytes_internal">scalar_uniform_from_64_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_uniform_from_64_bytes_internal">scalar_uniform_from_64_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_invert_internal"></a>

## Function `scalar_invert_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_invert_internal">scalar_invert_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_invert_internal">scalar_invert_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_from_sha512_internal"></a>

## Function `scalar_from_sha512_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_sha512_internal">scalar_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_sha512_internal">scalar_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul_internal">scalar_mul_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul_internal">scalar_mul_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_add_internal"></a>

## Function `scalar_add_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add_internal">scalar_add_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add_internal">scalar_add_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_sub_internal"></a>

## Function `scalar_sub_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub_internal">scalar_sub_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub_internal">scalar_sub_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_ristretto255_scalar_neg_internal"></a>

## Function `scalar_neg_internal`



<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg_internal">scalar_neg_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg_internal">scalar_neg_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_point_equals"></a>

### Function `point_equals`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_equals">point_equals</a>(g: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, h: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_double_scalar_mul"></a>

### Function `double_scalar_mul`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul">double_scalar_mul</a>(scalar1: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, point1: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, scalar2: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, point2: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_multi_scalar_mul"></a>

### Function `multi_scalar_mul`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul">multi_scalar_mul</a>(points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>




<pre><code><b>aborts_if</b> len(points) == 0;
<b>aborts_if</b> len(scalars) == 0;
<b>aborts_if</b> len(points) != len(scalars);
<b>ensures</b> result.handle == <a href="ristretto255.md#0x1_ristretto255_spec_multi_scalar_mul_internal">spec_multi_scalar_mul_internal</a>(points, scalars);
</code></pre>



<a id="@Specification_1_new_scalar_from_bytes"></a>

### Function `new_scalar_from_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">new_scalar_from_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_canonical_internal">spec_scalar_is_canonical_internal</a>(bytes) ==&gt; (std::option::spec_is_some(result)
    && std::option::spec_borrow(result).data == bytes);
<b>ensures</b> !<a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_canonical_internal">spec_scalar_is_canonical_internal</a>(bytes) ==&gt; std::option::spec_is_none(result);
</code></pre>



<a id="@Specification_1_new_scalar_from_sha2_512"></a>

### Function `new_scalar_from_sha2_512`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">new_scalar_from_sha2_512</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_sha512_internal">spec_scalar_from_sha512_internal</a>(sha2_512_input);
</code></pre>



<a id="@Specification_1_new_scalar_from_u8"></a>

### Function `new_scalar_from_u8`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u8">new_scalar_from_u8</a>(byte: u8): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data[0] == byte;
<b>ensures</b> <b>forall</b> i in 1..len(result.data): result.data[i] == 0;
</code></pre>



<a id="@Specification_1_new_scalar_from_u32"></a>

### Function `new_scalar_from_u32`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u32">new_scalar_from_u32</a>(four_bytes: u32): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u64_internal">spec_scalar_from_u64_internal</a>(four_bytes);
</code></pre>



<a id="@Specification_1_new_scalar_from_u64"></a>

### Function `new_scalar_from_u64`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u64">new_scalar_from_u64</a>(eight_bytes: u64): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u64_internal">spec_scalar_from_u64_internal</a>(eight_bytes);
</code></pre>



<a id="@Specification_1_new_scalar_from_u128"></a>

### Function `new_scalar_from_u128`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_from_u128">new_scalar_from_u128</a>(sixteen_bytes: u128): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u128_internal">spec_scalar_from_u128_internal</a>(sixteen_bytes);
</code></pre>



<a id="@Specification_1_new_scalar_reduced_from_32_bytes"></a>

### Function `new_scalar_reduced_from_32_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_reduced_from_32_bytes">new_scalar_reduced_from_32_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>




<pre><code><b>ensures</b> len(bytes) != 32 ==&gt; std::option::spec_is_none(result);
<b>ensures</b> len(bytes) == 32 ==&gt; std::option::spec_borrow(result).data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_reduced_from_32_bytes_internal">spec_scalar_reduced_from_32_bytes_internal</a>(bytes);
</code></pre>



<a id="@Specification_1_new_scalar_uniform_from_64_bytes"></a>

### Function `new_scalar_uniform_from_64_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_scalar_uniform_from_64_bytes">new_scalar_uniform_from_64_bytes</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>




<pre><code><b>ensures</b> len(bytes) != 64 ==&gt; std::option::spec_is_none(result);
<b>ensures</b> len(bytes) == 64 ==&gt; std::option::spec_borrow(result).data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_uniform_from_64_bytes_internal">spec_scalar_uniform_from_64_bytes_internal</a>(bytes);
</code></pre>



<a id="@Specification_1_scalar_zero"></a>

### Function `scalar_zero`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_zero">scalar_zero</a>(): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>ensures</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_zero">spec_scalar_is_zero</a>(result);
</code></pre>



<a id="@Specification_1_scalar_is_zero"></a>

### Function `scalar_is_zero`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_zero">scalar_is_zero</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): bool
</code></pre>




<pre><code><b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_zero">spec_scalar_is_zero</a>(s);
</code></pre>



<a id="@Specification_1_scalar_one"></a>

### Function `scalar_one`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_one">scalar_one</a>(): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>ensures</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_one">spec_scalar_is_one</a>(result);
</code></pre>



<a id="@Specification_1_scalar_is_one"></a>

### Function `scalar_is_one`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_one">scalar_is_one</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): bool
</code></pre>




<pre><code><b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_one">spec_scalar_is_one</a>(s);
</code></pre>



<a id="@Specification_1_scalar_equals"></a>

### Function `scalar_equals`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_equals">scalar_equals</a>(lhs: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, rhs: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == (lhs.data == rhs.data);
</code></pre>



<a id="@Specification_1_scalar_invert"></a>

### Function `scalar_invert`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_invert">scalar_invert</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_zero">spec_scalar_is_zero</a>(s) ==&gt; std::option::spec_is_none(result);
<b>ensures</b> !<a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_zero">spec_scalar_is_zero</a>(s) ==&gt; (std::option::spec_is_some(result) && std::option::spec_borrow(result).data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_invert_internal">spec_scalar_invert_internal</a>(s.data));
</code></pre>



<a id="@Specification_1_scalar_mul"></a>

### Function `scalar_mul`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul">scalar_mul</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_mul_internal">spec_scalar_mul_internal</a>(a.data, b.data);
</code></pre>



<a id="@Specification_1_scalar_mul_assign"></a>

### Function `scalar_mul_assign`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul_assign">scalar_mul_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_mul_internal">spec_scalar_mul_internal</a>(<b>old</b>(a).data, b.data);
</code></pre>



<a id="@Specification_1_scalar_add"></a>

### Function `scalar_add`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add">scalar_add</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_add_internal">spec_scalar_add_internal</a>(a.data, b.data);
</code></pre>



<a id="@Specification_1_scalar_add_assign"></a>

### Function `scalar_add_assign`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add_assign">scalar_add_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_add_internal">spec_scalar_add_internal</a>(<b>old</b>(a).data, b.data);
</code></pre>



<a id="@Specification_1_scalar_sub"></a>

### Function `scalar_sub`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub">scalar_sub</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_sub_internal">spec_scalar_sub_internal</a>(a.data, b.data);
</code></pre>



<a id="@Specification_1_scalar_sub_assign"></a>

### Function `scalar_sub_assign`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub_assign">scalar_sub_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, b: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_sub_internal">spec_scalar_sub_internal</a>(<b>old</b>(a).data, b.data);
</code></pre>



<a id="@Specification_1_scalar_neg"></a>

### Function `scalar_neg`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg">scalar_neg</a>(a: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_neg_internal">spec_scalar_neg_internal</a>(a.data);
</code></pre>



<a id="@Specification_1_scalar_neg_assign"></a>

### Function `scalar_neg_assign`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg_assign">scalar_neg_assign</a>(a: &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): &<b>mut</b> <a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a.data == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_neg_internal">spec_scalar_neg_internal</a>(<b>old</b>(a).data);
</code></pre>



<a id="@Specification_1_scalar_to_bytes"></a>

### Function `scalar_to_bytes`


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_to_bytes">scalar_to_bytes</a>(s: &<a href="ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == s.data;
</code></pre>



<a id="@Helper_functions_2"></a>

### Helper functions



<a id="0x1_ristretto255_spec_scalar_is_zero"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_zero">spec_scalar_is_zero</a>(s: <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): bool {
   s.data == x"0000000000000000000000000000000000000000000000000000000000000000"
}
</code></pre>




<a id="0x1_ristretto255_spec_scalar_is_one"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_one">spec_scalar_is_one</a>(s: <a href="ristretto255.md#0x1_ristretto255_Scalar">Scalar</a>): bool {
   s.data == x"0100000000000000000000000000000000000000000000000000000000000000"
}
</code></pre>




<a id="0x1_ristretto255_spec_point_is_canonical_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_point_is_canonical_internal">spec_point_is_canonical_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>




<a id="0x1_ristretto255_spec_double_scalar_mul_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_double_scalar_mul_internal">spec_double_scalar_mul_internal</a>(point1: u64, point2: u64, scalar1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar2: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>




<a id="0x1_ristretto255_spec_multi_scalar_mul_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_multi_scalar_mul_internal">spec_multi_scalar_mul_internal</a>&lt;P, S&gt;(points: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;P&gt;, scalars: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;S&gt;): u64;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_is_canonical_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_canonical_internal">spec_scalar_is_canonical_internal</a>(s: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_from_u64_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u64_internal">spec_scalar_from_u64_internal</a>(num: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_from_u128_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u128_internal">spec_scalar_from_u128_internal</a>(num: u128): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_reduced_from_32_bytes_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_reduced_from_32_bytes_internal">spec_scalar_reduced_from_32_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_uniform_from_64_bytes_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_uniform_from_64_bytes_internal">spec_scalar_uniform_from_64_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_invert_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_invert_internal">spec_scalar_invert_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_from_sha512_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_sha512_internal">spec_scalar_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_mul_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_mul_internal">spec_scalar_mul_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_add_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_add_internal">spec_scalar_add_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_sub_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_sub_internal">spec_scalar_sub_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>




<a id="0x1_ristretto255_spec_scalar_neg_internal"></a>


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_spec_scalar_neg_internal">spec_scalar_neg_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_1_new_point_from_sha512_internal"></a>

### Function `new_point_from_sha512_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_sha512_internal">new_point_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_new_point_from_64_uniform_bytes_internal"></a>

### Function `new_point_from_64_uniform_bytes_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_new_point_from_64_uniform_bytes_internal">new_point_from_64_uniform_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_is_canonical_internal"></a>

### Function `point_is_canonical_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_is_canonical_internal">point_is_canonical_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_point_is_canonical_internal">spec_point_is_canonical_internal</a>(bytes);
</code></pre>



<a id="@Specification_1_point_identity_internal"></a>

### Function `point_identity_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_identity_internal">point_identity_internal</a>(): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_decompress_internal"></a>

### Function `point_decompress_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_decompress_internal">point_decompress_internal</a>(maybe_non_canonical_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (u64, bool)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_clone_internal"></a>

### Function `point_clone_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_clone_internal">point_clone_internal</a>(point_handle: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_compress_internal"></a>

### Function `point_compress_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_compress_internal">point_compress_internal</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_mul_internal"></a>

### Function `point_mul_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_mul_internal">point_mul_internal</a>(point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, in_place: bool): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_basepoint_mul_internal"></a>

### Function `basepoint_mul_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_mul_internal">basepoint_mul_internal</a>(a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_basepoint_double_mul_internal"></a>

### Function `basepoint_double_mul_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_basepoint_double_mul_internal">basepoint_double_mul_internal</a>(a: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, some_point: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_add_internal"></a>

### Function `point_add_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_add_internal">point_add_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, in_place: bool): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_sub_internal"></a>

### Function `point_sub_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_sub_internal">point_sub_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, b: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, in_place: bool): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_point_neg_internal"></a>

### Function `point_neg_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_point_neg_internal">point_neg_internal</a>(a: &<a href="ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, in_place: bool): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_double_scalar_mul_internal"></a>

### Function `double_scalar_mul_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_double_scalar_mul_internal">double_scalar_mul_internal</a>(point1: u64, point2: u64, scalar1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar2: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_multi_scalar_mul_internal"></a>

### Function `multi_scalar_mul_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;P, S&gt;(points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;P&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;S&gt;): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_multi_scalar_mul_internal">spec_multi_scalar_mul_internal</a>&lt;P, S&gt;(points, scalars);
</code></pre>



<a id="@Specification_1_scalar_is_canonical_internal"></a>

### Function `scalar_is_canonical_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_is_canonical_internal">scalar_is_canonical_internal</a>(s: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_is_canonical_internal">spec_scalar_is_canonical_internal</a>(s);
</code></pre>



<a id="@Specification_1_scalar_from_u64_internal"></a>

### Function `scalar_from_u64_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_u64_internal">scalar_from_u64_internal</a>(num: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u64_internal">spec_scalar_from_u64_internal</a>(num);
</code></pre>



<a id="@Specification_1_scalar_from_u128_internal"></a>

### Function `scalar_from_u128_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_u128_internal">scalar_from_u128_internal</a>(num: u128): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_u128_internal">spec_scalar_from_u128_internal</a>(num);
</code></pre>



<a id="@Specification_1_scalar_reduced_from_32_bytes_internal"></a>

### Function `scalar_reduced_from_32_bytes_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_reduced_from_32_bytes_internal">scalar_reduced_from_32_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_reduced_from_32_bytes_internal">spec_scalar_reduced_from_32_bytes_internal</a>(bytes);
</code></pre>



<a id="@Specification_1_scalar_uniform_from_64_bytes_internal"></a>

### Function `scalar_uniform_from_64_bytes_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_uniform_from_64_bytes_internal">scalar_uniform_from_64_bytes_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_uniform_from_64_bytes_internal">spec_scalar_uniform_from_64_bytes_internal</a>(bytes);
</code></pre>



<a id="@Specification_1_scalar_invert_internal"></a>

### Function `scalar_invert_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_invert_internal">scalar_invert_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_invert_internal">spec_scalar_invert_internal</a>(bytes);
</code></pre>



<a id="@Specification_1_scalar_from_sha512_internal"></a>

### Function `scalar_from_sha512_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_from_sha512_internal">scalar_from_sha512_internal</a>(sha2_512_input: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_from_sha512_internal">spec_scalar_from_sha512_internal</a>(sha2_512_input);
</code></pre>



<a id="@Specification_1_scalar_mul_internal"></a>

### Function `scalar_mul_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_mul_internal">scalar_mul_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_mul_internal">spec_scalar_mul_internal</a>(a_bytes, b_bytes);
</code></pre>



<a id="@Specification_1_scalar_add_internal"></a>

### Function `scalar_add_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_add_internal">scalar_add_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_add_internal">spec_scalar_add_internal</a>(a_bytes, b_bytes);
</code></pre>



<a id="@Specification_1_scalar_sub_internal"></a>

### Function `scalar_sub_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_sub_internal">scalar_sub_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, b_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_sub_internal">spec_scalar_sub_internal</a>(a_bytes, b_bytes);
</code></pre>



<a id="@Specification_1_scalar_neg_internal"></a>

### Function `scalar_neg_internal`


<pre><code><b>fun</b> <a href="ristretto255.md#0x1_ristretto255_scalar_neg_internal">scalar_neg_internal</a>(a_bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="ristretto255.md#0x1_ristretto255_spec_scalar_neg_internal">spec_scalar_neg_internal</a>(a_bytes);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
