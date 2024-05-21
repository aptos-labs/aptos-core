
<a id="0x1_crypto_algebra"></a>

# Module `0x1::crypto_algebra`

This module provides generic structs/functions for operations of algebraic structures (e.g. fields and groups),
which can be used to build generic cryptographic schemes atop.
E.g., a Groth16 ZK proof verifier can be built to work over any pairing supported in this module.

In general, every structure implements basic operations like (de)serialization, equality check, random sampling.

A group may also implement the following operations. (Additive group notation is assumed.)
- <code>order()</code> for getting the group order.
- <code>zero()</code> for getting the group identity.
- <code>one()</code> for getting the group generator (if exists).
- <code>neg()</code> for group element inversion.
- <code>add()</code> for group operation (i.e., a group addition).
- <code>sub()</code> for group element subtraction.
- <code>double()</code> for efficient doubling.
- <code>scalar_mul()</code> for group scalar multiplication.
- <code>multi_scalar_mul()</code> for efficient group multi-scalar multiplication.
- <code>hash_to()</code> for hash-to-group.

A field may also implement the following operations.
- <code>zero()</code> for getting the field additive identity.
- <code>one()</code> for getting the field multiplicative identity.
- <code>add()</code> for field addition.
- <code>sub()</code> for field subtraction.
- <code>mul()</code> for field multiplication.
- <code>div()</code> for field division.
- <code>neg()</code> for field negation.
- <code>inv()</code> for field inversion.
- <code>sqr()</code> for efficient field element squaring.
- <code>from_u64()</code> for quick conversion from u64 to field element.

For 3 groups that admit a bilinear map, <code>pairing()</code> and <code>multi_pairing()</code> may be implemented.

For a subset/superset relationship between 2 structures, <code>upcast()</code> and <code>downcast()</code> may be implemented.
E.g., in BLS12-381 pairing, since <code>Gt</code> is a subset of <code>Fq12</code>,
<code>upcast&lt;Gt, Fq12&gt;()</code> and <code>downcast&lt;Fq12, Gt&gt;()</code> will be supported.

See <code>&#42;_algebra.move</code> for currently implemented algebraic structures.


