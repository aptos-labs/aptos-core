
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
-  [Function `bls12_381_fq_format`](#0x1_algebra_bls12381_bls12_381_fq_format)
-  [Function `bls12_381_fq_bendian_format`](#0x1_algebra_bls12381_bls12_381_fq_bendian_format)
-  [Function `bls12_381_fq2_format`](#0x1_algebra_bls12381_bls12_381_fq2_format)
-  [Function `bls12_381_fq2_format_bendian_fq`](#0x1_algebra_bls12381_bls12_381_fq2_format_bendian_fq)
-  [Function `bls12_381_fq6_format`](#0x1_algebra_bls12381_bls12_381_fq6_format)
-  [Function `bls12_381_fq12_format`](#0x1_algebra_bls12381_bls12_381_fq12_format)
-  [Function `bls12_381_g1_parent_uncompressed_format`](#0x1_algebra_bls12381_bls12_381_g1_parent_uncompressed_format)
-  [Function `bls12_381_g1_parent_compressed_format`](#0x1_algebra_bls12381_bls12_381_g1_parent_compressed_format)
-  [Function `bls12_381_g1_uncompressed_format`](#0x1_algebra_bls12381_bls12_381_g1_uncompressed_format)
-  [Function `bls12_381_g1_compressed_format`](#0x1_algebra_bls12381_bls12_381_g1_compressed_format)
-  [Function `bls12_381_g2_parent_uncompressed_format`](#0x1_algebra_bls12381_bls12_381_g2_parent_uncompressed_format)
-  [Function `bls12_381_g2_parent_compressed_format`](#0x1_algebra_bls12381_bls12_381_g2_parent_compressed_format)
-  [Function `bls12_381_g2_uncompressed_format`](#0x1_algebra_bls12381_bls12_381_g2_uncompressed_format)
-  [Function `bls12_381_g2_compressed_format`](#0x1_algebra_bls12381_bls12_381_g2_compressed_format)
-  [Function `bls12_381_gt_format`](#0x1_algebra_bls12381_bls12_381_gt_format)
-  [Function `bls12_381_fr_lendian_format`](#0x1_algebra_bls12381_bls12_381_fr_lendian_format)
-  [Function `bls12_381_fr_bendian_format`](#0x1_algebra_bls12381_bls12_381_fr_bendian_format)
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

A group constructed by the points on a curve $E(F_{q^2})$ and the point at inifinity under the elliptic curve point addition.
$E(F_{q^2})$ is an elliptic curve $y^2=x^3+4(u+1)$ defined over $F_{q^2}$.
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

The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
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

<a name="0x1_algebra_bls12381_bls12_381_fq_format"></a>

## Function `bls12_381_fq_format`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq">BLS12_381_Fq</a></code> elements.
In this format, an element is represented by a byte array <code>b[]</code> of size 48 using little-endian byte order.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq_format">bls12_381_fq_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq_format">bls12_381_fq_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"01" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fq_bendian_format"></a>

## Function `bls12_381_fq_bendian_format`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq">BLS12_381_Fq</a></code> elements.
In this format, an element is represented by a byte array <code>b[]</code> of size 48 using big-endian byte order.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq_bendian_format">bls12_381_fq_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq_bendian_format">bls12_381_fq_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0101" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fq2_format"></a>

## Function `bls12_381_fq2_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq2">BLS12_381_Fq2</a></code> elements.
In this format, an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96.
<code>b[0..48]</code> is $c_0$ serialized using <code>BLS12_381_Fq_Format</code>.
<code>b[48..96]</code> is $c_1$ serialized using <code>BLS12_381_Fq_Format</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq2_format">bls12_381_fq2_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq2_format">bls12_381_fq2_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"02" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fq2_format_bendian_fq"></a>

## Function `bls12_381_fq2_format_bendian_fq`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq2">BLS12_381_Fq2</a></code> elements.
In this format, an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96.
<code>b[0..48]</code> is $c_0$ serialized using <code>BLS12_381_Fq_Format_BEndianFq</code>.
<code>b[48..96]</code> is $c_1$ serialized using <code>BLS12_381_Fq_Format_BEndianFq</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq2_format_bendian_fq">bls12_381_fq2_format_bendian_fq</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq2_format_bendian_fq">bls12_381_fq2_format_bendian_fq</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0201" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fq6_format"></a>

## Function `bls12_381_fq6_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq6">BLS12_381_Fq6</a></code> elements.

In this format, an element $(c_0+c_1\cdot v+c_2\cdot v^2)$ is represented by a byte array <code>b[]</code> of size 288.
<code>b[0..96]</code> is $c_0$ serialized using <code>BLS12_381_Fq2_Format</code>.
<code>b[96..192]</code> is $c_1$ serialized using <code>BLS12_381_Fq2_Format</code>.
<code>b[192..288]</code> is $c_2$ serialized using <code>BLS12_381_Fq2_Format</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq6_format">bls12_381_fq6_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq6_format">bls12_381_fq6_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"03" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fq12_format"></a>

## Function `bls12_381_fq12_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fq12">BLS12_381_Fq12</a></code> elements.

In this format, an element $(c_0+c_1\cdot w)$ is represented by a byte array <code>b[]</code> of size 576.
<code>b[0..288]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq6_format">bls12_381_fq6_format</a>()</code>.
<code>b[288..576]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq6_format">bls12_381_fq6_format</a>()</code>.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq12_format">bls12_381_fq12_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq12_format">bls12_381_fq12_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"04" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g1_parent_uncompressed_format"></a>

## Function `bls12_381_g1_parent_uncompressed_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 96.
<code>b[95] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_q)$,
<code>[b[0], ..., b[47] & 0x3f]</code> is $x$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq_format">bls12_381_fq_format</a>()</code>, and
<code>[b[48], ..., b[95] & 0x3f]</code> is $y$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fq_format">bls12_381_fq_format</a>()</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"05" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g1_parent_compressed_format"></a>

## Function `bls12_381_g1_parent_compressed_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1_Parent">BLS12_381_G1_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 48.
<code>b[47] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(Fq)$,
<code>[b[0], ..., b[47] & 0x3f]</code> is $x$ serialized using <code>bls12_381_fq_format</code>, and
the positiveness flag <code>b_47 & 0x80</code> is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0501" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g1_uncompressed_format"></a>

## Function `bls12_381_g1_uncompressed_format`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements,
essentially the format represented by <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_parent_uncompressed_format">bls12_381_g1_parent_uncompressed_format</a>()</code>
but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_uncompressed_format">bls12_381_g1_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"06" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g1_compressed_format"></a>

## Function `bls12_381_g1_compressed_format`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements,
essentially the format represented by <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_parent_compressed_format">bls12_381_g1_parent_compressed_format</a>()</code>
but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G1">BLS12_381_G1</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g1_compressed_format">bls12_381_g1_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0601" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g2_parent_uncompressed_format"></a>

## Function `bls12_381_g2_parent_uncompressed_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 192.
<code>b[191] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
<code>b[0..96]</code> is $x$ serialized using <code>BLS12_381_Fq2_Format</code>, and
<code>[b[96], ..., b[191] & 0x3f]</code> is $y$ serialized using <code>BLS12_381_Fq2_Format</code>.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_parent_uncompressed_format">bls12_381_g2_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_parent_uncompressed_format">bls12_381_g2_parent_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"07" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g2_parent_compressed_format"></a>

## Function `bls12_381_g2_parent_compressed_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2_Parent">BLS12_381_G2_Parent</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 96.
<code>b[95] & 0x40</code> is the infinity flag.
The infinity flag is 1 if and only if the element is the point at infinity.
The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
<code>[b[0], ..., b[95] & 0x3f]</code> is $x$ serialized using <code>BLS12_381_Fq2_Format</code>, and
the positiveness flag <code>b[95] & 0x80</code> is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_parent_compressed_format">bls12_381_g2_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_parent_compressed_format">bls12_381_g2_parent_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0701" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g2_uncompressed_format"></a>

## Function `bls12_381_g2_uncompressed_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements.

Essentially <code>BLS12_381_G2_Parent_Format_Uncompressed</code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_uncompressed_format">bls12_381_g2_uncompressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"08" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_g2_compressed_format"></a>

## Function `bls12_381_g2_compressed_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements.

Essentially <code>BLS12_381_G2_Parent_Format_Compressed</code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_G2">BLS12_381_G2</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_g2_compressed_format">bls12_381_g2_compressed_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0801" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_gt_format"></a>

## Function `bls12_381_gt_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.

Essentially <code>BLS12_381_Fq12_Format</code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Gt">BLS12_381_Gt</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_gt_format">bls12_381_gt_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_gt_format">bls12_381_gt_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"09" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fr_lendian_format"></a>

## Function `bls12_381_fr_lendian_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 32 using little-endian byte order.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fr_lendian_format">bls12_381_fr_lendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a" }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_bls12_381_fr_bendian_format"></a>

## Function `bls12_381_fr_bendian_format`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_BLS12_381_Fr">BLS12_381_Fr</a></code> elements.

In this format, an element is represented by a byte array <code>b[]</code> of size 32 using big-endian byte order.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fr_bendian_format">bls12_381_fr_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_bls12_381_fr_bendian_format">bls12_381_fr_bendian_format</a>(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"0a01" }
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
