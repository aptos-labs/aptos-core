
<a name="0x1_algebra_bls12381"></a>

# Module `0x1::algebra_bls12381`

This module defines marker types, constants and test cases for working with BLS12-381 curves
using generic API defined in <code><a href="algebra.md#0x1_algebra">algebra</a>.<b>move</b></code>.

Below are the BLS12-381 structures currently supported.
- Field <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq12">BLS12_381_Fq12</a></code>.
- Group <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code>.
- Group <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code>.
- Group <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a></code>.
- Field <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code>.


-  [Struct `BLS12_381_Fq`](#0x1_algebra_bls12381_BLS12_381_Fq)
-  [Struct `BLS12_381_Fq2`](#0x1_algebra_bls12381_BLS12_381_Fq2)
-  [Struct `BLS12_381_Fq6`](#0x1_algebra_bls12381_BLS12_381_Fq6)
-  [Struct `BLS12_381_Fq12`](#0x1_algebra_bls12381_BLS12_381_Fq12)
-  [Struct `BLS12_381_G1_Parent`](#0x1_algebra_bls12381_BLS12_381_G1_Parent)
-  [Struct `BLS12_381_G1`](#0x1_algebra_bls12381_BLS12_381_G1)
-  [Struct `BLS12_381_G2_Parent`](#0x1_algebra_bls12381_BLS12_381_G2_Parent)
-  [Struct `BLS12_381_G2`](#0x1_algebra_bls12381_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_algebra_bls12381_BLS12_381_Gt)
-  [Struct `BLS12_381_Fr`](#0x1_algebra_bls12381_BLS12_381_Fr)
-  [Function `format_bls12381fq_lsb`](#0x1_algebra_bls12381_format_bls12381fq_lsb)
-  [Function `format_bls12381fq_msb`](#0x1_algebra_bls12381_format_bls12381fq_msb)
-  [Function `format_bls12381fq2_lsc_lsb`](#0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb)
-  [Function `format_bls12381fq2_msc_msb`](#0x1_algebra_bls12381_format_bls12381fq2_msc_msb)
-  [Function `format_bls12381fq6_lsc_lsc_lsb`](#0x1_algebra_bls12381_format_bls12381fq6_lsc_lsc_lsb)
-  [Function `format_bls12381fq12_lsc_lsc_lsc_lsb`](#0x1_algebra_bls12381_format_bls12381fq12_lsc_lsc_lsc_lsb)
-  [Function `format_bls12381g1_affine_parent_uncompressed`](#0x1_algebra_bls12381_format_bls12381g1_affine_parent_uncompressed)
-  [Function `format_bls12381g1_affine_parent_compressed`](#0x1_algebra_bls12381_format_bls12381g1_affine_parent_compressed)
-  [Function `format_bls12381g1_affine_uncompressed`](#0x1_algebra_bls12381_format_bls12381g1_affine_uncompressed)
-  [Function `format_bls12381g1_affine_compressed`](#0x1_algebra_bls12381_format_bls12381g1_affine_compressed)
-  [Function `format_bls12381g2_affine_parent_uncompressed`](#0x1_algebra_bls12381_format_bls12381g2_affine_parent_uncompressed)
-  [Function `format_bls12381g2_affine_parent_compressed`](#0x1_algebra_bls12381_format_bls12381g2_affine_parent_compressed)
-  [Function `format_bls12381g2_affine_uncompressed`](#0x1_algebra_bls12381_format_bls12381g2_affine_uncompressed)
-  [Function `format_bls12381g2_affine_compressed`](#0x1_algebra_bls12381_format_bls12381g2_affine_compressed)
-  [Function `format_bls12381gt`](#0x1_algebra_bls12381_format_bls12381gt)
-  [Function `format_bls12381fr_lsb`](#0x1_algebra_bls12381_format_bls12381fr_lsb)
-  [Function `format_bls12381fr_msb`](#0x1_algebra_bls12381_format_bls12381fr_msb)
-  [Function `h2s_suite_bls12381g1_xmd_sha_256_sswu_ro`](#0x1_algebra_bls12381_h2s_suite_bls12381g1_xmd_sha_256_sswu_ro)
-  [Function `h2s_suite_bls12381g2_xmd_sha_256_sswu_ro`](#0x1_algebra_bls12381_h2s_suite_bls12381g2_xmd_sha_256_sswu_ro)


<pre><code></code></pre>



<a name="0x1_algebra_bls12381_BLS12_381_Fq"></a>

## Struct `BLS12_381_Fq`

The finite field $F_q$ used in BLS12-381 curves.
It has a prime order $q$ equal to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq">BLS12_381_Fq</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_Fq2"></a>

## Struct `BLS12_381_Fq2`

The finite field $F_{q^2}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq">BLS12_381_Fq</a></code>, constructed as $F_{q^2}=F_q[u]/(u^2+1)$.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq2">BLS12_381_Fq2</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_Fq6"></a>

## Struct `BLS12_381_Fq6`

The finite field $F_{q^6}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq2">BLS12_381_Fq2</a></code>, constructed as $F_{q^6}=F_{q^2}[v]/(v^3-u-1)$.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq6">BLS12_381_Fq6</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_Fq12"></a>

## Struct `BLS12_381_Fq12`

The finite field $F_{q^12}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq6">BLS12_381_Fq6</a></code>, constructed as $F_{q^12}=F_{q^6}[w]/(w^2-v)$.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq12">BLS12_381_Fq12</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_G1_Parent"></a>

## Struct `BLS12_381_G1_Parent`

A group constructed by the points on the BLS12-381 curve $E(F_q): y^2=x^3+4$ and the point at inifinity,
under the elliptic curve point addition.
It contains the prime-order subgroup $G_1$ used in pairing.
The identity is the point at infinity.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`

The group $G_1$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is subgroup of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_G2_Parent"></a>

## Struct `BLS12_381_G2_Parent`

A group constructed by the points on a curve $E'(F_{q^2})$ and the point at inifinity under the elliptic curve point addition.
$E'(F_{q^2})$ is an elliptic curve $y^2=x^3+4(u+1)$ defined over $F_{q^2}$.
The identity of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> is the point at infinity.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`

The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a subgroup of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`

The group $G_t$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a multiplicative subgroup of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq12">BLS12_381_Fq12</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> is the scalar field).
The identity of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a></code> is 1.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a>
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

<a name="0x1_algebra_bls12381_BLS12_381_Fr"></a>

## Struct `BLS12_381_Fr`

The finite field $F_r$ that can be used as the scalar fields
for the groups $G_1$, $G_2$, $G_t$ in BLS12-381-based pairing.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a>
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

<a name="0x1_algebra_bls12381_format_bls12381fq_lsb"></a>

## Function `format_bls12381fq_lsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq">BLS12_381_Fq</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48 with the least signature byte coming first.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_lsb">format_bls12381fq_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_lsb">format_bls12381fq_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"01" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fq_msb"></a>

## Function `format_bls12381fq_msb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq">BLS12_381_Fq</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48 with the most significant byte coming first.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0101" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb"></a>

## Function `format_bls12381fq2_lsc_lsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq2">BLS12_381_Fq2</a></code> elements.
where an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96
with the following rules.
- <code>b[0..48]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_lsb">format_bls12381fq_lsb</a>()</code>.
- <code>b[48..96]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_lsb">format_bls12381fq_lsb</a>()</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb">format_bls12381fq2_lsc_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb">format_bls12381fq2_lsc_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"02" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fq2_msc_msb"></a>

## Function `format_bls12381fq2_msc_msb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq2">BLS12_381_Fq2</a></code> elements,
where an element in the form $(c_1\cdot u+c_0)$ is represented by a byte array <code>b[]</code> of size 96,
with the following rules.
- <code>b[0..48]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>()</code>.
- <code>b[48..96]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>()</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_msc_msb">format_bls12381fq2_msc_msb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_msc_msb">format_bls12381fq2_msc_msb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0201" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fq6_lsc_lsc_lsb"></a>

## Function `format_bls12381fq6_lsc_lsc_lsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq6">BLS12_381_Fq6</a></code> elements,
where an element $(c_0+c_1\cdot v+c_2\cdot v^2)$ is represented by a byte array <code>b[]</code> of size 288,
with the following rules.
- <code>b[0..96]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb">format_bls12381fq2_lsc_lsb</a>()</code>.
- <code>b[96..192]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb">format_bls12381fq2_lsc_lsb</a>()</code>.
- <code>b[192..288]</code> is $c_2$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_lsc_lsb">format_bls12381fq2_lsc_lsb</a>()</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq6_lsc_lsc_lsb">format_bls12381fq6_lsc_lsc_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq6_lsc_lsc_lsb">format_bls12381fq6_lsc_lsc_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"03" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fq12_lsc_lsc_lsc_lsb"></a>

## Function `format_bls12381fq12_lsc_lsc_lsc_lsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq12">BLS12_381_Fq12</a></code> elements,
where an element $(c_0+c_1\cdot w)$ is represented by a byte array <code>b[]</code> of size 576.
<code>b[0..288]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq6_lsc_lsc_lsb">format_bls12381fq6_lsc_lsc_lsb</a>()</code>.
<code>b[288..576]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq6_lsc_lsc_lsb">format_bls12381fq6_lsc_lsc_lsb</a>()</code>.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq12_lsc_lsc_lsc_lsb">format_bls12381fq12_lsc_lsc_lsc_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq12_lsc_lsc_lsc_lsb">format_bls12381fq12_lsc_lsc_lsc_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"04" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g1_affine_parent_uncompressed"></a>

## Function `format_bls12381g1_affine_parent_uncompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 96,
with the following rules deseribed from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 1.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag. This is ignored.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[47]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>()</code>. Abort if this failed.
1. Deserialize $y$ from <code>[b[48], ..., b[95]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>()</code>. Abort if this failed.
1. Abort if point $(x,y)$ is not on curve $E(F_q)$.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_parent_uncompressed">format_bls12381g1_affine_parent_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_parent_uncompressed">format_bls12381g1_affine_parent_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"05" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g1_affine_parent_compressed"></a>

## Function `format_bls12381g1_affine_parent_compressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48,
with the following rules deseribed from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 0.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[47]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq_msb">format_bls12381fq_msb</a>()</code>. Abort if this failed.
1. Try computing $y$ such that point $(x,y)$ is on the curve $E(F_q)$. Abort if there is no such $y$.
1. Let $\overline{y}=-y$.
1. Set $y$ as $\min(y,\overline{y})$ if the the lexicographical flag is 0, or $\max(y,\overline{y})$ otherwise.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_parent_compressed">format_bls12381g1_affine_parent_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_parent_compressed">format_bls12381g1_affine_parent_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0501" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g1_affine_uncompressed"></a>

## Function `format_bls12381g1_affine_uncompressed`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements,
essentially the format represented by <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_parent_uncompressed">format_bls12381g1_affine_parent_uncompressed</a>()</code>
but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_uncompressed">format_bls12381g1_affine_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_uncompressed">format_bls12381g1_affine_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"06" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g1_affine_compressed"></a>

## Function `format_bls12381g1_affine_compressed`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements,
essentially the format represented by <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_parent_compressed">format_bls12381g1_affine_parent_compressed</a>()</code>
but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_compressed">format_bls12381g1_affine_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g1_affine_compressed">format_bls12381g1_affine_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0601" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g2_affine_parent_uncompressed"></a>

## Function `format_bls12381g2_affine_parent_uncompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> elements.
where an element is represented by a byte array <code>b[]</code> of size 192,
with the following rules deseribed from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 1.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag. This is ignored.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[95]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_msc_msb">format_bls12381fq2_msc_msb</a>()</code>. Abort if this failed.
1. Deserialize $y$ from <code>[b[96], ..., b[191]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_msc_msb">format_bls12381fq2_msc_msb</a>()</code>. Abort if this failed.
1. Abort if point $(x,y)$ is not on curve $E'(F_{q^2})$.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_parent_uncompressed">format_bls12381g2_affine_parent_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_parent_uncompressed">format_bls12381g2_affine_parent_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"07" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g2_affine_parent_compressed"></a>

## Function `format_bls12381g2_affine_parent_compressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 96,
with the following rules deseribed from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 0.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[96]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq2_msc_msb">format_bls12381fq2_msc_msb</a>()</code>. Abort if this failed.
1. Try computing $y$ such that point $(x,y)$ is on the curve $E(F_{q^2})$. Abort if there is no such $y$.
1. Let $\overline{y}=-y$.
1. Set $y$ as $\min(y,\overline{y})$ if the the lexicographical flag is 0, or $\max(y,\overline{y})$ otherwise.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_parent_compressed">format_bls12381g2_affine_parent_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_parent_compressed">format_bls12381g2_affine_parent_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0701" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g2_affine_uncompressed"></a>

## Function `format_bls12381g2_affine_uncompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements,
essentially <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_parent_uncompressed">format_bls12381g2_affine_parent_uncompressed</a>()</code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_uncompressed">format_bls12381g2_affine_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_uncompressed">format_bls12381g2_affine_uncompressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"08" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381g2_affine_compressed"></a>

## Function `format_bls12381g2_affine_compressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements,
essentially <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_parent_compressed">format_bls12381g2_affine_parent_compressed</a>()</code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_compressed">format_bls12381g2_affine_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381g2_affine_compressed">format_bls12381g2_affine_compressed</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0801" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381gt"></a>

## Function `format_bls12381gt`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a></code> elements,
essentially <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fq12_lsc_lsc_lsc_lsb">format_bls12381fq12_lsc_lsc_lsc_lsb</a>()</code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381gt">format_bls12381gt</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381gt">format_bls12381gt</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"09" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fr_lsb"></a>

## Function `format_bls12381fr_lsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the least significant byte coming first.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_lsb">format_bls12381fr_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_lsb">format_bls12381fr_lsb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fr_msb"></a>

## Function `format_bls12381fr_msb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the most significant byte coming first.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_msb">format_bls12381fr_msb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_msb">format_bls12381fr_msb</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a01" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_h2s_suite_bls12381g1_xmd_sha_256_sswu_ro"></a>

## Function `h2s_suite_bls12381g1_xmd_sha_256_sswu_ro`

The hash-to-curve suite <code>BLS12381G1_XMD:SHA-256_SSWU_RO_</code>
defined in https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-16#name-bls12-381-g1.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_h2s_suite_bls12381g1_xmd_sha_256_sswu_ro">h2s_suite_bls12381g1_xmd_sha_256_sswu_ro</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_h2s_suite_bls12381g1_xmd_sha_256_sswu_ro">h2s_suite_bls12381g1_xmd_sha_256_sswu_ro</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0001" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_h2s_suite_bls12381g2_xmd_sha_256_sswu_ro"></a>

## Function `h2s_suite_bls12381g2_xmd_sha_256_sswu_ro`

The hash-to-curve suite <code>BLS12381G2_XMD:SHA-256_SSWU_RO_</code>
defined in https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-16#name-bls12-381-g2.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_h2s_suite_bls12381g2_xmd_sha_256_sswu_ro">h2s_suite_bls12381g2_xmd_sha_256_sswu_ro</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_h2s_suite_bls12381g2_xmd_sha_256_sswu_ro">h2s_suite_bls12381g2_xmd_sha_256_sswu_ro</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0002" }
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
