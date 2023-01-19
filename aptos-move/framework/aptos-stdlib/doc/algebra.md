
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
-  [Function `deserialize_compressed_checked`](#0x1_algebra_deserialize_compressed_checked)
-  [Function `deserialize_compressed_unchecked`](#0x1_algebra_deserialize_compressed_unchecked)
-  [Function `deserialize_uncompressed_checked`](#0x1_algebra_deserialize_uncompressed_checked)
-  [Function `deserialize_uncompressed_unchecked`](#0x1_algebra_deserialize_uncompressed_unchecked)
-  [Function `serialize_compressed`](#0x1_algebra_serialize_compressed)
-  [Function `serialize_uncompressed`](#0x1_algebra_serialize_uncompressed)
-  [Function `hash_to`](#0x1_algebra_hash_to)
-  [Function `validate`](#0x1_algebra_validate)
-  [Function `field_add`](#0x1_algebra_field_add)
-  [Function `field_add_identity`](#0x1_algebra_field_add_identity)
-  [Function `field_div`](#0x1_algebra_field_div)
-  [Function `field_eq`](#0x1_algebra_field_eq)
-  [Function `field_inv`](#0x1_algebra_field_inv)
-  [Function `field_mul`](#0x1_algebra_field_mul)
-  [Function `field_mul_identity`](#0x1_algebra_field_mul_identity)
-  [Function `field_neg`](#0x1_algebra_field_neg)
-  [Function `field_sub`](#0x1_algebra_field_sub)
-  [Function `field_element_from_u64`](#0x1_algebra_field_element_from_u64)
-  [Function `scalar_from_field_element`](#0x1_algebra_scalar_from_field_element)
-  [Function `group_add`](#0x1_algebra_group_add)
-  [Function `group_equal`](#0x1_algebra_group_equal)
-  [Function `group_generator`](#0x1_algebra_group_generator)
-  [Function `group_identity`](#0x1_algebra_group_identity)
-  [Function `group_multi_scalar_mul`](#0x1_algebra_group_multi_scalar_mul)
-  [Function `group_neg`](#0x1_algebra_group_neg)
-  [Function `group_scalar_mul`](#0x1_algebra_group_scalar_mul)
-  [Function `pairing`](#0x1_algebra_pairing)
-  [Function `pairing_product`](#0x1_algebra_pairing_product)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_algebra_BLS12_381_Fr"></a>

## Struct `BLS12_381_Fr`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fr">BLS12_381_Fr</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_Fq"></a>

## Struct `BLS12_381_Fq`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq">BLS12_381_Fq</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_Fq2"></a>

## Struct `BLS12_381_Fq2`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq2">BLS12_381_Fq2</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_Fq6"></a>

## Struct `BLS12_381_Fq6`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq6">BLS12_381_Fq6</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_Fq12"></a>

## Struct `BLS12_381_Fq12`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Fq12">BLS12_381_Fq12</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G1">BLS12_381_G1</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_G2">BLS12_381_G2</a> <b>has</b> <b>copy</b>, drop
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

<a name="0x1_algebra_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`



<pre><code><b>struct</b> <a href="algebra.md#0x1_algebra_BLS12_381_Gt">BLS12_381_Gt</a> <b>has</b> <b>copy</b>, drop
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



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_checked">deserialize_compressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_checked">deserialize_compressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;S&gt;;
</code></pre>



</details>

<a name="0x1_algebra_deserialize_compressed_unchecked"></a>

## Function `deserialize_compressed_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_unchecked">deserialize_compressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_compressed_unchecked">deserialize_compressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;S&gt;;
</code></pre>



</details>

<a name="0x1_algebra_deserialize_uncompressed_checked"></a>

## Function `deserialize_uncompressed_checked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_checked">deserialize_uncompressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_checked">deserialize_uncompressed_checked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;S&gt;;
</code></pre>



</details>

<a name="0x1_algebra_deserialize_uncompressed_unchecked"></a>

## Function `deserialize_uncompressed_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_unchecked">deserialize_uncompressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;S&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_deserialize_uncompressed_unchecked">deserialize_uncompressed_unchecked</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;S&gt;;
</code></pre>



</details>

<a name="0x1_algebra_serialize_compressed"></a>

## Function `serialize_compressed`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_compressed">serialize_compressed</a>&lt;S&gt;(element: &S): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_compressed">serialize_compressed</a>&lt;S&gt;(element: &S): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_serialize_uncompressed"></a>

## Function `serialize_uncompressed`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_uncompressed">serialize_uncompressed</a>&lt;S&gt;(element: &S): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_serialize_uncompressed">serialize_uncompressed</a>&lt;S&gt;(element: &S): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_hash_to"></a>

## Function `hash_to`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): S
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_hash_to">hash_to</a>&lt;S&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): S;
</code></pre>



</details>

<a name="0x1_algebra_validate"></a>

## Function `validate`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_validate">validate</a>&lt;S&gt;(element: &S): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_validate">validate</a>&lt;S&gt;(element: &S): bool;
</code></pre>



</details>

<a name="0x1_algebra_field_add"></a>

## Function `field_add`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add">field_add</a>&lt;F&gt;(element_0: &F, element_1: &F): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add">field_add</a>&lt;F&gt;(element_0: &F, element_1: &F): F;
</code></pre>



</details>

<a name="0x1_algebra_field_add_identity"></a>

## Function `field_add_identity`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add_identity">field_add_identity</a>&lt;F&gt;(): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_add_identity">field_add_identity</a>&lt;F&gt;(): F;
</code></pre>



</details>

<a name="0x1_algebra_field_div"></a>

## Function `field_div`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;F&gt;(element_0: &F, element_1: &F): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_div">field_div</a>&lt;F&gt;(element_0: &F, element_1: &F): F;
</code></pre>



</details>

<a name="0x1_algebra_field_eq"></a>

## Function `field_eq`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_eq">field_eq</a>&lt;F&gt;(element_0: &F, element_1: &F): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_eq">field_eq</a>&lt;F&gt;(element_0: &F, element_1: &F): bool;
</code></pre>



</details>

<a name="0x1_algebra_field_inv"></a>

## Function `field_inv`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;F&gt;(element: &F): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;F&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_inv">field_inv</a>&lt;F&gt;(element: &F): Option&lt;F&gt;;
</code></pre>



</details>

<a name="0x1_algebra_field_mul"></a>

## Function `field_mul`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul">field_mul</a>&lt;F&gt;(element_0: &F, element_1: &F): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul">field_mul</a>&lt;F&gt;(element_0: &F, element_1: &F): F;
</code></pre>



</details>

<a name="0x1_algebra_field_mul_identity"></a>

## Function `field_mul_identity`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul_identity">field_mul_identity</a>&lt;F&gt;(): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_mul_identity">field_mul_identity</a>&lt;F&gt;(): F;
</code></pre>



</details>

<a name="0x1_algebra_field_neg"></a>

## Function `field_neg`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg">field_neg</a>&lt;F&gt;(element: &F): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_neg">field_neg</a>&lt;F&gt;(element: &F): F;
</code></pre>



</details>

<a name="0x1_algebra_field_sub"></a>

## Function `field_sub`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub">field_sub</a>&lt;F&gt;(element_0: &F, element_1: &F): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_sub">field_sub</a>&lt;F&gt;(element_0: &F, element_1: &F): F;
</code></pre>



</details>

<a name="0x1_algebra_field_element_from_u64"></a>

## Function `field_element_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_element_from_u64">field_element_from_u64</a>&lt;F&gt;(val: u64): F
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_field_element_from_u64">field_element_from_u64</a>&lt;F&gt;(val: u64): F;
</code></pre>



</details>

<a name="0x1_algebra_scalar_from_field_element"></a>

## Function `scalar_from_field_element`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_from_field_element">scalar_from_field_element</a>&lt;F&gt;(e: &F): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_scalar_from_field_element">scalar_from_field_element</a>&lt;F&gt;(e: &F): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_algebra_group_add"></a>

## Function `group_add`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add">group_add</a>&lt;G&gt;(element_1: &G, element_2: &G): G
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_add">group_add</a>&lt;G&gt;(element_1: &G, element_2: &G): G;
</code></pre>



</details>

<a name="0x1_algebra_group_equal"></a>

## Function `group_equal`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_equal">group_equal</a>&lt;G&gt;(element_1: &G, element_2: &G): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_equal">group_equal</a>&lt;G&gt;(element_1: &G, element_2: &G): bool;
</code></pre>



</details>

<a name="0x1_algebra_group_generator"></a>

## Function `group_generator`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator">group_generator</a>&lt;G&gt;(): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_generator">group_generator</a>&lt;G&gt;(): Option&lt;G&gt;;
</code></pre>



</details>

<a name="0x1_algebra_group_identity"></a>

## Function `group_identity`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity">group_identity</a>&lt;G&gt;(): G
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_identity">group_identity</a>&lt;G&gt;(): G;
</code></pre>



</details>

<a name="0x1_algebra_group_multi_scalar_mul"></a>

## Function `group_multi_scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G&gt;(element: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;G&gt;, scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): G
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_multi_scalar_mul">group_multi_scalar_mul</a>&lt;G&gt;(element: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;G&gt;, scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): G;
</code></pre>



</details>

<a name="0x1_algebra_group_neg"></a>

## Function `group_neg`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg">group_neg</a>&lt;G&gt;(): G
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_neg">group_neg</a>&lt;G&gt;(): G;
</code></pre>



</details>

<a name="0x1_algebra_group_scalar_mul"></a>

## Function `group_scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G&gt;(element: &G, scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): G
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_group_scalar_mul">group_scalar_mul</a>&lt;G&gt;(element: &G, scalar: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): G;
</code></pre>



</details>

<a name="0x1_algebra_pairing"></a>

## Function `pairing`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1, G2, Gt&gt;(element_1: &G1, element_2: &G2): Gt
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing">pairing</a>&lt;G1,G2,Gt&gt;(element_1: &G1, element_2: &G2): Gt;
</code></pre>



</details>

<a name="0x1_algebra_pairing_product"></a>

## Function `pairing_product`



<pre><code><b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product">pairing_product</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;G1&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;G2&gt;): Gt
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="algebra.md#0x1_algebra_pairing_product">pairing_product</a>&lt;G1,G2,Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;G1&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;G2&gt;): Gt;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