-  [Struct `Element`](#0x1_crypto_algebra_Element)
-  [Constants](#@Constants_0)
-  [Function `eq`](#0x1_crypto_algebra_eq)
-  [Function `from_u64`](#0x1_crypto_algebra_from_u64)
-  [Function `zero`](#0x1_crypto_algebra_zero)
-  [Function `one`](#0x1_crypto_algebra_one)
-  [Function `neg`](#0x1_crypto_algebra_neg)
-  [Function `add`](#0x1_crypto_algebra_add)
-  [Function `sub`](#0x1_crypto_algebra_sub)
-  [Function `mul`](#0x1_crypto_algebra_mul)
-  [Function `div`](#0x1_crypto_algebra_div)
-  [Function `sqr`](#0x1_crypto_algebra_sqr)
-  [Function `inv`](#0x1_crypto_algebra_inv)
-  [Function `double`](#0x1_crypto_algebra_double)
-  [Function `multi_scalar_mul`](#0x1_crypto_algebra_multi_scalar_mul)
-  [Function `scalar_mul`](#0x1_crypto_algebra_scalar_mul)
-  [Function `multi_pairing`](#0x1_crypto_algebra_multi_pairing)
-  [Function `pairing`](#0x1_crypto_algebra_pairing)
-  [Function `deserialize`](#0x1_crypto_algebra_deserialize)
-  [Function `serialize`](#0x1_crypto_algebra_serialize)
-  [Function `order`](#0x1_crypto_algebra_order)
-  [Function `upcast`](#0x1_crypto_algebra_upcast)
-  [Function `downcast`](#0x1_crypto_algebra_downcast)
-  [Function `hash_to`](#0x1_crypto_algebra_hash_to)
-  [Function `abort_unless_cryptography_algebra_natives_enabled`](#0x1_crypto_algebra_abort_unless_cryptography_algebra_natives_enabled)
-  [Function `handles_from_elements`](#0x1_crypto_algebra_handles_from_elements)
-  [Function `add_internal`](#0x1_crypto_algebra_add_internal)
-  [Function `deserialize_internal`](#0x1_crypto_algebra_deserialize_internal)
-  [Function `div_internal`](#0x1_crypto_algebra_div_internal)
-  [Function `double_internal`](#0x1_crypto_algebra_double_internal)
-  [Function `downcast_internal`](#0x1_crypto_algebra_downcast_internal)
-  [Function `from_u64_internal`](#0x1_crypto_algebra_from_u64_internal)
-  [Function `eq_internal`](#0x1_crypto_algebra_eq_internal)
-  [Function `hash_to_internal`](#0x1_crypto_algebra_hash_to_internal)
-  [Function `inv_internal`](#0x1_crypto_algebra_inv_internal)
-  [Function `mul_internal`](#0x1_crypto_algebra_mul_internal)
-  [Function `multi_pairing_internal`](#0x1_crypto_algebra_multi_pairing_internal)
-  [Function `multi_scalar_mul_internal`](#0x1_crypto_algebra_multi_scalar_mul_internal)
-  [Function `neg_internal`](#0x1_crypto_algebra_neg_internal)
-  [Function `one_internal`](#0x1_crypto_algebra_one_internal)
-  [Function `order_internal`](#0x1_crypto_algebra_order_internal)
-  [Function `pairing_internal`](#0x1_crypto_algebra_pairing_internal)
-  [Function `scalar_mul_internal`](#0x1_crypto_algebra_scalar_mul_internal)
-  [Function `serialize_internal`](#0x1_crypto_algebra_serialize_internal)
-  [Function `sqr_internal`](#0x1_crypto_algebra_sqr_internal)
-  [Function `sub_internal`](#0x1_crypto_algebra_sub_internal)
-  [Function `upcast_internal`](#0x1_crypto_algebra_upcast_internal)
-  [Function `zero_internal`](#0x1_crypto_algebra_zero_internal)
-  [Specification](#@Specification_1)
    -  [Function `handles_from_elements`](#@Specification_1_handles_from_elements)
    -  [Function `add_internal`](#@Specification_1_add_internal)
    -  [Function `deserialize_internal`](#@Specification_1_deserialize_internal)
    -  [Function `div_internal`](#@Specification_1_div_internal)
    -  [Function `double_internal`](#@Specification_1_double_internal)
    -  [Function `downcast_internal`](#@Specification_1_downcast_internal)
    -  [Function `from_u64_internal`](#@Specification_1_from_u64_internal)
    -  [Function `eq_internal`](#@Specification_1_eq_internal)
    -  [Function `hash_to_internal`](#@Specification_1_hash_to_internal)
    -  [Function `inv_internal`](#@Specification_1_inv_internal)
    -  [Function `mul_internal`](#@Specification_1_mul_internal)
    -  [Function `multi_pairing_internal`](#@Specification_1_multi_pairing_internal)
    -  [Function `multi_scalar_mul_internal`](#@Specification_1_multi_scalar_mul_internal)
    -  [Function `neg_internal`](#@Specification_1_neg_internal)
    -  [Function `one_internal`](#@Specification_1_one_internal)
    -  [Function `order_internal`](#@Specification_1_order_internal)
    -  [Function `pairing_internal`](#@Specification_1_pairing_internal)
    -  [Function `scalar_mul_internal`](#@Specification_1_scalar_mul_internal)
    -  [Function `serialize_internal`](#@Specification_1_serialize_internal)
    -  [Function `sqr_internal`](#@Specification_1_sqr_internal)
    -  [Function `sub_internal`](#@Specification_1_sub_internal)
    -  [Function `upcast_internal`](#@Specification_1_upcast_internal)
    -  [Function `zero_internal`](#@Specification_1_zero_internal)


<pre><code>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::option;<br/></code></pre>



<a id="0x1_crypto_algebra_Element"></a>

## Struct `Element`

This struct represents an element of a structure <code>S</code>.


<pre><code>struct Element&lt;S&gt; has copy, drop<br/></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_crypto_algebra_E_NON_EQUAL_LENGTHS"></a>



<pre><code>const E_NON_EQUAL_LENGTHS: u64 &#61; 2;<br/></code></pre>



<a id="0x1_crypto_algebra_E_NOT_IMPLEMENTED"></a>



<pre><code>const E_NOT_IMPLEMENTED: u64 &#61; 1;<br/></code></pre>



<a id="0x1_crypto_algebra_E_TOO_MUCH_MEMORY_USED"></a>



<pre><code>const E_TOO_MUCH_MEMORY_USED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_crypto_algebra_eq"></a>

## Function `eq`

Check if <code>x &#61;&#61; y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.


<pre><code>public fun eq&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;, y: &amp;crypto_algebra::Element&lt;S&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun eq&lt;S&gt;(x: &amp;Element&lt;S&gt;, y: &amp;Element&lt;S&gt;): bool &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    eq_internal&lt;S&gt;(x.handle, y.handle)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_from_u64"></a>

## Function `from_u64`

Convert a u64 to an element of a structure <code>S</code>.


<pre><code>public fun from_u64&lt;S&gt;(value: u64): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun from_u64&lt;S&gt;(value: u64): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: from_u64_internal&lt;S&gt;(value)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_zero"></a>

## Function `zero`

Return the additive identity of field <code>S</code>, or the identity of group <code>S</code>.


<pre><code>public fun zero&lt;S&gt;(): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun zero&lt;S&gt;(): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: zero_internal&lt;S&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_one"></a>

## Function `one`

Return the multiplicative identity of field <code>S</code>, or a fixed generator of group <code>S</code>.


<pre><code>public fun one&lt;S&gt;(): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun one&lt;S&gt;(): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: one_internal&lt;S&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_neg"></a>

## Function `neg`

Compute <code>&#45;x</code> for an element <code>x</code> of a structure <code>S</code>.


<pre><code>public fun neg&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun neg&lt;S&gt;(x: &amp;Element&lt;S&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: neg_internal&lt;S&gt;(x.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_add"></a>

## Function `add`

Compute <code>x &#43; y</code> for elements <code>x</code> and <code>y</code> of structure <code>S</code>.


<pre><code>public fun add&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;, y: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;S&gt;(x: &amp;Element&lt;S&gt;, y: &amp;Element&lt;S&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: add_internal&lt;S&gt;(x.handle, y.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_sub"></a>

## Function `sub`

Compute <code>x &#45; y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.


<pre><code>public fun sub&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;, y: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sub&lt;S&gt;(x: &amp;Element&lt;S&gt;, y: &amp;Element&lt;S&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: sub_internal&lt;S&gt;(x.handle, y.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_mul"></a>

## Function `mul`

Compute <code>x &#42; y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.


<pre><code>public fun mul&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;, y: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mul&lt;S&gt;(x: &amp;Element&lt;S&gt;, y: &amp;Element&lt;S&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: mul_internal&lt;S&gt;(x.handle, y.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_div"></a>

## Function `div`

Try computing <code>x / y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.
Return none if <code>y</code> does not have a multiplicative inverse in the structure <code>S</code>
(e.g., when <code>S</code> is a field, and <code>y</code> is zero).


<pre><code>public fun div&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;, y: &amp;crypto_algebra::Element&lt;S&gt;): option::Option&lt;crypto_algebra::Element&lt;S&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun div&lt;S&gt;(x: &amp;Element&lt;S&gt;, y: &amp;Element&lt;S&gt;): Option&lt;Element&lt;S&gt;&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    let (succ, handle) &#61; div_internal&lt;S&gt;(x.handle, y.handle);<br/>    if (succ) &#123;<br/>        some(Element&lt;S&gt; &#123; handle &#125;)<br/>    &#125; else &#123;<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_sqr"></a>

## Function `sqr`

Compute <code>x^2</code> for an element <code>x</code> of a structure <code>S</code>. Faster and cheaper than <code>mul(x, x)</code>.


<pre><code>public fun sqr&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sqr&lt;S&gt;(x: &amp;Element&lt;S&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: sqr_internal&lt;S&gt;(x.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_inv"></a>

## Function `inv`

Try computing <code>x^(&#45;1)</code> for an element <code>x</code> of a structure <code>S</code>.
Return none if <code>x</code> does not have a multiplicative inverse in the structure <code>S</code>
(e.g., when <code>S</code> is a field, and <code>x</code> is zero).


<pre><code>public fun inv&lt;S&gt;(x: &amp;crypto_algebra::Element&lt;S&gt;): option::Option&lt;crypto_algebra::Element&lt;S&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun inv&lt;S&gt;(x: &amp;Element&lt;S&gt;): Option&lt;Element&lt;S&gt;&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    let (succeeded, handle) &#61; inv_internal&lt;S&gt;(x.handle);<br/>    if (succeeded) &#123;<br/>        let scalar &#61; Element&lt;S&gt; &#123; handle &#125;;<br/>        some(scalar)<br/>    &#125; else &#123;<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_double"></a>

## Function `double`

Compute <code>2&#42;P</code> for an element <code>P</code> of a structure <code>S</code>. Faster and cheaper than <code>add(P, P)</code>.


<pre><code>public fun double&lt;S&gt;(element_p: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun double&lt;S&gt;(element_p: &amp;Element&lt;S&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;S&gt; &#123;<br/>        handle: double_internal&lt;S&gt;(element_p.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_multi_scalar_mul"></a>

## Function `multi_scalar_mul`

Compute <code>k[0]&#42;P[0]&#43;...&#43;k[n&#45;1]&#42;P[n&#45;1]</code>, where
<code>P[]</code> are <code>n</code> elements of group <code>G</code> represented by parameter <code>elements</code>, and
<code>k[]</code> are <code>n</code> elements of the scalarfield <code>S</code> of group <code>G</code> represented by parameter <code>scalars</code>.

Abort with code <code>std::error::invalid_argument(E_NON_EQUAL_LENGTHS)</code> if the sizes of <code>elements</code> and <code>scalars</code> do not match.


<pre><code>public fun multi_scalar_mul&lt;G, S&gt;(elements: &amp;vector&lt;crypto_algebra::Element&lt;G&gt;&gt;, scalars: &amp;vector&lt;crypto_algebra::Element&lt;S&gt;&gt;): crypto_algebra::Element&lt;G&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multi_scalar_mul&lt;G, S&gt;(elements: &amp;vector&lt;Element&lt;G&gt;&gt;, scalars: &amp;vector&lt;Element&lt;S&gt;&gt;): Element&lt;G&gt; &#123;<br/>    let element_handles &#61; handles_from_elements(elements);<br/>    let scalar_handles &#61; handles_from_elements(scalars);<br/>    Element&lt;G&gt; &#123;<br/>        handle: multi_scalar_mul_internal&lt;G, S&gt;(element_handles, scalar_handles)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_scalar_mul"></a>

## Function `scalar_mul`

Compute <code>k&#42;P</code>, where <code>P</code> is an element of a group <code>G</code> and <code>k</code> is an element of the scalar field <code>S</code> associated to the group <code>G</code>.


<pre><code>public fun scalar_mul&lt;G, S&gt;(element_p: &amp;crypto_algebra::Element&lt;G&gt;, scalar_k: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;G&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun scalar_mul&lt;G, S&gt;(element_p: &amp;Element&lt;G&gt;, scalar_k: &amp;Element&lt;S&gt;): Element&lt;G&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;G&gt; &#123;<br/>        handle: scalar_mul_internal&lt;G, S&gt;(element_p.handle, scalar_k.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_multi_pairing"></a>

## Function `multi_pairing`

Efficiently compute <code>e(P[0],Q[0])&#43;...&#43;e(P[n&#45;1],Q[n&#45;1])</code>,
where <code>e: (G1,G2) &#45;&gt; (Gt)</code> is the pairing function from groups <code>(G1,G2)</code> to group <code>Gt</code>,
<code>P[]</code> are <code>n</code> elements of group <code>G1</code> represented by parameter <code>g1_elements</code>, and
<code>Q[]</code> are <code>n</code> elements of group <code>G2</code> represented by parameter <code>g2_elements</code>.

Abort with code <code>std::error::invalid_argument(E_NON_EQUAL_LENGTHS)</code> if the sizes of <code>g1_elements</code> and <code>g2_elements</code> do not match.

NOTE: we are viewing the target group <code>Gt</code> of the pairing as an additive group,
rather than a multiplicative one (which is typically the case).


<pre><code>public fun multi_pairing&lt;G1, G2, Gt&gt;(g1_elements: &amp;vector&lt;crypto_algebra::Element&lt;G1&gt;&gt;, g2_elements: &amp;vector&lt;crypto_algebra::Element&lt;G2&gt;&gt;): crypto_algebra::Element&lt;Gt&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multi_pairing&lt;G1,G2,Gt&gt;(g1_elements: &amp;vector&lt;Element&lt;G1&gt;&gt;, g2_elements: &amp;vector&lt;Element&lt;G2&gt;&gt;): Element&lt;Gt&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    let g1_handles &#61; handles_from_elements(g1_elements);<br/>    let g2_handles &#61; handles_from_elements(g2_elements);<br/>    Element&lt;Gt&gt; &#123;<br/>        handle: multi_pairing_internal&lt;G1,G2,Gt&gt;(g1_handles, g2_handles)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_pairing"></a>

## Function `pairing`

Compute the pairing function (a.k.a., bilinear map) on a <code>G1</code> element and a <code>G2</code> element.
Return an element in the target group <code>Gt</code>.


<pre><code>public fun pairing&lt;G1, G2, Gt&gt;(element_1: &amp;crypto_algebra::Element&lt;G1&gt;, element_2: &amp;crypto_algebra::Element&lt;G2&gt;): crypto_algebra::Element&lt;Gt&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pairing&lt;G1,G2,Gt&gt;(element_1: &amp;Element&lt;G1&gt;, element_2: &amp;Element&lt;G2&gt;): Element&lt;Gt&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;Gt&gt; &#123;<br/>        handle: pairing_internal&lt;G1,G2,Gt&gt;(element_1.handle, element_2.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_deserialize"></a>

## Function `deserialize`

Try deserializing a byte array to an element of an algebraic structure <code>S</code> using a given serialization format <code>F</code>.
Return none if the deserialization failed.


<pre><code>public fun deserialize&lt;S, F&gt;(bytes: &amp;vector&lt;u8&gt;): option::Option&lt;crypto_algebra::Element&lt;S&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deserialize&lt;S, F&gt;(bytes: &amp;vector&lt;u8&gt;): Option&lt;Element&lt;S&gt;&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    let (succeeded, handle) &#61; deserialize_internal&lt;S, F&gt;(bytes);<br/>    if (succeeded) &#123;<br/>        some(Element&lt;S&gt; &#123; handle &#125;)<br/>    &#125; else &#123;<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_serialize"></a>

## Function `serialize`

Serialize an element of an algebraic structure <code>S</code> to a byte array using a given serialization format <code>F</code>.


<pre><code>public fun serialize&lt;S, F&gt;(element: &amp;crypto_algebra::Element&lt;S&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun serialize&lt;S, F&gt;(element: &amp;Element&lt;S&gt;): vector&lt;u8&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    serialize_internal&lt;S, F&gt;(element.handle)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_order"></a>

## Function `order`

Get the order of structure <code>S</code>, a big integer little-endian encoded as a byte array.


<pre><code>public fun order&lt;S&gt;(): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun order&lt;S&gt;(): vector&lt;u8&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    order_internal&lt;S&gt;()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_upcast"></a>

## Function `upcast`

Cast an element of a structure <code>S</code> to a parent structure <code>L</code>.


<pre><code>public fun upcast&lt;S, L&gt;(element: &amp;crypto_algebra::Element&lt;S&gt;): crypto_algebra::Element&lt;L&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upcast&lt;S,L&gt;(element: &amp;Element&lt;S&gt;): Element&lt;L&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element&lt;L&gt; &#123;<br/>        handle: upcast_internal&lt;S,L&gt;(element.handle)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_downcast"></a>

## Function `downcast`

Try casting an element <code>x</code> of a structure <code>L</code> to a sub-structure <code>S</code>.
Return none if <code>x</code> is not a member of <code>S</code>.

NOTE: Membership check in <code>S</code> is performed inside, which can be expensive, depending on the structures <code>L</code> and <code>S</code>.


<pre><code>public fun downcast&lt;L, S&gt;(element_x: &amp;crypto_algebra::Element&lt;L&gt;): option::Option&lt;crypto_algebra::Element&lt;S&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun downcast&lt;L,S&gt;(element_x: &amp;Element&lt;L&gt;): Option&lt;Element&lt;S&gt;&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    let (succ, new_handle) &#61; downcast_internal&lt;L,S&gt;(element_x.handle);<br/>    if (succ) &#123;<br/>        some(Element&lt;S&gt; &#123; handle: new_handle &#125;)<br/>    &#125; else &#123;<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_hash_to"></a>

## Function `hash_to`

Hash an arbitrary-length byte array <code>msg</code> into structure <code>S</code> with a domain separation tag <code>dst</code>
using the given hash-to-structure suite <code>H</code>.

NOTE: some hashing methods do not accept a <code>dst</code> and will abort if a non-empty one is provided.


<pre><code>public fun hash_to&lt;S, H&gt;(dst: &amp;vector&lt;u8&gt;, msg: &amp;vector&lt;u8&gt;): crypto_algebra::Element&lt;S&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun hash_to&lt;S, H&gt;(dst: &amp;vector&lt;u8&gt;, msg: &amp;vector&lt;u8&gt;): Element&lt;S&gt; &#123;<br/>    abort_unless_cryptography_algebra_natives_enabled();<br/>    Element &#123;<br/>        handle: hash_to_internal&lt;S, H&gt;(dst, msg)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_abort_unless_cryptography_algebra_natives_enabled"></a>

## Function `abort_unless_cryptography_algebra_natives_enabled`



<pre><code>fun abort_unless_cryptography_algebra_natives_enabled()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun abort_unless_cryptography_algebra_natives_enabled() &#123;<br/>    if (features::cryptography_algebra_enabled()) return;<br/>    abort(std::error::not_implemented(0))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_handles_from_elements"></a>

## Function `handles_from_elements`



<pre><code>fun handles_from_elements&lt;S&gt;(elements: &amp;vector&lt;crypto_algebra::Element&lt;S&gt;&gt;): vector&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun handles_from_elements&lt;S&gt;(elements: &amp;vector&lt;Element&lt;S&gt;&gt;): vector&lt;u64&gt; &#123;<br/>    let num_elements &#61; std::vector::length(elements);<br/>    let element_handles &#61; std::vector::empty();<br/>    let i &#61; 0;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant len(element_handles) &#61;&#61; i;<br/>            invariant forall k in 0..i: element_handles[k] &#61;&#61; elements[k].handle;<br/>        &#125;;<br/>        i &lt; num_elements<br/>    &#125;) &#123;<br/>        std::vector::push_back(&amp;mut element_handles, std::vector::borrow(elements, i).handle);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    element_handles<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_add_internal"></a>

## Function `add_internal`



<pre><code>fun add_internal&lt;S&gt;(handle_1: u64, handle_2: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun add_internal&lt;S&gt;(handle_1: u64, handle_2: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code>fun deserialize_internal&lt;S, F&gt;(bytes: &amp;vector&lt;u8&gt;): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun deserialize_internal&lt;S, F&gt;(bytes: &amp;vector&lt;u8&gt;): (bool, u64);<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_div_internal"></a>

## Function `div_internal`



<pre><code>fun div_internal&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun div_internal&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64);<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_double_internal"></a>

## Function `double_internal`



<pre><code>fun double_internal&lt;G&gt;(element_handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun double_internal&lt;G&gt;(element_handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_downcast_internal"></a>

## Function `downcast_internal`



<pre><code>fun downcast_internal&lt;L, S&gt;(handle: u64): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun downcast_internal&lt;L,S&gt;(handle: u64): (bool, u64);<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_from_u64_internal"></a>

## Function `from_u64_internal`



<pre><code>fun from_u64_internal&lt;S&gt;(value: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun from_u64_internal&lt;S&gt;(value: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_eq_internal"></a>

## Function `eq_internal`



<pre><code>fun eq_internal&lt;S&gt;(handle_1: u64, handle_2: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun eq_internal&lt;S&gt;(handle_1: u64, handle_2: u64): bool;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_hash_to_internal"></a>

## Function `hash_to_internal`



<pre><code>fun hash_to_internal&lt;S, H&gt;(dst: &amp;vector&lt;u8&gt;, bytes: &amp;vector&lt;u8&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun hash_to_internal&lt;S, H&gt;(dst: &amp;vector&lt;u8&gt;, bytes: &amp;vector&lt;u8&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_inv_internal"></a>

## Function `inv_internal`



<pre><code>fun inv_internal&lt;F&gt;(handle: u64): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun inv_internal&lt;F&gt;(handle: u64): (bool, u64);<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_mul_internal"></a>

## Function `mul_internal`



<pre><code>fun mul_internal&lt;F&gt;(handle_1: u64, handle_2: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun mul_internal&lt;F&gt;(handle_1: u64, handle_2: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_multi_pairing_internal"></a>

## Function `multi_pairing_internal`



<pre><code>fun multi_pairing_internal&lt;G1, G2, Gt&gt;(g1_handles: vector&lt;u64&gt;, g2_handles: vector&lt;u64&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun multi_pairing_internal&lt;G1,G2,Gt&gt;(g1_handles: vector&lt;u64&gt;, g2_handles: vector&lt;u64&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_multi_scalar_mul_internal"></a>

## Function `multi_scalar_mul_internal`



<pre><code>fun multi_scalar_mul_internal&lt;G, S&gt;(element_handles: vector&lt;u64&gt;, scalar_handles: vector&lt;u64&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun multi_scalar_mul_internal&lt;G, S&gt;(element_handles: vector&lt;u64&gt;, scalar_handles: vector&lt;u64&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_neg_internal"></a>

## Function `neg_internal`



<pre><code>fun neg_internal&lt;F&gt;(handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun neg_internal&lt;F&gt;(handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_one_internal"></a>

## Function `one_internal`



<pre><code>fun one_internal&lt;S&gt;(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun one_internal&lt;S&gt;(): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_order_internal"></a>

## Function `order_internal`



<pre><code>fun order_internal&lt;G&gt;(): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun order_internal&lt;G&gt;(): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_pairing_internal"></a>

## Function `pairing_internal`



<pre><code>fun pairing_internal&lt;G1, G2, Gt&gt;(g1_handle: u64, g2_handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun pairing_internal&lt;G1,G2,Gt&gt;(g1_handle: u64, g2_handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code>fun scalar_mul_internal&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun scalar_mul_internal&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_serialize_internal"></a>

## Function `serialize_internal`



<pre><code>fun serialize_internal&lt;S, F&gt;(handle: u64): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun serialize_internal&lt;S, F&gt;(handle: u64): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_sqr_internal"></a>

## Function `sqr_internal`



<pre><code>fun sqr_internal&lt;G&gt;(handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun sqr_internal&lt;G&gt;(handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_sub_internal"></a>

## Function `sub_internal`



<pre><code>fun sub_internal&lt;G&gt;(handle_1: u64, handle_2: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun sub_internal&lt;G&gt;(handle_1: u64, handle_2: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_upcast_internal"></a>

## Function `upcast_internal`



<pre><code>fun upcast_internal&lt;S, L&gt;(handle: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun upcast_internal&lt;S,L&gt;(handle: u64): u64;<br/></code></pre>



</details>

<a id="0x1_crypto_algebra_zero_internal"></a>

## Function `zero_internal`



<pre><code>fun zero_internal&lt;S&gt;(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun zero_internal&lt;S&gt;(): u64;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_handles_from_elements"></a>

### Function `handles_from_elements`


<pre><code>fun handles_from_elements&lt;S&gt;(elements: &amp;vector&lt;crypto_algebra::Element&lt;S&gt;&gt;): vector&lt;u64&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures forall i in 0..len(elements): result[i] &#61;&#61; elements[i].handle;<br/></code></pre>



<a id="@Specification_1_add_internal"></a>

### Function `add_internal`


<pre><code>fun add_internal&lt;S&gt;(handle_1: u64, handle_2: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_deserialize_internal"></a>

### Function `deserialize_internal`


<pre><code>fun deserialize_internal&lt;S, F&gt;(bytes: &amp;vector&lt;u8&gt;): (bool, u64)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_div_internal"></a>

### Function `div_internal`


<pre><code>fun div_internal&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_double_internal"></a>

### Function `double_internal`


<pre><code>fun double_internal&lt;G&gt;(element_handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_downcast_internal"></a>

### Function `downcast_internal`


<pre><code>fun downcast_internal&lt;L, S&gt;(handle: u64): (bool, u64)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_from_u64_internal"></a>

### Function `from_u64_internal`


<pre><code>fun from_u64_internal&lt;S&gt;(value: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_eq_internal"></a>

### Function `eq_internal`


<pre><code>fun eq_internal&lt;S&gt;(handle_1: u64, handle_2: u64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_hash_to_internal"></a>

### Function `hash_to_internal`


<pre><code>fun hash_to_internal&lt;S, H&gt;(dst: &amp;vector&lt;u8&gt;, bytes: &amp;vector&lt;u8&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_inv_internal"></a>

### Function `inv_internal`


<pre><code>fun inv_internal&lt;F&gt;(handle: u64): (bool, u64)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_mul_internal"></a>

### Function `mul_internal`


<pre><code>fun mul_internal&lt;F&gt;(handle_1: u64, handle_2: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_multi_pairing_internal"></a>

### Function `multi_pairing_internal`


<pre><code>fun multi_pairing_internal&lt;G1, G2, Gt&gt;(g1_handles: vector&lt;u64&gt;, g2_handles: vector&lt;u64&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_multi_scalar_mul_internal"></a>

### Function `multi_scalar_mul_internal`


<pre><code>fun multi_scalar_mul_internal&lt;G, S&gt;(element_handles: vector&lt;u64&gt;, scalar_handles: vector&lt;u64&gt;): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_neg_internal"></a>

### Function `neg_internal`


<pre><code>fun neg_internal&lt;F&gt;(handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_one_internal"></a>

### Function `one_internal`


<pre><code>fun one_internal&lt;S&gt;(): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_order_internal"></a>

### Function `order_internal`


<pre><code>fun order_internal&lt;G&gt;(): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_pairing_internal"></a>

### Function `pairing_internal`


<pre><code>fun pairing_internal&lt;G1, G2, Gt&gt;(g1_handle: u64, g2_handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_scalar_mul_internal"></a>

### Function `scalar_mul_internal`


<pre><code>fun scalar_mul_internal&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_serialize_internal"></a>

### Function `serialize_internal`


<pre><code>fun serialize_internal&lt;S, F&gt;(handle: u64): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_sqr_internal"></a>

### Function `sqr_internal`


<pre><code>fun sqr_internal&lt;G&gt;(handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_sub_internal"></a>

### Function `sub_internal`


<pre><code>fun sub_internal&lt;G&gt;(handle_1: u64, handle_2: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_upcast_internal"></a>

### Function `upcast_internal`


<pre><code>fun upcast_internal&lt;S, L&gt;(handle: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_zero_internal"></a>

### Function `zero_internal`


<pre><code>fun zero_internal&lt;S&gt;(): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
