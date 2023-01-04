
<a name="0x1_curves"></a>

# Module `0x1::curves`



-  [Struct `BLS12_381_G1`](#0x1_curves_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_curves_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_curves_BLS12_381_Gt)
-  [Struct `Scalar`](#0x1_curves_Scalar)
-  [Struct `Element`](#0x1_curves_Element)
-  [Constants](#@Constants_0)
-  [Function `pairing`](#0x1_curves_pairing)
-  [Function `multi_pairing`](#0x1_curves_multi_pairing)
-  [Function `scalar_from_u64`](#0x1_curves_scalar_from_u64)
-  [Function `scalar_neg`](#0x1_curves_scalar_neg)
-  [Function `scalar_add`](#0x1_curves_scalar_add)
-  [Function `scalar_mul`](#0x1_curves_scalar_mul)
-  [Function `scalar_inv`](#0x1_curves_scalar_inv)
-  [Function `scalar_eq`](#0x1_curves_scalar_eq)
-  [Function `identity`](#0x1_curves_identity)
-  [Function `generator`](#0x1_curves_generator)
-  [Function `element_neg`](#0x1_curves_element_neg)
-  [Function `element_add`](#0x1_curves_element_add)
-  [Function `element_mul`](#0x1_curves_element_mul)
-  [Function `simul_point_mul`](#0x1_curves_simul_point_mul)
-  [Function `scalar_from_bytes`](#0x1_curves_scalar_from_bytes)
-  [Function `scalar_to_bytes`](#0x1_curves_scalar_to_bytes)
-  [Function `serialize_element_uncompressed`](#0x1_curves_serialize_element_uncompressed)
-  [Function `serialize_element_compressed`](#0x1_curves_serialize_element_compressed)
-  [Function `deserialize_element_uncompressed`](#0x1_curves_deserialize_element_uncompressed)
-  [Function `deserialize_element_compressed`](#0x1_curves_deserialize_element_compressed)
-  [Function `element_eq`](#0x1_curves_element_eq)
-  [Function `abort_if_feature_disabled`](#0x1_curves_abort_if_feature_disabled)
-  [Function `deserialize_element_uncompressed_internal`](#0x1_curves_deserialize_element_uncompressed_internal)
-  [Function `deserialize_element_compressed_internal`](#0x1_curves_deserialize_element_compressed_internal)
-  [Function `scalar_from_u64_internal`](#0x1_curves_scalar_from_u64_internal)
-  [Function `scalar_from_bytes_internal`](#0x1_curves_scalar_from_bytes_internal)
-  [Function `scalar_neg_internal`](#0x1_curves_scalar_neg_internal)
-  [Function `scalar_add_internal`](#0x1_curves_scalar_add_internal)
-  [Function `scalar_mul_internal`](#0x1_curves_scalar_mul_internal)
-  [Function `scalar_inv_internal`](#0x1_curves_scalar_inv_internal)
-  [Function `scalar_eq_internal`](#0x1_curves_scalar_eq_internal)
-  [Function `scalar_to_bytes_internal`](#0x1_curves_scalar_to_bytes_internal)
-  [Function `element_add_internal`](#0x1_curves_element_add_internal)
-  [Function `element_eq_internal`](#0x1_curves_element_eq_internal)
-  [Function `identity_internal`](#0x1_curves_identity_internal)
-  [Function `generator_internal`](#0x1_curves_generator_internal)
-  [Function `element_mul_internal`](#0x1_curves_element_mul_internal)
-  [Function `element_neg_internal`](#0x1_curves_element_neg_internal)
-  [Function `serialize_element_uncompressed_internal`](#0x1_curves_serialize_element_uncompressed_internal)
-  [Function `serialize_element_compressed_internal`](#0x1_curves_serialize_element_compressed_internal)
-  [Function `pairing_internal`](#0x1_curves_pairing_internal)
-  [Function `multi_pairing_internal`](#0x1_curves_multi_pairing_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_curves_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`

A phantom type that represents the 1st pairing input group <code>G1</code> in BLS12-381 pairing.

In BLS12-381, a finite field <code>Fq</code> is used, where
<code>q</code> equals to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.
A curve <code>E(Fq)</code> is defined as <code>y^2=x^3+4</code> over <code>Fq</code>.
<code>G1</code> is formed by a subset of points on <code>E(Fq)</code>.
<code>G1</code> has a prime order <code>r</code> with value 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.

A <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> is an integer between 0 and <code>r-1</code>.

Function <code><a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> and <code><a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>
assumes a 32-byte little-endian encoding of a <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>.

An <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> is an element in <code>G1</code>.

Function <code><a href="curves.md#0x1_curves_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> and <code><a href="curves.md#0x1_curves_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>
assumes a 96-byte encoding <code>[b_0, ..., b_95]</code> of an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>, with the following rules.
- <code>b_95 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq)</code>, with the following rules.
- <code>[b_0, ..., b_47 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x</code>.
- <code>[b_48, ..., b_95 & 0x3f]</code> is a 48-byte little-endian encoding of 'y'.

Function <code><a href="curves.md#0x1_curves_serialize_element_compressed">serialize_element_compressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> and <code><a href="curves.md#0x1_curves_deserialize_element_compressed">deserialize_element_compressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code>
assumes a 48-byte encoding <code>[b_0, ..., b_47]</code> of an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;</code> with the following rules.
- <code>b_47 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve, with the following rules.
- <code>[b_0, ..., b_47 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x</code>.
- <code>b_47 & 0x80</code> is the positiveness flag.
- The positiveness flag is 1 if and only if <code>y &gt; -y</code>.


<pre><code><b>struct</b> <a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>
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

<a name="0x1_curves_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`

A phantom type that represents the 2nd pairing input group <code>G2</code> in BLS12-381 pairing.

In BLS12-381, a finite field <code>Fq</code> is used, where
<code>q</code> equals to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.
<code>Fq2=Fq[u]/(u^2+1)</code> is a quadratic extension of <code>Fq</code>.
A curve <code>E(Fq2)</code> is defined as <code>y^2=x^3+4(u+1)</code> over <code>Fq2</code>.
<code>G2</code> is formed by a subset of points on <code>E(Fq2)</code>.
<code>G2</code> has a prime order <code>r</code> with value 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.

A <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> is an integer between 0 and <code>r-1</code>.

Function <code><a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> and <code><a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>
assumes a 32-byte little-endian encoding of a <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>.

An <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> is an element in <code>G2</code>.

Function <code><a href="curves.md#0x1_curves_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> and <code><a href="curves.md#0x1_curves_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>
assumes a 192-byte encoding <code>[b_0, ..., b_191]</code> of an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>, with the following rules.
- <code>b_191 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>, with the following rules.
- <code>[b_0, ..., b_95]</code> is a 96-byte encoding of <code>x=(x_0+x_1*u)</code>.
- <code>[b_0, ..., b_47]</code> is a 48-byte little-endian encoding of <code>x_0</code>.
- <code>[b_48, ..., b_95]</code> is a 48-byte little-endian encoding of <code>x_1</code>.
- <code>[b_96, ..., b_191 & 0x3f]</code> is a 96-byte encoding of 'y=(y_0+y_1*u)'.
- <code>[b_96, ..., b_143]</code> is a 48-byte little-endian encoding of <code>y_0</code>.
- <code>[b_144, ..., b_191 & 0x3f]</code> is a 48-byte little-endian encoding of <code>y_1</code>.

Function <code><a href="curves.md#0x1_curves_serialize_element_compressed">serialize_element_compressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> and <code><a href="curves.md#0x1_curves_deserialize_element_compressed">deserialize_element_compressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code>
assumes a 96-byte encoding <code>[b_0, ..., b_95]</code> of an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> with the following rules.
- <code>b_95 & 0x40</code> is the infinity flag.
- The infinity flag is 1 if and only if the element is the point at infinity.
- The infinity flag is 0 if and only if the element is a point <code>(x,y)</code> on curve <code>E(Fq2)</code>, with the following rules.
- <code>[b_0, ..., b_95 & 0x3f]</code> is a 96-byte little-endian encoding of <code>x=(x_0+x_1*u)</code>.
- <code>[b_0, ..., b_47]</code> is a 48-byte little-endian encoding of <code>x_0</code>.
- <code>[b_48, ..., b_95 & 0x3f]</code> is a 48-byte little-endian encoding of <code>x_1</code>.
- <code>b_95 & 0x80</code> is the positiveness flag.
- The positiveness flag is 1 if and only if <code>y &gt; -y</code>.
- Here <code>a=(a_0+a_1*u)</code> is considered greater than <code>b=(b_0+b_1*u)</code> if <code>a_1&gt;b_1 OR (a_1=b_1 AND a_0&gt;b_0)</code>.


<pre><code><b>struct</b> <a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>
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

<a name="0x1_curves_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`

A phantom type that represents the 2nd pairing input group <code>G2</code> in BLS12-381 pairing.

In BLS12-381, a finite field <code>Fq</code> is used, where
<code>q</code> equals to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.
<code>Fq2</code> is an extension field of <code>Fq</code>, constructed as <code>Fq2=Fq[u]/(u^2+1)</code>.
<code>Fq6</code> is an extension field of <code>Fq2</code>, constructed as <code>Fq6=Fq2[v]/(v^2-u-1)</code>.
<code>Fq12</code> is an extension field of <code>Fq6</code>, constructed as <code>Fq12=Fq6[w]/(w^2-v)</code>.
<code>Gt</code> is the multiplicative subgroup of <code>Fq12</code>.
<code>Gt</code> has a prime order <code>r</code> with value 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.

A <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;</code> is an integer between 0 and <code>r-1</code>.

Function <code><a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> and <code><a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>
assumes a 32-byte little-endian encoding of a <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>.

An <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> is an element in <code>Gt</code>.

Function <code><a href="curves.md#0x1_curves_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> and <code><a href="curves.md#0x1_curves_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>,
as well as <code>serialize_element_ompressed&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code> and <code><a href="curves.md#0x1_curves_deserialize_element_compressed">deserialize_element_compressed</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>,
assume a 576-byte encoding <code>[b_0, ..., b_575]</code> of an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;</code>, with the following rules.
- Assume the given element is <code>e=c_0+c_1*w</code> where <code>c_i=c_i0+c_i1*v+c_i2*v^2 for i=0..1</code> and <code>c_ij=c_ij0+c_ij1*u for j=0..2</code>.
- <code>[b_0, ..., b_575]</code> is a concatenation of 12 encoded <code>Fq</code> elements: <code>c_000, c_001, c_010, c_011, c_020, c_021, c_100, c_101, c_110, c_111, c_120, c_121</code>.
- Every <code>c_ijk</code> uses a 48-byte little-endian encoding.


<pre><code><b>struct</b> <a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>
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

<a name="0x1_curves_Scalar"></a>

## Struct `Scalar`

This struct represents a scalar, usually an integer between 0 and <code>r-1</code>,
where <code>r</code> is the prime order of a group, where the group is determined by the type argument <code>G</code>.
See the comments on the specific <code>G</code> for more details about <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;</code>.


<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; <b>has</b> <b>copy</b>, drop
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

<a name="0x1_curves_Element"></a>

## Struct `Element`

This struct represents a group element, usually a point in an elliptic curve.
The group is determined by the type argument <code>G</code>.
See the comments on the specific <code>G</code> for more details about <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;</code>.


<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; <b>has</b> <b>copy</b>, drop
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


<a name="0x1_curves_E_NATIVE_FUN_NOT_AVAILABLE"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>: u64 = 1;
</code></pre>



<a name="0x1_curves_pairing"></a>

## Function `pairing`

Perform a bilinear mapping.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1, G2, Gt&gt;(point_1: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G1&gt;, point_2: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1,G2,Gt&gt;(point_1: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G1&gt;, point_2: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Element">Element</a>&lt;Gt&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>if</b> (!std::features::generic_curves_enabled()) {
        <b>abort</b>(std::error::invalid_state(1))
    };
    <a href="curves.md#0x1_curves_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(point_1.handle, point_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_multi_pairing"></a>

## Function `multi_pairing`

Compute the product of multiple pairing: <code>e(p1_1,p2_1) * ... * e(p1_n,p2_n)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_multi_pairing">multi_pairing</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G2&gt;&gt;): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_multi_pairing">multi_pairing</a>&lt;G1,G2,Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Element">Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Element">Element</a>&lt;G2&gt;&gt;): <a href="curves.md#0x1_curves_Element">Element</a>&lt;Gt&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>let</b> num_g1 = std::vector::length(g1_elements);
    <b>let</b> num_g2 = std::vector::length(g2_elements);
    <b>assert</b>!(num_g1 == num_g2, std::error::invalid_argument(1));
    <b>let</b> g1_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> g2_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_g2) {
        std::vector::push_back(&<b>mut</b> g1_handles, std::vector::borrow(g1_elements, i).handle);
        std::vector::push_back(&<b>mut</b> g2_handles, std::vector::borrow(g2_elements, i).handle);
        i = i + 1;
    };

    <a href="curves.md#0x1_curves_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="curves.md#0x1_curves_multi_pairing_internal">multi_pairing_internal</a>&lt;G1,G2,Gt&gt;(num_g1, g1_handles, num_g2, g2_handles)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_from_u64"></a>

## Function `scalar_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64">scalar_from_u64</a>&lt;G&gt;(value: u64): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64">scalar_from_u64</a>&lt;G&gt;(value: u64): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>if</b> (!std::features::generic_curves_enabled()) {
        <b>abort</b>(std::error::invalid_state(1))
    };
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_neg"></a>

## Function `scalar_neg`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>if</b> (!std::features::generic_curves_enabled()) {
        <b>abort</b>(std::error::invalid_state(1))
    };
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(scalar_1.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_add"></a>

## Function `scalar_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add">scalar_add</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add">scalar_add</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>if</b> (!std::features::generic_curves_enabled()) {
        <b>abort</b>(std::error::invalid_state(1))
    };
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(scalar_1.handle, scalar_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_mul"></a>

## Function `scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul">scalar_mul</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul">scalar_mul</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(scalar_1.handle, scalar_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_inv"></a>

## Function `scalar_inv`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): Option&lt;<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>let</b> (succeeded, handle) = <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(scalar.handle);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; { handle };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_eq"></a>

## Function `scalar_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): bool {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(scalar_1.handle, scalar_2.handle)
}
</code></pre>



</details>

<a name="0x1_curves_identity"></a>

## Function `identity`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_identity">identity</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_identity">identity</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_identity_internal">identity_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_curves_generator"></a>

## Function `generator`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_generator">generator</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_generator">generator</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_generator_internal">generator_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_curves_element_neg"></a>

## Function `element_neg`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_neg">element_neg</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_neg">element_neg</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_element_neg_internal">element_neg_internal</a>&lt;G&gt;(point.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_element_add"></a>

## Function `element_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_add">element_add</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_add">element_add</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_element_add_internal">element_add_internal</a>&lt;G&gt;(point_1.handle, point_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_element_mul"></a>

## Function `element_mul`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_mul">element_mul</a>&lt;G&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, _point: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_mul">element_mul</a>&lt;G&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, _point: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_element_mul_internal">element_mul_internal</a>&lt;G&gt;(_scalar.handle, _point.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_simul_point_mul"></a>

## Function `simul_point_mul`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_simul_point_mul">simul_point_mul</a>&lt;G&gt;(scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;&gt;, points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;&gt;): <a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_simul_point_mul">simul_point_mul</a>&lt;G&gt;(scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;&gt;, points: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;&gt;): <a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    //TODO: replace the naive implementation.
    <b>let</b> result = <a href="curves.md#0x1_curves_identity">identity</a>&lt;G&gt;();
    <b>let</b> num_points = std::vector::length(points);
    <b>let</b> num_scalars = std::vector::length(scalars);
    <b>assert</b>!(num_points == num_scalars, 1);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_points) {
        <b>let</b> scalar = std::vector::borrow(scalars, i);
        <b>let</b> point = std::vector::borrow(points, i);
        result = <a href="curves.md#0x1_curves_element_add">element_add</a>(&result, &<a href="curves.md#0x1_curves_element_mul">element_mul</a>(scalar, point));
        i = i + 1;
    };
    result
}
</code></pre>



</details>

<a name="0x1_curves_scalar_from_bytes"></a>

## Function `scalar_from_bytes`

Decode a <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;</code> from a byte array.
See the comments on the actual type <code>G</code> for the format details.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;G&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;G&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>let</b> (succeeded, handle) = <a href="curves.md#0x1_curves_scalar_from_bytes_internal">scalar_from_bytes_internal</a>&lt;G&gt;(*bytes);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
            handle
        };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes"></a>

## Function `scalar_to_bytes`

Encode a <code><a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;</code> to a byte array.
See the comments on the actual type <code>G</code> for the format details.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>&lt;G&gt;(scalar.handle)
}
</code></pre>



</details>

<a name="0x1_curves_serialize_element_uncompressed"></a>

## Function `serialize_element_uncompressed`

Encode an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;</code> to a byte array with an uncompressed format.
See the comments on the actual type <code>G</code> for the format details.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_serialize_element_uncompressed">serialize_element_uncompressed</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_serialize_element_uncompressed_internal">serialize_element_uncompressed_internal</a>&lt;G&gt;(point.handle)
}
</code></pre>



</details>

<a name="0x1_curves_serialize_element_compressed"></a>

## Function `serialize_element_compressed`

Encode an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;</code> to a byte array with a compressed format.
See the comments on the actual type <code>G</code> for the format details.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_serialize_element_compressed">serialize_element_compressed</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_serialize_element_compressed">serialize_element_compressed</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_serialize_element_compressed_internal">serialize_element_compressed_internal</a>&lt;G&gt;(point.handle)
}
</code></pre>



</details>

<a name="0x1_curves_deserialize_element_uncompressed"></a>

## Function `deserialize_element_uncompressed`

Decode an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;</code> from a byte array with an uncompressed format.
See the comments on the actual type <code>G</code> for the format details.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_uncompressed">deserialize_element_uncompressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>let</b> (succ, handle) = <a href="curves.md#0x1_curves_deserialize_element_uncompressed_internal">deserialize_element_uncompressed_internal</a>&lt;G&gt;(bytes);
    <b>if</b> (succ) {
        std::option::some(<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; { handle })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_curves_deserialize_element_compressed"></a>

## Function `deserialize_element_compressed`

Decode an <code><a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;</code> from a byte array with a compressed format.
See the comments on the actual type <code>G</code> for the format details.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_compressed">deserialize_element_compressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_compressed">deserialize_element_compressed</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;&gt; {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <b>let</b> (succ, handle) = <a href="curves.md#0x1_curves_deserialize_element_compressed_internal">deserialize_element_compressed_internal</a>&lt;G&gt;(bytes);
    <b>if</b> (succ) {
        std::option::some(<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt; { handle })
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_curves_element_eq"></a>

## Function `element_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_eq">element_eq</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Element">curves::Element</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_eq">element_eq</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Element">Element</a>&lt;G&gt;): bool {
    <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>();
    <a href="curves.md#0x1_curves_element_eq_internal">element_eq_internal</a>&lt;G&gt;(point_1.handle, point_2.handle)
}
</code></pre>



</details>

<a name="0x1_curves_abort_if_feature_disabled"></a>

## Function `abort_if_feature_disabled`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="curves.md#0x1_curves_abort_if_feature_disabled">abort_if_feature_disabled</a>() {
    <b>if</b> (!std::features::generic_curves_enabled()) {
        <b>abort</b>(std::error::invalid_state(<a href="curves.md#0x1_curves_E_NATIVE_FUN_NOT_AVAILABLE">E_NATIVE_FUN_NOT_AVAILABLE</a>))
    };
}
</code></pre>



</details>

<a name="0x1_curves_deserialize_element_uncompressed_internal"></a>

## Function `deserialize_element_uncompressed_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_uncompressed_internal">deserialize_element_uncompressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_uncompressed_internal">deserialize_element_uncompressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_curves_deserialize_element_compressed_internal"></a>

## Function `deserialize_element_compressed_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_compressed_internal">deserialize_element_compressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_deserialize_element_compressed_internal">deserialize_element_compressed_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_curves_scalar_from_u64_internal"></a>

## Function `scalar_from_u64_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_from_bytes_internal"></a>

## Function `scalar_from_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes_internal">scalar_from_bytes_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes_internal">scalar_from_bytes_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_curves_scalar_neg_internal"></a>

## Function `scalar_neg_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_add_internal"></a>

## Function `scalar_add_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_inv_internal"></a>

## Function `scalar_inv_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(handle: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(handle: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_curves_scalar_eq_internal"></a>

## Function `scalar_eq_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes_internal"></a>

## Function `scalar_to_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>&lt;G&gt;(h: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>&lt;G&gt;(h: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_curves_element_add_internal"></a>

## Function `element_add_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_element_add_internal">element_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_element_add_internal">element_add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_element_eq_internal"></a>

## Function `element_eq_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_element_eq_internal">element_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_element_eq_internal">element_eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_curves_identity_internal"></a>

## Function `identity_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_identity_internal">identity_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_identity_internal">identity_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_curves_generator_internal"></a>

## Function `generator_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_generator_internal">generator_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_generator_internal">generator_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_curves_element_mul_internal"></a>

## Function `element_mul_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_element_mul_internal">element_mul_internal</a>&lt;G&gt;(scalar_handle: u64, point_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_element_mul_internal">element_mul_internal</a>&lt;G&gt;(scalar_handle: u64, point_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_element_neg_internal"></a>

## Function `element_neg_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_element_neg_internal">element_neg_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_element_neg_internal">element_neg_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_serialize_element_uncompressed_internal"></a>

## Function `serialize_element_uncompressed_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_serialize_element_uncompressed_internal">serialize_element_uncompressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_serialize_element_uncompressed_internal">serialize_element_uncompressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_curves_serialize_element_compressed_internal"></a>

## Function `serialize_element_compressed_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_serialize_element_compressed_internal">serialize_element_compressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_serialize_element_compressed_internal">serialize_element_compressed_internal</a>&lt;G&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_curves_pairing_internal"></a>

## Function `pairing_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handle: u64, g2_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handle: u64, g2_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_multi_pairing_internal"></a>

## Function `multi_pairing_internal`

TODO: Remove <code>g1_handle_count</code> and <code>g2_handle_count</code> once working with <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code> in rust is well supported.


<pre><code><b>fun</b> <a href="curves.md#0x1_curves_multi_pairing_internal">multi_pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handle_count: u64, g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handle_count: u64, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_multi_pairing_internal">multi_pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handle_count: u64, g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handle_count: u64, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
