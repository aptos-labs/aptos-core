
<a name="0x1_algebra"></a>

# Module `0x1::algebra`



-  [Struct `BLS12_381_Fr`](#0x1_algebra_BLS12_381_Fr)
-  [Struct `BLS12_381_Fq`](#0x1_algebra_BLS12_381_Fq)
-  [Struct `BLS12_381_Fq2`](#0x1_algebra_BLS12_381_Fq2)
-  [Struct `BLS12_381_Fq6`](#0x1_algebra_BLS12_381_Fq6)
-  [Struct `BLS12_381_Fq12`](#0x1_algebra_BLS12_381_Fq12)
-  [Struct `BLS12_381_G1`](#0x1_algebra_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_algebra_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_algebra_BLS12_381_Gt)
-  [Struct `Element`](#0x1_algebra_Element)
-  [Function `deserialize_compressed_checked`](#0x1_algebra_deserialize_compressed_checked)
-  [Function `deserialize_compressed_unchecked`](#0x1_algebra_deserialize_compressed_unchecked)
-  [Function `deserialize_uncompressed_checked`](#0x1_algebra_deserialize_uncompressed_checked)
-  [Function `deserialize_uncompressed_unchecked`](#0x1_algebra_deserialize_uncompressed_unchecked)
-  [Function `serialize_compressed`](#0x1_algebra_serialize_compressed)
-  [Function `serialize_uncompressed`](#0x1_algebra_serialize_uncompressed)
-  [Function `hash_to`](#0x1_algebra_hash_to)
-  [Function `validate`](#0x1_algebra_validate)
-  [Function `field_add`](#0x1_algebra_field_add)
-  [Function `field_zero`](#0x1_algebra_field_zero)
-  [Function `field_div`](#0x1_algebra_field_div)
-  [Function `field_eq`](#0x1_algebra_field_eq)
-  [Function `field_inv`](#0x1_algebra_field_inv)
-  [Function `field_mul`](#0x1_algebra_field_mul)
-  [Function `field_one`](#0x1_algebra_field_one)
-  [Function `field_neg`](#0x1_algebra_field_neg)
-  [Function `field_sub`](#0x1_algebra_field_sub)
-  [Function `field_element_from_u64`](#0x1_algebra_field_element_from_u64)
-  [Function `scalar_from_field_element`](#0x1_algebra_scalar_from_field_element)
-  [Function `group_add`](#0x1_algebra_group_add)
-  [Function `group_eq`](#0x1_algebra_group_eq)
-  [Function `group_generator`](#0x1_algebra_group_generator)
-  [Function `group_identity`](#0x1_algebra_group_identity)
-  [Function `group_multi_scalar_mul`](#0x1_algebra_group_multi_scalar_mul)
-  [Function `group_neg`](#0x1_algebra_group_neg)
-  [Function `group_scalar_mul`](#0x1_algebra_group_scalar_mul)
-  [Function `pairing`](#0x1_algebra_pairing)
-  [Function `pairing_product`](#0x1_algebra_pairing_product)
-  [Function `deserialize_internal`](#0x1_algebra_deserialize_internal)
-  [Function `field_add_internal`](#0x1_algebra_field_add_internal)
-  [Function `field_div_internal`](#0x1_algebra_field_div_internal)
-  [Function `field_eq_internal`](#0x1_algebra_field_eq_internal)
-  [Function `field_inv_internal`](#0x1_algebra_field_inv_internal)
-  [Function `field_mul_internal`](#0x1_algebra_field_mul_internal)
-  [Function `field_neg_internal`](#0x1_algebra_field_neg_internal)
-  [Function `field_one_internal`](#0x1_algebra_field_one_internal)
-  [Function `field_sub_internal`](#0x1_algebra_field_sub_internal)
-  [Function `field_zero_internal`](#0x1_algebra_field_zero_internal)
-  [Function `field_element_from_u64_internal`](#0x1_algebra_field_element_from_u64_internal)
-  [Function `group_add_internal`](#0x1_algebra_group_add_internal)
-  [Function `group_eq_internal`](#0x1_algebra_group_eq_internal)
-  [Function `group_generator_internal`](#0x1_algebra_group_generator_internal)
-  [Function `group_identity_internal`](#0x1_algebra_group_identity_internal)
-  [Function `hash_to_internal`](#0x1_algebra_hash_to_internal)
-  [Function `serialize_internal`](#0x1_algebra_serialize_internal)
-  [Function `validate_internal`](#0x1_algebra_validate_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_algebra_BLS12_381_Fr"></a>

## Struct `BLS12_381_Fr`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a>
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

<a name="0x1_algebra_BLS12_381_Fq"></a>

## Struct `BLS12_381_Fq`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a>
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

<a name="0x1_algebra_BLS12_381_Fq2"></a>

## Struct `BLS12_381_Fq2`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a>
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

<a name="0x1_algebra_BLS12_381_Fq6"></a>

## Struct `BLS12_381_Fq6`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a>
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

<a name="0x1_algebra_BLS12_381_Fq12"></a>

## Struct `BLS12_381_Fq12`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a>
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

<a name="0x1_algebra_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a>
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

<a name="0x1_algebra_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a>
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

<a name="0x1_algebra_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a>
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

<a name="0x1_algebra_Element"></a>

## Struct `Element`



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

<a name="0x1_algebra_deserialize_compressed_checked"></a>

## Function `deserialize_compressed_checked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_checked">deserialize_compressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_checked">deserialize_compressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(*bytes, <b>true</b>, <b>true</b>);
    <b>if</b> (succeeded) {
        std::option::some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;{ handle })
    } <b>else</b> {
        std::option::none&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize_compressed_unchecked"></a>

## Function `deserialize_compressed_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_unchecked">deserialize_compressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_unchecked">deserialize_compressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(*bytes, <b>true</b>, <b>false</b>);
    <b>if</b> (succeeded) {
        std::option::some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;{ handle })
    } <b>else</b> {
        std::option::none&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize_uncompressed_checked"></a>

## Function `deserialize_uncompressed_checked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_checked">deserialize_uncompressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_checked">deserialize_uncompressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(*bytes, <b>false</b>, <b>true</b>);
    <b>if</b> (succeeded) {
        std::option::some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;{ handle })
    } <b>else</b> {
        std::option::none&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize_uncompressed_unchecked"></a>

## Function `deserialize_uncompressed_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_unchecked">deserialize_uncompressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_unchecked">deserialize_uncompressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(*bytes, <b>false</b>, <b>false</b>);
    <b>if</b> (succeeded) {
        std::option::some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;{ handle })
    } <b>else</b> {
        std::option::none&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_serialize_compressed"></a>

## Function `serialize_compressed`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_compressed">serialize_compressed</a>&lt;S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_compressed">serialize_compressed</a>&lt;S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(element.handle, <b>true</b>)
}
</code></pre>



</details>

<a name="0x1_algebra_serialize_uncompressed"></a>

## Function `serialize_uncompressed`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_uncompressed">serialize_uncompressed</a>&lt;S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_uncompressed">serialize_uncompressed</a>&lt;S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(element.handle, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_algebra_hash_to"></a>

## Function `hash_to`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt; {
        handle: <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;S&gt;(*bytes)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_validate"></a>

## Function `validate`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_validate">validate</a>&lt;S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_validate">validate</a>&lt;S&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;S&gt;): bool {
    <a href="algebra.md#0x1_algebra_validate_internal">validate_internal</a>&lt;S&gt;(element.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_field_add"></a>

## Function `field_add`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add">field_add</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add">field_add</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_add_internal">field_add_internal</a>&lt;F&gt;(element_0.handle, element_1.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_zero"></a>

## Function `field_zero`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_zero">field_zero</a>&lt;F&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_zero">field_zero</a>&lt;F&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_zero_internal">field_zero_internal</a>&lt;F&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_div"></a>

## Function `field_div`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;F&gt;(element_0.handle, element_1.handle);
    <b>if</b> (succeeded) {
        std::option::some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;{ handle })
    } <b>else</b> {
        std::option::none&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_eq"></a>

## Function `field_eq`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_eq">field_eq</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_eq">field_eq</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): bool {
    <a href="algebra.md#0x1_algebra_field_eq_internal">field_eq_internal</a>&lt;F&gt;(element_0.handle, element_1.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_field_inv"></a>

## Function `field_inv`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;F&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;F&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): Option&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;F&gt;(element.handle);
    <b>if</b> (succeeded) {
        std::option::some(<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;{ handle })
    } <b>else</b> {
        std::option::none&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_mul"></a>

## Function `field_mul`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul">field_mul</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul">field_mul</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;F&gt;(element_0.handle, element_1.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_one"></a>

## Function `field_one`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_one">field_one</a>&lt;F&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_one">field_one</a>&lt;F&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_one_internal">field_one_internal</a>&lt;F&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_neg"></a>

## Function `field_neg`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg">field_neg</a>&lt;F&gt;(element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg">field_neg</a>&lt;F&gt;(element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_neg_internal">field_neg_internal</a>&lt;F&gt;(element.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_sub"></a>

## Function `field_sub`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub">field_sub</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub">field_sub</a>&lt;F&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_sub_internal">field_sub_internal</a>&lt;F&gt;(element_0.handle, element_1.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_field_element_from_u64"></a>

## Function `field_element_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_element_from_u64">field_element_from_u64</a>&lt;F&gt;(val: u64): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_element_from_u64">field_element_from_u64</a>&lt;F&gt;(val: u64): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt; {
        handle: <a href="algebra.md#0x1_algebra_field_element_from_u64_internal">field_element_from_u64_internal</a>&lt;F&gt;(val)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_scalar_from_field_element"></a>

## Function `scalar_from_field_element`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_from_field_element">scalar_from_field_element</a>&lt;F&gt;(_element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;F&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_from_field_element">scalar_from_field_element</a>&lt;F&gt;(_element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;F&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    //TODO
    <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[3, 4, 5]
}
</code></pre>



</details>

<a name="0x1_algebra_group_add"></a>

## Function `group_add`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add">group_add</a>&lt;G&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add">group_add</a>&lt;G&gt;(element_0: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;G&gt;(element_0.handle, element_1.handle)
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_eq"></a>

## Function `group_eq`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_eq">group_eq</a>&lt;G&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_eq">group_eq</a>&lt;G&gt;(element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, element_2: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;): bool {
    <a href="algebra.md#0x1_algebra_group_eq_internal">group_eq_internal</a>&lt;G&gt;(element_1.handle, element_2.handle)
}
</code></pre>



</details>

<a name="0x1_algebra_group_generator"></a>

## Function `group_generator`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator">group_generator</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator">group_generator</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_identity"></a>

## Function `group_identity`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity">group_identity</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity">group_identity</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
        handle: <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul"></a>

## Function `group_multi_scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G&gt;(_element: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;&gt;, _scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G&gt;(_element: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;&gt;, _scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    //TODO
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; { handle: 0 }
}
</code></pre>



</details>

<a name="0x1_algebra_group_neg"></a>

## Function `group_neg`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg">group_neg</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg">group_neg</a>&lt;G&gt;(): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    //TODO
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; { handle: 0 }
}
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul"></a>

## Function `group_scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G&gt;(_element: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;, _scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G&gt;(_element: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt;, _scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; {
    //TODO
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G&gt; { handle: 0 }
}
</code></pre>



</details>

<a name="0x1_algebra_pairing"></a>

## Function `pairing`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1, G2, Gt&gt;(_element_1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, _element_2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1,G2,Gt&gt;(_element_1: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;, _element_2: &<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    //TODO
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; { handle: 0 }
}
</code></pre>



</details>

<a name="0x1_algebra_pairing_product"></a>

## Function `pairing_product`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product">pairing_product</a>&lt;G1, G2, Gt&gt;(_g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;, _g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product">pairing_product</a>&lt;G1,G2,Gt&gt;(_g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G1&gt;&gt;, _g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">Element</a>&lt;G2&gt;&gt;): <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; {
    //TODO
    <a href="algebra.md#0x1_algebra_Element">Element</a>&lt;Gt&gt; { handle: 0 }
}
</code></pre>



</details>

<a name="0x1_algebra_deserialize_internal"></a>

## Function `deserialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, compressed: bool, checked: bool): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_internal">deserialize_internal</a>&lt;S&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, compressed: bool, checked: bool): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_field_add_internal"></a>

## Function `field_add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_add_internal">field_add_internal</a>&lt;S&gt;(handle_0: u64, handle_1: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add_internal">field_add_internal</a>&lt;S&gt;(handle_0: u64, handle_1: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_div_internal"></a>

## Function `field_div_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div_internal">field_div_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_field_eq_internal"></a>

## Function `field_eq_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_eq_internal">field_eq_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_eq_internal">field_eq_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): bool;
</code></pre>



</details>

<a name="0x1_algebra_field_inv_internal"></a>

## Function `field_inv_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;F&gt;(handle: u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv_internal">field_inv_internal</a>&lt;F&gt;(handle: u64): (bool, u64);
</code></pre>



</details>

<a name="0x1_algebra_field_mul_internal"></a>

## Function `field_mul_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul_internal">field_mul_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_neg_internal"></a>

## Function `field_neg_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_neg_internal">field_neg_internal</a>&lt;F&gt;(handle: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg_internal">field_neg_internal</a>&lt;F&gt;(handle: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_one_internal"></a>

## Function `field_one_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_one_internal">field_one_internal</a>&lt;F&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_one_internal">field_one_internal</a>&lt;F&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_sub_internal"></a>

## Function `field_sub_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_sub_internal">field_sub_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub_internal">field_sub_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_zero_internal"></a>

## Function `field_zero_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_zero_internal">field_zero_internal</a>&lt;F&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_zero_internal">field_zero_internal</a>&lt;F&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_field_element_from_u64_internal"></a>

## Function `field_element_from_u64_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_field_element_from_u64_internal">field_element_from_u64_internal</a>&lt;F&gt;(val: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_element_from_u64_internal">field_element_from_u64_internal</a>&lt;F&gt;(val: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_add_internal"></a>

## Function `group_add_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;S&gt;(handle_0: u64, handle_1: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add_internal">group_add_internal</a>&lt;S&gt;(handle_0: u64, handle_1: u64): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_eq_internal"></a>

## Function `group_eq_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_eq_internal">group_eq_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_eq_internal">group_eq_internal</a>&lt;F&gt;(handle_0: u64, handle_1: u64): bool;
</code></pre>



</details>

<a name="0x1_algebra_group_generator_internal"></a>

## Function `group_generator_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;F&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator_internal">group_generator_internal</a>&lt;F&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_group_identity_internal"></a>

## Function `group_identity_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;F&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity_internal">group_identity_internal</a>&lt;F&gt;(): u64;
</code></pre>



</details>

<a name="0x1_algebra_hash_to_internal"></a>

## Function `hash_to_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;S&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to_internal">hash_to_internal</a>&lt;S&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<a name="0x1_algebra_serialize_internal"></a>

## Function `serialize_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(handle: u64, compressed: bool): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_internal">serialize_internal</a>&lt;S&gt;(handle: u64, compressed: bool): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_validate_internal"></a>

## Function `validate_internal`



<pre><code><b>fun</b> <a href="algebra.md#0x1_algebra_validate_internal">validate_internal</a>&lt;S&gt;(handle: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="algebra.md#0x1_algebra_validate_internal">validate_internal</a>&lt;S&gt;(handle: u64): bool;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
