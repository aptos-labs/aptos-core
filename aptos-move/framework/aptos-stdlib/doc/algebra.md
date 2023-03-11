
<a name="0x1_algebra"></a>

# Module `0x1::algebra`

Module <code><a href="algebra.md#0x1_algebra">algebra</a></code> provides structs/functions for doing arithmetic and other common operations
on algebraic structures (mostly groups and fields) that are widely used in cryptographic systems.

Different from existing modules like <code><a href="ristretto255.md#0x1_ristretto255">ristretto255</a>.<b>move</b></code>, the functions here are generic.
Typically, each function represent an operation defined for ANY group/field
and require 1 (or 2+) marker type(s) which represents the actual structure(s) to work with.
See the test cases in <code><a href="algebra.md#0x1_algebra">algebra</a>.<b>move</b></code> for more examples.

The generic APIs should allow Move developers to build generic cryptographic schemes on top of them
and use the schemes with different underlying algebraic structures by simply changing some type parameters.
E.g., Groth16 proof verifier that accepts a generic pairing is now possible.

Below are the structures currently supported.
- BLS12-381 structures.
- Group <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>.
- Group <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code>.
- Group <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.
- Field <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code>.
- Field <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code>.

Below are the operations currently supported.
- Serialization/deserialization.
- Group operations.
- Getting group order.
- Getting group identity.
- Getting group generator.
- Addition.
- Subtraction.
- Negation.
- Efficient sclar multiplication.
- Efficient doubling.
- Equal-to-identity check.
- Field operations.
- Getting additive identity.
- Getting multiplicative identity.
- Conversion from integers.
- Addition.
- Negation.
- Subtraction.
- Multiplication.
- Inversion.
- Division.
- Efficient squaring.
- Equal-to-additive-identity check.
- Equal-to-multiplicative-identity check.
- Equality check.
- Upcasting/downcasting between structures.

Note: in <code><a href="algebra.md#0x1_algebra">algebra</a>.<b>move</b></code> additive group notions are used.


