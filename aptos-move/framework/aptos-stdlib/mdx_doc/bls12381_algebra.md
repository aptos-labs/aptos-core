
<a id="0x1_bls12381_algebra"></a>

# Module `0x1::bls12381_algebra`

This module defines marker types, constants and test cases for working with BLS12&#45;381 curves
using the generic API defined in <code>algebra.<b>move</b></code>.
See https://datatracker.ietf.org/doc/html/draft&#45;irtf&#45;cfrg&#45;pairing&#45;friendly&#45;curves&#45;11#name&#45;bls&#45;curves&#45;for&#45;the&#45;128&#45;bit&#45;
for the full specification of BLS12&#45;381 curves.

Currently&#45;supported BLS12&#45;381 structures include <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fq12">Fq12</a></code>, <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a></code>, <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code>, <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> and <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a></code>,
along with their widely&#45;used serialization formats,
the pairing between <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code>, <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> and <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a></code>,
and the hash&#45;to&#45;curve operations for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> and <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> defined in https://datatracker.ietf.org/doc/html/draft&#45;irtf&#45;cfrg&#45;hash&#45;to&#45;curve&#45;16.

Other unimplemented BLS12&#45;381 structures and serialization formats are also listed here,
as they help define some of the currently supported structures.
Their implementation may also be added in the future.

<code>Fq</code>: the finite field $F_q$ used in BLS12&#45;381 curves with a prime order $q$ equal to
0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.

<code>FormatFqLsb</code>: a serialization format for <code>Fq</code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48 with the least significant byte (LSB) coming first.

<code>FormatFqMsb</code>: a serialization format for <code>Fq</code> elements,
where an element is represented by a byte array <code>b[]</code> of size 48 with the most significant byte (MSB) coming first.

<code>Fq2</code>: the finite field $F_&#123;q^2&#125;$ used in BLS12&#45;381 curves,
which is an extension field of <code>Fq</code>, constructed as $F_&#123;q^2&#125;&#61;F_q[u]/(u^2&#43;1)$.

