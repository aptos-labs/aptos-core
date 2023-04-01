
<a name="0x1_algebra_bls12381"></a>

# Module `0x1::algebra_bls12381`

This module defines marker types, constants and test cases for working with BLS12-381 curves
using generic API defined in <code><a href="algebra.md#0x1_algebra">algebra</a>.<b>move</b></code>.

See https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-pairing-friendly-curves-11#name-bls-curves-for-the-128-bit-
for the full specification of BLS12-381 curves.

Currently-supported BLS12-381 structures include <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq12">Fq12</a></code>, <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code>, <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code>, <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> and <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Gt">Gt</a></code>,
along with their widely-used serialization formats,
the pairing between <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code>, <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> and <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Gt">Gt</a></code>,
and the hash-to-curve operations for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> and <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> defined in https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-16.


-  [Struct `Fq`](#0x1_algebra_bls12381_Fq)
-  [Struct `FqFormatLsb`](#0x1_algebra_bls12381_FqFormatLsb)
-  [Struct `FqFormatMsb`](#0x1_algebra_bls12381_FqFormatMsb)
-  [Struct `Fq2`](#0x1_algebra_bls12381_Fq2)
-  [Struct `Fq2FormatLscLsb`](#0x1_algebra_bls12381_Fq2FormatLscLsb)
-  [Struct `Fq2FormatMscMsb`](#0x1_algebra_bls12381_Fq2FormatMscMsb)
-  [Struct `Fq6`](#0x1_algebra_bls12381_Fq6)
-  [Struct `Fq6FormatLscLsb`](#0x1_algebra_bls12381_Fq6FormatLscLsb)
-  [Struct `Fq12`](#0x1_algebra_bls12381_Fq12)
-  [Struct `Fq12FormatLscLsb`](#0x1_algebra_bls12381_Fq12FormatLscLsb)
-  [Struct `G1AffineParent`](#0x1_algebra_bls12381_G1AffineParent)
-  [Struct `G1AffineParentFormatUncompressed`](#0x1_algebra_bls12381_G1AffineParentFormatUncompressed)
-  [Struct `G1AffineParentFormatCompressed`](#0x1_algebra_bls12381_G1AffineParentFormatCompressed)
-  [Struct `G1Affine`](#0x1_algebra_bls12381_G1Affine)
-  [Struct `G1AffineFormatUncompressed`](#0x1_algebra_bls12381_G1AffineFormatUncompressed)
-  [Struct `G1AffineFormatCompressed`](#0x1_algebra_bls12381_G1AffineFormatCompressed)
-  [Struct `G2AffineParent`](#0x1_algebra_bls12381_G2AffineParent)
-  [Struct `G2AffineParentFormatUncompressed`](#0x1_algebra_bls12381_G2AffineParentFormatUncompressed)
-  [Struct `G2AffineParentFormatCompressed`](#0x1_algebra_bls12381_G2AffineParentFormatCompressed)
-  [Struct `G2Affine`](#0x1_algebra_bls12381_G2Affine)
-  [Struct `G2AffineFormatUncompressed`](#0x1_algebra_bls12381_G2AffineFormatUncompressed)
-  [Struct `G2AffineFormatCompressed`](#0x1_algebra_bls12381_G2AffineFormatCompressed)
-  [Struct `Gt`](#0x1_algebra_bls12381_Gt)
-  [Struct `GtFormat`](#0x1_algebra_bls12381_GtFormat)
-  [Struct `Fr`](#0x1_algebra_bls12381_Fr)
-  [Struct `FrFormatLsb`](#0x1_algebra_bls12381_FrFormatLsb)
-  [Struct `FrFormatMsb`](#0x1_algebra_bls12381_FrFormatMsb)
-  [Struct `H2SSuiteBls12381g1XmdSha256SswuRo`](#0x1_algebra_bls12381_H2SSuiteBls12381g1XmdSha256SswuRo)
-  [Struct `H2SSuiteBls12381g2XmdSha256SswuRo`](#0x1_algebra_bls12381_H2SSuiteBls12381g2XmdSha256SswuRo)


<pre><code></code></pre>



<a name="0x1_algebra_bls12381_Fq"></a>

## Struct `Fq`

The finite field $F_q$ used in BLS12-381 curves.
It has a prime order $q$ equal to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq">Fq</a>
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

<a name="0x1_algebra_bls12381_FqFormatLsb"></a>

## Struct `FqFormatLsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq">Fq</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48 with the least signature byte coming first.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatLsb">FqFormatLsb</a>
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

<a name="0x1_algebra_bls12381_FqFormatMsb"></a>

## Struct `FqFormatMsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq">Fq</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48 with the most significant byte coming first.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatMsb">FqFormatMsb</a>
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

<a name="0x1_algebra_bls12381_Fq2"></a>

## Struct `Fq2`

The finite field $F_{q^2}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq">Fq</a></code>, constructed as $F_{q^2}=F_q[u]/(u^2+1)$.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2">Fq2</a>
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

<a name="0x1_algebra_bls12381_Fq2FormatLscLsb"></a>

## Struct `Fq2FormatLscLsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2">Fq2</a></code> elements.
where an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96
with the following rules.
- <code>b[0..48]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatLsb">FqFormatLsb</a></code>.
- <code>b[48..96]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatLsb">FqFormatLsb</a></code>.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatLscLsb">Fq2FormatLscLsb</a>
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

<a name="0x1_algebra_bls12381_Fq2FormatMscMsb"></a>

## Struct `Fq2FormatMscMsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2">Fq2</a></code> elements,
where an element in the form $(c_1\cdot u+c_0)$ is represented by a byte array <code>b[]</code> of size 96,
with the following rules.
- <code>b[0..48]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatMsb">FqFormatMsb</a></code>.
- <code>b[48..96]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatMsb">FqFormatMsb</a></code>.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatMscMsb">Fq2FormatMscMsb</a>
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

<a name="0x1_algebra_bls12381_Fq6"></a>

## Struct `Fq6`

The finite field $F_{q^6}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2">Fq2</a></code>, constructed as $F_{q^6}=F_{q^2}[v]/(v^3-u-1)$.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq6">Fq6</a>
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

<a name="0x1_algebra_bls12381_Fq6FormatLscLsb"></a>

## Struct `Fq6FormatLscLsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq6">Fq6</a></code> elements,
where an element $(c_0+c_1\cdot v+c_2\cdot v^2)$ is represented by a byte array <code>b[]</code> of size 288,
with the following rules.
- <code>b[0..96]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatLscLsb">Fq2FormatLscLsb</a></code>.
- <code>b[96..192]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatLscLsb">Fq2FormatLscLsb</a></code>.
- <code>b[192..288]</code> is $c_2$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatLscLsb">Fq2FormatLscLsb</a></code>.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq6FormatLscLsb">Fq6FormatLscLsb</a>
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

<a name="0x1_algebra_bls12381_Fq12"></a>

## Struct `Fq12`

The finite field $F_{q^12}$ used in BLS12-381 curves.
It is an extension field of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq6">Fq6</a></code>, constructed as $F_{q^12}=F_{q^6}[w]/(w^2-v)$.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq12">Fq12</a>
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

<a name="0x1_algebra_bls12381_Fq12FormatLscLsb"></a>

## Struct `Fq12FormatLscLsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq12">Fq12</a></code> elements,
where an element $(c_0+c_1\cdot w)$ is represented by a byte array <code>b[]</code> of size 576.
<code>b[0..288]</code> is $c_0$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq6FormatLscLsb">Fq6FormatLscLsb</a></code>.
<code>b[288..576]</code> is $c_1$ serialized using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq6FormatLscLsb">Fq6FormatLscLsb</a></code>.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq12FormatLscLsb">Fq12FormatLscLsb</a>
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

<a name="0x1_algebra_bls12381_G1AffineParent"></a>

## Struct `G1AffineParent`

A group constructed by the points on the BLS12-381 curve $E(F_q): y^2=x^3+4$ and the point at infinity,
under the elliptic curve point addition.
It contains the prime-order subgroup $G_1$ used in pairing.
The identity is the point at infinity.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParent">G1AffineParent</a>
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

<a name="0x1_algebra_bls12381_G1AffineParentFormatUncompressed"></a>

## Struct `G1AffineParentFormatUncompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParent">G1AffineParent</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 96,
with the following rules described from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 1.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag. This is ignored.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[47]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatMsb">FqFormatMsb</a></code>. Abort if this failed.
1. Deserialize $y$ from <code>[b[48], ..., b[95]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatMsb">FqFormatMsb</a></code>. Abort if this failed.
1. Abort if point $(x,y)$ is not on curve $E(F_q)$.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParentFormatUncompressed">G1AffineParentFormatUncompressed</a>
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

<a name="0x1_algebra_bls12381_G1AffineParentFormatCompressed"></a>

## Struct `G1AffineParentFormatCompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParent">G1AffineParent</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48,
with the following rules described from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 0.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[47]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_FqFormatMsb">FqFormatMsb</a></code>. Abort if this failed.
1. Try computing $y$ such that point $(x,y)$ is on the curve $E(F_q)$. Abort if there is no such $y$.
1. Let $\overline{y}=-y$.
1. Set $y$ as $\min(y,\overline{y})$ if the the lexicographical flag is 0, or $\max(y,\overline{y})$ otherwise.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParentFormatCompressed">G1AffineParentFormatCompressed</a>
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

<a name="0x1_algebra_bls12381_G1Affine"></a>

## Struct `G1Affine`

The group $G_1$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is subgroup of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParent">G1AffineParent</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a>
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

<a name="0x1_algebra_bls12381_G1AffineFormatUncompressed"></a>

## Struct `G1AffineFormatUncompressed`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> elements,
essentially the format represented by <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParentFormatUncompressed">G1AffineParentFormatUncompressed</a></code>
but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineFormatUncompressed">G1AffineFormatUncompressed</a>
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

<a name="0x1_algebra_bls12381_G1AffineFormatCompressed"></a>

## Struct `G1AffineFormatCompressed`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> elements,
essentially the format represented by <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParentFormatCompressed">G1AffineParentFormatCompressed</a></code>
but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineFormatCompressed">G1AffineFormatCompressed</a>
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

<a name="0x1_algebra_bls12381_G2AffineParent"></a>

## Struct `G2AffineParent`

A group constructed by the points on a curve $E'(F_{q^2})$ and the point at infinity under the elliptic curve point addition.
$E'(F_{q^2})$ is an elliptic curve $y^2=x^3+4(u+1)$ defined over $F_{q^2}$.
The identity of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> is the point at infinity.

NOTE: currently information-only and no operations are implemented for this structure.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParent">G2AffineParent</a>
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

<a name="0x1_algebra_bls12381_G2AffineParentFormatUncompressed"></a>

## Struct `G2AffineParentFormatUncompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParent">G2AffineParent</a></code> elements.
where an element is represented by a byte array <code>b[]</code> of size 192,
with the following rules described from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 1.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag. This is ignored.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[95]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatMscMsb">Fq2FormatMscMsb</a></code>. Abort if this failed.
1. Deserialize $y$ from <code>[b[96], ..., b[191]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatMscMsb">Fq2FormatMscMsb</a></code>. Abort if this failed.
1. Abort if point $(x,y)$ is not on curve $E'(F_{q^2})$.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParentFormatUncompressed">G2AffineParentFormatUncompressed</a>
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

<a name="0x1_algebra_bls12381_G2AffineParentFormatCompressed"></a>

## Struct `G2AffineParentFormatCompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1AffineParent">G1AffineParent</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 96,
with the following rules described from the perspective of deserialization.
1. Read <code>b[0] & 0x80</code> as the compression flag. Abort if it is 0.
1. Read <code>b[0] & 0x40</code> as the infinity flag.
1. Read <code>b[0] & 0x20</code> as the lexicographical flag.
1. If the infinity flag is 1, return the point at infinity.
1. Deserialize $x$ from <code>[b[0] & 0x1f, ..., b[96]]</code> using <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq2FormatMscMsb">Fq2FormatMscMsb</a></code>. Abort if this failed.
1. Try computing $y$ such that point $(x,y)$ is on the curve $E(F_{q^2})$. Abort if there is no such $y$.
1. Let $\overline{y}=-y$.
1. Set $y$ as $\min(y,\overline{y})$ if the the lexicographical flag is 0, or $\max(y,\overline{y})$ otherwise.
1. Return $(x,y)$.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParentFormatCompressed">G2AffineParentFormatCompressed</a>
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

<a name="0x1_algebra_bls12381_G2Affine"></a>

## Struct `G2Affine`

The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a subgroup of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParent">G2AffineParent</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a>
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

<a name="0x1_algebra_bls12381_G2AffineFormatUncompressed"></a>

## Struct `G2AffineFormatUncompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> elements,
essentially <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParentFormatUncompressed">G2AffineParentFormatUncompressed</a></code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineFormatUncompressed">G2AffineFormatUncompressed</a>
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

<a name="0x1_algebra_bls12381_G2AffineFormatCompressed"></a>

## Struct `G2AffineFormatCompressed`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> elements,
essentially <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineParentFormatCompressed">G2AffineParentFormatCompressed</a></code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G2Affine">G2Affine</a></code> elements.

NOTE: currently information-only, not implemented.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_G2AffineFormatCompressed">G2AffineFormatCompressed</a>
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

<a name="0x1_algebra_bls12381_Gt"></a>

## Struct `Gt`

The group $G_t$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a multiplicative subgroup of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq12">Fq12</a></code>.
It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> is the scalar field).
The identity of <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Gt">Gt</a></code> is 1.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Gt">Gt</a>
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

<a name="0x1_algebra_bls12381_GtFormat"></a>

## Struct `GtFormat`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Gt">Gt</a></code> elements,
essentially <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fq12FormatLscLsb">Fq12FormatLscLsb</a></code> but only applicable to <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Gt">Gt</a></code> elements.

NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.4.0).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_GtFormat">GtFormat</a>
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

<a name="0x1_algebra_bls12381_Fr"></a>

## Struct `Fr`

The finite field $F_r$ that can be used as the scalar fields
for the groups $G_1$, $G_2$, $G_t$ in BLS12-381-based pairing.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a>
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

<a name="0x1_algebra_bls12381_FrFormatLsb"></a>

## Struct `FrFormatLsb`

A serialization format for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the least significant byte coming first.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_FrFormatLsb">FrFormatLsb</a>
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

<a name="0x1_algebra_bls12381_FrFormatMsb"></a>

## Struct `FrFormatMsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the most significant byte coming first.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_FrFormatMsb">FrFormatMsb</a>
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

<a name="0x1_algebra_bls12381_H2SSuiteBls12381g1XmdSha256SswuRo"></a>

## Struct `H2SSuiteBls12381g1XmdSha256SswuRo`

The hash-to-curve suite <code>BLS12381G1_XMD:SHA-256_SSWU_RO_</code> that hashes a byte array into <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> elements.

Full specification is defined in https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-16#name-bls12-381-g1.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_H2SSuiteBls12381g1XmdSha256SswuRo">H2SSuiteBls12381g1XmdSha256SswuRo</a>
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

<a name="0x1_algebra_bls12381_H2SSuiteBls12381g2XmdSha256SswuRo"></a>

## Struct `H2SSuiteBls12381g2XmdSha256SswuRo`

The hash-to-curve suite <code>BLS12381G2_XMD:SHA-256_SSWU_RO_</code> that hashes a byte array into <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_G1Affine">G1Affine</a></code> elements.

Full specification is defined in https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-16#name-bls12-381-g2.


<pre><code><b>struct</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_H2SSuiteBls12381g2XmdSha256SswuRo">H2SSuiteBls12381g2XmdSha256SswuRo</a>
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


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
