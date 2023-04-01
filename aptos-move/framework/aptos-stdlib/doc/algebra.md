
<a name="0x1_algebra"></a>

# Module `0x1::algebra`

This module provides generic structs/functions for operations of algebraic structures (e.g. fields and groups),
which can be used to build generic cryptographic schemes atop.
E.g., a Groth16 ZK proof verifier can be built to work over any pairing supported in this module.

In general, every structure implements basic operations like (de)serialization, equality check, random sampling.

A group may also implement the following operations. (Additive group notation is assumed.)
- <code><a href="algebra.md#0x1_algebra_order">order</a>()</code> for getting the group order.
- <code><a href="algebra.md#0x1_algebra_zero">zero</a>()</code> for getting the group identity.
- <code><a href="algebra.md#0x1_algebra_one">one</a>()</code> for getting the group generator (if exists).
- <code><a href="algebra.md#0x1_algebra_neg">neg</a>()</code> for group element inversion.
- <code><a href="algebra.md#0x1_algebra_add">add</a>()</code> for group operation (i.e., a group addition).
- <code><a href="algebra.md#0x1_algebra_sub">sub</a>()</code> for group element subtraction.
- <code><a href="algebra.md#0x1_algebra_double">double</a>()</code> for efficient doubling.
- <code><a href="algebra.md#0x1_algebra_scalar_mul">scalar_mul</a>()</code> for group scalar multiplication.
- <code><a href="algebra.md#0x1_algebra_multi_scalar_mul">multi_scalar_mul</a>()</code> for efficient group multi-scalar multiplication.
- <code><a href="algebra.md#0x1_algebra_hash_to">hash_to</a>()</code> for hash-to-group.

A field may also implement the following operations.
- <code><a href="algebra.md#0x1_algebra_zero">zero</a>()</code> for getting the field additive identity.
- <code><a href="algebra.md#0x1_algebra_one">one</a>()</code> for getting the field multiplicative identity.
- <code><a href="algebra.md#0x1_algebra_add">add</a>()</code> for field addition.
- <code><a href="algebra.md#0x1_algebra_sub">sub</a>()</code> for field subtraction.
- <code><a href="algebra.md#0x1_algebra_mul">mul</a>()</code> for field multiplication.
- <code><a href="algebra.md#0x1_algebra_div">div</a>()</code> for field division.
- <code><a href="algebra.md#0x1_algebra_neg">neg</a>()</code> for field negation.
- <code><a href="algebra.md#0x1_algebra_inv">inv</a>()</code> for field inversion.
- <code><a href="algebra.md#0x1_algebra_sqr">sqr</a>()</code> for efficient field element squaring.
- <code><a href="algebra.md#0x1_algebra_from_u64">from_u64</a>()</code> for quick conversion from u64 to field element.

For 3 groups that admit a bilinear map, <code><a href="algebra.md#0x1_algebra_pairing">pairing</a>()</code> and <code><a href="algebra.md#0x1_algebra_multi_pairing">multi_pairing</a>()</code> may be implemented.

For a subset/superset relationship between 2 structures, <code><a href="algebra.md#0x1_algebra_upcast">upcast</a>()</code> and <code><a href="algebra.md#0x1_algebra_downcast">downcast</a>()</code> may be implemented.
E.g., in BLS12-381 pairing, since <code>Gt</code> is a subset of <code>Fq12</code>,
<code><a href="algebra.md#0x1_algebra_upcast">upcast</a>&lt;Gt, Fq12&gt;()</code> and <code><a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;Fq12, Gt&gt;()</code> will be supported.

See <code>algebra_*.<b>move</b></code> for currently implemented algebraic structures.