<code>FormatFq2LscLsb</code>: a serialization format for <code>Fq2</code> elements,
where an element in the form $(c_0&#43;c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96,
which is a concatenation of its coefficients serialized, with the least significant coefficient (LSC) coming first:
&#45; <code>b[0..48]</code> is $c_0$ serialized using <code>FormatFqLsb</code>.
&#45; <code>b[48..96]</code> is $c_1$ serialized using <code>FormatFqLsb</code>.

<code>FormatFq2MscMsb</code>: a serialization format for <code>Fq2</code> elements,
where an element in the form $(c_0&#43;c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size 96,
which is a concatenation of its coefficients serialized, with the most significant coefficient (MSC) coming first:
&#45; <code>b[0..48]</code> is $c_1$ serialized using <code>FormatFqLsb</code>.
&#45; <code>b[48..96]</code> is $c_0$ serialized using <code>FormatFqLsb</code>.

<code>Fq6</code>: the finite field $F_&#123;q^6&#125;$ used in BLS12&#45;381 curves,
which is an extension field of <code>Fq2</code>, constructed as $F_&#123;q^6&#125;&#61;F_&#123;q^2&#125;[v]/(v^3&#45;u&#45;1)$.

<code>FormatFq6LscLsb</code>: a serialization scheme for <code>Fq6</code> elements,
where an element in the form $(c_0&#43;c_1\cdot v&#43;c_2\cdot v^2)$ is represented by a byte array <code>b[]</code> of size 288,
which is a concatenation of its coefficients serialized, with the least significant coefficient (LSC) coming first:
&#45; <code>b[0..96]</code> is $c_0$ serialized using <code>FormatFq2LscLsb</code>.
&#45; <code>b[96..192]</code> is $c_1$ serialized using <code>FormatFq2LscLsb</code>.
&#45; <code>b[192..288]</code> is $c_2$ serialized using <code>FormatFq2LscLsb</code>.

<code>G1Full</code>: a group constructed by the points on the BLS12&#45;381 curve $E(F_q): y^2&#61;x^3&#43;4$ and the point at infinity,
under the elliptic curve point addition.
It contains the prime&#45;order subgroup $G_1$ used in pairing.

<code>G2Full</code>: a group constructed by the points on a curve $E&apos;(F_&#123;q^2&#125;): y^2&#61;x^3&#43;4(u&#43;1)$ and the point at infinity,
under the elliptic curve point addition.
It contains the prime&#45;order subgroup $G_2$ used in pairing.


-  [Struct `Fq12`](#0x1_bls12381_algebra_Fq12)
-  [Struct `FormatFq12LscLsb`](#0x1_bls12381_algebra_FormatFq12LscLsb)
-  [Struct `G1`](#0x1_bls12381_algebra_G1)
-  [Struct `FormatG1Uncompr`](#0x1_bls12381_algebra_FormatG1Uncompr)
-  [Struct `FormatG1Compr`](#0x1_bls12381_algebra_FormatG1Compr)
-  [Struct `G2`](#0x1_bls12381_algebra_G2)
-  [Struct `FormatG2Uncompr`](#0x1_bls12381_algebra_FormatG2Uncompr)
-  [Struct `FormatG2Compr`](#0x1_bls12381_algebra_FormatG2Compr)
-  [Struct `Gt`](#0x1_bls12381_algebra_Gt)
-  [Struct `FormatGt`](#0x1_bls12381_algebra_FormatGt)
-  [Struct `Fr`](#0x1_bls12381_algebra_Fr)
-  [Struct `FormatFrLsb`](#0x1_bls12381_algebra_FormatFrLsb)
-  [Struct `FormatFrMsb`](#0x1_bls12381_algebra_FormatFrMsb)
-  [Struct `HashG1XmdSha256SswuRo`](#0x1_bls12381_algebra_HashG1XmdSha256SswuRo)
-  [Struct `HashG2XmdSha256SswuRo`](#0x1_bls12381_algebra_HashG2XmdSha256SswuRo)


<pre><code></code></pre>



<a id="0x1_bls12381_algebra_Fq12"></a>

## Struct `Fq12`

The finite field $F_&#123;q^12&#125;$ used in BLS12&#45;381 curves,
which is an extension field of <code>Fq6</code> (defined in the module documentation), constructed as $F_&#123;q^12&#125;&#61;F_&#123;q^6&#125;[w]/(w^2&#45;v)$.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_Fq12">Fq12</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatFq12LscLsb"></a>

## Struct `FormatFq12LscLsb`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fq12">Fq12</a></code> elements,
where an element $(c_0&#43;c_1\cdot w)$ is represented by a byte array <code>b[]</code> of size 576,
which is a concatenation of its coefficients serialized, with the least significant coefficient (LSC) coming first.
&#45; <code>b[0..288]</code> is $c_0$ serialized using <code>FormatFq6LscLsb</code> (defined in the module documentation).
&#45; <code>b[288..576]</code> is $c_1$ serialized using <code>FormatFq6LscLsb</code>.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatFq12LscLsb">FormatFq12LscLsb</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_G1"></a>

## Struct `G1`

The group $G_1$ in BLS12&#45;381&#45;based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a subgroup of <code>G1Full</code> (defined in the module documentation) with a prime order $r$
equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a></code> is the associated scalar field).


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatG1Uncompr"></a>

## Struct `FormatG1Uncompr`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> elements derived from
https://www.ietf.org/archive/id/draft&#45;irtf&#45;cfrg&#45;pairing&#45;friendly&#45;curves&#45;11.html#name&#45;zcash&#45;serialization&#45;format&#45;.

Below is the serialization procedure that takes a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> element <code>p</code> and outputs a byte array of size 96.
1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.
1. Serialize <code>x</code> and <code>y</code> into <code>b_x[]</code> and <code>b_y[]</code> respectively using <code>FormatFqMsb</code> (defined in the module documentation).
1. Concatenate <code>b_x[]</code> and <code>b_y[]</code> into <code>b[]</code>.
1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[0]: &#61; b[0] &#124; 0x40</code>.
1. Return <code>b[]</code>.

Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> element or none.
1. If the size of <code>b[]</code> is not 96, return none.
1. Compute the compression flag as <code>b[0] &amp; 0x80 !&#61; 0</code>.
1. If the compression flag is true, return none.
1. Compute the infinity flag as <code>b[0] &amp; 0x40 !&#61; 0</code>.
1. If the infinity flag is set, return the point at infinity.
1. Deserialize <code>[b[0] &amp; 0x1f, b[1], ..., b[47]]</code> to <code>x</code> using <code>FormatFqMsb</code>. If <code>x</code> is none, return none.
1. Deserialize <code>[b[48], ..., b[95]]</code> to <code>y</code> using <code>FormatFqMsb</code>. If <code>y</code> is none, return none.
1. Check if <code>(x,y)</code> is on curve <code>E</code>. If not, return none.
1. Check if <code>(x,y)</code> is in the subgroup of order <code>r</code>. If not, return none.
1. Return <code>(x,y)</code>.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatG1Uncompr">FormatG1Uncompr</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatG1Compr"></a>

## Struct `FormatG1Compr`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> elements derived from
https://www.ietf.org/archive/id/draft&#45;irtf&#45;cfrg&#45;pairing&#45;friendly&#45;curves&#45;11.html#name&#45;zcash&#45;serialization&#45;format&#45;.

Below is the serialization procedure that takes a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> element <code>p</code> and outputs a byte array of size 48.
1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.
1. Serialize <code>x</code> into <code>b[]</code> using <code>FormatFqMsb</code> (defined in the module documentation).
1. Set the compression bit: <code>b[0] :&#61; b[0] &#124; 0x80</code>.
1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[0]: &#61; b[0] &#124; 0x40</code>.
1. If <code>y &gt; &#45;y</code>, set the lexicographical flag: <code>b[0] :&#61; b[0] &#124; 0x20</code>.
1. Return <code>b[]</code>.

Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> element or none.
1. If the size of <code>b[]</code> is not 48, return none.
1. Compute the compression flag as <code>b[0] &amp; 0x80 !&#61; 0</code>.
1. If the compression flag is false, return none.
1. Compute the infinity flag as <code>b[0] &amp; 0x40 !&#61; 0</code>.
1. If the infinity flag is set, return the point at infinity.
1. Compute the lexicographical flag as <code>b[0] &amp; 0x20 !&#61; 0</code>.
1. Deserialize <code>[b[0] &amp; 0x1f, b[1], ..., b[47]]</code> to <code>x</code> using <code>FormatFqMsb</code>. If <code>x</code> is none, return none.
1. Solve the curve equation with <code>x</code> for <code>y</code>. If no such <code>y</code> exists, return none.
1. Let <code>y&apos;</code> be <code>max(y,&#45;y)</code> if the lexicographical flag is set, or <code><b>min</b>(y,&#45;y)</code> otherwise.
1. Check if <code>(x,y&apos;)</code> is in the subgroup of order <code>r</code>. If not, return none.
1. Return <code>(x,y&apos;)</code>.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatG1Compr">FormatG1Compr</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_G2"></a>

## Struct `G2`

The group $G_2$ in BLS12&#45;381&#45;based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a subgroup of <code>G2Full</code> (defined in the module documentation) with a prime order $r$ equal to
0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a></code> is the scalar field).


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatG2Uncompr"></a>

## Struct `FormatG2Uncompr`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> elements derived from
https://www.ietf.org/archive/id/draft&#45;irtf&#45;cfrg&#45;pairing&#45;friendly&#45;curves&#45;11.html#name&#45;zcash&#45;serialization&#45;format&#45;.

Below is the serialization procedure that takes a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> element <code>p</code> and outputs a byte array of size 192.
1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.
1. Serialize <code>x</code> and <code>y</code> into <code>b_x[]</code> and <code>b_y[]</code> respectively using <code>FormatFq2MscMsb</code> (defined in the module documentation).
1. Concatenate <code>b_x[]</code> and <code>b_y[]</code> into <code>b[]</code>.
1. If <code>p</code> is the point at infinity, set the infinity bit in <code>b[]</code>: <code>b[0]: &#61; b[0] &#124; 0x40</code>.
1. Return <code>b[]</code>.

Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> element or none.
1. If the size of <code>b[]</code> is not 192, return none.
1. Compute the compression flag as <code>b[0] &amp; 0x80 !&#61; 0</code>.
1. If the compression flag is true, return none.
1. Compute the infinity flag as <code>b[0] &amp; 0x40 !&#61; 0</code>.
1. If the infinity flag is set, return the point at infinity.
1. Deserialize <code>[b[0] &amp; 0x1f, ..., b[95]]</code> to <code>x</code> using <code>FormatFq2MscMsb</code>. If <code>x</code> is none, return none.
1. Deserialize <code>[b[96], ..., b[191]]</code> to <code>y</code> using <code>FormatFq2MscMsb</code>. If <code>y</code> is none, return none.
1. Check if <code>(x,y)</code> is on the curve <code>E&apos;</code>. If not, return none.
1. Check if <code>(x,y)</code> is in the subgroup of order <code>r</code>. If not, return none.
1. Return <code>(x,y)</code>.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatG2Uncompr">FormatG2Uncompr</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatG2Compr"></a>

## Struct `FormatG2Compr`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> elements derived from
https://www.ietf.org/archive/id/draft&#45;irtf&#45;cfrg&#45;pairing&#45;friendly&#45;curves&#45;11.html#name&#45;zcash&#45;serialization&#45;format&#45;.

Below is the serialization procedure that takes a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> element <code>p</code> and outputs a byte array of size 96.
1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.
1. Serialize <code>x</code> into <code>b[]</code> using <code>FormatFq2MscMsb</code> (defined in the module documentation).
1. Set the compression bit: <code>b[0] :&#61; b[0] &#124; 0x80</code>.
1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[0]: &#61; b[0] &#124; 0x40</code>.
1. If <code>y &gt; &#45;y</code>, set the lexicographical flag: <code>b[0] :&#61; b[0] &#124; 0x20</code>.
1. Return <code>b[]</code>.

Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> element or none.
1. If the size of <code>b[]</code> is not 96, return none.
1. Compute the compression flag as <code>b[0] &amp; 0x80 !&#61; 0</code>.
1. If the compression flag is false, return none.
1. Compute the infinity flag as <code>b[0] &amp; 0x40 !&#61; 0</code>.
1. If the infinity flag is set, return the point at infinity.
1. Compute the lexicographical flag as <code>b[0] &amp; 0x20 !&#61; 0</code>.
1. Deserialize <code>[b[0] &amp; 0x1f, b[1], ..., b[95]]</code> to <code>x</code> using <code>FormatFq2MscMsb</code>. If <code>x</code> is none, return none.
1. Solve the curve equation with <code>x</code> for <code>y</code>. If no such <code>y</code> exists, return none.
1. Let <code>y&apos;</code> be <code>max(y,&#45;y)</code> if the lexicographical flag is set, or <code><b>min</b>(y,&#45;y)</code> otherwise.
1. Check if <code>(x,y&apos;)</code> is in the subgroup of order <code>r</code>. If not, return none.
1. Return <code>(x,y&apos;)</code>.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatG2Compr">FormatG2Compr</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_Gt"></a>

## Struct `Gt`

The group $G_t$ in BLS12&#45;381&#45;based pairing $G_1 \times G_2 \rightarrow G_t$.
It is a multiplicative subgroup of <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fq12">Fq12</a></code>,
with a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
(so <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a></code> is the scalar field).
The identity of <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a></code> is 1.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatGt"></a>

## Struct `FormatGt`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a></code> elements.

To serialize, it treats a <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a></code> element <code>p</code> as an <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fq12">Fq12</a></code> element and serialize it using <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatFq12LscLsb">FormatFq12LscLsb</a></code>.

To deserialize, it uses <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatFq12LscLsb">FormatFq12LscLsb</a></code> to try deserializing to an <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fq12">Fq12</a></code> element then test the membership in <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Gt">Gt</a></code>.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatGt">FormatGt</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_Fr"></a>

## Struct `Fr`

The finite field $F_r$ that can be used as the scalar fields
associated with the groups $G_1$, $G_2$, $G_t$ in BLS12&#45;381&#45;based pairing.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatFrLsb"></a>

## Struct `FormatFrLsb`

A serialization format for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the least significant byte (LSB) coming first.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0, blst&#45;0.3.7.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatFrLsb">FormatFrLsb</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_FormatFrMsb"></a>

## Struct `FormatFrMsb`

A serialization scheme for <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_Fr">Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the most significant byte (MSB) coming first.

NOTE: other implementation(s) using this format: ark&#45;bls12&#45;381&#45;0.4.0, blst&#45;0.3.7.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_FormatFrMsb">FormatFrMsb</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_HashG1XmdSha256SswuRo"></a>

## Struct `HashG1XmdSha256SswuRo`

The hash&#45;to&#45;curve suite <code>BLS12381G1_XMD:SHA&#45;256_SSWU_RO_</code> that hashes a byte array into <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G1">G1</a></code> elements.

Full specification is defined in https://datatracker.ietf.org/doc/html/draft&#45;irtf&#45;cfrg&#45;hash&#45;to&#45;curve&#45;16#name&#45;bls12&#45;381&#45;g1.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_HashG1XmdSha256SswuRo">HashG1XmdSha256SswuRo</a><br /></code></pre>



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

<a id="0x1_bls12381_algebra_HashG2XmdSha256SswuRo"></a>

## Struct `HashG2XmdSha256SswuRo`

The hash&#45;to&#45;curve suite <code>BLS12381G2_XMD:SHA&#45;256_SSWU_RO_</code> that hashes a byte array into <code><a href="bls12381_algebra.md#0x1_bls12381_algebra_G2">G2</a></code> elements.

Full specification is defined in https://datatracker.ietf.org/doc/html/draft&#45;irtf&#45;cfrg&#45;hash&#45;to&#45;curve&#45;16#name&#45;bls12&#45;381&#45;g2.


<pre><code><b>struct</b> <a href="bls12381_algebra.md#0x1_bls12381_algebra_HashG2XmdSha256SswuRo">HashG2XmdSha256SswuRo</a><br /></code></pre>



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


[move-book]: https://aptos.dev/move/book/SUMMARY
