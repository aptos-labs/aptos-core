
<a name="0x1_algebra"></a>

# Module `0x1::algebra`



-  [Struct `BLS12_381_G1`](#0x1_algebra_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_algebra_BLS12_381_G2)
-  [Struct `BLS12_381_Fq12`](#0x1_algebra_BLS12_381_Fq12)
-  [Struct `BLS12_381_Gt`](#0x1_algebra_BLS12_381_Gt)
-  [Struct `BLS12_381_Fr`](#0x1_algebra_BLS12_381_Fr)
-  [Struct `Element`](#0x1_algebra_Element)
-  [Function `bls12_381_g1_serialization_scheme_uncompressed`](#0x1_algebra_bls12_381_g1_serialization_scheme_uncompressed)
-  [Function `bls12_381_g1_serialization_scheme_compressed`](#0x1_algebra_bls12_381_g1_serialization_scheme_compressed)
-  [Function `bls12_381_g2_serialization_scheme_uncompressed`](#0x1_algebra_bls12_381_g2_serialization_scheme_uncompressed)
-  [Function `bls12_381_g2_serialization_scheme_compressed`](#0x1_algebra_bls12_381_g2_serialization_scheme_compressed)
-  [Function `bls12_381_fq12_serialization_scheme`](#0x1_algebra_bls12_381_fq12_serialization_scheme)
-  [Function `bls12_381_gt_serialization_scheme`](#0x1_algebra_bls12_381_gt_serialization_scheme)
-  [Function `bls12_381_fr_serialization_scheme_lendian`](#0x1_algebra_bls12_381_fr_serialization_scheme_lendian)
-  [Function `bls12_381_fr_serialization_scheme_bendian`](#0x1_algebra_bls12_381_fr_serialization_scheme_bendian)
-  [Function `pairing`](#0x1_algebra_pairing)
-  [Function `pairing_product`](#0x1_algebra_pairing_product)
-  [Function `from_u64`](#0x1_algebra_from_u64)
-  [Function `field_zero`](#0x1_algebra_field_zero)
-  [Function `field_one`](#0x1_algebra_field_one)
-  [Function `field_neg`](#0x1_algebra_field_neg)
-  [Function `field_add`](#0x1_algebra_field_add)
-  [Function `field_sub`](#0x1_algebra_field_sub)
-  [Function `field_mul`](#0x1_algebra_field_mul)
-  [Function `field_div`](#0x1_algebra_field_div)
-  [Function `field_inv`](#0x1_algebra_field_inv)
-  [Function `field_pow`](#0x1_algebra_field_pow)
-  [Function `eq`](#0x1_algebra_eq)
-  [Function `group_identity`](#0x1_algebra_group_identity)
-  [Function `group_generator`](#0x1_algebra_group_generator)
-  [Function `group_neg`](#0x1_algebra_group_neg)
-  [Function `group_add`](#0x1_algebra_group_add)
-  [Function `group_double`](#0x1_algebra_group_double)
-  [Function `group_scalar_mul`](#0x1_algebra_group_scalar_mul)
-  [Function `group_multi_scalar_mul`](#0x1_algebra_group_multi_scalar_mul)
-  [Function `deserialize`](#0x1_algebra_deserialize)
-  [Function `serialize`](#0x1_algebra_serialize)
-  [Function `group_order`](#0x1_algebra_group_order)
-  [Function `upcast`](#0x1_algebra_upcast)
-  [Function `downcast`](#0x1_algebra_downcast)
-  [Function `abort_if_generic_group_basic_operations_disabled`](#0x1_algebra_abort_if_generic_group_basic_operations_disabled)
-  [Function `abort_unless_structure_enabled`](#0x1_algebra_abort_unless_structure_enabled)
-  [Function `deserialize_internal`](#0x1_algebra_deserialize_internal)
-  [Function `serialize_internal`](#0x1_algebra_serialize_internal)
-  [Function `from_u64_internal`](#0x1_algebra_from_u64_internal)
-  [Function `field_add_internal`](#0x1_algebra_field_add_internal)
-  [Function `field_div_internal`](#0x1_algebra_field_div_internal)
-  [Function `field_inv_internal`](#0x1_algebra_field_inv_internal)
-  [Function `field_mul_internal`](#0x1_algebra_field_mul_internal)
-  [Function `field_neg_internal`](#0x1_algebra_field_neg_internal)
-  [Function `field_one_internal`](#0x1_algebra_field_one_internal)
-  [Function `field_pow_internal`](#0x1_algebra_field_pow_internal)
-  [Function `field_sub_internal`](#0x1_algebra_field_sub_internal)
-  [Function `field_zero_internal`](#0x1_algebra_field_zero_internal)
-  [Function `element_add_internal`](#0x1_algebra_element_add_internal)
-  [Function `eq_internal`](#0x1_algebra_eq_internal)
-  [Function `group_identity_internal`](#0x1_algebra_group_identity_internal)
-  [Function `group_order_internal`](#0x1_algebra_group_order_internal)
-  [Function `group_generator_internal`](#0x1_algebra_group_generator_internal)
-  [Function `element_mul_internal`](#0x1_algebra_element_mul_internal)
-  [Function `element_double_internal`](#0x1_algebra_element_double_internal)
-  [Function `element_neg_internal`](#0x1_algebra_element_neg_internal)
-  [Function `element_multi_scalar_mul_internal`](#0x1_algebra_element_multi_scalar_mul_internal)
-  [Function `pairing_product_internal`](#0x1_algebra_pairing_product_internal)
-  [Function `upcast_internal`](#0x1_algebra_upcast_internal)
-  [Function `downcast_internal`](#0x1_algebra_downcast_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_algebra_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`

<code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> represents a group used in BLS12-381 pairing.
<code>Fq</code> is a finite field with <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.
<code>E(Fq)</code> is an elliptic curve <code>y^2=x^3+4</code> defined over <code>Fq</code>.
<code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> is constructed by a subset of the points on <code>E(Fq)</code> and the point at infinity, under point addition. (A subgroup of prime order on <code>E(Fq)</code>.)
The prime order <code>r</code> of <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> is the point at infinity.
There exists a bilinear mapping from <code>(<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>, <a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>)</code> to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.

An <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> represents an element in group <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>.
Scalar multiplication on <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> requires a <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>.



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

<a name="0x1_algebra_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`

<code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> represents a group used in BLS12-381 pairing.
<code>Fq</code> is a finite field with <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.
<code>Fq2</code> is an extension field of <code>Fq</code>, constructed as <code>Fq2=Fq[u]/(u^2+1)</code>.
<code>E(Fq2)</code> is an elliptic curve <code>y^2=x^3+4(u+1)</code> defined over <code>Fq2</code>.
<code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is constructed by a subset of the points on <code>E(Fq2)</code> and the point at infinity, under point addition. (A subgroup of prime order on <code>E(Fq2)</code>.)
The prime order <code>r</code> of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001, same as <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>.
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is the point at infinity.
There exists a bilinear mapping from <code>(<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>, <a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>)</code> to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.

Scalar multiplication on <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> requires a <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>.


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

<a name="0x1_algebra_BLS12_381_Fq12"></a>

## Struct `BLS12_381_Fq12`

A field used in BLS12-381 pairing.
<code>Fq</code> is a finite field with <code>q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab</code>.
<code>Fq2</code> is an extension field of <code>Fq</code>, constructed as <code>Fq2=Fq[u]/(u^2+1)</code>.
<code>Fq6</code> is an extension field of <code>Fq2</code>, constructed as <code>Fq6=Fq2[v]/(v^2-u-1)</code>.
<code>Fq12</code> is an extension field of <code>Fq6</code>, constructed as <code>Fq12=Fq6[w]/(w^2-v)</code>.


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

<a name="0x1_algebra_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`

<code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> represents the target group of the pairing defined over the BLS12-381 curves.
<code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> is a multiplicative subgroup of <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code>.
The order <code>r</code> of <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001. (Same as <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> and <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code>.)
The identity of <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> is 1.
There exists a bilinear mapping from <code>(<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>, <a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>)</code> to <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.
Scalar multiplication on <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> requires a <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code>.


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

The scalar field for groups <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>, <code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code> and <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.
A <code><a href="algebra.md#0x1_algebra_Element">Element</a>&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;</code> is an integer between 0 and <code>r-1</code> where <code>r</code> is the order of <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code>/<code><a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a></code>/<code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code>.


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

This struct represents an element of some algebraic structure <code>S</code>.


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

<a name="0x1_algebra_bls12_381_g1_serialization_scheme_uncompressed"></a>

## Function `bls12_381_g1_serialization_scheme_uncompressed`

A serialization scheme for <code>BLS12-381-G1</code> elements.
It assumes a 96-byte serialization <code>[b_0, ..., b_95]</code> with the following rules.
- <code>b_95 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq)</code>, with the following rules.
- <code>[b_0, ..., b_47 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x</code>.
- <code>[b_48, ..., b_95 & 0x3f]</code> is a 48-byte little-endian encoding of 'y'.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_serialization_scheme_uncompressed">bls12_381_g1_serialization_scheme_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_serialization_scheme_uncompressed">bls12_381_g1_serialization_scheme_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(0)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g1_serialization_scheme_compressed"></a>

## Function `bls12_381_g1_serialization_scheme_compressed`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a></code> elements.
It assumes a 48-byte serialization <code>[b_0, ..., b_47]</code> with the following rules.
- <code>b_47 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve, with the following rules.
- <code>[b_0, ..., b_47 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x</code>.
- <code>b_47 & 0x80</code> is the positiveness flag.
- The positiveness flag is 1 if and only if <code>y &gt; -y</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_serialization_scheme_compressed">bls12_381_g1_serialization_scheme_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g1_serialization_scheme_compressed">bls12_381_g1_serialization_scheme_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(1)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_serialization_scheme_uncompressed"></a>

## Function `bls12_381_g2_serialization_scheme_uncompressed`

A serialization scheme for <code>BLS12-381-G2</code> elements.
It assumes a 192-byte serialization <code>[b_0, ..., b_191]</code>, with the following rules.
- <code>b_191 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>, with the following rules.
- <code>[b_0, ..., b_95]</code> is a 96-byte serialization of <code>x=(x_0+x_1*u)</code>.
- <code>[b_0, ..., b_47]</code> is a 48-byte little-endian encoding of <code>x_0</code>.
- <code>[b_48, ..., b_95]</code> is a 48-byte little-endian encoding of <code>x_1</code>.
- <code>[b_96, ..., b_191 & 0x3f]</code> is a 96-byte serialization of 'y=(y_0+y_1*u)'.
- <code>[b_96, ..., b_143]</code> is a 48-byte little-endian encoding of <code>y_0</code>.
- <code>[b_144, ..., b_191 & 0x3f]</code> is a 48-byte little-endian encoding of <code>y_1</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_serialization_scheme_uncompressed">bls12_381_g2_serialization_scheme_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_serialization_scheme_uncompressed">bls12_381_g2_serialization_scheme_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(2)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_g2_serialization_scheme_compressed"></a>

## Function `bls12_381_g2_serialization_scheme_compressed`

A serialization scheme for <code>BLS12-381-G2</code> elements.
It assumes a 96-byte serialization <code>[b_0, ..., b_95]</code>, with the following rules.
- <code>b_95 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>, with the following rules.
- <code>[b_0, ..., b_95 & 0x3f]</code> is a 96-byte little-endian encoding of <code>x=(x_0+x_1*u)</code>.
- <code>[b_0, ..., b_47]</code> is a 48-byte little-endian encoding of <code>x_0</code>.
- <code>[b_48, ..., b_95 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x_1</code>.
- <code>b_95 & 0x80</code> is the positiveness flag.
- The positiveness flag is 1 if and only if <code>y &gt; -y</code>.
- Here <code>a=(a_0+a_1*u)</code> is considered greater than <code>b=(b_0+b_1*u)</code> if <code>a_1&gt;b_1 OR (a_1=b_1 AND a_0&gt;b_0)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_serialization_scheme_compressed">bls12_381_g2_serialization_scheme_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_g2_serialization_scheme_compressed">bls12_381_g2_serialization_scheme_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(3)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fq12_serialization_scheme"></a>

## Function `bls12_381_fq12_serialization_scheme`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a></code> elements.
It assumes a 576-byte serialization <code>[b_0, ..., b_575]</code>, with the following rules.
- Assume the given element is <code>e=c_0+c_1*w</code> where <code>c_i=c_i0+c_i1*v+c_i2*v^2 for i=0..1</code> and <code>c_ij=c_ij0+c_ij1*u for j=0..2</code>.
- <code>[b_0, ..., b_575]</code> is a concatenation of 12 encoded <code>Fq</code> elements: <code>c_000, c_001, c_010, c_011, c_020, c_021, c_100, c_101, c_110, c_111, c_120, c_121</code>.
- Every <code>c_ijk</code> uses a 48-byte little-endian encoding.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq12_serialization_scheme">bls12_381_fq12_serialization_scheme</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fq12_serialization_scheme">bls12_381_fq12_serialization_scheme</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(5)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_gt_serialization_scheme"></a>

## Function `bls12_381_gt_serialization_scheme`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.
It assumes a 576-byte serialization <code>[b_0, ..., b_575]</code>, with the following rules.
- Assume the given element is <code>e=c_0+c_1*w</code> where <code>c_i=c_i0+c_i1*v+c_i2*v^2 for i=0..1</code> and <code>c_ij=c_ij0+c_ij1*u for j=0..2</code>.
- <code>[b_0, ..., b_575]</code> is a concatenation of 12 encoded <code>Fq</code> elements: <code>c_000, c_001, c_010, c_011, c_020, c_021, c_100, c_101, c_110, c_111, c_120, c_121</code>.
- Every <code>c_ijk</code> uses a 48-byte little-endian encoding.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_gt_serialization_scheme">bls12_381_gt_serialization_scheme</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_gt_serialization_scheme">bls12_381_gt_serialization_scheme</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(9)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fr_serialization_scheme_lendian"></a>

## Function `bls12_381_fr_serialization_scheme_lendian`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> elements.
It assumes a 32-byte little-endian serialization.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_serialization_scheme_lendian">bls12_381_fr_serialization_scheme_lendian</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_serialization_scheme_lendian">bls12_381_fr_serialization_scheme_lendian</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(0)
}
</code></pre>



</details>

<a name="0x1_algebra_bls12_381_fr_serialization_scheme_bendian"></a>

## Function `bls12_381_fr_serialization_scheme_bendian`

A serialization scheme for <code><a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a></code> elements.
It assumes a 32-byte big-endian serialization.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_serialization_scheme_bendian">bls12_381_fr_serialization_scheme_bendian</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_bls12_381_fr_serialization_scheme_bendian">bls12_381_fr_serialization_scheme_bendian</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    std::vector::singleton(1)
}
</code></pre>



</details>

<a name="0x1_algebra_pairing"></a>

## Function `pairing`

Computes a pairing function (a.k.a., bilinear map) on <code>element_1</code> and <code>element_2</code>.
Returns an element in the target group <code>Gt</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1, G2, Gt&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1,G2,Gt&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="algebra.md#0x1_algebra_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(std::vector::singleton(element_1.handle), std::vector::singleton(element_2.handle))
    }
}
</code></pre>



</details>

<a name="0x1_algebra_pairing_product"></a>

## Function `pairing_product`

Compute the product of multiple pairings.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product">pairing_product</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product">pairing_product</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G1&gt;();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G2&gt;();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;Gt&gt;();
    <b>let</b> num_g1 = std::vector::length(g1_elements);
    <b>let</b> num_g2 = std::vector::length(g2_elements);
    <b>assert</b>!(num_g1 == num_g2, std::error::invalid_argument(1));
    <b>let</b> g1_handles = std::vector::empty();
    <b>let</b> g2_handles = std::vector::empty();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_g2) {
        std::vector::push_back(&<b>mut</b> g1_handles, std::vector::borrow(g1_elements, i).handle);
        std::vector::push_back(&<b>mut</b> g2_handles, std::vector::borrow(g2_elements, i).handle);
        i = i + 1;
    };

    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="algebra.md#0x1_algebra_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(g1_handles, g2_handles)
    }
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> (succ, handle) = <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;S&gt;(x.handle, y.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle })
    } <b>else</b> {
        none()
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;S&gt;(x.handle);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_pow"></a>

## Function `field_pow`

Compute <code>b^e</code> for an element <code>b</code> of a field <code>S</code> and an integer <code>e</code> in little-endian encoding.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_pow">field_pow</a>&lt;S&gt;(b: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, e: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_pow">field_pow</a>&lt;S&gt;(b: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, e: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_pow_internal">field_pow_internal</a>&lt;S&gt;(b.handle, *e)
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_eq_internal">eq_internal</a>&lt;S&gt;(x.handle, y.handle)
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_element_neg_internal">element_neg_internal</a>&lt;G&gt;(element_p.handle)
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_element_add_internal">element_add_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_element_double_internal">element_double_internal</a>&lt;G&gt;(element_p.handle)
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_element_mul_internal">element_mul_internal</a>&lt;G, S&gt;(element_p.handle, scalar_k.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul"></a>

## Function `group_multi_scalar_mul`

Compute <code>k[0]*P[0]+...+k[n-1]*P[n-1]</code> where
<code>P[]</code> are elements of group <code>G</code>,
<code>k[]</code> are elements of the scalar field <code>S</code> of group <code>G</code>,
and both <code>P[]</code> and <code>k[]</code> have the same size <code>n</code>.
Abort if the number of elements and that of scalars do not match.
This function is much faster and cheaper than calling <code>group_scalar_mul</code> and adding up the results using <code>group_add</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> num_scalars = std::vector::length(scalars);
    <b>let</b> scalar_handles = std::vector::empty();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_scalars) {
        std::vector::push_back(&<b>mut</b> scalar_handles, std::vector::borrow(scalars, i).handle);
        i = i + 1;
    };

    <b>let</b> num_elements = std::vector::length(elements);
    <b>let</b> element_handles = std::vector::empty();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_elements) {
        std::vector::push_back(&<b>mut</b> element_handles, std::vector::borrow(elements, i).handle);
        i = i + 1;
    };

    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_element_multi_scalar_mul_internal">element_multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles, scalar_handles)
    }

}
</code></pre>



</details>

<a name="0x1_algebra_deserialize"></a>

## Function `deserialize`

Deserializate a byte array to an element of an algebraic structure <code>S</code> with a given scheme.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(scheme_id, *bytes);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
            handle
        };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_serialize"></a>

## Function `serialize`

Serialize an element of an algebraic structure <code>S</code> to a byte array with a given scheme.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize">serialize</a>&lt;S&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize">serialize</a>&lt;S&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, scalar: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(scheme_id, scalar.handle)
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;G&gt;();
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
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;L&gt;();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt; {
        handle: <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S,L&gt;(element.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_downcast"></a>

## Function `downcast`

Cast an element of a structure <code>L</code> to a sub structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L, S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;L&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L,S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;();
    <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;L&gt;();
    <b>let</b> (succ, new_handle) = <a href="algebra.md#0x1_algebra_downcast_internal">downcast_internal</a>&lt;L,S&gt;(element.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle: new_handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_abort_if_generic_group_basic_operations_disabled"></a>

## Function `abort_if_generic_group_basic_operations_disabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_if_generic_group_basic_operations_disabled">abort_if_generic_group_basic_operations_disabled</a>() {
    <b>if</b> (!std::features::generic_group_basic_operations_enabled()) {
        <b>abort</b>(std::error::not_implemented(0))
    }
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_structure_enabled"></a>

## Function `abort_unless_structure_enabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_structure_enabled">abort_unless_structure_enabled</a>&lt;S&gt;() {
    <b>let</b> type = type_of&lt;S&gt;();
    <b>if</b> ((type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>&gt;() || type == type_of&lt;<a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>&gt;())
        && std::features::bls12_381_groups_enabled()
    ) {
        // Let go.
    } <b>else</b> {
        <b>abort</b>(std::error::not_implemented(0))
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G&gt;(scheme_id: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
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

<a name="0x1_algebra_field_pow_internal"></a>

## Function `field_pow_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_pow_internal">field_pow_internal</a>&lt;F&gt;(handle: u64, e: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_pow_internal">field_pow_internal</a>&lt;F&gt;(handle: u64, e: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
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

<a name="0x1_algebra_element_add_internal"></a>

## Function `element_add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_element_add_internal">element_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_element_add_internal">element_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
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

<a name="0x1_algebra_element_mul_internal"></a>

## Function `element_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_element_mul_internal">element_mul_internal</a>&lt;G, S&gt;(scalar_handle: u64, element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_element_mul_internal">element_mul_internal</a>&lt;G, S&gt;(scalar_handle: u64, element_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_element_double_internal"></a>

## Function `element_double_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_element_double_internal">element_double_internal</a>&lt;G&gt;(element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_element_double_internal">element_double_internal</a>&lt;G&gt;(element_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_element_neg_internal"></a>

## Function `element_neg_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_element_neg_internal">element_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_element_neg_internal">element_neg_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_element_multi_scalar_mul_internal"></a>

## Function `element_multi_scalar_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_element_multi_scalar_mul_internal">element_multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_element_multi_scalar_mul_internal">element_multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_pairing_product_internal"></a>

## Function `pairing_product_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product_internal">pairing_product_internal</a>&lt;G1, G2, Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
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