-  [Struct `BLS12_381_Fq`](#0x1_algebra_BLS12_381_Fq)
-  [Struct `BLS12_381_Fq2`](#0x1_algebra_BLS12_381_Fq2)
-  [Struct `BLS12_381_Fq6`](#0x1_algebra_BLS12_381_Fq6)
-  [Struct `BLS12_381_Fq12`](#0x1_algebra_BLS12_381_Fq12)
-  [Struct `BLS12_381_G1_Parent`](#0x1_algebra_BLS12_381_G1_Parent)
-  [Struct `BLS12_381_G1`](#0x1_algebra_BLS12_381_G1)
-  [Struct `BLS12_381_G2_Parent`](#0x1_algebra_BLS12_381_G2_Parent)
-  [Struct `BLS12_381_G2`](#0x1_algebra_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_algebra_BLS12_381_Gt)
-  [Struct `BLS12_381_Fr`](#0x1_algebra_BLS12_381_Fr)
-  [Struct `HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_`](#0x1_algebra_HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_)
-  [Struct `HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_`](#0x1_algebra_HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_)
-  [Struct `Element`](#0x1_algebra_Element)
-  [Function `bls12_381_fq_format`](#0x1_algebra_bls12_381_fq_format)
-  [Function `bls12_381_fq_bendian_format`](#0x1_algebra_bls12_381_fq_bendian_format)
-  [Function `bls12_381_fq2_format`](#0x1_algebra_bls12_381_fq2_format)
-  [Function `bls12_381_fq2_format_bendian_fq`](#0x1_algebra_bls12_381_fq2_format_bendian_fq)
-  [Function `bls12_381_fq6_format`](#0x1_algebra_bls12_381_fq6_format)
-  [Function `bls12_381_fq12_format`](#0x1_algebra_bls12_381_fq12_format)
-  [Function `bls12_381_g1_parent_uncompressed_format`](#0x1_algebra_bls12_381_g1_parent_uncompressed_format)
-  [Function `bls12_381_g1_parent_compressed_format`](#0x1_algebra_bls12_381_g1_parent_compressed_format)
-  [Function `bls12_381_g1_uncompressed_format`](#0x1_algebra_bls12_381_g1_uncompressed_format)
-  [Function `bls12_381_g1_compressed_format`](#0x1_algebra_bls12_381_g1_compressed_format)
-  [Function `bls12_381_g2_parent_uncompressed_format`](#0x1_algebra_bls12_381_g2_parent_uncompressed_format)
-  [Function `bls12_381_g2_parent_compressed_format`](#0x1_algebra_bls12_381_g2_parent_compressed_format)
-  [Function `bls12_381_g2_uncompressed_format`](#0x1_algebra_bls12_381_g2_uncompressed_format)
-  [Function `bls12_381_g2_compressed_format`](#0x1_algebra_bls12_381_g2_compressed_format)
-  [Function `bls12_381_gt_format`](#0x1_algebra_bls12_381_gt_format)
-  [Function `bls12_381_fr_lendian_format`](#0x1_algebra_bls12_381_fr_lendian_format)
-  [Function `bls12_381_fr_bendian_format`](#0x1_algebra_bls12_381_fr_bendian_format)
-  [Function `pairing`](#0x1_algebra_pairing)
-  [Function `eq`](#0x1_algebra_eq)
-  [Function `from_u64`](#0x1_algebra_from_u64)
-  [Function `field_zero`](#0x1_algebra_field_zero)
-  [Function `field_one`](#0x1_algebra_field_one)
-  [Function `field_neg`](#0x1_algebra_field_neg)
-  [Function `field_add`](#0x1_algebra_field_add)
-  [Function `field_sub`](#0x1_algebra_field_sub)
-  [Function `field_mul`](#0x1_algebra_field_mul)
-  [Function `field_div`](#0x1_algebra_field_div)
-  [Function `field_sqr`](#0x1_algebra_field_sqr)
-  [Function `field_inv`](#0x1_algebra_field_inv)
-  [Function `field_is_one`](#0x1_algebra_field_is_one)
-  [Function `field_is_zero`](#0x1_algebra_field_is_zero)
-  [Function `group_add`](#0x1_algebra_group_add)
-  [Function `group_double`](#0x1_algebra_group_double)
-  [Function `group_generator`](#0x1_algebra_group_generator)
-  [Function `group_identity`](#0x1_algebra_group_identity)
-  [Function `group_multi_scalar_mul`](#0x1_algebra_group_multi_scalar_mul)
-  [Function `group_multi_scalar_mul_typed`](#0x1_algebra_group_multi_scalar_mul_typed)
-  [Function `handles_from_elements`](#0x1_algebra_handles_from_elements)
-  [Function `group_neg`](#0x1_algebra_group_neg)
-  [Function `group_scalar_mul_typed`](#0x1_algebra_group_scalar_mul_typed)
-  [Function `group_scalar_mul`](#0x1_algebra_group_scalar_mul)
-  [Function `group_sub`](#0x1_algebra_group_sub)
-  [Function `deserialize`](#0x1_algebra_deserialize)
-  [Function `serialize`](#0x1_algebra_serialize)
-  [Function `group_order`](#0x1_algebra_group_order)
-  [Function `group_is_identity`](#0x1_algebra_group_is_identity)
-  [Function `upcast`](#0x1_algebra_upcast)
-  [Function `downcast`](#0x1_algebra_downcast)
-  [Function `hash_to`](#0x1_algebra_hash_to)
-  [Function `hash_to_group`](#0x1_algebra_hash_to_group)
-  [Function `deserialize_internal`](#0x1_algebra_deserialize_internal)
-  [Function `downcast_internal`](#0x1_algebra_downcast_internal)
-  [Function `eq_internal`](#0x1_algebra_eq_internal)
-  [Function `field_add_internal`](#0x1_algebra_field_add_internal)
-  [Function `field_div_internal`](#0x1_algebra_field_div_internal)
-  [Function `field_inv_internal`](#0x1_algebra_field_inv_internal)
-  [Function `field_is_one_internal`](#0x1_algebra_field_is_one_internal)
-  [Function `field_is_zero_internal`](#0x1_algebra_field_is_zero_internal)
-  [Function `field_mul_internal`](#0x1_algebra_field_mul_internal)
-  [Function `field_neg_internal`](#0x1_algebra_field_neg_internal)
-  [Function `field_one_internal`](#0x1_algebra_field_one_internal)
-  [Function `field_sqr_internal`](#0x1_algebra_field_sqr_internal)
-  [Function `field_sub_internal`](#0x1_algebra_field_sub_internal)
-  [Function `field_zero_internal`](#0x1_algebra_field_zero_internal)
-  [Function `from_u64_internal`](#0x1_algebra_from_u64_internal)
-  [Function `group_add_internal`](#0x1_algebra_group_add_internal)
-  [Function `group_double_internal`](#0x1_algebra_group_double_internal)
-  [Function `group_generator_internal`](#0x1_algebra_group_generator_internal)
-  [Function `group_identity_internal`](#0x1_algebra_group_identity_internal)
-  [Function `group_is_identity_internal`](#0x1_algebra_group_is_identity_internal)
-  [Function `group_multi_scalar_mul_internal`](#0x1_algebra_group_multi_scalar_mul_internal)
-  [Function `group_multi_scalar_mul_typed_internal`](#0x1_algebra_group_multi_scalar_mul_typed_internal)
-  [Function `group_neg_internal`](#0x1_algebra_group_neg_internal)
-  [Function `group_order_internal`](#0x1_algebra_group_order_internal)
-  [Function `group_scalar_mul_typed_internal`](#0x1_algebra_group_scalar_mul_typed_internal)
-  [Function `group_scalar_mul_internal`](#0x1_algebra_group_scalar_mul_internal)
-  [Function `group_sub_internal`](#0x1_algebra_group_sub_internal)
-  [Function `hash_to_internal`](#0x1_algebra_hash_to_internal)
-  [Function `hash_to_group_internal`](#0x1_algebra_hash_to_group_internal)
-  [Function `pairing_internal`](#0x1_algebra_pairing_internal)
-  [Function `serialize_internal`](#0x1_algebra_serialize_internal)
-  [Function `upcast_internal`](#0x1_algebra_upcast_internal)
-  [Function `abort_unless_structure_hashsuite_pair_enabled_for_hash`](#0x1_algebra_abort_unless_structure_hashsuite_pair_enabled_for_hash)
-  [Function `abort_unless_generic_algebraic_structures_basic_operations_enabled`](#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled)
-  [Function `abort_unless_type_enabled_for_basic_operation`](#0x1_algebra_abort_unless_type_enabled_for_basic_operation)
-  [Function `abort_unless_type_serialization_scheme_enabled`](#0x1_algebra_abort_unless_type_serialization_scheme_enabled)
-  [Function `abort_unless_type_triplet_enabled_for_pairing`](#0x1_algebra_abort_unless_type_triplet_enabled_for_pairing)
-  [Function `abort_unless_type_pair_enabled_for_group_scalar_mul`](#0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul)
-  [Function `abort_unless_type_pair_enabled_for_upcast`](#0x1_algebra_abort_unless_type_pair_enabled_for_upcast)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a name="0x1_algebra_BLS12_381_Fq"></a>

## Struct `BLS12_381_Fq`

The finite field $F_q$ used in BLS12-381 curves.
It has a prime order $q$ equal to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_Fq2"></a>

## Struct `BLS12_381_Fq2`

The finite field $F_{q^2}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code>, constructed as $F_{q^2}=F_q[u]/(u^2+1)$.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_Fq6"></a>

## Struct `BLS12_381_Fq6`

The finite field $F_{q^6}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a></code>, constructed as $F_{q^6}=F_{q^2}[v]/(v^3-u-1)$.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_Fq12"></a>

## Struct `BLS12_381_Fq12`

The finite field $F_{q^12}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a></code>, constructed as $F_{q^12}=F_{q^6}[w]/(w^2-v)$.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_G1_Parent"></a>

## Struct `BLS12_381_G1_Parent`

A group constructed by the points on the BLS12-381 curve $E(F_q): y^2=x^3+4$ and the point at inifinity,
under the elliptic curve point addition.
It contains the prime-order subgroup $G_1$ used in pairing.
The identity is the point at infinity.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`

The group $G_1$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_G2_Parent"></a>

## Struct `BLS12_381_G2_Parent`

A group constructed by the points on a curve $E(F_{q^2})$ and the point at inifinity under the elliptic curve point addition.
$E(F_{q^2})$ is an elliptic curve $y^2=x^3+4(u+1)$ defined over $F_{q^2}$.
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is the point at infinity.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`

The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`

The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a multiplicative subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> is the scalar field).
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> is 1.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_BLS12_381_Fr"></a>

## Struct `BLS12_381_Fr`

The finite field $F_r$ that can be used as the scalar fields
for the groups $G_1$, $G_2$, $G_t$ in BLS12-381-based pairing.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_"></a>

## Struct `HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_`

A ciphersuite for hashing bytes to a <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> element.
Defined in https://datatracker.ietf.org/doc/draft-irtf-cfrg-hash-to-curve/.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_">HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_"></a>

## Struct `HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_`

A ciphersuite for hashing bytes to a <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> element.
Defined in https://datatracker.ietf.org/doc/draft-irtf-cfrg-hash-to-curve/.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_">HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_algebra_Element"></a>

## Struct `Element`

This struct represents an element of an algebraic structure <code>S</code>.


<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_bls12_381_fq_format"></a>

## Function `bls12_381_fq_format`

A serialization format for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code> elements.
In this format, an element is represented by a byte array <code>b[]</code> of size 48 using little-endian byte order.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_format">bls12_381_fq_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_format">bls12_381_fq_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"01" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq_bendian_format"></a>

## Function `bls12_381_fq_bendian_format`

A serialization format for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code> elements.
In this format, an element is represented by a byte array <code>b[]</code> of size 48 using big-endian byte order.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_bendian_format">bls12_381_fq_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_bendian_format">bls12_381_fq_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0101" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq2_format"></a>

## Function `bls12_381_fq2_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a></code> elements.
In this format, an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96.
<code>b[0..48]</code> is $c_0$ serialized using <code>BLS12_381_Fq_Format</code>.
<code>b[48..96]</code> is $c_1$ serialized using <code>BLS12_381_Fq_Format</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq2_format">bls12_381_fq2_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq2_format">bls12_381_fq2_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"02" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq2_format_bendian_fq"></a>

## Function `bls12_381_fq2_format_bendian_fq`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a></code> elements.
In this format, an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96.
<code>b[0..48]</code> is $c_0$ serialized using <code>BLS12_381_Fq_Format_BEndianFq</code>.
<code>b[48..96]</code> is $c_1$ serialized using <code>BLS12_381_Fq_Format_BEndianFq</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq2_format_bendian_fq">bls12_381_fq2_format_bendian_fq</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq2_format_bendian_fq">bls12_381_fq2_format_bendian_fq</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0201" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq6_format"></a>

## Function `bls12_381_fq6_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a></code> elements.

In this format, an element $(c_0+c_1\cdot v+c_2\cdot v^2)$ is represented by a byte array <code>b[]</code> of size 288.
<code>b[0..96]</code> is $c_0$ serialized using <code>BLS12_381_Fq2_Format</code>.
<code>b[96..192]</code> is $c_1$ serialized using <code>BLS12_381_Fq2_Format</code>.
<code>b[192..288]</code> is $c_2$ serialized using <code>BLS12_381_Fq2_Format</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq6_format">bls12_381_fq6_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq6_format">bls12_381_fq6_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"03" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq12_format"></a>

## Function `bls12_381_fq12_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code> elements.

In this format, an element $(c_0+c_1\cdot w)$ is represented by a byte array <code>b[]</code> of size 576.
<code>b[0..288]</code> is $c_0$ serialized using <code><a href="algebra.md#0x1_algebra_bls12_381_fq6_format">bls12_381_fq6_format</a>()</code>.
<code>b[288..576]</code> is $c_1$ serialized using <code><a href="algebra.md#0x1_algebra_bls12_381_fq6_format">bls12_381_fq6_format</a>()</code>.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq12_format">bls12_381_fq12_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq12_format">bls12_381_fq12_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"04" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_parent_uncompressed_format"></a>

## Function `bls12_381_g1_parent_uncompressed_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 96.
<code>b[95] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_q)$,
<code>[b[0], ..., b[47] & 0x3f]</code> is $x$ serialized using <code><a href="algebra.md#0x1_algebra_bls12_381_fq_format">bls12_381_fq_format</a>()</code>, and
<code>[b[48], ..., b[95] & 0x3f]</code> is $y$ serialized using <code><a href="algebra.md#0x1_algebra_bls12_381_fq_format">bls12_381_fq_format</a>()</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"05" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_parent_compressed_format"></a>

## Function `bls12_381_g1_parent_compressed_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 48.
<code>b[47] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(Fq)$,
<code>[b[0], ..., b[47] & 0x3f]</code> is $x$ serialized using <code>bls12_381_fq_format</code>, and
the positiveness flag <code>b_47 & 0x80</code> is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0501" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_uncompressed_format"></a>

## Function `bls12_381_g1_uncompressed_format`

A serialization format for <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements,
essentially the format represented by <code><a href="algebra.md#0x1_algebra_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>()</code>
but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"06" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_compressed_format"></a>

## Function `bls12_381_g1_compressed_format`

A serialization format for <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements,
essentially the format represented by <code><a href="algebra.md#0x1_algebra_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>()</code>
but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0601" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_parent_uncompressed_format"></a>

## Function `bls12_381_g2_parent_uncompressed_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 192.
<code>b[191] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
<code>b[0..96]</code> is $x$ serialized using <code>BLS12_381_Fq2_Format</code>, and
<code>[b[96], ..., b[191] & 0x3f]</code> is $y$ serialized using <code>BLS12_381_Fq2_Format</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_uncompressed_format">bls12_381_g2_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_uncompressed_format">bls12_381_g2_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"07" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_parent_compressed_format"></a>

## Function `bls12_381_g2_parent_compressed_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 96.
<code>b[95] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
<code>[b[0], ..., b[95] & 0x3f]</code> is $x$ serialized using <code>BLS12_381_Fq2_Format</code>, and
the positiveness flag <code>b[95] & 0x80</code> is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_compressed_format">bls12_381_g2_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_compressed_format">bls12_381_g2_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0701" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_uncompressed_format"></a>

## Function `bls12_381_g2_uncompressed_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> elements.

Essentially <code>BLS12_381_G2_Parent_Format_Uncompressed</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"08" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_compressed_format"></a>

## Function `bls12_381_g2_compressed_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> elements.

Essentially <code>BLS12_381_G2_Parent_Format_Compressed</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0801" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_gt_format"></a>

## Function `bls12_381_gt_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.

Essentially <code>BLS12_381_Fq12_Format</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_gt_format">bls12_381_gt_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_gt_format">bls12_381_gt_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"09" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fr_lendian_format"></a>

## Function `bls12_381_fr_lendian_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 32 using little-endian byte order.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fr_bendian_format"></a>

## Function `bls12_381_fr_bendian_format`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 32 using big-endian byte order.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_bendian_format">bls12_381_fr_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_bendian_format">bls12_381_fr_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a01" }
</code></pre>



</details>

<a name="0x1_algebra_pairing"></a>

## Function `pairing`

Compute a pre-compiled pairing function (a.k.a., bilinear map) on <code>element_1</code> and <code>element_2</code>.
Return an element in the target group <code>Gt</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1, G2, Gt&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1,G2,Gt&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_triplet_enabled_for_pairing">abort_unless_type_triplet_enabled_for_pairing</a>&lt;G1,G2,Gt&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="algebra.md#0x1_algebra_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(element_1.handle, element_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_eq"></a>

## Function `eq`

Check if <code>x == y</code> for elements <code>x</code> and <code>y</code> of an algebraic structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_eq">eq</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_eq">eq</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): bool {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_eq_internal">eq_internal</a>&lt;S&gt;(x.handle, y.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_from_u64"></a>

## Function `from_u64`

Convert a u64 to an element of an algebraic structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64">from_u64</a>&lt;S&gt;(value: u64): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64">from_u64</a>&lt;S&gt;(value: u64): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_zero"></a>

## Function `field_zero`

Return the additive identity of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_zero">field_zero</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_zero">field_zero</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_zero_internal">field_zero_internal</a>&lt;S&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_one"></a>

## Function `field_one`

Return the multiplicative identity of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_one">field_one</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_one">field_one</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_one_internal">field_one_internal</a>&lt;S&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_neg"></a>

## Function `field_neg`

Compute <code>-x</code> for an element <code>x</code> of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg">field_neg</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg">field_neg</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_neg_internal">field_neg_internal</a>&lt;S&gt;(x.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_add"></a>

## Function `field_add`

Compute <code>x + y</code> for elements <code>x</code> and <code>y</code> of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add">field_add</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add">field_add</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_add_internal">field_add_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_sub"></a>

## Function `field_sub`

Compute <code>x - y</code> for elements <code>x</code> and <code>y</code> of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub">field_sub</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub">field_sub</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_sub_internal">field_sub_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_mul"></a>

## Function `field_mul`

Compute <code>x * y</code> for elements <code>x</code> and <code>y</code> of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul">field_mul</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul">field_mul</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_div"></a>

## Function `field_div`

Try computing <code>x / y</code> for elements <code>x</code> and <code>y</code> of a field <code>S</code>.
Return none if y is the additive identity of field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <b>let</b> (succ, handle) = <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;S&gt;(x.handle, y.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_sqr"></a>

## Function `field_sqr`

Compute <code>x^2</code> for an element <code>x</code> of a field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sqr">field_sqr</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sqr">field_sqr</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_sqr_internal">field_sqr_internal</a>&lt;S&gt;(x.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_inv"></a>

## Function `field_inv`

Try computing <code>x^(-1)</code> for an element <code>x</code> of a field <code>S</code>.
Return none if <code>x</code> is the additive identity of field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;S&gt;(x.handle);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle };
        some(scalar)
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_is_one"></a>

## Function `field_is_one`

Check if an element <code>x</code> is the multiplicative identity of field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_is_one">field_is_one</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_is_one">field_is_one</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): bool {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_field_is_one_internal">field_is_one_internal</a>&lt;S&gt;(x.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_field_is_zero"></a>

## Function `field_is_zero`

Check if an element <code>x</code> is the aditive identity of field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_is_zero">field_is_zero</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_is_zero">field_is_zero</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): bool {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_field_is_zero_internal">field_is_zero_internal</a>&lt;S&gt;(x.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_group_add"></a>

## Function `group_add`

Compute <code>P + Q</code> for elements <code>P</code> and <code>Q</code> of a group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add">group_add</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, element_q: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add">group_add</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, element_q: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_double"></a>

## Function `group_double`

Compute <code>2*P</code> for an element <code>P</code> of a group <code>G</code>. Faster and cheaper than <code>P + P</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_double">group_double</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_double">group_double</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_double_internal">group_double_internal</a>&lt;G&gt;(element_p.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_generator"></a>

## Function `group_generator`

Get the fixed generator of a cyclic group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator">group_generator</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator">group_generator</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_identity"></a>

## Function `group_identity`

Get the identity of a group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity">group_identity</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity">group_identity</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul"></a>

## Function `group_multi_scalar_mul`

Compute <code>k[0]*P[0]+...+k[n-1]*P[n-1]</code> where <code>P[]</code> are <code>n</code> elements of group <code>G</code>,
and <code>k[]</code> are <code>n</code> scalars represented by a byte array <code>scalars</code>.
<code>k[]</code> will be parsed assuming <code>bin(k[0]) || ... || bin(k[n-1]) == scalar_bin[0..w*n]</code>, where
<code>w</code> is the scalar bit length, specified by parameter <code>scalar_size_in_bits</code>,
<code>bin(x)</code> is the least-significant-bit-first <code>w</code>-bit representation of an integer <code>x</code>,
<code>||</code> is bit array concatenation,
<code>scalar_bin</code> is all <code>bin(scalar[i])</code> concatenated, then extended at the back with 0s if not long enough.

NOTE: in some groups, this function is much faster and cheaper than
calling <code>element_scalar_mul</code> and adding up the results using <code>scalar_add</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar_size_in_bits: u64): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar_size_in_bits: u64): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <b>let</b> element_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(elements);
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_internal">group_multi_scalar_mul_internal</a>&lt;G&gt;(element_handles, scalars, scalar_size_in_bits)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul_typed"></a>

## Function `group_multi_scalar_mul_typed`

Compute <code>k[0]*P[0]+...+k[n-1]*P[n-1]</code> where <code>P[]</code> are <code>n</code> elements of group <code>G</code>
and <code>k[]</code> are <code>n</code> elements of the scalarfield <code>S</code> of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_typed">group_multi_scalar_mul_typed</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_typed">group_multi_scalar_mul_typed</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <b>let</b> element_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(elements);
    <b>let</b> scalar_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(scalars);
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_typed_internal">group_multi_scalar_mul_typed_internal</a>&lt;G&gt;(element_handles, scalar_handles)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_handles_from_elements"></a>

## Function `handles_from_elements`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>&lt;S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>&lt;S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>let</b> num_elements = std::vector::length(elements);
    <b>let</b> element_handles = std::vector::empty();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_elements) {
        std::vector::push_back(&<b>mut</b> element_handles, std::vector::borrow(elements, i).handle);
        i = i + 1;
    };
    element_handles
}
</code></pre>



</details>

<a name="0x1_algebra_group_neg"></a>

## Function `group_neg`

Compute <code>-P</code> for an element <code>P</code> of a group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg">group_neg</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg">group_neg</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_neg_internal">group_neg_internal</a>&lt;G&gt;(element_p.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul_typed"></a>

## Function `group_scalar_mul_typed`

Compute <code>k*P</code>, where <code>P</code> is an element of a group <code>G</code> and <code>k</code> is an element of the scalar field <code>S</code> of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_typed">group_scalar_mul_typed</a>&lt;G, S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, scalar_k: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_typed">group_scalar_mul_typed</a>&lt;G, S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, scalar_k: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul">abort_unless_type_pair_enabled_for_group_scalar_mul</a>&lt;G,S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_scalar_mul_typed_internal">group_scalar_mul_typed_internal</a>&lt;G, S&gt;(element_p.handle, scalar_k.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul"></a>

## Function `group_scalar_mul`

Compute <code>k*P</code>, where <code>P</code> is an element of a group <code>G</code> and <code>k</code> is an element of the scalar field <code>S</code> of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, scalar_encoded: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, scalar_encoded: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_scalar_mul_internal">group_scalar_mul_internal</a>&lt;G&gt;(element_p.handle, scalar_encoded)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_sub"></a>

## Function `group_sub`

Compute <code>P - Q</code> for elements <code>P</code> and <code>Q</code> of a group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_sub">group_sub</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, element_q: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_sub">group_sub</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, element_q: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_sub_internal">group_sub_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
    }

}
</code></pre>



</details>

<a name="0x1_algebra_deserialize"></a>

## Function `deserialize`

Try deserializing a byte array to an element of an algebraic structure <code>S</code> using a given <code>scheme</code>.
Return none if the deserialization failed.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_serialization_scheme_enabled">abort_unless_type_serialization_scheme_enabled</a>&lt;S&gt;(scheme);
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(scheme, bytes);
    <b>if</b> (succeeded) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_serialize"></a>

## Function `serialize`

Serialize an element of an algebraic structure <code>S</code> to a byte array using a given <code>scheme</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize">serialize</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize">serialize</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_serialization_scheme_enabled">abort_unless_type_serialization_scheme_enabled</a>&lt;S&gt;(scheme);
    <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(scheme, element.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_group_order"></a>

## Function `group_order`

Get the order of group <code>G</code>, a big integer little-endian encoded as a byte array.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_order">group_order</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_order">group_order</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_group_order_internal">group_order_internal</a>&lt;G&gt;()
}
</code></pre>



</details>

<a name="0x1_algebra_group_is_identity"></a>

## Function `group_is_identity`

Check if an element <code>x</code> is the identity of its group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_is_identity">group_is_identity</a>&lt;G&gt;(element_x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_is_identity">group_is_identity</a>&lt;G&gt;(element_x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): bool {
    <a href="algebra.md#0x1_algebra_group_is_identity_internal">group_is_identity_internal</a>&lt;G&gt;(element_x.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_upcast"></a>

## Function `upcast`

Cast an element of a structure <code>S</code> to a parent structure <code>L</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_upcast">upcast</a>&lt;S, L&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;L&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_upcast">upcast</a>&lt;S,L&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_upcast">abort_unless_type_pair_enabled_for_upcast</a>&lt;S,L&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt; {
        handle: <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S,L&gt;(element.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_downcast"></a>

## Function `downcast`

Try casting an element <code>x</code> of a structure <code>L</code> to a sub-structure <code>S</code>.
Return none if <code>x</code> is not a member of <code>S</code>.

NOTE: Membership check is performed inside, which can be expensive, depending on the structures <code>L</code> and <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L, S&gt;(element_x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;L&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L,S&gt;(element_x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_upcast">abort_unless_type_pair_enabled_for_upcast</a>&lt;S,L&gt;();
    <b>let</b> (succ, new_handle) = <a href="algebra.md#0x1_algebra_downcast_internal">downcast_internal</a>&lt;L,S&gt;(element_x.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle: new_handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_hash_to"></a>

## Function `hash_to`

Hash some bytes into structure <code>Struc</code> using suite <code>Suite</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;Struc, Suite&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Struc&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;Struc, Suite&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Struc&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_hashsuite_pair_enabled_for_hash">abort_unless_structure_hashsuite_pair_enabled_for_hash</a>&lt;Struc, Suite&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a> {
        handle: <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;Struc, Suite&gt;(bytes)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_hash_to_group"></a>

## Function `hash_to_group`

Hash some bytes with domain separation tag <code>dst</code> into group <code>G</code> using suite <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_group">hash_to_group</a>&lt;G, S&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_group">hash_to_group</a>&lt;G, S&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_hashsuite_pair_enabled_for_hash">abort_unless_structure_hashsuite_pair_enabled_for_hash</a>&lt;G, S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a> {
        handle: <a href="algebra.md#0x1_algebra_hash_to_group_internal">hash_to_group_internal</a>&lt;G, S&gt;(dst, bytes)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_downcast_internal"></a>

## Function `downcast_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_downcast_internal">downcast_internal</a>&lt;L, S&gt;(handle: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast_internal">downcast_internal</a>&lt;L,S&gt;(handle: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_eq_internal"></a>

## Function `eq_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_eq_internal">eq_internal</a>&lt;S&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_eq_internal">eq_internal</a>&lt;S&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_algebra_field_add_internal"></a>

## Function `field_add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_add_internal">field_add_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add_internal">field_add_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_div_internal"></a>

## Function `field_div_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_field_inv_internal"></a>

## Function `field_inv_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;F&gt;(handle: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;F&gt;(handle: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_field_is_one_internal"></a>

## Function `field_is_one_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_is_one_internal">field_is_one_internal</a>&lt;F&gt;(handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_is_one_internal">field_is_one_internal</a>&lt;F&gt;(handle: u64): bool;
</code></pre>



</details>

<a name="0x1_algebra_field_is_zero_internal"></a>

## Function `field_is_zero_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_is_zero_internal">field_is_zero_internal</a>&lt;F&gt;(handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_is_zero_internal">field_is_zero_internal</a>&lt;F&gt;(handle: u64): bool;
</code></pre>



</details>

<a name="0x1_algebra_field_mul_internal"></a>

## Function `field_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_neg_internal"></a>

## Function `field_neg_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_neg_internal">field_neg_internal</a>&lt;F&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg_internal">field_neg_internal</a>&lt;F&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_one_internal"></a>

## Function `field_one_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_one_internal">field_one_internal</a>&lt;S&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_one_internal">field_one_internal</a>&lt;S&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_sqr_internal"></a>

## Function `field_sqr_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_sqr_internal">field_sqr_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sqr_internal">field_sqr_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_sub_internal"></a>

## Function `field_sub_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_sub_internal">field_sub_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub_internal">field_sub_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_zero_internal"></a>

## Function `field_zero_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_zero_internal">field_zero_internal</a>&lt;S&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_zero_internal">field_zero_internal</a>&lt;S&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_from_u64_internal"></a>

## Function `from_u64_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_add_internal"></a>

## Function `group_add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_double_internal"></a>

## Function `group_double_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_double_internal">group_double_internal</a>&lt;G&gt;(element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_double_internal">group_double_internal</a>&lt;G&gt;(element_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_generator_internal"></a>

## Function `group_generator_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_identity_internal"></a>

## Function `group_identity_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_is_identity_internal"></a>

## Function `group_is_identity_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_is_identity_internal">group_is_identity_internal</a>&lt;G&gt;(handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_is_identity_internal">group_is_identity_internal</a>&lt;G&gt;(handle: u64): bool;
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul_internal"></a>

## Function `group_multi_scalar_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_internal">group_multi_scalar_mul_internal</a>&lt;G&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalars_encoded: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar_size_in_bits: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_internal">group_multi_scalar_mul_internal</a>&lt;G&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalars_encoded: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar_size_in_bits: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul_typed_internal"></a>

## Function `group_multi_scalar_mul_typed_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_typed_internal">group_multi_scalar_mul_typed_internal</a>&lt;G&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul_typed_internal">group_multi_scalar_mul_typed_internal</a>&lt;G&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_neg_internal"></a>

## Function `group_neg_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_neg_internal">group_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg_internal">group_neg_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_order_internal"></a>

## Function `group_order_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_order_internal">group_order_internal</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_order_internal">group_order_internal</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul_typed_internal"></a>

## Function `group_scalar_mul_typed_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_typed_internal">group_scalar_mul_typed_internal</a>&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_typed_internal">group_scalar_mul_typed_internal</a>&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul_internal"></a>

## Function `group_scalar_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_internal">group_scalar_mul_internal</a>&lt;G&gt;(element_handle: u64, scalar_encoded: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_internal">group_scalar_mul_internal</a>&lt;G&gt;(element_handle: u64, scalar_encoded: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_sub_internal"></a>

## Function `group_sub_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_sub_internal">group_sub_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_sub_internal">group_sub_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_hash_to_internal"></a>

## Function `hash_to_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;S, C&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;S,C&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_hash_to_group_internal"></a>

## Function `hash_to_group_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_group_internal">hash_to_group_internal</a>&lt;G, C&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_group_internal">hash_to_group_internal</a>&lt;G,C&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_pairing_internal"></a>

## Function `pairing_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_pairing_internal">pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handle: u64, g2_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handle: u64, g2_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_serialize_internal"></a>

## Function `serialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, h: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, h: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_upcast_internal"></a>

## Function `upcast_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S, L&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S,L&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_structure_hashsuite_pair_enabled_for_hash"></a>

## Function `abort_unless_structure_hashsuite_pair_enabled_for_hash`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_structure_hashsuite_pair_enabled_for_hash">abort_unless_structure_hashsuite_pair_enabled_for_hash</a>&lt;S, H&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_structure_hashsuite_pair_enabled_for_hash">abort_unless_structure_hashsuite_pair_enabled_for_hash</a>&lt;S, H&gt;() {
    <b>let</b> structure_type = type_of&lt;S&gt;();
    <b>let</b> hashsuite_type = type_of&lt;H&gt;();
    <b>if</b> (structure_type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;() && hashsuite_type == type_of&lt;<a href="algebra.md#0x1_algebra_HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_">HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_</a>&gt;() && bls12_381_structures_enabled()) <b>return</b>;
    <b>if</b> (structure_type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;() && hashsuite_type == type_of&lt;<a href="algebra.md#0x1_algebra_HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_">HASH_SUITE_BLS12381G2_XMD_SHA_256_SSWU_RO_</a>&gt;() && bls12_381_structures_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled"></a>

## Function `abort_unless_generic_algebraic_structures_basic_operations_enabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_generic_algebraic_structures_basic_operations_enabled">abort_unless_generic_algebraic_structures_basic_operations_enabled</a>() {
    <b>if</b> (generic_algebraic_structures_basic_operations_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_type_enabled_for_basic_operation"></a>

## Function `abort_unless_type_enabled_for_basic_operation`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;() {
    <b>let</b> type = type_of&lt;S&gt;();
    <b>if</b> ((type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a>&gt;()) && bls12_381_structures_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_type_serialization_scheme_enabled"></a>

## Function `abort_unless_type_serialization_scheme_enabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_serialization_scheme_enabled">abort_unless_type_serialization_scheme_enabled</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_serialization_scheme_enabled">abort_unless_type_serialization_scheme_enabled</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> type = type_of&lt;S&gt;();
    <b>if</b> (type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;() && (scheme == <a href="algebra.md#0x1_algebra_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>() || scheme == <a href="algebra.md#0x1_algebra_bls12_381_fr_bendian_format">bls12_381_fr_bendian_format</a>()) && bls12_381_structures_enabled()) <b>return</b>;
    <b>if</b> (type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a>&gt;() && scheme == <a href="algebra.md#0x1_algebra_bls12_381_fq12_format">bls12_381_fq12_format</a>() && bls12_381_structures_enabled()) <b>return</b>;
    <b>if</b> (type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;() && (scheme == <a href="algebra.md#0x1_algebra_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>() || scheme == <a href="algebra.md#0x1_algebra_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>()) && bls12_381_structures_enabled()) <b>return</b>;
    <b>if</b> (type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;() && (scheme == <a href="algebra.md#0x1_algebra_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>() || scheme == <a href="algebra.md#0x1_algebra_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>()) && bls12_381_structures_enabled()) <b>return</b>;
    <b>if</b> (type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;() && scheme == <a href="algebra.md#0x1_algebra_bls12_381_gt_format">bls12_381_gt_format</a>() && bls12_381_structures_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_type_triplet_enabled_for_pairing"></a>

## Function `abort_unless_type_triplet_enabled_for_pairing`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_triplet_enabled_for_pairing">abort_unless_type_triplet_enabled_for_pairing</a>&lt;G1, G2, Gt&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_triplet_enabled_for_pairing">abort_unless_type_triplet_enabled_for_pairing</a>&lt;G1, G2, Gt&gt;() {
    <b>let</b> g1_type = type_of&lt;G1&gt;();
    <b>let</b> g2_type = type_of&lt;G2&gt;();
    <b>let</b> gt_type = type_of&lt;Gt&gt;();
    <b>if</b> (g1_type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;() && g2_type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;() && gt_type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;() && bls12_381_structures_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul"></a>

## Function `abort_unless_type_pair_enabled_for_group_scalar_mul`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul">abort_unless_type_pair_enabled_for_group_scalar_mul</a>&lt;G, S&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul">abort_unless_type_pair_enabled_for_group_scalar_mul</a>&lt;G, S&gt;() {
    <b>let</b> group = type_of&lt;G&gt;();
    <b>let</b> scalar_field = type_of&lt;S&gt;();
    <b>if</b> ((group == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;() || group == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;() || group == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;()) && scalar_field == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;() && bls12_381_structures_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_type_pair_enabled_for_upcast"></a>

## Function `abort_unless_type_pair_enabled_for_upcast`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_upcast">abort_unless_type_pair_enabled_for_upcast</a>&lt;S, L&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_upcast">abort_unless_type_pair_enabled_for_upcast</a>&lt;S, L&gt;() {
    <b>let</b> super = type_of&lt;L&gt;();
    <b>let</b> sub = type_of&lt;S&gt;();
    <b>if</b> (super == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a>&gt;() && sub == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;() && bls12_381_structures_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
