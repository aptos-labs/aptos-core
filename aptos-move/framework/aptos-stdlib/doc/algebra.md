
<a name="0x1_algebra"></a>

# Module `0x1::algebra`

This module provides generic structs/functions for operations of algebraic structures (e.g. fields and groups),
which can be used to build generic cryptographic schemes atop.
See <code>algebra_*.<b>move</b></code> for currently implemented algebraic structures.

Currently supported operations include element serialization/deserialization and addition.


-  [Struct `Element`](#0x1_algebra_Element)
-  [Function `add`](#0x1_algebra_add)
-  [Function `deserialize`](#0x1_algebra_deserialize)
-  [Function `serialize`](#0x1_algebra_serialize)
-  [Function `deserialize_internal`](#0x1_algebra_deserialize_internal)
-  [Function `add_internal`](#0x1_algebra_add_internal)
-  [Function `serialize_internal`](#0x1_algebra_serialize_internal)
-  [Function `abort_unless_cryptography_algebra_natives_enabled`](#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled)
-  [Specification](#@Specification_0)
    -  [Function `deserialize_internal`](#@Specification_0_deserialize_internal)
    -  [Function `add_internal`](#@Specification_0_add_internal)
    -  [Function `serialize_internal`](#@Specification_0_serialize_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_algebra_Element"></a>

## Struct `Element`

This struct represents an element of an algebraic structure <code>S</code>.


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

<a name="0x1_algebra_add"></a>

## Function `add`

Compute <code>x + y</code> for elements <code>x</code> and <code>y</code> of an algebraic structure <code>S</code>.


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

<a name="0x1_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_add_internal"></a>

## Function `add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_serialize_internal"></a>

## Function `serialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;G, F&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;G, F&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_abort_unless_cryptography_algebra_natives_enabled"></a>

## Function `abort_unless_cryptography_algebra_natives_enabled`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_abort_unless_cryptography_algebra_natives_enabled">abort_unless_cryptography_algebra_natives_enabled</a>() {
    <b>if</b> (cryptography_algebra_enabled()) <b>return</b>;
    <b>abort</b>(std::error::not_implemented(0))
}
</code></pre>



</details>

<a name="@Specification_0"></a>

## Specification


<a name="@Specification_0_deserialize_internal"></a>

### Function `deserialize_internal`


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;G, F&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u64)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_0_add_internal"></a>

### Function `add_internal`


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_add_internal">add_internal</a>&lt;F&gt;(handle_1: u64, handle_2: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_0_serialize_internal"></a>

### Function `serialize_internal`


<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;G, F&gt;(handle: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
