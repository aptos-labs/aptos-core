
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

Currently supported structures include:
- BLS12-381 structures,
- (groups) BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt,
- (fields) BLS12_381_Fq12, BLS12_381_Fr.

Currently supported operations include:
- serialization/deserialization,
- group/field metadata,
- group/field basic arithmetic,
- pairing,
- upcasting/downcasting.

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
-  [Struct `Element`](#0x1_algebra_Element)
-  [Constants](#@Constants_0)
-  [Function `bls12_381_fq_format`](#0x1_algebra_bls12_381_fq_format)
-  [Function `bls12_381_fq_bendian_format`](#0x1_algebra_bls12_381_fq_bendian_format)
-  [Function `bls12_381_fq2_format`](#0x1_algebra_bls12_381_fq2_format)
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
-  [Function `group_identity`](#0x1_algebra_group_identity)
-  [Function `group_generator`](#0x1_algebra_group_generator)
-  [Function `group_neg`](#0x1_algebra_group_neg)
-  [Function `group_add`](#0x1_algebra_group_add)
-  [Function `group_double`](#0x1_algebra_group_double)
-  [Function `group_scalar_mul`](#0x1_algebra_group_scalar_mul)
-  [Function `deserialize`](#0x1_algebra_deserialize)
-  [Function `serialize`](#0x1_algebra_serialize)
-  [Function `group_order`](#0x1_algebra_group_order)
-  [Function `upcast`](#0x1_algebra_upcast)
-  [Function `downcast`](#0x1_algebra_downcast)
-  [Function `abort_unless_generic_algebra_basic_operations_enabled`](#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled)
-  [Function `abort_unless_type_enabled_for_basic_operation`](#0x1_algebra_abort_unless_type_enabled_for_basic_operation)
-  [Function `abort_unless_type_serialization_scheme_enabled`](#0x1_algebra_abort_unless_type_serialization_scheme_enabled)
-  [Function `abort_unless_type_triplet_enabled_for_pairing`](#0x1_algebra_abort_unless_type_triplet_enabled_for_pairing)
-  [Function `abort_unless_type_pair_enabled_for_group_scalar_mul`](#0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul)
-  [Function `abort_unless_type_pair_enabled_for_upcast`](#0x1_algebra_abort_unless_type_pair_enabled_for_upcast)
-  [Function `deserialize_internal`](#0x1_algebra_deserialize_internal)
-  [Function `serialize_internal`](#0x1_algebra_serialize_internal)
-  [Function `from_u64_internal`](#0x1_algebra_from_u64_internal)
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
-  [Function `group_add_internal`](#0x1_algebra_group_add_internal)
-  [Function `eq_internal`](#0x1_algebra_eq_internal)
-  [Function `group_identity_internal`](#0x1_algebra_group_identity_internal)
-  [Function `group_order_internal`](#0x1_algebra_group_order_internal)
-  [Function `group_generator_internal`](#0x1_algebra_group_generator_internal)
-  [Function `group_scalar_mul_internal`](#0x1_algebra_group_scalar_mul_internal)
-  [Function `group_double_internal`](#0x1_algebra_group_double_internal)
-  [Function `group_neg_internal`](#0x1_algebra_group_neg_internal)
-  [Function `pairing_internal`](#0x1_algebra_pairing_internal)
-  [Function `upcast_internal`](#0x1_algebra_upcast_internal)
-  [Function `downcast_internal`](#0x1_algebra_downcast_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a name="0x1_algebra_BLS12_381_Fq"></a>

## Struct `BLS12_381_Fq`

A finite field used in BLS12-381 curves.
It has a prime order <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.


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

An extension field of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code>, constructed as <code>Fq2=Fq[u]/(u^2+1)</code>.


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

An extension field of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a></code>, constructed as <code>Fq6=Fq2[v]/(v^3-u-1)</code>.


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

An extension field of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a></code>, constructed as <code>Fq12=Fq6[w]/(w^2-v)</code>.


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

A group constructed by the points on a curve <code>E(Fq)</code> and the point at inifinity, under the elliptic curve point addition.
<code>E(Fq)</code> is an elliptic curve <code>y^2=x^3+4</code> defined over <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code>.
The identity of <code>BLS12_381_G1_PARENT</code> is the point at infinity.


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

A subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code>.
It has a prime order <code>r=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001</code>.
There exists a bilinear map from (<code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>, <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code>) to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.


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

A group constructed by the points on a curve <code>E(Fq2)</code> and the point at inifinity under the elliptic curve point addition.
<code>E(Fq2)</code> is an elliptic curve <code>y^2=x^3+4(u+1)</code> defined over <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a></code>.
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is the point at infinity.


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

A subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code>.
It has a prime order <code>r=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001</code>.
There exists a bilinear map from (<code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>, <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code>) to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.


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

<code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> represents the target group of the pairing defined over the BLS12-381 curves.
A multiplicative subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code>.
It has a prime order <code>r=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001</code>,
same as <code>BLS12_381_G1_SUB</code> and <code>BLS12_381_G2_SUB</code>.
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is 1.
There exists a bilinear map from (<code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>, <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code>) to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.


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

A finite field that shares the same prime number <code>r</code> with groups <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>, <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> and <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>,
and thus can be their scalar field.


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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_algebra_BLS12_381_FQ12_ONE_SERIALIZED"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_FQ12_ONE_SERIALIZED">BLS12_381_FQ12_ONE_SERIALIZED</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a name="0x1_algebra_BLS12_381_FQ12_VAL_7_NEG_SERIALIZED"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_FQ12_VAL_7_NEG_SERIALIZED">BLS12_381_FQ12_VAL_7_NEG_SERIALIZED</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [164, 170, 255, 255, 255, 255, 254, 185, 255, 255, 83, 177, 254, 255, 171, 30, 36, 246, 176, 246, 160, 210, 48, 103, 191, 18, 133, 243, 132, 75, 119, 100, 215, 172, 75, 67, 182, 167, 27, 75, 154, 230, 127, 57, 234, 17, 1, 26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a name="0x1_algebra_BLS12_381_FQ12_VAL_7_SERIALIZED"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_FQ12_VAL_7_SERIALIZED">BLS12_381_FQ12_VAL_7_SERIALIZED</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a name="0x1_algebra_BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN">BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [250, 255, 255, 255, 254, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8, 216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115];
</code></pre>



<a name="0x1_algebra_BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN">BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7];
</code></pre>



<a name="0x1_algebra_BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN">BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP">BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [183, 252, 126, 98, 112, 90, 239, 84, 45, 188, 197, 212, 188, 230, 42, 123, 242, 46, 239, 22, 145, 190, 243, 13, 172, 18, 31, 178, 0, 202, 125, 201, 164, 64, 59, 144, 218, 69, 1, 207, 238, 25, 53, 185, 190, 243, 40, 25];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP">BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [183, 252, 126, 98, 112, 90, 239, 84, 45, 188, 197, 212, 188, 230, 42, 123, 242, 46, 239, 22, 145, 190, 243, 13, 172, 18, 31, 178, 0, 202, 125, 201, 164, 64, 59, 144, 218, 69, 1, 207, 238, 25, 53, 185, 190, 243, 40, 25, 143, 144, 103, 215, 129, 19, 237, 95, 115, 79, 178, 225, 180, 151, 229, 32, 19, 218, 12, 157, 103, 154, 89, 45, 167, 53, 246, 113, 61, 46, 237, 41, 19, 249, 193, 18, 8, 210, 225, 244, 85, 176, 201, 148, 47, 100, 115, 9];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP">BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [183, 252, 126, 98, 112, 90, 239, 84, 45, 188, 197, 212, 188, 230, 42, 123, 242, 46, 239, 22, 145, 190, 243, 13, 172, 18, 31, 178, 0, 202, 125, 201, 164, 64, 59, 144, 218, 69, 1, 207, 238, 25, 53, 185, 190, 243, 40, 153];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP">BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [183, 252, 126, 98, 112, 90, 239, 84, 45, 188, 197, 212, 188, 230, 42, 123, 242, 46, 239, 22, 145, 190, 243, 13, 172, 18, 31, 178, 0, 202, 125, 201, 164, 64, 59, 144, 218, 69, 1, 207, 238, 25, 53, 185, 190, 243, 40, 25, 28, 26, 152, 40, 126, 236, 17, 90, 140, 176, 161, 207, 73, 104, 198, 253, 16, 28, 164, 89, 57, 56, 215, 57, 24, 221, 142, 129, 71, 29, 138, 58, 196, 179, 137, 48, 174, 213, 57, 86, 68, 54, 182, 164, 186, 173, 141, 16];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_GENERATOR_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_GENERATOR_SERIALIZED_COMP">BLS12_381_G1_GENERATOR_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [187, 198, 34, 219, 10, 240, 58, 251, 239, 26, 122, 249, 63, 232, 85, 108, 88, 172, 27, 23, 63, 58, 78, 161, 5, 185, 116, 151, 79, 140, 104, 195, 15, 172, 169, 79, 140, 99, 149, 38, 148, 215, 151, 49, 167, 211, 241, 23];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP">BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [187, 198, 34, 219, 10, 240, 58, 251, 239, 26, 122, 249, 63, 232, 85, 108, 88, 172, 27, 23, 63, 58, 78, 161, 5, 185, 116, 151, 79, 140, 104, 195, 15, 172, 169, 79, 140, 99, 149, 38, 148, 215, 151, 49, 167, 211, 241, 23, 225, 231, 197, 70, 41, 35, 170, 12, 228, 138, 136, 162, 68, 199, 60, 208, 237, 179, 4, 44, 203, 24, 219, 0, 246, 10, 208, 213, 149, 224, 245, 252, 228, 138, 29, 116, 237, 48, 158, 160, 241, 160, 170, 227, 129, 244, 179, 8];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_INF_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_INF_SERIALIZED_COMP">BLS12_381_G1_INF_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64];
</code></pre>



<a name="0x1_algebra_BLS12_381_G1_INF_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1_INF_SERIALIZED_UNCOMP">BLS12_381_G1_INF_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP">BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [60, 141, 211, 246, 138, 54, 15, 156, 91, 168, 31, 173, 43, 227, 64, 139, 220, 48, 112, 97, 155, 199, 191, 55, 148, 133, 27, 214, 35, 104, 90, 80, 54, 239, 95, 19, 136, 192, 84, 30, 88, 195, 210, 178, 219, 209, 156, 4, 200, 52, 114, 36, 116, 70, 177, 189, 212, 68, 22, 173, 28, 31, 146, 154, 63, 1, 237, 52, 91, 227, 91, 155, 75, 162, 15, 23, 204, 242, 181, 32, 142, 61, 236, 131, 128, 214, 184, 195, 55, 237, 49, 191, 246, 115, 2, 141];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP">BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [60, 141, 211, 246, 138, 54, 15, 156, 91, 168, 31, 173, 43, 227, 64, 139, 220, 48, 112, 97, 155, 199, 191, 55, 148, 133, 27, 214, 35, 104, 90, 80, 54, 239, 95, 19, 136, 192, 84, 30, 88, 195, 210, 178, 219, 209, 156, 4, 200, 52, 114, 36, 116, 70, 177, 189, 212, 68, 22, 173, 28, 31, 146, 154, 63, 1, 237, 52, 91, 227, 91, 155, 75, 162, 15, 23, 204, 242, 181, 32, 142, 61, 236, 131, 128, 214, 184, 195, 55, 237, 49, 191, 246, 115, 2, 13, 206, 221, 235, 102, 50, 7, 172, 223, 77, 29, 139, 212, 194, 243, 195, 4, 238, 198, 118, 228, 198, 123, 61, 236, 208, 126, 177, 106, 104, 164, 22, 128, 111, 24, 31, 177, 115, 31, 183, 164, 130, 186, 255, 121, 156, 99, 73, 17, 139, 58, 5, 113, 118, 200, 138, 79, 23, 210, 252, 196, 114, 156, 18, 167, 43, 39, 2, 4, 134, 193, 249, 9, 218, 230, 130, 18, 63, 111, 61, 98, 188, 184, 128, 139, 199, 252, 133, 244, 17, 69, 200, 228, 179, 24, 20, 20];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP">BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [60, 141, 211, 246, 138, 54, 15, 156, 91, 168, 31, 173, 43, 227, 64, 139, 220, 48, 112, 97, 155, 199, 191, 55, 148, 133, 27, 214, 35, 104, 90, 80, 54, 239, 95, 19, 136, 192, 84, 30, 88, 195, 210, 178, 219, 209, 156, 4, 200, 52, 114, 36, 116, 70, 177, 189, 212, 68, 22, 173, 28, 31, 146, 154, 63, 1, 237, 52, 91, 227, 91, 155, 75, 162, 15, 23, 204, 242, 181, 32, 142, 61, 236, 131, 128, 214, 184, 195, 55, 237, 49, 191, 246, 115, 2, 13];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP">BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [60, 141, 211, 246, 138, 54, 15, 156, 91, 168, 31, 173, 43, 227, 64, 139, 220, 48, 112, 97, 155, 199, 191, 55, 148, 133, 27, 214, 35, 104, 90, 80, 54, 239, 95, 19, 136, 192, 84, 30, 88, 195, 210, 178, 219, 209, 156, 4, 200, 52, 114, 36, 116, 70, 177, 189, 212, 68, 22, 173, 28, 31, 146, 154, 63, 1, 237, 52, 91, 227, 91, 155, 75, 162, 15, 23, 204, 242, 181, 32, 142, 61, 236, 131, 128, 214, 184, 195, 55, 237, 49, 191, 246, 115, 2, 13, 221, 204, 19, 153, 205, 248, 82, 218, 177, 226, 200, 220, 59, 12, 232, 25, 54, 47, 58, 18, 218, 86, 243, 122, 238, 147, 211, 136, 28, 167, 96, 228, 103, 148, 44, 146, 66, 136, 100, 166, 23, 44, 128, 191, 77, 174, 183, 8, 32, 112, 250, 142, 137, 55, 116, 106, 232, 45, 87, 236, 139, 99, 153, 119, 248, 206, 174, 242, 26, 17, 55, 93, 229, 43, 2, 225, 69, 220, 57, 2, 27, 244, 202, 183, 238, 170, 149, 86, 136, 161, 183, 84, 54, 249, 236, 5];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_GENERATOR_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_GENERATOR_SERIALIZED_COMP">BLS12_381_G2_GENERATOR_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [184, 189, 33, 193, 200, 86, 128, 212, 239, 187, 5, 168, 38, 3, 172, 11, 119, 209, 227, 122, 100, 11, 81, 180, 2, 59, 64, 250, 212, 122, 228, 198, 81, 16, 197, 45, 39, 5, 8, 38, 145, 10, 143, 240, 178, 162, 74, 2, 126, 43, 4, 93, 5, 125, 172, 229, 87, 93, 148, 19, 18, 241, 76, 51, 73, 80, 127, 220, 187, 97, 218, 181, 26, 182, 32, 153, 208, 208, 107, 89, 101, 79, 39, 136, 160, 211, 172, 125, 96, 159, 113, 82, 96, 43, 224, 19];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP">BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [184, 189, 33, 193, 200, 86, 128, 212, 239, 187, 5, 168, 38, 3, 172, 11, 119, 209, 227, 122, 100, 11, 81, 180, 2, 59, 64, 250, 212, 122, 228, 198, 81, 16, 197, 45, 39, 5, 8, 38, 145, 10, 143, 240, 178, 162, 74, 2, 126, 43, 4, 93, 5, 125, 172, 229, 87, 93, 148, 19, 18, 241, 76, 51, 73, 80, 127, 220, 187, 97, 218, 181, 26, 182, 32, 153, 208, 208, 107, 89, 101, 79, 39, 136, 160, 211, 172, 125, 96, 159, 113, 82, 96, 43, 224, 19, 1, 40, 184, 8, 134, 84, 147, 225, 137, 162, 172, 59, 204, 201, 58, 146, 44, 209, 96, 81, 105, 154, 66, 109, 167, 211, 189, 140, 170, 155, 253, 173, 26, 53, 46, 218, 198, 205, 201, 140, 17, 110, 125, 114, 39, 213, 229, 12, 190, 121, 95, 240, 95, 7, 169, 170, 161, 29, 236, 92, 39, 13, 55, 63, 171, 153, 46, 87, 171, 146, 116, 38, 175, 99, 167, 133, 126, 40, 62, 203, 153, 139, 194, 43, 176, 210, 172, 50, 204, 52, 167, 46, 160, 196, 6, 6];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_INF_SERIALIZED_COMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_INF_SERIALIZED_COMP">BLS12_381_G2_INF_SERIALIZED_COMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64];
</code></pre>



<a name="0x1_algebra_BLS12_381_G2_INF_SERIALIZED_UNCOMP"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2_INF_SERIALIZED_UNCOMP">BLS12_381_G2_INF_SERIALIZED_UNCOMP</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64];
</code></pre>



<a name="0x1_algebra_BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED">BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [32, 65, 234, 123, 102, 193, 150, 128, 226, 192, 187, 35, 36, 90, 113, 145, 135, 83, 34, 11, 49, 248, 138, 146, 90, 169, 177, 225, 146, 231, 193, 136, 160, 179, 101, 203, 153, 75, 62, 197, 232, 9, 32, 97, 23, 198, 65, 18, 66, 185, 64, 177, 12, 170, 55, 206, 115, 68, 150, 179, 183, 198, 53, 120, 160, 227, 192, 118, 249, 179, 26, 124, 161, 58, 113, 98, 98, 224, 228, 205, 164, 172, 153, 78, 251, 158, 25, 137, 60, 191, 228, 212, 100, 185, 33, 13, 9, 157, 128, 138, 8, 179, 196, 195, 132, 110, 117, 41, 152, 72, 153, 71, 134, 57, 196, 230, 196, 97, 82, 239, 73, 160, 74, 249, 200, 230, 255, 68, 45, 40, 108, 70, 19, 163, 218, 198, 164, 190, 228, 180, 14, 31, 107, 3, 15, 40, 113, 218, 190, 66, 35, 178, 80, 195, 24, 30, 205, 59, 198, 129, 144, 4, 116, 90, 235, 107, 172, 86, 116, 7, 242, 185, 199, 209, 151, 140, 69, 238, 103, 18, 174, 70, 147, 11, 192, 6, 56, 56, 63, 102, 150, 21, 139, 173, 72, 140, 190, 118, 99, 214, 129, 201, 108, 3, 84, 129, 219, 207, 120, 231, 167, 251, 174, 195, 121, 145, 99, 170, 105, 20, 206, 243, 54, 81, 86, 189, 195, 229, 51, 167, 200, 131, 213, 151, 78, 52, 98, 172, 111, 25, 227, 249, 206, 38, 128, 10, 226, 72, 164, 92, 95, 13, 211, 164, 138, 24, 89, 105, 34, 78, 108, 214, 175, 154, 4, 130, 65, 189, 202, 201, 128, 13, 148, 174, 238, 151, 14, 8, 72, 143, 185, 97, 227, 106, 118, 155, 108, 24, 78, 146, 164, 185, 250, 35, 102, 177, 174, 142, 189, 245, 84, 47, 161, 224, 236, 57, 12, 144, 223, 64, 169, 30, 82, 97, 128, 5, 129, 181, 73, 43, 217, 100, 13, 28, 83, 82, 186, 188, 85, 29, 26, 73, 153, 143, 69, 23, 49, 47, 85, 180, 51, 146, 114, 178, 138, 62, 107, 12, 125, 24, 46, 43, 182, 27, 215, 215, 43, 41, 174, 54, 150, 219, 143, 175, 227, 43, 144, 74, 181, 208, 118, 78, 70, 191, 33, 249, 160, 201, 161, 247, 190, 220, 107, 18, 185, 246, 72, 32, 252, 139, 63, 212, 162, 101, 65, 71, 43, 227, 201, 201, 61, 120, 76, 221, 83, 160, 89, 209, 96, 75, 243, 41, 47, 237, 209, 186, 191, 176, 3, 152, 18, 142, 50, 65, 188, 99, 165, 164, 123, 94, 146, 7, 252, 176, 200, 143, 123, 253, 220, 55, 106, 36, 44, 159, 12, 3, 43, 162, 142, 236, 134, 112, 241, 250, 29, 71, 86, 117, 147, 180, 87, 28, 152, 59, 128, 21, 223, 145, 207, 161, 36, 27, 127, 184, 165, 126, 14, 110, 1, 20, 91, 152, 222, 1, 126, 204, 194, 166, 110, 131, 206, 217, 216, 49, 25, 165, 5, 229, 82, 70, 120, 56, 211, 91, 140, 226, 244, 215, 204, 154, 137, 79, 109, 238, 146, 47, 53, 240, 231, 43, 126, 150, 240, 135, 155, 12, 134, 20, 211, 249, 229, 245, 97, 139, 91, 233, 184, 35, 129, 98, 132, 72, 100, 26, 139, 176, 253, 29, 255, 177, 108, 112, 230, 131, 29, 141, 105, 246, 31, 42, 46, 249, 233, 12, 66, 31, 122, 91, 28, 231, 165, 209, 19, 199, 235, 1];
</code></pre>



<a name="0x1_algebra_BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED">BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [32, 65, 234, 123, 102, 193, 150, 128, 226, 192, 187, 35, 36, 90, 113, 145, 135, 83, 34, 11, 49, 248, 138, 146, 90, 169, 177, 225, 146, 231, 193, 136, 160, 179, 101, 203, 153, 75, 62, 197, 232, 9, 32, 97, 23, 198, 65, 18, 66, 185, 64, 177, 12, 170, 55, 206, 115, 68, 150, 179, 183, 198, 53, 120, 160, 227, 192, 118, 249, 179, 26, 124, 161, 58, 113, 98, 98, 224, 228, 205, 164, 172, 153, 78, 251, 158, 25, 137, 60, 191, 228, 212, 100, 185, 33, 13, 9, 157, 128, 138, 8, 179, 196, 195, 132, 110, 117, 41, 152, 72, 153, 71, 134, 57, 196, 230, 196, 97, 82, 239, 73, 160, 74, 249, 200, 230, 255, 68, 45, 40, 108, 70, 19, 163, 218, 198, 164, 190, 228, 180, 14, 31, 107, 3, 15, 40, 113, 218, 190, 66, 35, 178, 80, 195, 24, 30, 205, 59, 198, 129, 144, 4, 116, 90, 235, 107, 172, 86, 116, 7, 242, 185, 199, 209, 151, 140, 69, 238, 103, 18, 174, 70, 147, 11, 192, 6, 56, 56, 63, 102, 150, 21, 139, 173, 72, 140, 190, 118, 99, 214, 129, 201, 108, 3, 84, 129, 219, 207, 120, 231, 167, 251, 174, 195, 121, 145, 99, 170, 105, 20, 206, 243, 54, 81, 86, 189, 195, 229, 51, 167, 200, 131, 213, 151, 78, 52, 98, 172, 111, 25, 227, 249, 206, 38, 128, 10, 226, 72, 164, 92, 95, 13, 211, 164, 138, 24, 89, 105, 34, 78, 108, 214, 175, 154, 4, 130, 65, 189, 202, 201, 128, 13, 148, 174, 238, 151, 14, 8, 72, 143, 185, 97, 227, 106, 118, 155, 108, 24, 93, 24, 91, 70, 5, 220, 152, 8, 81, 113, 150, 187, 169, 208, 10, 62, 55, 188, 164, 102, 193, 145, 135, 72, 109, 177, 4, 238, 3, 150, 45, 57, 254, 71, 62, 39, 99, 85, 97, 142, 68, 201, 101, 240, 80, 130, 187, 2, 122, 123, 170, 75, 204, 109, 140, 7, 117, 193, 232, 164, 129, 231, 125, 243, 109, 218, 217, 30, 117, 169, 130, 48, 41, 55, 245, 67, 161, 31, 231, 25, 34, 220, 212, 244, 111, 232, 249, 81, 249, 28, 222, 65, 43, 53, 149, 7, 242, 179, 182, 223, 3, 116, 191, 229, 92, 154, 18, 106, 211, 28, 226, 84, 230, 125, 100, 25, 77, 50, 215, 149, 94, 199, 145, 201, 85, 94, 165, 169, 23, 252, 71, 171, 163, 25, 233, 9, 222, 130, 218, 148, 110, 179, 110, 18, 175, 249, 54, 112, 132, 2, 34, 130, 149, 219, 39, 18, 242, 252, 128, 124, 149, 9, 42, 134, 175, 215, 18, 32, 105, 157, 241, 62, 45, 47, 223, 40, 87, 151, 108, 177, 230, 5, 247, 47, 27, 46, 218, 186, 219, 163, 255, 5, 80, 18, 33, 254, 129, 51, 60, 19, 145, 124, 133, 215, 37, 206, 146, 121, 30, 17, 94, 176, 40, 154, 93, 11, 51, 48, 144, 27, 184, 176, 237, 20, 106, 190, 184, 19, 129, 183, 51, 31, 28, 80, 143, 177, 78, 5, 123, 5, 216, 176, 25, 10, 158, 116, 163, 208, 70, 220, 210, 78, 122, 183, 71, 4, 153, 69, 179, 216, 161, 32, 196, 246, 216, 142, 103, 102, 27, 85, 87, 58, 169, 179, 97, 54, 116, 136, 161, 239, 125, 255, 217, 103, 214, 74, 21, 24];
</code></pre>



<a name="0x1_algebra_BLS12_381_GT_GENERATOR_SERIALIZED"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_GT_GENERATOR_SERIALIZED">BLS12_381_GT_GENERATOR_SERIALIZED</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [182, 137, 23, 202, 170, 5, 67, 168, 8, 197, 57, 8, 246, 148, 209, 182, 231, 179, 141, 233, 12, 233, 216, 61, 80, 92, 161, 239, 27, 68, 45, 39, 39, 215, 208, 104, 49, 216, 178, 167, 146, 10, 252, 113, 216, 235, 80, 18, 15, 23, 160, 234, 152, 42, 136, 89, 29, 159, 67, 80, 62, 148, 168, 241, 171, 175, 46, 69, 137, 246, 90, 175, 183, 146, 60, 72, 69, 64, 168, 104, 136, 52, 50, 165, 198, 14, 117, 134, 11, 17, 229, 70, 91, 28, 154, 8, 135, 62, 194, 158, 132, 76, 28, 136, 140, 179, 150, 147, 48, 87, 255, 221, 84, 27, 3, 165, 34, 14, 218, 22, 178, 179, 166, 114, 142, 166, 120, 3, 76, 227, 156, 104, 57, 242, 3, 151, 32, 45, 124, 92, 68, 187, 104, 19, 79, 147, 25, 60, 236, 33, 80, 49, 177, 115, 153, 87, 122, 29, 229, 255, 31, 91, 6, 102, 189, 216, 144, 124, 97, 167, 101, 30, 78, 121, 224, 55, 41, 81, 80, 90, 7, 250, 115, 194, 87, 136, 219, 110, 184, 2, 53, 25, 165, 170, 151, 181, 31, 28, 173, 29, 67, 216, 170, 187, 255, 77, 195, 25, 199, 154, 88, 202, 252, 3, 82, 24, 116, 124, 47, 117, 218, 248, 242, 251, 124, 0, 196, 77, 168, 91, 18, 145, 19, 23, 61, 71, 34, 245, 178, 1, 182, 180, 69, 64, 98, 233, 234, 139, 167, 140, 92, 163, 202, 218, 247, 35, 139, 71, 186, 206, 92, 229, 97, 128, 74, 225, 107, 143, 75, 99, 218, 70, 69, 184, 69, 122, 147, 121, 60, 189, 100, 167, 37, 79, 21, 7, 129, 1, 157, 232, 126, 228, 38, 130, 148, 15, 62, 112, 168, 134, 131, 213, 18, 187, 44, 63, 183, 178, 67, 77, 165, 222, 219, 178, 208, 179, 251, 132, 135, 200, 77, 160, 213, 195, 21, 189, 214, 156, 70, 251, 5, 210, 55, 99, 242, 25, 26, 171, 213, 213, 194, 225, 42, 16, 184, 240, 2, 255, 104, 27, 253, 27, 46, 224, 191, 97, 157, 128, 210, 167, 149, 235, 34, 242, 170, 123, 133, 213, 255, 182, 113, 167, 12, 148, 128, 159, 13, 175, 197, 183, 62, 162, 251, 6, 87, 186, 226, 51, 115, 180, 147, 27, 201, 250, 50, 30, 136, 72, 239, 120, 137, 78, 152, 123, 255, 21, 13, 125, 103, 26, 238, 48, 179, 147, 26, 200, 197, 14, 11, 59, 8, 104, 239, 252, 56, 191, 72, 205, 36, 180, 184, 17, 162, 153, 90, 194, 160, 145, 34, 190, 217, 253, 159, 160, 197, 16, 168, 123, 16, 41, 8, 54, 173, 6, 200, 32, 51, 151, 181, 106, 120, 233, 160, 198, 28, 119, 229, 108, 203, 79, 27, 195, 211, 252, 174, 167, 85, 15, 53, 3, 239, 227, 15, 45, 36, 240, 8, 145, 203, 69, 98, 6, 5, 252, 250, 164, 41, 38, 135, 179, 167, 219, 124, 28, 5, 84, 169, 53, 121, 232, 137, 161, 33, 253, 143, 114, 100, 155, 36, 2, 153, 106, 8, 77, 35, 129, 197, 4, 49, 102, 103, 59, 56, 73, 228, 253, 30, 126, 228, 175, 36, 170, 142, 212, 67, 245, 109, 253, 107, 104, 255, 222, 68, 53, 169, 44, 215, 164, 172, 59, 199, 126, 26, 208, 203, 114, 134, 6, 207, 8, 191, 99, 134, 229, 65, 15];
</code></pre>



<a name="0x1_algebra_BLS12_381_R"></a>



<pre><code><b>const</b> <a href="algebra.md#0x1_algebra_BLS12_381_R">BLS12_381_R</a>: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [1, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8, 216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115];
</code></pre>



<a name="0x1_algebra_bls12_381_fq_format"></a>

## Function `bls12_381_fq_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code> element is represented by a byte array <code>b[]</code> of size 48 using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_format">bls12_381_fq_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_format">bls12_381_fq_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"01" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq_bendian_format"></a>

## Function `bls12_381_fq_bendian_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a></code> element is represented by a byte array <code>b[]</code> of size 48 using big-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_bendian_format">bls12_381_fq_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq_bendian_format">bls12_381_fq_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0101" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq2_format"></a>

## Function `bls12_381_fq2_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a></code> element in form <code>(c_0+c_1*u)</code> is represented by a byte array <code>b[]</code> of size 96.
<code>b[0..48]</code> is <code>c_0</code> serialized in <code>bls12_381_fq_format</code>.
<code>b[48..96]</code> is <code>c_1</code> serialized in <code>bls12_381_fq_format</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq2_format">bls12_381_fq2_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq2_format">bls12_381_fq2_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"02" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq6_format"></a>

## Function `bls12_381_fq6_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a></code> element in form <code>(c_0+c_1*v+c_2*v^2)</code> is represented by a byte array <code>b[]</code> of size 288.
<code>b[0..96]</code> is <code>c_0</code> serialized in <code>bls12_381_fq2_format</code>.
<code>b[96..192]</code> is <code>c_1</code> serialized in <code>bls12_381_fq2_format</code>.
<code>b[192..288]</code> is <code>c_2</code> serialized in <code>bls12_381_fq2_format</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq6_format">bls12_381_fq6_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq6_format">bls12_381_fq6_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"03" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq12_format"></a>

## Function `bls12_381_fq12_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code> element in form <code>(c_0+c_1*w)</code> is represented by a byte array <code>b[]</code> of size 576.
<code>b[0..288]</code> is <code>c_0</code> serialized in <code>bls12_381_fq6_format</code>.
<code>b[288..576]</code> is <code>c_1</code> serialized in <code>bls12_381_fq6_format</code>.
Also used in <code>ark_bls12_381::Fq12::deserialize()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq12_format">bls12_381_fq12_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq12_format">bls12_381_fq12_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"04" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_parent_uncompressed_format"></a>

## Function `bls12_381_g1_parent_uncompressed_format`

A serialization scheme where an <code><a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> element is represented by a byte array <code>b[]</code> of size 96.
<code>b[95] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq)</code>,
<code>[b[0], ..., b[47] & 0x3f]</code> is <code>x</code> serialized in <code>bls12_381_fq_format</code>, and
<code>[b[48], ..., b[95] & 0x3f]</code> is <code>y</code> serialized in <code>bls12_381_fq_format</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"05" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_parent_compressed_format"></a>

## Function `bls12_381_g1_parent_compressed_format`

A serialization scheme where an <code><a href="algebra.md#0x1_algebra_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> element is represented by a byte array <code>b[]</code> of size 48.
<code>b[47] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq)</code>,
<code>[b[0], ..., b[47] & 0x3f]</code> is <code>x</code> serialized in <code>bls12_381_fq_format</code>, and
the positiveness flag <code>b_47 & 0x80</code> is 1 if and only if <code>y &gt; -y</code> (as unsigned integers).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0501" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_uncompressed_format"></a>

## Function `bls12_381_g1_uncompressed_format`

Effectively <code>bls12_381_g1_parent_uncompressed_format</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"06" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_compressed_format"></a>

## Function `bls12_381_g1_compressed_format`

Effectively <code>bls12_381_g1_parent_compressed_format</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0601" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_parent_uncompressed_format"></a>

## Function `bls12_381_g2_parent_uncompressed_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> element is represented by a byte array <code>b[]</code> of size 192.
<code>b[191] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>,
<code>b[0..96]</code> is <code>x</code> serialized in <code>bls12_381_fq2_format</code>, and
<code>[b[96], ..., b[191] & 0x3f]</code> is <code>y</code> serialized in <code>bls12_381_fq2_format</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_uncompressed_format">bls12_381_g2_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_uncompressed_format">bls12_381_g2_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"07" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_parent_compressed_format"></a>

## Function `bls12_381_g2_parent_compressed_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> element is represented by a byte array <code>b[]</code> of size 96.
<code>b[95] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>,
<code>[b[0], ..., b[95] & 0x3f]</code> is <code>x</code> serialized in <code>bls12_381_fq2_format</code>, and
the positiveness flag <code>b[95] & 0x80</code> is 1 if and only if <code>y &gt; -y</code> (<code>y</code> and <code>-y</code> treated as unsigned integers).


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_compressed_format">bls12_381_g2_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_parent_compressed_format">bls12_381_g2_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0701" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_uncompressed_format"></a>

## Function `bls12_381_g2_uncompressed_format`

Effectively <code>bls12_381_g2_parent_uncompressed_format</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> elements.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"08" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_compressed_format"></a>

## Function `bls12_381_g2_compressed_format`

Effectively <code>bls12_381_g2_parent_compressed_format</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> elements.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0801" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_gt_format"></a>

## Function `bls12_381_gt_format`

Effectively <code><a href="algebra.md#0x1_algebra_bls12_381_fq12_format">bls12_381_fq12_format</a>()</code> but only applicable to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_gt_format">bls12_381_gt_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_gt_format">bls12_381_gt_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"09" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fr_lendian_format"></a>

## Function `bls12_381_fr_lendian_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> element is represented by a byte array <code>b[]</code> of size 32 using little-endian byte order.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a" }
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fr_bendian_format"></a>

## Function `bls12_381_fr_bendian_format`

A serialization scheme where a <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> element is represented by a byte array <code>b[]</code> of size 32 using big-endian byte order.


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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_div"></a>

## Function `field_div`

Compute <code>x / y</code> for elements <code>x</code> and <code>y</code> of a field <code>S</code>.
Return none if y is the additive identity of field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_sqr_internal">field_sqr_internal</a>&lt;S&gt;(x.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_inv"></a>

## Function `field_inv`

Compute <code>x^(-1)</code> for an element <code>x</code> of a field <code>S</code>.
Return none if <code>x</code> is the additive identity of field <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_field_is_zero_internal">field_is_zero_internal</a>&lt;S&gt;(x.handle)
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;()
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;()
    }
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_neg_internal">group_neg_internal</a>&lt;G&gt;(element_p.handle)
    }
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_double"></a>

## Function `group_double`

Compute <code>2*P</code> for an element <code>P</code> of a group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_double">group_double</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_double">group_double</a>&lt;G&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_double_internal">group_double_internal</a>&lt;G&gt;(element_p.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul"></a>

## Function `group_scalar_mul`

Compute <code>k*p</code>, where <code>p</code> is an element of a group <code>G</code> and <code>k</code> is an element of the scalar field <code>S</code> of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G, S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, scalar_k: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G, S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, scalar_k: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_group_scalar_mul">abort_unless_type_pair_enabled_for_group_scalar_mul</a>&lt;G,S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_scalar_mul_internal">group_scalar_mul_internal</a>&lt;G, S&gt;(element_p.handle, scalar_k.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize"></a>

## Function `deserialize`

Deserializate a byte array to an element of an algebraic structure <code>S</code> using a given <code>scheme</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S&gt;(scheme: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_serialization_scheme_enabled">abort_unless_type_serialization_scheme_enabled</a>&lt;S&gt;(scheme);
    <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(scheme, element.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_group_order"></a>

## Function `group_order`

Get the order of group <code>G</code>, little-endian encoded as a byte array.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_order">group_order</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_order">group_order</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_enabled_for_basic_operation">abort_unless_type_enabled_for_basic_operation</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_group_order_internal">group_order_internal</a>&lt;G&gt;()
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
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_upcast">abort_unless_type_pair_enabled_for_upcast</a>&lt;S,L&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt; {
        handle: <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S,L&gt;(element.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_downcast"></a>

## Function `downcast`

Try casting an element of a structure <code>L</code> to a sub structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L, S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;L&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L,S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_type_pair_enabled_for_upcast">abort_unless_type_pair_enabled_for_upcast</a>&lt;S,L&gt;();
    <b>let</b> (succ, new_handle) = <a href="algebra.md#0x1_algebra_downcast_internal">downcast_internal</a>&lt;L,S&gt;(element.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle: new_handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled"></a>

## Function `abort_unless_generic_algebra_basic_operations_enabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_generic_algebra_basic_operations_enabled">abort_unless_generic_algebra_basic_operations_enabled</a>() {
    <b>if</b> (generic_algebra_basic_operations_enabled()) <b>return</b>;
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

<a name="0x1_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
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

<a name="0x1_algebra_from_u64_internal"></a>

## Function `from_u64_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value: u64): u64;
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

<a name="0x1_algebra_group_add_internal"></a>

## Function `group_add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
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

<a name="0x1_algebra_group_identity_internal"></a>

## Function `group_identity_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;(): u64;
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

<a name="0x1_algebra_group_generator_internal"></a>

## Function `group_generator_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul_internal"></a>

## Function `group_scalar_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_internal">group_scalar_mul_internal</a>&lt;G, S&gt;(scalar_handle: u64, element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul_internal">group_scalar_mul_internal</a>&lt;G, S&gt;(scalar_handle: u64, element_handle: u64): u64;
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

<a name="0x1_algebra_group_neg_internal"></a>

## Function `group_neg_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_neg_internal">group_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg_internal">group_neg_internal</a>&lt;G&gt;(handle: u64): u64;
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

<a name="0x1_algebra_upcast_internal"></a>

## Function `upcast_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S, L&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S,L&gt;(handle: u64): u64;
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


[move-book]: https://move-language.github.io/move/introduction.html
