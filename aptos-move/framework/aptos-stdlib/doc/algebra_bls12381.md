
<a name="0x1_algebra_bls12381"></a>

# Module `0x1::algebra_bls12381`

This module defines marker types, constants and test cases for working with BLS12-381 curves
using generic API defined in <code><a href="algebra.md#0x1_algebra">algebra</a>.<b>move</b></code>.

Below are the BLS12-381 structures currently supported.
- Field <code>Fq12</code>.
- Group <code>G1Affine</code>.
- Group <code>G2Affine</code>.
- Group <code>Gt</code>.
- Field <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code>.


-  [Struct `Fr`](#0x1_algebra_bls12381_Fr)
-  [Function `format_bls12381fr_lsb`](#0x1_algebra_bls12381_format_bls12381fr_lsb)
-  [Function `format_bls12381fr_msb`](#0x1_algebra_bls12381_format_bls12381fr_msb)


<pre><code></code></pre>



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

<a name="0x1_algebra_bls12381_format_bls12381fr_lsb"></a>

## Function `format_bls12381fr_lsb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the least significant byte coming first.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_lsb">format_bls12381fr_lsb</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_lsb">format_bls12381fr_lsb</a>(): u64 { 0x0a00000000000000 }
</code></pre>



</details>

<a name="0x1_algebra_bls12381_format_bls12381fr_msb"></a>

## Function `format_bls12381fr_msb`

A serialization scheme for <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code> elements,
where an element is represented by a byte array <code>b[]</code> of size 32 with the most significant byte coming first.

NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_msb">format_bls12381fr_msb</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra_bls12381.md#0x1_algebra_bls12381_format_bls12381fr_msb">format_bls12381fr_msb</a>(): u64 { 0x0a01000000000000 }
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
