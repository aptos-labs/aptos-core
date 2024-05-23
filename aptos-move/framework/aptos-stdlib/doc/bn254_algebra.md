
<a id="0x1_bn254_algebra"></a>

# Module `0x1::bn254_algebra`

This module defines marker types, constants and test cases for working with BN254 curves using the generic API defined in <code>algebra.move</code>.<br/> BN254 was sampled as part of the [\[BCTV14\]](https://eprint.iacr.org/2013/879.pdf) paper .<br/> The name denotes that it is a Barreto&#45;Naehrig curve of embedding degree 12, defined over a 254&#45;bit (prime) field.<br/> The scalar field is highly 2&#45;adic which supports subgroups of roots of unity of size &lt;&#61; 2^28.<br/> (as (21888242871839275222246405745257275088548364400416034343698204186575808495617 &#45; 1) mod 2^28 &#61; 0)<br/><br/> This curve is also implemented in [libff](https://github.com/scipr&#45;lab/libff/tree/master/libff/algebra/curves/alt_bn128) under the name <code>bn128</code>.<br/> It is the same as the <code>bn254</code> curve used in Ethereum (eg: [go&#45;ethereum](https://github.com/ethereum/go&#45;ethereum/tree/master/crypto/bn254/cloudflare)).<br/><br/> &#35;CAUTION<br/> &#42;&#42;This curve does not satisfy the 128&#45;bit security level anymore.&#42;&#42;<br/><br/> Its current security is estimated at 128&#45;bits (see &quot;Updating Key Size Estimations for Pairings&quot;; by Barbulescu, Razvan and Duquesne, Sylvain; in Journal of Cryptology; 2019; https://doi.org/10.1007/s00145&#45;018&#45;9280&#45;5)<br/><br/><br/> Curve information:<br/> &#42; Base field: q &#61;<br/>   21888242871839275222246405745257275088696311157297823662689037894645226208583<br/> &#42; Scalar field: r &#61;<br/>   21888242871839275222246405745257275088548364400416034343698204186575808495617<br/> &#42; valuation(q &#45; 1, 2) &#61; 1<br/> &#42; valuation(r &#45; 1, 2) &#61; 28<br/> &#42; G1 curve equation: y^2 &#61; x^3 &#43; 3<br/> &#42; G2 curve equation: y^2 &#61; x^3 &#43; B, where<br/>    &#42; B &#61; 3/(u&#43;9) where Fq2 is represented as Fq\[u\]/(u^2&#43;1) &#61;<br/>      Fq2(19485874751759354771024239261021720505790618469301721065564631296452457478373,<br/>      266929791119991161246907387137283842545076965332900288569378510910307636690)<br/><br/><br/> Currently&#45;supported BN254 structures include <code>Fq12</code>, <code>Fr</code>, <code>Fq</code>, <code>Fq2</code>, <code>G1</code>, <code>G2</code> and <code>Gt</code>,<br/> along with their widely&#45;used serialization formats,<br/> the pairing between <code>G1</code>, <code>G2</code> and <code>Gt</code>.<br/><br/> Other unimplemented BN254 structures and serialization formats are also listed here,<br/> as they help define some of the currently supported structures.<br/> Their implementation may also be added in the future.<br/><br/> <code>Fq2</code>: The finite field $F_&#123;q^2&#125;$ that can be used as the base field of $G_2$<br/> which is an extension field of <code>Fq</code>, constructed as $F_&#123;q^2&#125;&#61;F_&#123;q&#125;[u]/(u^2&#43;1)$.<br/><br/> <code>FormatFq2LscLsb</code>: A serialization scheme for <code>Fq2</code> elements,<br/> where an element $(c_0&#43;c_1\cdot u)$ is represented by a byte array <code>b[]</code> of size N&#61;64,<br/> which is a concatenation of its coefficients serialized, with the least significant coefficient (LSC) coming first.<br/> &#45; <code>b[0..32]</code> is $c_0$ serialized using <code>FormatFqLscLsb</code>.<br/> &#45; <code>b[32..64]</code> is $c_1$ serialized using <code>FormatFqLscLsb</code>.<br/><br/> <code>Fq6</code>: the finite field $F_&#123;q^6&#125;$ used in BN254 curves,<br/> which is an extension field of <code>Fq2</code>, constructed as $F_&#123;q^6&#125;&#61;F_&#123;q^2&#125;[v]/(v^3&#45;u&#45;9)$.<br/><br/> <code>FormatFq6LscLsb</code>: a serialization scheme for <code>Fq6</code> elements,<br/> where an element in the form $(c_0&#43;c_1\cdot v&#43;c_2\cdot v^2)$ is represented by a byte array <code>b[]</code> of size 192,<br/> which is a concatenation of its coefficients serialized, with the least significant coefficient (LSC) coming first:<br/> &#45; <code>b[0..64]</code> is $c_0$ serialized using <code>FormatFq2LscLsb</code>.<br/> &#45; <code>b[64..128]</code> is $c_1$ serialized using <code>FormatFq2LscLsb</code>.<br/> &#45; <code>b[128..192]</code> is $c_2$ serialized using <code>FormatFq2LscLsb</code>.<br/><br/> <code>G1Full</code>: a group constructed by the points on the BN254 curve $E(F_q): y^2&#61;x^3&#43;3$ and the point at infinity,<br/> under the elliptic curve point addition.<br/> It contains the prime&#45;order subgroup $G_1$ used in pairing.<br/><br/> <code>G2Full</code>: a group constructed by the points on a curve $E&apos;(F_&#123;q^2&#125;): y^2&#61;x^3&#43;3/(u&#43;9)$ and the point at infinity,<br/> under the elliptic curve point addition.<br/> It contains the prime&#45;order subgroup $G_2$ used in pairing.


-  [Struct `Fr`](#0x1_bn254_algebra_Fr)
-  [Struct `FormatFrLsb`](#0x1_bn254_algebra_FormatFrLsb)
-  [Struct `FormatFrMsb`](#0x1_bn254_algebra_FormatFrMsb)
-  [Struct `Fq`](#0x1_bn254_algebra_Fq)
-  [Struct `FormatFqLsb`](#0x1_bn254_algebra_FormatFqLsb)
-  [Struct `FormatFqMsb`](#0x1_bn254_algebra_FormatFqMsb)
-  [Struct `Fq12`](#0x1_bn254_algebra_Fq12)
-  [Struct `FormatFq12LscLsb`](#0x1_bn254_algebra_FormatFq12LscLsb)
-  [Struct `G1`](#0x1_bn254_algebra_G1)
-  [Struct `FormatG1Uncompr`](#0x1_bn254_algebra_FormatG1Uncompr)
-  [Struct `FormatG1Compr`](#0x1_bn254_algebra_FormatG1Compr)
-  [Struct `G2`](#0x1_bn254_algebra_G2)
-  [Struct `FormatG2Uncompr`](#0x1_bn254_algebra_FormatG2Uncompr)
-  [Struct `FormatG2Compr`](#0x1_bn254_algebra_FormatG2Compr)
-  [Struct `Gt`](#0x1_bn254_algebra_Gt)
-  [Struct `FormatGt`](#0x1_bn254_algebra_FormatGt)


<pre><code></code></pre>



<a id="0x1_bn254_algebra_Fr"></a>

## Struct `Fr`

The finite field $F_r$ that can be used as the scalar fields<br/> associated with the groups $G_1$, $G_2$, $G_t$ in BN254&#45;based pairing.


<pre><code>struct Fr<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatFrLsb"></a>

## Struct `FormatFrLsb`

A serialization format for <code>Fr</code> elements,<br/> where an element is represented by a byte array <code>b[]</code> of size 32 with the least significant byte (LSB) coming first.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatFrLsb<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatFrMsb"></a>

## Struct `FormatFrMsb`

A serialization scheme for <code>Fr</code> elements,<br/> where an element is represented by a byte array <code>b[]</code> of size 32 with the most significant byte (MSB) coming first.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatFrMsb<br/></code></pre>



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

<a id="0x1_bn254_algebra_Fq"></a>

## Struct `Fq`

The finite field $F_q$ that can be used as the base field of $G_1$


<pre><code>struct Fq<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatFqLsb"></a>

## Struct `FormatFqLsb`

A serialization format for <code>Fq</code> elements,<br/> where an element is represented by a byte array <code>b[]</code> of size 32 with the least significant byte (LSB) coming first.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatFqLsb<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatFqMsb"></a>

## Struct `FormatFqMsb`

A serialization scheme for <code>Fq</code> elements,<br/> where an element is represented by a byte array <code>b[]</code> of size 32 with the most significant byte (MSB) coming first.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatFqMsb<br/></code></pre>



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

<a id="0x1_bn254_algebra_Fq12"></a>

## Struct `Fq12`

The finite field $F_&#123;q^12&#125;$ used in BN254 curves,<br/> which is an extension field of <code>Fq6</code> (defined in the module documentation), constructed as $F_&#123;q^12&#125;&#61;F_&#123;q^6&#125;[w]/(w^2&#45;v)$.<br/> The field can downcast to <code>Gt</code> if it&apos;s an element of the multiplicative subgroup <code>Gt</code> of <code>Fq12</code><br/> with a prime order $r$ &#61; 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.


<pre><code>struct Fq12<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatFq12LscLsb"></a>

## Struct `FormatFq12LscLsb`

A serialization scheme for <code>Fq12</code> elements,<br/> where an element $(c_0&#43;c_1\cdot w)$ is represented by a byte array <code>b[]</code> of size 384,<br/> which is a concatenation of its coefficients serialized, with the least significant coefficient (LSC) coming first.<br/> &#45; <code>b[0..192]</code> is $c_0$ serialized using <code>FormatFq6LscLsb</code> (defined in the module documentation).<br/> &#45; <code>b[192..384]</code> is $c_1$ serialized using <code>FormatFq6LscLsb</code>.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatFq12LscLsb<br/></code></pre>



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

<a id="0x1_bn254_algebra_G1"></a>

## Struct `G1`

The group $G_1$ in BN254&#45;based pairing $G_1 \times G_2 \rightarrow G_t$.<br/> It is a subgroup of <code>G1Full</code> (defined in the module documentation) with a prime order $r$<br/> equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.<br/> (so <code>Fr</code> is the associated scalar field).


<pre><code>struct G1<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatG1Uncompr"></a>

## Struct `FormatG1Uncompr`

A serialization scheme for <code>G1</code> elements derived from arkworks.rs.<br/><br/> Below is the serialization procedure that takes a <code>G1</code> element <code>p</code> and outputs a byte array of size N&#61;64.<br/> 1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.<br/> 1. Serialize <code>x</code> and <code>y</code> into <code>b_x[]</code> and <code>b_y[]</code> respectively using <code>FormatFqLsb</code> (defined in the module documentation).<br/> 1. Concatenate <code>b_x[]</code> and <code>b_y[]</code> into <code>b[]</code>.<br/> 1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[N&#45;1]: &#61; b[N&#45;1] &#124; 0b0100_0000</code>.<br/> 1. If <code>y &gt; &#45;y</code>, set the lexicographical bit:  <code>b[N&#45;1]: &#61; b[N&#45;1] &#124; 0b1000_0000</code>.<br/> 1. Return <code>b[]</code>.<br/><br/> Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code>G1</code> element or none.<br/> 1. If the size of <code>b[]</code> is not N, return none.<br/> 1. Compute the infinity flag as <code>b[N&#45;1] &amp; 0b0100_0000 !&#61; 0</code>.<br/> 1. If the infinity flag is set, return the point at infinity.<br/> 1. Deserialize <code>[b[0], b[1], ..., b[N/2&#45;1]]</code> to <code>x</code> using <code>FormatFqLsb</code>. If <code>x</code> is none, return none.<br/> 1. Deserialize <code>[b[N/2], ..., b[N] &amp; 0b0011_1111]</code> to <code>y</code> using <code>FormatFqLsb</code>. If <code>y</code> is none, return none.<br/> 1. Check if <code>(x,y)</code> is on curve <code>E</code>. If not, return none.<br/> 1. Check if <code>(x,y)</code> is in the subgroup of order <code>r</code>. If not, return none.<br/> 1. Return <code>(x,y)</code>.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatG1Uncompr<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatG1Compr"></a>

## Struct `FormatG1Compr`

A serialization scheme for <code>G1</code> elements derived from arkworks.rs<br/><br/> Below is the serialization procedure that takes a <code>G1</code> element <code>p</code> and outputs a byte array of size N&#61;32.<br/> 1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.<br/> 1. Serialize <code>x</code> into <code>b[]</code> using <code>FormatFqLsb</code> (defined in the module documentation).<br/> 1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[N&#45;1]: &#61; b[N&#45;1] &#124; 0b0100_0000</code>.<br/> 1. If <code>y &gt; &#45;y</code>, set the lexicographical flag: <code>b[N&#45;1] :&#61; b[N&#45;1] &#124; 0x1000_0000</code>.<br/> 1. Return <code>b[]</code>.<br/><br/> Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code>G1</code> element or none.<br/> 1. If the size of <code>b[]</code> is not N, return none.<br/> 1. Compute the infinity flag as <code>b[N&#45;1] &amp; 0b0100_0000 !&#61; 0</code>.<br/> 1. If the infinity flag is set, return the point at infinity.<br/> 1. Compute the lexicographical flag as <code>b[N&#45;1] &amp; 0b1000_0000 !&#61; 0</code>.<br/> 1. Deserialize <code>[b[0], b[1], ..., b[N/2&#45;1] &amp; 0b0011_1111]</code> to <code>x</code> using <code>FormatFqLsb</code>. If <code>x</code> is none, return none.<br/> 1. Solve the curve equation with <code>x</code> for <code>y</code>. If no such <code>y</code> exists, return none.<br/> 1. Let <code>y&apos;</code> be <code>max(y,&#45;y)</code> if the lexicographical flag is set, or <code>min(y,&#45;y)</code> otherwise.<br/> 1. Check if <code>(x,y&apos;)</code> is in the subgroup of order <code>r</code>. If not, return none.<br/> 1. Return <code>(x,y&apos;)</code>.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatG1Compr<br/></code></pre>



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

<a id="0x1_bn254_algebra_G2"></a>

## Struct `G2`

The group $G_2$ in BN254&#45;based pairing $G_1 \times G_2 \rightarrow G_t$.<br/> It is a subgroup of <code>G2Full</code> (defined in the module documentation) with a prime order $r$ equal to<br/> 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.<br/> (so <code>Fr</code> is the scalar field).


<pre><code>struct G2<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatG2Uncompr"></a>

## Struct `FormatG2Uncompr`

A serialization scheme for <code>G2</code> elements derived from arkworks.rs.<br/><br/> Below is the serialization procedure that takes a <code>G2</code> element <code>p</code> and outputs a byte array of size N&#61;128.<br/> 1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.<br/> 1. Serialize <code>x</code> and <code>y</code> into <code>b_x[]</code> and <code>b_y[]</code> respectively using <code>FormatFq2LscLsb</code> (defined in the module documentation).<br/> 1. Concatenate <code>b_x[]</code> and <code>b_y[]</code> into <code>b[]</code>.<br/> 1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[N&#45;1]: &#61; b[N&#45;1] &#124; 0b0100_0000</code>.<br/> 1. If <code>y &gt; &#45;y</code>, set the lexicographical bit:  <code>b[N&#45;1]: &#61; b[N&#45;1] &#124; 0b1000_0000</code>.<br/> 1. Return <code>b[]</code>.<br/><br/> Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code>G1</code> element or none.<br/> 1. If the size of <code>b[]</code> is not N, return none.<br/> 1. Compute the infinity flag as <code>b[N&#45;1] &amp; 0b0100_0000 !&#61; 0</code>.<br/> 1. If the infinity flag is set, return the point at infinity.<br/> 1. Deserialize <code>[b[0], b[1], ..., b[N/2&#45;1]]</code> to <code>x</code> using <code>FormatFq2LscLsb</code>. If <code>x</code> is none, return none.<br/> 1. Deserialize <code>[b[N/2], ..., b[N] &amp; 0b0011_1111]</code> to <code>y</code> using <code>FormatFq2LscLsb</code>. If <code>y</code> is none, return none.<br/> 1. Check if <code>(x,y)</code> is on curve <code>E</code>. If not, return none.<br/> 1. Check if <code>(x,y)</code> is in the subgroup of order <code>r</code>. If not, return none.<br/> 1. Return <code>(x,y)</code>.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatG2Uncompr<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatG2Compr"></a>

## Struct `FormatG2Compr`

A serialization scheme for <code>G1</code> elements derived from arkworks.rs<br/><br/> Below is the serialization procedure that takes a <code>G1</code> element <code>p</code> and outputs a byte array of size N&#61;64.<br/> 1. Let <code>(x,y)</code> be the coordinates of <code>p</code> if <code>p</code> is on the curve, or <code>(0,0)</code> otherwise.<br/> 1. Serialize <code>x</code> into <code>b[]</code> using <code>FormatFq2LscLsb</code> (defined in the module documentation).<br/> 1. If <code>p</code> is the point at infinity, set the infinity bit: <code>b[N&#45;1]: &#61; b[N&#45;1] &#124; 0b0100_0000</code>.<br/> 1. If <code>y &gt; &#45;y</code>, set the lexicographical flag: <code>b[N&#45;1] :&#61; b[N&#45;1] &#124; 0x1000_0000</code>.<br/> 1. Return <code>b[]</code>.<br/><br/> Below is the deserialization procedure that takes a byte array <code>b[]</code> and outputs either a <code>G1</code> element or none.<br/> 1. If the size of <code>b[]</code> is not N, return none.<br/> 1. Compute the infinity flag as <code>b[N&#45;1] &amp; 0b0100_0000 !&#61; 0</code>.<br/> 1. If the infinity flag is set, return the point at infinity.<br/> 1. Compute the lexicographical flag as <code>b[N&#45;1] &amp; 0b1000_0000 !&#61; 0</code>.<br/> 1. Deserialize <code>[b[0], b[1], ..., b[N/2&#45;1] &amp; 0b0011_1111]</code> to <code>x</code> using <code>FormatFq2LscLsb</code>. If <code>x</code> is none, return none.<br/> 1. Solve the curve equation with <code>x</code> for <code>y</code>. If no such <code>y</code> exists, return none.<br/> 1. Let <code>y&apos;</code> be <code>max(y,&#45;y)</code> if the lexicographical flag is set, or <code>min(y,&#45;y)</code> otherwise.<br/> 1. Check if <code>(x,y&apos;)</code> is in the subgroup of order <code>r</code>. If not, return none.<br/> 1. Return <code>(x,y&apos;)</code>.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatG2Compr<br/></code></pre>



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

<a id="0x1_bn254_algebra_Gt"></a>

## Struct `Gt`

The group $G_t$ in BN254&#45;based pairing $G_1 \times G_2 \rightarrow G_t$.<br/> It is a multiplicative subgroup of <code>Fq12</code>, so it  can upcast to <code>Fq12</code>.<br/> with a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.<br/> (so <code>Fr</code> is the scalar field).<br/> The identity of <code>Gt</code> is 1.


<pre><code>struct Gt<br/></code></pre>



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

<a id="0x1_bn254_algebra_FormatGt"></a>

## Struct `FormatGt`

A serialization scheme for <code>Gt</code> elements.<br/><br/> To serialize, it treats a <code>Gt</code> element <code>p</code> as an <code>Fq12</code> element and serialize it using <code>FormatFq12LscLsb</code>.<br/><br/> To deserialize, it uses <code>FormatFq12LscLsb</code> to try deserializing to an <code>Fq12</code> element then test the membership in <code>Gt</code>.<br/><br/> NOTE: other implementation(s) using this format: ark&#45;bn254&#45;0.4.0.


<pre><code>struct FormatGt<br/></code></pre>



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
