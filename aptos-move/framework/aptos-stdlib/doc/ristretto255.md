
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
    -  [Helper functions](#@Helper_functions_2)
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


<pre><code>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::option;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_ristretto255_Scalar"></a>

## Struct `Scalar`

This struct represents a scalar as a little-endian byte encoding of an integer in $\mathbb{Z}_\ell$, which is
stored in <code>data</code>. Here, \ell denotes the order of the scalar field (and the underlying elliptic curve group).


<pre><code>struct Scalar has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: vector&lt;u8&gt;</code>
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


<pre><code>struct CompressedRistretto has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: vector&lt;u8&gt;</code>
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


<pre><code>struct RistrettoPoint has drop<br/></code></pre>



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


<pre><code>const E_NATIVE_FUN_NOT_AVAILABLE: u64 &#61; 5;<br/></code></pre>



<a id="0x1_ristretto255_BASE_POINT"></a>

The basepoint (generator) of the Ristretto255 group


<pre><code>const BASE_POINT: vector&lt;u8&gt; &#61; [226, 242, 174, 10, 106, 188, 78, 113, 168, 132, 169, 97, 197, 0, 81, 95, 88, 227, 11, 106, 165, 130, 221, 141, 182, 166, 89, 69, 224, 141, 45, 118];<br/></code></pre>



<a id="0x1_ristretto255_E_DIFFERENT_NUM_POINTS_AND_SCALARS"></a>

The number of scalars does not match the number of points.


<pre><code>const E_DIFFERENT_NUM_POINTS_AND_SCALARS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_ristretto255_E_TOO_MANY_POINTS_CREATED"></a>

Too many points have been created in the current transaction execution.


<pre><code>const E_TOO_MANY_POINTS_CREATED: u64 &#61; 4;<br/></code></pre>



<a id="0x1_ristretto255_E_ZERO_POINTS"></a>

Expected more than zero points as input.


<pre><code>const E_ZERO_POINTS: u64 &#61; 2;<br/></code></pre>



<a id="0x1_ristretto255_E_ZERO_SCALARS"></a>

Expected more than zero scalars as input.


<pre><code>const E_ZERO_SCALARS: u64 &#61; 3;<br/></code></pre>



<a id="0x1_ristretto255_HASH_BASE_POINT"></a>

The hash of the basepoint of the Ristretto255 group using SHA3_512


<pre><code>const HASH_BASE_POINT: vector&lt;u8&gt; &#61; [140, 146, 64, 180, 86, 169, 230, 220, 101, 195, 119, 161, 4, 141, 116, 95, 148, 160, 140, 219, 127, 68, 203, 205, 123, 70, 243, 64, 72, 135, 17, 52];<br/></code></pre>



<a id="0x1_ristretto255_L_MINUS_ONE"></a>

<code>ORDER_ELL</code> - 1: i.e., the "largest", reduced scalar in the field


<pre><code>const L_MINUS_ONE: vector&lt;u8&gt; &#61; [236, 211, 245, 92, 26, 99, 18, 88, 214, 156, 247, 162, 222, 249, 222, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16];<br/></code></pre>



<a id="0x1_ristretto255_MAX_POINT_NUM_BYTES"></a>

The maximum size in bytes of a canonically-encoded Ristretto255 point is 32 bytes.


<pre><code>const MAX_POINT_NUM_BYTES: u64 &#61; 32;<br/></code></pre>



<a id="0x1_ristretto255_MAX_SCALAR_NUM_BITS"></a>

The maximum size in bits of a canonically-encoded Scalar is 256 bits.


<pre><code>const MAX_SCALAR_NUM_BITS: u64 &#61; 256;<br/></code></pre>



<a id="0x1_ristretto255_MAX_SCALAR_NUM_BYTES"></a>

The maximum size in bytes of a canonically-encoded Scalar is 32 bytes.


<pre><code>const MAX_SCALAR_NUM_BYTES: u64 &#61; 32;<br/></code></pre>



<a id="0x1_ristretto255_ORDER_ELL"></a>

The order of the Ristretto255 group and its scalar field, in little-endian.


<pre><code>const ORDER_ELL: vector&lt;u8&gt; &#61; [237, 211, 245, 92, 26, 99, 18, 88, 214, 156, 247, 162, 222, 249, 222, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16];<br/></code></pre>



<a id="0x1_ristretto255_point_identity_compressed"></a>

## Function `point_identity_compressed`

Returns the identity point as a CompressedRistretto.


<pre><code>public fun point_identity_compressed(): ristretto255::CompressedRistretto<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_identity_compressed(): CompressedRistretto &#123;<br/>    CompressedRistretto &#123;<br/>        data: x&quot;0000000000000000000000000000000000000000000000000000000000000000&quot;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_identity"></a>

## Function `point_identity`

Returns the identity point as a CompressedRistretto.


<pre><code>public fun point_identity(): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_identity(): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: point_identity_internal()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_basepoint_compressed"></a>

## Function `basepoint_compressed`

Returns the basepoint (generator) of the Ristretto255 group as a compressed point


<pre><code>public fun basepoint_compressed(): ristretto255::CompressedRistretto<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun basepoint_compressed(): CompressedRistretto &#123;<br/>    CompressedRistretto &#123;<br/>        data: BASE_POINT<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_hash_to_point_base"></a>

## Function `hash_to_point_base`

Returns the hash-to-point result of serializing the basepoint of the Ristretto255 group.
For use as the random value basepoint in Pedersen commitments


<pre><code>public fun hash_to_point_base(): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun hash_to_point_base(): RistrettoPoint &#123;<br/>    let comp_res &#61; CompressedRistretto &#123; data: HASH_BASE_POINT &#125;;<br/>    point_decompress(&amp;comp_res)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_basepoint"></a>

## Function `basepoint`

Returns the basepoint (generator) of the Ristretto255 group


<pre><code>public fun basepoint(): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun basepoint(): RistrettoPoint &#123;<br/>    let (handle, _) &#61; point_decompress_internal(BASE_POINT);<br/><br/>    RistrettoPoint &#123;<br/>        handle<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_basepoint_mul"></a>

## Function `basepoint_mul`

Multiplies the basepoint (generator) of the Ristretto255 group by a scalar and returns the result.
This call is much faster than <code>point_mul(&amp;basepoint(), &amp;some_scalar)</code> because of precomputation tables.


<pre><code>public fun basepoint_mul(a: &amp;ristretto255::Scalar): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun basepoint_mul(a: &amp;Scalar): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: basepoint_mul_internal(a.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_compressed_point_from_bytes"></a>

## Function `new_compressed_point_from_bytes`

Creates a new CompressedRistretto point from a sequence of 32 bytes. If those bytes do not represent a valid
point, returns None.


<pre><code>public fun new_compressed_point_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::CompressedRistretto&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_compressed_point_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;CompressedRistretto&gt; &#123;<br/>    if (point_is_canonical_internal(bytes)) &#123;<br/>        std::option::some(CompressedRistretto &#123;<br/>            data: bytes<br/>        &#125;)<br/>    &#125; else &#123;<br/>        std::option::none&lt;CompressedRistretto&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_bytes"></a>

## Function `new_point_from_bytes`

Creates a new RistrettoPoint from a sequence of 32 bytes. If those bytes do not represent a valid point,
returns None.


<pre><code>public fun new_point_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::RistrettoPoint&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_point_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;RistrettoPoint&gt; &#123;<br/>    let (handle, is_canonical) &#61; point_decompress_internal(bytes);<br/>    if (is_canonical) &#123;<br/>        std::option::some(RistrettoPoint &#123; handle &#125;)<br/>    &#125; else &#123;<br/>        std::option::none&lt;RistrettoPoint&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_compressed_point_to_bytes"></a>

## Function `compressed_point_to_bytes`

Given a compressed ristretto point <code>point</code>, returns the byte representation of that point


<pre><code>public fun compressed_point_to_bytes(point: ristretto255::CompressedRistretto): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun compressed_point_to_bytes(point: CompressedRistretto): vector&lt;u8&gt; &#123;<br/>    point.data<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_sha512"></a>

## Function `new_point_from_sha512`

DEPRECATED: Use the more clearly-named <code>new_point_from_sha2_512</code>

Hashes the input to a uniformly-at-random RistrettoPoint via SHA512.


<pre><code>public fun new_point_from_sha512(sha2_512_input: vector&lt;u8&gt;): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_point_from_sha512(sha2_512_input: vector&lt;u8&gt;): RistrettoPoint &#123;<br/>    new_point_from_sha2_512(sha2_512_input)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_sha2_512"></a>

## Function `new_point_from_sha2_512`

Hashes the input to a uniformly-at-random RistrettoPoint via SHA2-512.


<pre><code>public fun new_point_from_sha2_512(sha2_512_input: vector&lt;u8&gt;): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_point_from_sha2_512(sha2_512_input: vector&lt;u8&gt;): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: new_point_from_sha512_internal(sha2_512_input)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_64_uniform_bytes"></a>

## Function `new_point_from_64_uniform_bytes`

Samples a uniformly-at-random RistrettoPoint given a sequence of 64 uniformly-at-random bytes. This function
can be used to build a collision-resistant hash function that maps 64-byte messages to RistrettoPoint's.


<pre><code>public fun new_point_from_64_uniform_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::RistrettoPoint&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_point_from_64_uniform_bytes(bytes: vector&lt;u8&gt;): Option&lt;RistrettoPoint&gt; &#123;<br/>    if (std::vector::length(&amp;bytes) &#61;&#61; 64) &#123;<br/>        std::option::some(RistrettoPoint &#123;<br/>            handle: new_point_from_64_uniform_bytes_internal(bytes)<br/>        &#125;)<br/>    &#125; else &#123;<br/>        std::option::none&lt;RistrettoPoint&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_decompress"></a>

## Function `point_decompress`

Decompresses a CompressedRistretto from storage into a RistrettoPoint which can be used for fast arithmetic.


<pre><code>public fun point_decompress(point: &amp;ristretto255::CompressedRistretto): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_decompress(point: &amp;CompressedRistretto): RistrettoPoint &#123;<br/>    // NOTE: Our CompressedRistretto invariant assures us that every CompressedRistretto in storage is a valid<br/>    // RistrettoPoint<br/>    let (handle, _) &#61; point_decompress_internal(point.data);<br/>    RistrettoPoint &#123; handle &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_clone"></a>

## Function `point_clone`

Clones a RistrettoPoint.


<pre><code>public fun point_clone(point: &amp;ristretto255::RistrettoPoint): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_clone(point: &amp;RistrettoPoint): RistrettoPoint &#123;<br/>    if(!features::bulletproofs_enabled()) &#123;<br/>        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))<br/>    &#125;;<br/><br/>    RistrettoPoint &#123;<br/>        handle: point_clone_internal(point.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_compress"></a>

## Function `point_compress`

Compresses a RistrettoPoint to a CompressedRistretto which can be put in storage.


<pre><code>public fun point_compress(point: &amp;ristretto255::RistrettoPoint): ristretto255::CompressedRistretto<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_compress(point: &amp;RistrettoPoint): CompressedRistretto &#123;<br/>    CompressedRistretto &#123;<br/>        data: point_compress_internal(point)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_to_bytes"></a>

## Function `point_to_bytes`

Returns the sequence of bytes representin this Ristretto point.
To convert a RistrettoPoint 'p' to bytes, first compress it via <code>c &#61; point_compress(&amp;p)</code>, and then call this
function on <code>c</code>.


<pre><code>public fun point_to_bytes(point: &amp;ristretto255::CompressedRistretto): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_to_bytes(point: &amp;CompressedRistretto): vector&lt;u8&gt; &#123;<br/>    point.data<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_mul"></a>

## Function `point_mul`

Returns a * point.


<pre><code>public fun point_mul(point: &amp;ristretto255::RistrettoPoint, a: &amp;ristretto255::Scalar): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_mul(point: &amp;RistrettoPoint, a: &amp;Scalar): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: point_mul_internal(point, a.data, false)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_mul_assign"></a>

## Function `point_mul_assign`

Sets a *= point and returns 'a'.


<pre><code>public fun point_mul_assign(point: &amp;mut ristretto255::RistrettoPoint, a: &amp;ristretto255::Scalar): &amp;mut ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_mul_assign(point: &amp;mut RistrettoPoint, a: &amp;Scalar): &amp;mut RistrettoPoint &#123;<br/>    point_mul_internal(point, a.data, true);<br/>    point<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_basepoint_double_mul"></a>

## Function `basepoint_double_mul`

Returns (a * a_base + b * base_point), where base_point is the Ristretto basepoint encoded in <code>BASE_POINT</code>.


<pre><code>public fun basepoint_double_mul(a: &amp;ristretto255::Scalar, a_base: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::Scalar): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun basepoint_double_mul(a: &amp;Scalar, a_base: &amp;RistrettoPoint, b: &amp;Scalar): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: basepoint_double_mul_internal(a.data, a_base, b.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_add"></a>

## Function `point_add`

Returns a + b


<pre><code>public fun point_add(a: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_add(a: &amp;RistrettoPoint, b: &amp;RistrettoPoint): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: point_add_internal(a, b, false)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_add_assign"></a>

## Function `point_add_assign`

Sets a += b and returns 'a'.


<pre><code>public fun point_add_assign(a: &amp;mut ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint): &amp;mut ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_add_assign(a: &amp;mut RistrettoPoint, b: &amp;RistrettoPoint): &amp;mut RistrettoPoint &#123;<br/>    point_add_internal(a, b, true);<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_sub"></a>

## Function `point_sub`

Returns a - b


<pre><code>public fun point_sub(a: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_sub(a: &amp;RistrettoPoint, b: &amp;RistrettoPoint): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: point_sub_internal(a, b, false)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_sub_assign"></a>

## Function `point_sub_assign`

Sets a -= b and returns 'a'.


<pre><code>public fun point_sub_assign(a: &amp;mut ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint): &amp;mut ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_sub_assign(a: &amp;mut RistrettoPoint, b: &amp;RistrettoPoint): &amp;mut RistrettoPoint &#123;<br/>    point_sub_internal(a, b, true);<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_neg"></a>

## Function `point_neg`

Returns -a


<pre><code>public fun point_neg(a: &amp;ristretto255::RistrettoPoint): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_neg(a: &amp;RistrettoPoint): RistrettoPoint &#123;<br/>    RistrettoPoint &#123;<br/>        handle: point_neg_internal(a, false)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_neg_assign"></a>

## Function `point_neg_assign`

Sets a = -a, and returns 'a'.


<pre><code>public fun point_neg_assign(a: &amp;mut ristretto255::RistrettoPoint): &amp;mut ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun point_neg_assign(a: &amp;mut RistrettoPoint): &amp;mut RistrettoPoint &#123;<br/>    point_neg_internal(a, true);<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_equals"></a>

## Function `point_equals`

Returns true if the two RistrettoPoints are the same points on the elliptic curve.


<pre><code>public fun point_equals(g: &amp;ristretto255::RistrettoPoint, h: &amp;ristretto255::RistrettoPoint): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun point_equals(g: &amp;RistrettoPoint, h: &amp;RistrettoPoint): bool;<br/></code></pre>



</details>

<a id="0x1_ristretto255_double_scalar_mul"></a>

## Function `double_scalar_mul`

Computes a double-scalar multiplication, returning a_1 p_1 + a_2 p_2
This function is much faster than computing each a_i p_i using <code>point_mul</code> and adding up the results using <code>point_add</code>.


<pre><code>public fun double_scalar_mul(scalar1: &amp;ristretto255::Scalar, point1: &amp;ristretto255::RistrettoPoint, scalar2: &amp;ristretto255::Scalar, point2: &amp;ristretto255::RistrettoPoint): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun double_scalar_mul(scalar1: &amp;Scalar, point1: &amp;RistrettoPoint, scalar2: &amp;Scalar, point2: &amp;RistrettoPoint): RistrettoPoint &#123;<br/>    if(!features::bulletproofs_enabled()) &#123;<br/>        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))<br/>    &#125;;<br/><br/>    RistrettoPoint &#123;<br/>        handle: double_scalar_mul_internal(point1.handle, point2.handle, scalar1.data, scalar2.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_multi_scalar_mul"></a>

## Function `multi_scalar_mul`

Computes a multi-scalar multiplication, returning a_1 p_1 + a_2 p_2 + ... + a_n p_n.
This function is much faster than computing each a_i p_i using <code>point_mul</code> and adding up the results using <code>point_add</code>.


<pre><code>public fun multi_scalar_mul(points: &amp;vector&lt;ristretto255::RistrettoPoint&gt;, scalars: &amp;vector&lt;ristretto255::Scalar&gt;): ristretto255::RistrettoPoint<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multi_scalar_mul(points: &amp;vector&lt;RistrettoPoint&gt;, scalars: &amp;vector&lt;Scalar&gt;): RistrettoPoint &#123;<br/>    assert!(!std::vector::is_empty(points), std::error::invalid_argument(E_ZERO_POINTS));<br/>    assert!(!std::vector::is_empty(scalars), std::error::invalid_argument(E_ZERO_SCALARS));<br/>    assert!(std::vector::length(points) &#61;&#61; std::vector::length(scalars), std::error::invalid_argument(E_DIFFERENT_NUM_POINTS_AND_SCALARS));<br/><br/>    RistrettoPoint &#123;<br/>        handle: multi_scalar_mul_internal&lt;RistrettoPoint, Scalar&gt;(points, scalars)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_bytes"></a>

## Function `new_scalar_from_bytes`

Given a sequence of 32 bytes, checks if they canonically-encode a Scalar and return it.
Otherwise, returns None.


<pre><code>public fun new_scalar_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_bytes(bytes: vector&lt;u8&gt;): Option&lt;Scalar&gt; &#123;<br/>    if (scalar_is_canonical_internal(bytes)) &#123;<br/>        std::option::some(Scalar &#123;<br/>            data: bytes<br/>        &#125;)<br/>    &#125; else &#123;<br/>        std::option::none&lt;Scalar&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_sha512"></a>

## Function `new_scalar_from_sha512`

DEPRECATED: Use the more clearly-named <code>new_scalar_from_sha2_512</code>

Hashes the input to a uniformly-at-random Scalar via SHA2-512


<pre><code>public fun new_scalar_from_sha512(sha2_512_input: vector&lt;u8&gt;): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_sha512(sha2_512_input: vector&lt;u8&gt;): Scalar &#123;<br/>    new_scalar_from_sha2_512(sha2_512_input)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_sha2_512"></a>

## Function `new_scalar_from_sha2_512`

Hashes the input to a uniformly-at-random Scalar via SHA2-512


<pre><code>public fun new_scalar_from_sha2_512(sha2_512_input: vector&lt;u8&gt;): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_sha2_512(sha2_512_input: vector&lt;u8&gt;): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_from_sha512_internal(sha2_512_input)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u8"></a>

## Function `new_scalar_from_u8`

Creates a Scalar from an u8.


<pre><code>public fun new_scalar_from_u8(byte: u8): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_u8(byte: u8): Scalar &#123;<br/>    let s &#61; scalar_zero();<br/>    let byte_zero &#61; std::vector::borrow_mut(&amp;mut s.data, 0);<br/>    &#42;byte_zero &#61; byte;<br/><br/>    s<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u32"></a>

## Function `new_scalar_from_u32`

Creates a Scalar from an u32.


<pre><code>public fun new_scalar_from_u32(four_bytes: u32): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_u32(four_bytes: u32): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_from_u64_internal((four_bytes as u64))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u64"></a>

## Function `new_scalar_from_u64`

Creates a Scalar from an u64.


<pre><code>public fun new_scalar_from_u64(eight_bytes: u64): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_u64(eight_bytes: u64): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_from_u64_internal(eight_bytes)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_from_u128"></a>

## Function `new_scalar_from_u128`

Creates a Scalar from an u128.


<pre><code>public fun new_scalar_from_u128(sixteen_bytes: u128): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_from_u128(sixteen_bytes: u128): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_from_u128_internal(sixteen_bytes)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_reduced_from_32_bytes"></a>

## Function `new_scalar_reduced_from_32_bytes`

Creates a Scalar from 32 bytes by reducing the little-endian-encoded number in those bytes modulo $\ell$.


<pre><code>public fun new_scalar_reduced_from_32_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_reduced_from_32_bytes(bytes: vector&lt;u8&gt;): Option&lt;Scalar&gt; &#123;<br/>    if (std::vector::length(&amp;bytes) &#61;&#61; 32) &#123;<br/>        std::option::some(Scalar &#123;<br/>            data: scalar_reduced_from_32_bytes_internal(bytes)<br/>        &#125;)<br/>    &#125; else &#123;<br/>        std::option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_scalar_uniform_from_64_bytes"></a>

## Function `new_scalar_uniform_from_64_bytes`

Samples a scalar uniformly-at-random given 64 uniform-at-random bytes as input by reducing the little-endian-encoded number
in those bytes modulo $\ell$.


<pre><code>public fun new_scalar_uniform_from_64_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_scalar_uniform_from_64_bytes(bytes: vector&lt;u8&gt;): Option&lt;Scalar&gt; &#123;<br/>    if (std::vector::length(&amp;bytes) &#61;&#61; 64) &#123;<br/>        std::option::some(Scalar &#123;<br/>            data: scalar_uniform_from_64_bytes_internal(bytes)<br/>        &#125;)<br/>    &#125; else &#123;<br/>        std::option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_zero"></a>

## Function `scalar_zero`

Returns 0 as a Scalar.


<pre><code>public fun scalar_zero(): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_zero(): Scalar &#123;<br/>    Scalar &#123;<br/>        data: x&quot;0000000000000000000000000000000000000000000000000000000000000000&quot;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_is_zero"></a>

## Function `scalar_is_zero`

Returns true if the given Scalar equals 0.


<pre><code>public fun scalar_is_zero(s: &amp;ristretto255::Scalar): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_is_zero(s: &amp;Scalar): bool &#123;<br/>    s.data &#61;&#61; x&quot;0000000000000000000000000000000000000000000000000000000000000000&quot;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_one"></a>

## Function `scalar_one`

Returns 1 as a Scalar.


<pre><code>public fun scalar_one(): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_one(): Scalar &#123;<br/>    Scalar &#123;<br/>        data: x&quot;0100000000000000000000000000000000000000000000000000000000000000&quot;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_is_one"></a>

## Function `scalar_is_one`

Returns true if the given Scalar equals 1.


<pre><code>public fun scalar_is_one(s: &amp;ristretto255::Scalar): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_is_one(s: &amp;Scalar): bool &#123;<br/>    s.data &#61;&#61; x&quot;0100000000000000000000000000000000000000000000000000000000000000&quot;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_equals"></a>

## Function `scalar_equals`

Returns true if the two scalars are equal.


<pre><code>public fun scalar_equals(lhs: &amp;ristretto255::Scalar, rhs: &amp;ristretto255::Scalar): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_equals(lhs: &amp;Scalar, rhs: &amp;Scalar): bool &#123;<br/>    lhs.data &#61;&#61; rhs.data<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_invert"></a>

## Function `scalar_invert`

Returns the inverse s^{-1} mod \ell of a scalar s.
Returns None if s is zero.


<pre><code>public fun scalar_invert(s: &amp;ristretto255::Scalar): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_invert(s: &amp;Scalar): Option&lt;Scalar&gt; &#123;<br/>    if (scalar_is_zero(s)) &#123;<br/>        std::option::none&lt;Scalar&gt;()<br/>    &#125; else &#123;<br/>        std::option::some(Scalar &#123;<br/>            data: scalar_invert_internal(s.data)<br/>        &#125;)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_mul"></a>

## Function `scalar_mul`

Returns the product of the two scalars.


<pre><code>public fun scalar_mul(a: &amp;ristretto255::Scalar, b: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_mul(a: &amp;Scalar, b: &amp;Scalar): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_mul_internal(a.data, b.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_mul_assign"></a>

## Function `scalar_mul_assign`

Computes the product of 'a' and 'b' and assigns the result to 'a'.
Returns 'a'.


<pre><code>public fun scalar_mul_assign(a: &amp;mut ristretto255::Scalar, b: &amp;ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_mul_assign(a: &amp;mut Scalar, b: &amp;Scalar): &amp;mut Scalar &#123;<br/>    a.data &#61; scalar_mul(a, b).data;<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_add"></a>

## Function `scalar_add`

Returns the sum of the two scalars.


<pre><code>public fun scalar_add(a: &amp;ristretto255::Scalar, b: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_add(a: &amp;Scalar, b: &amp;Scalar): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_add_internal(a.data, b.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_add_assign"></a>

## Function `scalar_add_assign`

Computes the sum of 'a' and 'b' and assigns the result to 'a'
Returns 'a'.


<pre><code>public fun scalar_add_assign(a: &amp;mut ristretto255::Scalar, b: &amp;ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_add_assign(a: &amp;mut Scalar, b: &amp;Scalar): &amp;mut Scalar &#123;<br/>    a.data &#61; scalar_add(a, b).data;<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_sub"></a>

## Function `scalar_sub`

Returns the difference of the two scalars.


<pre><code>public fun scalar_sub(a: &amp;ristretto255::Scalar, b: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_sub(a: &amp;Scalar, b: &amp;Scalar): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_sub_internal(a.data, b.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_sub_assign"></a>

## Function `scalar_sub_assign`

Subtracts 'b' from 'a' and assigns the result to 'a'.
Returns 'a'.


<pre><code>public fun scalar_sub_assign(a: &amp;mut ristretto255::Scalar, b: &amp;ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_sub_assign(a: &amp;mut Scalar, b: &amp;Scalar): &amp;mut Scalar &#123;<br/>    a.data &#61; scalar_sub(a, b).data;<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_neg"></a>

## Function `scalar_neg`

Returns the negation of 'a': i.e., $(0 - a) \mod \ell$.


<pre><code>public fun scalar_neg(a: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_neg(a: &amp;Scalar): Scalar &#123;<br/>    Scalar &#123;<br/>        data: scalar_neg_internal(a.data)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_neg_assign"></a>

## Function `scalar_neg_assign`

Replaces 'a' by its negation.
Returns 'a'.


<pre><code>public fun scalar_neg_assign(a: &amp;mut ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_neg_assign(a: &amp;mut Scalar): &amp;mut Scalar &#123;<br/>    a.data &#61; scalar_neg(a).data;<br/>    a<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_to_bytes"></a>

## Function `scalar_to_bytes`

Returns the byte-representation of the scalar.


<pre><code>public fun scalar_to_bytes(s: &amp;ristretto255::Scalar): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_to_bytes(s: &amp;Scalar): vector&lt;u8&gt; &#123;<br/>    s.data<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_sha512_internal"></a>

## Function `new_point_from_sha512_internal`



<pre><code>fun new_point_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun new_point_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_new_point_from_64_uniform_bytes_internal"></a>

## Function `new_point_from_64_uniform_bytes_internal`



<pre><code>fun new_point_from_64_uniform_bytes_internal(bytes: vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun new_point_from_64_uniform_bytes_internal(bytes: vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_is_canonical_internal"></a>

## Function `point_is_canonical_internal`



<pre><code>fun point_is_canonical_internal(bytes: vector&lt;u8&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_is_canonical_internal(bytes: vector&lt;u8&gt;): bool;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_identity_internal"></a>

## Function `point_identity_internal`



<pre><code>fun point_identity_internal(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_identity_internal(): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_decompress_internal"></a>

## Function `point_decompress_internal`



<pre><code>fun point_decompress_internal(maybe_non_canonical_bytes: vector&lt;u8&gt;): (u64, bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_decompress_internal(maybe_non_canonical_bytes: vector&lt;u8&gt;): (u64, bool);<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_clone_internal"></a>

## Function `point_clone_internal`



<pre><code>fun point_clone_internal(point_handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_clone_internal(point_handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_compress_internal"></a>

## Function `point_compress_internal`



<pre><code>fun point_compress_internal(point: &amp;ristretto255::RistrettoPoint): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_compress_internal(point: &amp;RistrettoPoint): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_mul_internal"></a>

## Function `point_mul_internal`



<pre><code>fun point_mul_internal(point: &amp;ristretto255::RistrettoPoint, a: vector&lt;u8&gt;, in_place: bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_mul_internal(point: &amp;RistrettoPoint, a: vector&lt;u8&gt;, in_place: bool): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_basepoint_mul_internal"></a>

## Function `basepoint_mul_internal`



<pre><code>fun basepoint_mul_internal(a: vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun basepoint_mul_internal(a: vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_basepoint_double_mul_internal"></a>

## Function `basepoint_double_mul_internal`



<pre><code>fun basepoint_double_mul_internal(a: vector&lt;u8&gt;, some_point: &amp;ristretto255::RistrettoPoint, b: vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun basepoint_double_mul_internal(a: vector&lt;u8&gt;, some_point: &amp;RistrettoPoint, b: vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_add_internal"></a>

## Function `point_add_internal`



<pre><code>fun point_add_internal(a: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint, in_place: bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_add_internal(a: &amp;RistrettoPoint, b: &amp;RistrettoPoint, in_place: bool): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_sub_internal"></a>

## Function `point_sub_internal`



<pre><code>fun point_sub_internal(a: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint, in_place: bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_sub_internal(a: &amp;RistrettoPoint, b: &amp;RistrettoPoint, in_place: bool): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_point_neg_internal"></a>

## Function `point_neg_internal`



<pre><code>fun point_neg_internal(a: &amp;ristretto255::RistrettoPoint, in_place: bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun point_neg_internal(a: &amp;RistrettoPoint, in_place: bool): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_double_scalar_mul_internal"></a>

## Function `double_scalar_mul_internal`



<pre><code>fun double_scalar_mul_internal(point1: u64, point2: u64, scalar1: vector&lt;u8&gt;, scalar2: vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun double_scalar_mul_internal(point1: u64, point2: u64, scalar1: vector&lt;u8&gt;, scalar2: vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_multi_scalar_mul_internal"></a>

## Function `multi_scalar_mul_internal`

The generic arguments are needed to deal with some Move VM peculiarities which prevent us from borrowing the
points (or scalars) inside a &vector in Rust.

WARNING: This function can only be called with P = RistrettoPoint and S = Scalar.


<pre><code>fun multi_scalar_mul_internal&lt;P, S&gt;(points: &amp;vector&lt;P&gt;, scalars: &amp;vector&lt;S&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun multi_scalar_mul_internal&lt;P, S&gt;(points: &amp;vector&lt;P&gt;, scalars: &amp;vector&lt;S&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_is_canonical_internal"></a>

## Function `scalar_is_canonical_internal`



<pre><code>fun scalar_is_canonical_internal(s: vector&lt;u8&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_is_canonical_internal(s: vector&lt;u8&gt;): bool;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_from_u64_internal"></a>

## Function `scalar_from_u64_internal`



<pre><code>fun scalar_from_u64_internal(num: u64): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_from_u64_internal(num: u64): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_from_u128_internal"></a>

## Function `scalar_from_u128_internal`



<pre><code>fun scalar_from_u128_internal(num: u128): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_from_u128_internal(num: u128): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_reduced_from_32_bytes_internal"></a>

## Function `scalar_reduced_from_32_bytes_internal`



<pre><code>fun scalar_reduced_from_32_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_reduced_from_32_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_uniform_from_64_bytes_internal"></a>

## Function `scalar_uniform_from_64_bytes_internal`



<pre><code>fun scalar_uniform_from_64_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_uniform_from_64_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_invert_internal"></a>

## Function `scalar_invert_internal`



<pre><code>fun scalar_invert_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_invert_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_from_sha512_internal"></a>

## Function `scalar_from_sha512_internal`



<pre><code>fun scalar_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code>fun scalar_mul_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_mul_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_add_internal"></a>

## Function `scalar_add_internal`



<pre><code>fun scalar_add_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_add_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_sub_internal"></a>

## Function `scalar_sub_internal`



<pre><code>fun scalar_sub_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_sub_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_ristretto255_scalar_neg_internal"></a>

## Function `scalar_neg_internal`



<pre><code>fun scalar_neg_internal(a_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_neg_internal(a_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Helper_functions_2"></a>

### Helper functions



<a id="0x1_ristretto255_spec_scalar_is_zero"></a>


<pre><code>fun spec_scalar_is_zero(s: Scalar): bool &#123;<br/>   s.data &#61;&#61; x&quot;0000000000000000000000000000000000000000000000000000000000000000&quot;<br/>&#125;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_is_one"></a>


<pre><code>fun spec_scalar_is_one(s: Scalar): bool &#123;<br/>   s.data &#61;&#61; x&quot;0100000000000000000000000000000000000000000000000000000000000000&quot;<br/>&#125;<br/></code></pre>




<a id="0x1_ristretto255_spec_point_is_canonical_internal"></a>


<pre><code>fun spec_point_is_canonical_internal(bytes: vector&lt;u8&gt;): bool;<br/></code></pre>




<a id="0x1_ristretto255_spec_double_scalar_mul_internal"></a>


<pre><code>fun spec_double_scalar_mul_internal(point1: u64, point2: u64, scalar1: vector&lt;u8&gt;, scalar2: vector&lt;u8&gt;): u64;<br/></code></pre>




<a id="0x1_ristretto255_spec_multi_scalar_mul_internal"></a>


<pre><code>fun spec_multi_scalar_mul_internal&lt;P, S&gt;(points: vector&lt;P&gt;, scalars: vector&lt;S&gt;): u64;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_is_canonical_internal"></a>


<pre><code>fun spec_scalar_is_canonical_internal(s: vector&lt;u8&gt;): bool;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_from_u64_internal"></a>


<pre><code>fun spec_scalar_from_u64_internal(num: u64): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_from_u128_internal"></a>


<pre><code>fun spec_scalar_from_u128_internal(num: u128): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_reduced_from_32_bytes_internal"></a>


<pre><code>fun spec_scalar_reduced_from_32_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_uniform_from_64_bytes_internal"></a>


<pre><code>fun spec_scalar_uniform_from_64_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_invert_internal"></a>


<pre><code>fun spec_scalar_invert_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_from_sha512_internal"></a>


<pre><code>fun spec_scalar_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_mul_internal"></a>


<pre><code>fun spec_scalar_mul_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_add_internal"></a>


<pre><code>fun spec_scalar_add_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_sub_internal"></a>


<pre><code>fun spec_scalar_sub_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>




<a id="0x1_ristretto255_spec_scalar_neg_internal"></a>


<pre><code>fun spec_scalar_neg_internal(a_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;<br/></code></pre>



<a id="@Specification_1_point_equals"></a>

### Function `point_equals`


<pre><code>public fun point_equals(g: &amp;ristretto255::RistrettoPoint, h: &amp;ristretto255::RistrettoPoint): bool<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_double_scalar_mul"></a>

### Function `double_scalar_mul`


<pre><code>public fun double_scalar_mul(scalar1: &amp;ristretto255::Scalar, point1: &amp;ristretto255::RistrettoPoint, scalar2: &amp;ristretto255::Scalar, point2: &amp;ristretto255::RistrettoPoint): ristretto255::RistrettoPoint<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_multi_scalar_mul"></a>

### Function `multi_scalar_mul`


<pre><code>public fun multi_scalar_mul(points: &amp;vector&lt;ristretto255::RistrettoPoint&gt;, scalars: &amp;vector&lt;ristretto255::Scalar&gt;): ristretto255::RistrettoPoint<br/></code></pre>




<pre><code>aborts_if len(points) &#61;&#61; 0;<br/>aborts_if len(scalars) &#61;&#61; 0;<br/>aborts_if len(points) !&#61; len(scalars);<br/>ensures result.handle &#61;&#61; spec_multi_scalar_mul_internal(points, scalars);<br/></code></pre>



<a id="@Specification_1_new_scalar_from_bytes"></a>

### Function `new_scalar_from_bytes`


<pre><code>public fun new_scalar_from_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures spec_scalar_is_canonical_internal(bytes) &#61;&#61;&gt; (std::option::spec_is_some(result)<br/>    &amp;&amp; std::option::spec_borrow(result).data &#61;&#61; bytes);<br/>ensures !spec_scalar_is_canonical_internal(bytes) &#61;&#61;&gt; std::option::spec_is_none(result);<br/></code></pre>



<a id="@Specification_1_new_scalar_from_sha2_512"></a>

### Function `new_scalar_from_sha2_512`


<pre><code>public fun new_scalar_from_sha2_512(sha2_512_input: vector&lt;u8&gt;): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_from_sha512_internal(sha2_512_input);<br/></code></pre>



<a id="@Specification_1_new_scalar_from_u8"></a>

### Function `new_scalar_from_u8`


<pre><code>public fun new_scalar_from_u8(byte: u8): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data[0] &#61;&#61; byte;<br/>ensures forall i in 1..len(result.data): result.data[i] &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_new_scalar_from_u32"></a>

### Function `new_scalar_from_u32`


<pre><code>public fun new_scalar_from_u32(four_bytes: u32): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_from_u64_internal(four_bytes);<br/></code></pre>



<a id="@Specification_1_new_scalar_from_u64"></a>

### Function `new_scalar_from_u64`


<pre><code>public fun new_scalar_from_u64(eight_bytes: u64): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_from_u64_internal(eight_bytes);<br/></code></pre>



<a id="@Specification_1_new_scalar_from_u128"></a>

### Function `new_scalar_from_u128`


<pre><code>public fun new_scalar_from_u128(sixteen_bytes: u128): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_from_u128_internal(sixteen_bytes);<br/></code></pre>



<a id="@Specification_1_new_scalar_reduced_from_32_bytes"></a>

### Function `new_scalar_reduced_from_32_bytes`


<pre><code>public fun new_scalar_reduced_from_32_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>




<pre><code>ensures len(bytes) !&#61; 32 &#61;&#61;&gt; std::option::spec_is_none(result);<br/>ensures len(bytes) &#61;&#61; 32 &#61;&#61;&gt; std::option::spec_borrow(result).data &#61;&#61; spec_scalar_reduced_from_32_bytes_internal(bytes);<br/></code></pre>



<a id="@Specification_1_new_scalar_uniform_from_64_bytes"></a>

### Function `new_scalar_uniform_from_64_bytes`


<pre><code>public fun new_scalar_uniform_from_64_bytes(bytes: vector&lt;u8&gt;): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>




<pre><code>ensures len(bytes) !&#61; 64 &#61;&#61;&gt; std::option::spec_is_none(result);<br/>ensures len(bytes) &#61;&#61; 64 &#61;&#61;&gt; std::option::spec_borrow(result).data &#61;&#61; spec_scalar_uniform_from_64_bytes_internal(bytes);<br/></code></pre>



<a id="@Specification_1_scalar_zero"></a>

### Function `scalar_zero`


<pre><code>public fun scalar_zero(): ristretto255::Scalar<br/></code></pre>




<pre><code>ensures spec_scalar_is_zero(result);<br/></code></pre>



<a id="@Specification_1_scalar_is_zero"></a>

### Function `scalar_is_zero`


<pre><code>public fun scalar_is_zero(s: &amp;ristretto255::Scalar): bool<br/></code></pre>




<pre><code>ensures result &#61;&#61; spec_scalar_is_zero(s);<br/></code></pre>



<a id="@Specification_1_scalar_one"></a>

### Function `scalar_one`


<pre><code>public fun scalar_one(): ristretto255::Scalar<br/></code></pre>




<pre><code>ensures spec_scalar_is_one(result);<br/></code></pre>



<a id="@Specification_1_scalar_is_one"></a>

### Function `scalar_is_one`


<pre><code>public fun scalar_is_one(s: &amp;ristretto255::Scalar): bool<br/></code></pre>




<pre><code>ensures result &#61;&#61; spec_scalar_is_one(s);<br/></code></pre>



<a id="@Specification_1_scalar_equals"></a>

### Function `scalar_equals`


<pre><code>public fun scalar_equals(lhs: &amp;ristretto255::Scalar, rhs: &amp;ristretto255::Scalar): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; (lhs.data &#61;&#61; rhs.data);<br/></code></pre>



<a id="@Specification_1_scalar_invert"></a>

### Function `scalar_invert`


<pre><code>public fun scalar_invert(s: &amp;ristretto255::Scalar): option::Option&lt;ristretto255::Scalar&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures spec_scalar_is_zero(s) &#61;&#61;&gt; std::option::spec_is_none(result);<br/>ensures !spec_scalar_is_zero(s) &#61;&#61;&gt; (std::option::spec_is_some(result) &amp;&amp; std::option::spec_borrow(result).data &#61;&#61; spec_scalar_invert_internal(s.data));<br/></code></pre>



<a id="@Specification_1_scalar_mul"></a>

### Function `scalar_mul`


<pre><code>public fun scalar_mul(a: &amp;ristretto255::Scalar, b: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_mul_internal(a.data, b.data);<br/></code></pre>



<a id="@Specification_1_scalar_mul_assign"></a>

### Function `scalar_mul_assign`


<pre><code>public fun scalar_mul_assign(a: &amp;mut ristretto255::Scalar, b: &amp;ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures a.data &#61;&#61; spec_scalar_mul_internal(old(a).data, b.data);<br/></code></pre>



<a id="@Specification_1_scalar_add"></a>

### Function `scalar_add`


<pre><code>public fun scalar_add(a: &amp;ristretto255::Scalar, b: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_add_internal(a.data, b.data);<br/></code></pre>



<a id="@Specification_1_scalar_add_assign"></a>

### Function `scalar_add_assign`


<pre><code>public fun scalar_add_assign(a: &amp;mut ristretto255::Scalar, b: &amp;ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures a.data &#61;&#61; spec_scalar_add_internal(old(a).data, b.data);<br/></code></pre>



<a id="@Specification_1_scalar_sub"></a>

### Function `scalar_sub`


<pre><code>public fun scalar_sub(a: &amp;ristretto255::Scalar, b: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_sub_internal(a.data, b.data);<br/></code></pre>



<a id="@Specification_1_scalar_sub_assign"></a>

### Function `scalar_sub_assign`


<pre><code>public fun scalar_sub_assign(a: &amp;mut ristretto255::Scalar, b: &amp;ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures a.data &#61;&#61; spec_scalar_sub_internal(old(a).data, b.data);<br/></code></pre>



<a id="@Specification_1_scalar_neg"></a>

### Function `scalar_neg`


<pre><code>public fun scalar_neg(a: &amp;ristretto255::Scalar): ristretto255::Scalar<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result.data &#61;&#61; spec_scalar_neg_internal(a.data);<br/></code></pre>



<a id="@Specification_1_scalar_neg_assign"></a>

### Function `scalar_neg_assign`


<pre><code>public fun scalar_neg_assign(a: &amp;mut ristretto255::Scalar): &amp;mut ristretto255::Scalar<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures a.data &#61;&#61; spec_scalar_neg_internal(old(a).data);<br/></code></pre>



<a id="@Specification_1_scalar_to_bytes"></a>

### Function `scalar_to_bytes`


<pre><code>public fun scalar_to_bytes(s: &amp;ristretto255::Scalar): vector&lt;u8&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; s.data;<br/></code></pre>



<a id="@Specification_1_new_point_from_sha512_internal"></a>

### Function `new_point_from_sha512_internal`


<pre><code>fun new_point_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_new_point_from_64_uniform_bytes_internal"></a>

### Function `new_point_from_64_uniform_bytes_internal`


<pre><code>fun new_point_from_64_uniform_bytes_internal(bytes: vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_is_canonical_internal"></a>

### Function `point_is_canonical_internal`


<pre><code>fun point_is_canonical_internal(bytes: vector&lt;u8&gt;): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_point_is_canonical_internal(bytes);<br/></code></pre>



<a id="@Specification_1_point_identity_internal"></a>

### Function `point_identity_internal`


<pre><code>fun point_identity_internal(): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_decompress_internal"></a>

### Function `point_decompress_internal`


<pre><code>fun point_decompress_internal(maybe_non_canonical_bytes: vector&lt;u8&gt;): (u64, bool)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_clone_internal"></a>

### Function `point_clone_internal`


<pre><code>fun point_clone_internal(point_handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_compress_internal"></a>

### Function `point_compress_internal`


<pre><code>fun point_compress_internal(point: &amp;ristretto255::RistrettoPoint): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_mul_internal"></a>

### Function `point_mul_internal`


<pre><code>fun point_mul_internal(point: &amp;ristretto255::RistrettoPoint, a: vector&lt;u8&gt;, in_place: bool): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_basepoint_mul_internal"></a>

### Function `basepoint_mul_internal`


<pre><code>fun basepoint_mul_internal(a: vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_basepoint_double_mul_internal"></a>

### Function `basepoint_double_mul_internal`


<pre><code>fun basepoint_double_mul_internal(a: vector&lt;u8&gt;, some_point: &amp;ristretto255::RistrettoPoint, b: vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_add_internal"></a>

### Function `point_add_internal`


<pre><code>fun point_add_internal(a: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint, in_place: bool): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_sub_internal"></a>

### Function `point_sub_internal`


<pre><code>fun point_sub_internal(a: &amp;ristretto255::RistrettoPoint, b: &amp;ristretto255::RistrettoPoint, in_place: bool): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_point_neg_internal"></a>

### Function `point_neg_internal`


<pre><code>fun point_neg_internal(a: &amp;ristretto255::RistrettoPoint, in_place: bool): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_double_scalar_mul_internal"></a>

### Function `double_scalar_mul_internal`


<pre><code>fun double_scalar_mul_internal(point1: u64, point2: u64, scalar1: vector&lt;u8&gt;, scalar2: vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_multi_scalar_mul_internal"></a>

### Function `multi_scalar_mul_internal`


<pre><code>fun multi_scalar_mul_internal&lt;P, S&gt;(points: &amp;vector&lt;P&gt;, scalars: &amp;vector&lt;S&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_multi_scalar_mul_internal&lt;P, S&gt;(points, scalars);<br/></code></pre>



<a id="@Specification_1_scalar_is_canonical_internal"></a>

### Function `scalar_is_canonical_internal`


<pre><code>fun scalar_is_canonical_internal(s: vector&lt;u8&gt;): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_is_canonical_internal(s);<br/></code></pre>



<a id="@Specification_1_scalar_from_u64_internal"></a>

### Function `scalar_from_u64_internal`


<pre><code>fun scalar_from_u64_internal(num: u64): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_from_u64_internal(num);<br/></code></pre>



<a id="@Specification_1_scalar_from_u128_internal"></a>

### Function `scalar_from_u128_internal`


<pre><code>fun scalar_from_u128_internal(num: u128): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_from_u128_internal(num);<br/></code></pre>



<a id="@Specification_1_scalar_reduced_from_32_bytes_internal"></a>

### Function `scalar_reduced_from_32_bytes_internal`


<pre><code>fun scalar_reduced_from_32_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>ensures result &#61;&#61; spec_scalar_reduced_from_32_bytes_internal(bytes);<br/></code></pre>



<a id="@Specification_1_scalar_uniform_from_64_bytes_internal"></a>

### Function `scalar_uniform_from_64_bytes_internal`


<pre><code>fun scalar_uniform_from_64_bytes_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_uniform_from_64_bytes_internal(bytes);<br/></code></pre>



<a id="@Specification_1_scalar_invert_internal"></a>

### Function `scalar_invert_internal`


<pre><code>fun scalar_invert_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_invert_internal(bytes);<br/></code></pre>



<a id="@Specification_1_scalar_from_sha512_internal"></a>

### Function `scalar_from_sha512_internal`


<pre><code>fun scalar_from_sha512_internal(sha2_512_input: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_from_sha512_internal(sha2_512_input);<br/></code></pre>



<a id="@Specification_1_scalar_mul_internal"></a>

### Function `scalar_mul_internal`


<pre><code>fun scalar_mul_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_mul_internal(a_bytes, b_bytes);<br/></code></pre>



<a id="@Specification_1_scalar_add_internal"></a>

### Function `scalar_add_internal`


<pre><code>fun scalar_add_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_add_internal(a_bytes, b_bytes);<br/></code></pre>



<a id="@Specification_1_scalar_sub_internal"></a>

### Function `scalar_sub_internal`


<pre><code>fun scalar_sub_internal(a_bytes: vector&lt;u8&gt;, b_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_sub_internal(a_bytes, b_bytes);<br/></code></pre>



<a id="@Specification_1_scalar_neg_internal"></a>

### Function `scalar_neg_internal`


<pre><code>fun scalar_neg_internal(a_bytes: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_scalar_neg_internal(a_bytes);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
