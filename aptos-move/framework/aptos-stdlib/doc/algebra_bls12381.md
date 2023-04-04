
<a name="0x1_algebra_bls12381"></a>

# Module `0x1::algebra_bls12381`

This module defines marker types, constants and test cases for working with BLS12-381 curves
using generic API defined in <code><a href="algebra.md#0x1_algebra">algebra</a>.<b>move</b></code>.

Currently supported BLS12-381 structures include field <code><a href="algebra_bls12381.md#0x1_algebra_bls12381_Fr">Fr</a></code>.


-  [Struct `Fr`](#0x1_algebra_bls12381_Fr)
-  [Struct `FrFormatLsb`](#0x1_algebra_bls12381_FrFormatLsb)


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


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