-  [Struct `Element`](#0x1_algebra_Element)
-  [Function `eq`](#0x1_algebra_eq)
-  [Function `from_u64`](#0x1_algebra_from_u64)
-  [Function `zero`](#0x1_algebra_zero)
-  [Function `one`](#0x1_algebra_one)
-  [Function `neg`](#0x1_algebra_neg)
-  [Function `add`](#0x1_algebra_add)
-  [Function `sub`](#0x1_algebra_sub)
-  [Function `mul`](#0x1_algebra_mul)
-  [Function `div`](#0x1_algebra_div)
-  [Function `sqr`](#0x1_algebra_sqr)
-  [Function `inv`](#0x1_algebra_inv)
-  [Function `double`](#0x1_algebra_double)
-  [Function `multi_scalar_mul`](#0x1_algebra_multi_scalar_mul)
-  [Function `scalar_mul`](#0x1_algebra_scalar_mul)
-  [Function `multi_pairing`](#0x1_algebra_multi_pairing)
-  [Function `pairing`](#0x1_algebra_pairing)
-  [Function `deserialize`](#0x1_algebra_deserialize)
-  [Function `serialize`](#0x1_algebra_serialize)
-  [Function `order`](#0x1_algebra_order)
-  [Function `upcast`](#0x1_algebra_upcast)
-  [Function `downcast`](#0x1_algebra_downcast)
-  [Function `hash_to`](#0x1_algebra_hash_to)
-  [Function `abort_unless_cryptography_algebra_natives_enabled`](#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled)
-  [Function `handles_from_elements`](#0x1_algebra_handles_from_elements)
-  [Function `add_internal`](#0x1_algebra_add_internal)
-  [Function `deserialize_internal`](#0x1_algebra_deserialize_internal)
-  [Function `div_internal`](#0x1_algebra_div_internal)
-  [Function `double_internal`](#0x1_algebra_double_internal)
-  [Function `downcast_internal`](#0x1_algebra_downcast_internal)
-  [Function `from_u64_internal`](#0x1_algebra_from_u64_internal)
-  [Function `eq_internal`](#0x1_algebra_eq_internal)
-  [Function `hash_to_internal`](#0x1_algebra_hash_to_internal)
-  [Function `inv_internal`](#0x1_algebra_inv_internal)
-  [Function `mul_internal`](#0x1_algebra_mul_internal)
-  [Function `multi_pairing_internal`](#0x1_algebra_multi_pairing_internal)
-  [Function `multi_scalar_mul_internal`](#0x1_algebra_multi_scalar_mul_internal)
-  [Function `neg_internal`](#0x1_algebra_neg_internal)
-  [Function `one_internal`](#0x1_algebra_one_internal)
-  [Function `order_internal`](#0x1_algebra_order_internal)
-  [Function `pairing_internal`](#0x1_algebra_pairing_internal)
-  [Function `scalar_mul_internal`](#0x1_algebra_scalar_mul_internal)
-  [Function `serialize_internal`](#0x1_algebra_serialize_internal)
-  [Function `sqr_internal`](#0x1_algebra_sqr_internal)
-  [Function `sub_internal`](#0x1_algebra_sub_internal)
-  [Function `upcast_internal`](#0x1_algebra_upcast_internal)
-  [Function `zero_internal`](#0x1_algebra_zero_internal)
-  [Specification](#@Specification_0)
    -  [Function `add_internal`](#@Specification_0_add_internal)
    -  [Function `deserialize_internal`](#@Specification_0_deserialize_internal)
    -  [Function `serialize_internal`](#@Specification_0_serialize_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_algebra_Element"></a>

## Struct `Element`

This struct represents an element of a structure <code>S</code>.


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

<a name="0x1_algebra_eq"></a>

## Function `eq`

Check if <code>x == y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_eq">eq</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_eq">eq</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): bool {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_eq_internal">eq_internal</a>&lt;S&gt;(x.handle, y.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_from_u64"></a>

## Function `from_u64`

Convert a u64 to an element of a structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64">from_u64</a>&lt;S&gt;(value: u64): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64">from_u64</a>&lt;S&gt;(value: u64): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_zero"></a>

## Function `zero`

Return the additive identity of field <code>S</code>, or the identity of group <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_zero">zero</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_zero">zero</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_zero_internal">zero_internal</a>&lt;S&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_one"></a>

## Function `one`

Return the multiplicative identity of field <code>S</code>, or a fixed generator of group <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_one">one</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_one">one</a>&lt;S&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_one_internal">one_internal</a>&lt;S&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_neg"></a>

## Function `neg`

Compute <code>-x</code> for an element <code>x</code> of a structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_neg">neg</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_neg">neg</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_neg_internal">neg_internal</a>&lt;S&gt;(x.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_add"></a>

## Function `add`

Compute <code>x + y</code> for elements <code>x</code> and <code>y</code> of structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_add">add</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_add">add</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_sub"></a>

## Function `sub`

Compute <code>x - y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_sub">sub</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_sub">sub</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_sub_internal">sub_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_mul"></a>

## Function `mul`

Compute <code>x * y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_mul">mul</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_mul">mul</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_mul_internal">mul_internal</a>&lt;S&gt;(x.handle, y.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_div"></a>

## Function `div`

Try computing <code>x / y</code> for elements <code>x</code> and <code>y</code> of a structure <code>S</code>.
Return none if y equals to <code><a href="algebra.md#0x1_algebra_zero">zero</a>&lt;S&gt;()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_div">div</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_div">div</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;, y: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <b>let</b> (succ, handle) = <a href="algebra.md#0x1_algebra_div_internal">div_internal</a>&lt;S&gt;(x.handle, y.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_sqr"></a>

## Function `sqr`

Compute <code>x^2</code> for an element <code>x</code> of a structure <code>S</code>. Faster and cheaper than <code><a href="algebra.md#0x1_algebra_mul">mul</a>(x, x)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_sqr">sqr</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_sqr">sqr</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_sqr_internal">sqr_internal</a>&lt;S&gt;(x.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_inv"></a>

## Function `inv`

Try computing <code>x^(-1)</code> for an element <code>x</code> of a structure <code>S</code>.
Return none if <code>x</code> equals to <code><a href="algebra.md#0x1_algebra_zero">zero</a>&lt;S&gt;()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_inv">inv</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_inv">inv</a>&lt;S&gt;(x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_inv_internal">inv_internal</a>&lt;S&gt;(x.handle);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle };
        some(scalar)
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_double"></a>

## Function `double`

Compute <code>2*P</code> for an element <code>P</code> of a structure <code>S</code>. Faster and cheaper than <code>P + P</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_double">double</a>&lt;S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_double">double</a>&lt;S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_double_internal">double_internal</a>&lt;S&gt;(element_p.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_multi_scalar_mul"></a>

## Function `multi_scalar_mul`

Compute <code>k[0]*P[0]+...+k[n-1]*P[n-1]</code>, where
<code>P[]</code> are <code>n</code> elements of group <code>G</code> represented by parameter <code>elements</code>, and
<code>k[]</code> are <code>n</code> elements of the scalarfield <code>S</code> of group <code>G</code> represented by parameter <code>scalars</code>.

Abort with code 0x010000 if the sizes of <code>elements</code> and <code>scalars</code> do not match.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_multi_scalar_mul">multi_scalar_mul</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_multi_scalar_mul">multi_scalar_mul</a>&lt;G, S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;&gt;, scalars: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <b>let</b> element_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(elements);
    <b>let</b> scalar_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(scalars);
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles, scalar_handles)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_scalar_mul"></a>

## Function `scalar_mul`

Compute <code>k*P</code>, where <code>P</code> is an element of a group <code>G</code> and <code>k</code> is an element of the scalar field <code>S</code> of a structure <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_mul">scalar_mul</a>&lt;G, S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, scalar_k: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_mul">scalar_mul</a>&lt;G, S&gt;(element_p: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, scalar_k: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_scalar_mul_internal">scalar_mul_internal</a>&lt;G, S&gt;(element_p.handle, scalar_k.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_multi_pairing"></a>

## Function `multi_pairing`

Efficiently compute <code>e(P[0],Q[0])+...+e(P[n-1],Q[n-1])</code>,
where <code>e: (G1,G2) -&gt; (Gt)</code> is a pre-compiled pairing function from groups <code>(G1,G2)</code> to group <code>Gt</code>,
<code>P[]</code> are <code>n</code> elements of group <code>G1</code> represented by parameter <code>g1_elements</code>, and
<code>Q[]</code> are <code>n</code> elements of group <code>G2</code> represented by parameter <code>g2_elements</code>.

Abort with code 0x010000 if the sizes of <code>g1_elements</code> and <code>g2_elements</code> do not match.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_multi_pairing">multi_pairing</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_multi_pairing">multi_pairing</a>&lt;G1,G2,Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <b>let</b> g1_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(g1_elements);
    <b>let</b> g2_handles = <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>(g2_elements);
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="algebra.md#0x1_algebra_multi_pairing_internal">multi_pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handles, g2_handles)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_pairing"></a>

## Function `pairing`

Compute a pre-compiled pairing function (a.k.a., bilinear map) on <code>element_1</code> and <code>element_2</code>.
Return an element in the target group <code>Gt</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1, G2, Gt&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1,G2,Gt&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="algebra.md#0x1_algebra_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(element_1.handle, element_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize"></a>

## Function `deserialize`

Try deserializing a byte array to an element of an algebraic structure <code>S</code> using a given serialization format <code>F</code>.
Return none if the deserialization failed.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize">deserialize</a>&lt;S, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S, F&gt;(bytes);
    <b>if</b> (succeeded) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_serialize"></a>

## Function `serialize`

Serialize an element of an algebraic structure <code>S</code> to a byte array using a given serialization format <code>F</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize">serialize</a>&lt;S, F&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize">serialize</a>&lt;S, F&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S, F&gt;(element.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_order"></a>

## Function `order`

Get the order of group <code>G</code>, a big integer little-endian encoded as a byte array.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_order">order</a>&lt;S&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_order">order</a>&lt;S&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_order_internal">order_internal</a>&lt;S&gt;()
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
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt; {
        handle: <a href="algebra.md#0x1_algebra_upcast_internal">upcast_internal</a>&lt;S,L&gt;(element.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_downcast"></a>

## Function `downcast`

Try casting an element <code>x</code> of a structure <code>L</code> to a sub-structure <code>S</code>.
Return none if <code>x</code> is not a member of <code>S</code>.

NOTE: Membership check is performed inside, which can be expensive, depending on the structures <code>L</code> and <code>S</code>.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L, S&gt;(element_x: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;L&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_downcast">downcast</a>&lt;L,S&gt;(element_x: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;L&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <b>let</b> (succ, new_handle) = <a href="algebra.md#0x1_algebra_downcast_internal">downcast_internal</a>&lt;L,S&gt;(element_x.handle);
    <b>if</b> (succ) {
        some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; { handle: new_handle })
    } <b>else</b> {
        none()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_hash_to"></a>

## Function `hash_to`

Hash an arbitrary-length byte array <code>msg</code> into structure <code>S</code> using the given <code>suite</code>.
A unique domain separation tag <code>dst</code> of size 255 bytes or shorter is required
for each independent collision-resistent mapping involved in the protocol built atop.
Abort if <code>dst</code> is too long.


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;St, Su&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;St&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;St, Su&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, msg: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;St&gt; {
    <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>();
    <a href="algebra.md#0x1_algebra_Element">Element</a> {
        handle: <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;St, Su&gt;(dst, msg)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_cryptography_algebra_natives_enabled"></a>

## Function `abort_unless_cryptography_algebra_natives_enabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>() {
    <b>if</b> (<a href="../../move-stdlib/doc/features.md#0x1_features_cryptography_algebra_enabled">features::cryptography_algebra_enabled</a>()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="0x1_algebra_handles_from_elements"></a>

## Function `handles_from_elements`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>&lt;S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_handles_from_elements">handles_from_elements</a>&lt;S&gt;(elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>let</b> num_elements = std::vector::length(elements);
    <b>let</b> element_handles = std::vector::empty();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_elements) {
        std::vector::push_back(&<b>mut</b> element_handles, std::vector::borrow(elements, i).handle);
        i = i + 1;
    };
    element_handles
}
</code></pre>



</details>

<a name="0x1_algebra_add_internal"></a>

## Function `add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;S&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;S&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_div_internal"></a>

## Function `div_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_div_internal">div_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_div_internal">div_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_double_internal"></a>

## Function `double_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_double_internal">double_internal</a>&lt;G&gt;(element_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_double_internal">double_internal</a>&lt;G&gt;(element_handle: u64): u64;
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

<a name="0x1_algebra_from_u64_internal"></a>

## Function `from_u64_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_from_u64_internal">from_u64_internal</a>&lt;S&gt;(value: u64): u64;
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

<a name="0x1_algebra_hash_to_internal"></a>

## Function `hash_to_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;St, Su&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;St, Su&gt;(dst: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_inv_internal"></a>

## Function `inv_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_inv_internal">inv_internal</a>&lt;F&gt;(handle: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_inv_internal">inv_internal</a>&lt;F&gt;(handle: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_mul_internal"></a>

## Function `mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_mul_internal">mul_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_mul_internal">mul_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_multi_pairing_internal"></a>

## Function `multi_pairing_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_multi_pairing_internal">multi_pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_multi_pairing_internal">multi_pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_multi_scalar_mul_internal"></a>

## Function `multi_scalar_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_multi_scalar_mul_internal">multi_scalar_mul_internal</a>&lt;G, S&gt;(element_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_neg_internal"></a>

## Function `neg_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_neg_internal">neg_internal</a>&lt;F&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_neg_internal">neg_internal</a>&lt;F&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_one_internal"></a>

## Function `one_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_one_internal">one_internal</a>&lt;S&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_one_internal">one_internal</a>&lt;S&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_order_internal"></a>

## Function `order_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_order_internal">order_internal</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_order_internal">order_internal</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_pairing_internal"></a>

## Function `pairing_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_pairing_internal">pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handle: u64, g2_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handle: u64, g2_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_scalar_mul_internal">scalar_mul_internal</a>&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_mul_internal">scalar_mul_internal</a>&lt;G, S&gt;(element_handle: u64, scalar_handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_serialize_internal"></a>

## Function `serialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S, F&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S, F&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_sqr_internal"></a>

## Function `sqr_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_sqr_internal">sqr_internal</a>&lt;G&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_sqr_internal">sqr_internal</a>&lt;G&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_sub_internal"></a>

## Function `sub_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_sub_internal">sub_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_sub_internal">sub_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): u64;
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

<a name="0x1_algebra_zero_internal"></a>

## Function `zero_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_zero_internal">zero_internal</a>&lt;S&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_zero_internal">zero_internal</a>&lt;S&gt;(): u64;
</code></pre>



</details>

<a name="@Specification_0"></a>

## Specification


<a name="@Specification_0_add_internal"></a>

### Function `add_internal`


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;S&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_0_deserialize_internal"></a>

### Function `deserialize_internal`


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_0_serialize_internal"></a>

### Function `serialize_internal`


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S, F&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
