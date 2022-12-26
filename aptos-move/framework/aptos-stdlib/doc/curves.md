
<a name="0x1_curves"></a>

# Module `0x1::curves`



-  [Struct `BLS12_381_G1`](#0x1_curves_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_curves_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_curves_BLS12_381_Gt)
-  [Struct `Scalar`](#0x1_curves_Scalar)
-  [Struct `Point`](#0x1_curves_Point)
-  [Function `pairing`](#0x1_curves_pairing)
-  [Function `multi_pairing`](#0x1_curves_multi_pairing)
-  [Function `scalar_from_u64`](#0x1_curves_scalar_from_u64)
-  [Function `scalar_neg`](#0x1_curves_scalar_neg)
-  [Function `scalar_add`](#0x1_curves_scalar_add)
-  [Function `scalar_mul`](#0x1_curves_scalar_mul)
-  [Function `scalar_inv`](#0x1_curves_scalar_inv)
-  [Function `scalar_eq`](#0x1_curves_scalar_eq)
-  [Function `scalar_from_bytes`](#0x1_curves_scalar_from_bytes)
-  [Function `point_identity`](#0x1_curves_point_identity)
-  [Function `point_generator`](#0x1_curves_point_generator)
-  [Function `point_add`](#0x1_curves_point_add)
-  [Function `point_mul`](#0x1_curves_point_mul)
-  [Function `scalar_to_bytes`](#0x1_curves_scalar_to_bytes)
-  [Function `point_to_bytes`](#0x1_curves_point_to_bytes)
-  [Function `element_from_bytes`](#0x1_curves_element_from_bytes)
-  [Function `point_eq`](#0x1_curves_point_eq)
-  [Function `element_from_bytes_internal`](#0x1_curves_element_from_bytes_internal)
-  [Function `scalar_from_u64_internal`](#0x1_curves_scalar_from_u64_internal)
-  [Function `scalar_from_bytes_internal`](#0x1_curves_scalar_from_bytes_internal)
-  [Function `scalar_neg_internal`](#0x1_curves_scalar_neg_internal)
-  [Function `scalar_add_internal`](#0x1_curves_scalar_add_internal)
-  [Function `scalar_mul_internal`](#0x1_curves_scalar_mul_internal)
-  [Function `scalar_inv_internal`](#0x1_curves_scalar_inv_internal)
-  [Function `scalar_eq_internal`](#0x1_curves_scalar_eq_internal)
-  [Function `scalar_to_bytes_internal`](#0x1_curves_scalar_to_bytes_internal)
-  [Function `pairing_internal`](#0x1_curves_pairing_internal)
-  [Function `multi_pairing_internal`](#0x1_curves_multi_pairing_internal)
-  [Function `point_add_internal`](#0x1_curves_point_add_internal)
-  [Function `point_eq_internal`](#0x1_curves_point_eq_internal)
-  [Function `point_identity_internal`](#0x1_curves_point_identity_internal)
-  [Function `point_generator_internal`](#0x1_curves_point_generator_internal)
-  [Function `point_mul_internal`](#0x1_curves_point_mul_internal)
-  [Function `point_to_bytes_internal`](#0x1_curves_point_to_bytes_internal)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_curves_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>
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

<a name="0x1_curves_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>
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

<a name="0x1_curves_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>
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

<a name="0x1_curves_Scalar"></a>

## Struct `Scalar`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;Group&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_curves_Point"></a>

## Struct `Point`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Point">Point</a>&lt;Group&gt; <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_curves_pairing"></a>

## Function `pairing`

Perform a bilinear mapping.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1, G2, Gt&gt;(point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1,G2,Gt&gt;(point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G1&gt;, point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
        handle: <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(point_1.handle, point_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_multi_pairing"></a>

## Function `multi_pairing`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_multi_pairing">multi_pairing</a>&lt;G1, G2, Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_multi_pairing">multi_pairing</a>&lt;G1,G2,Gt&gt;(g1_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">Point</a>&lt;G1&gt;&gt;, g2_elements: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="curves.md#0x1_curves_Point">Point</a>&lt;G2&gt;&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
    <b>let</b> num_g1 = std::vector::length(g1_elements);
    <b>let</b> num_g2 = std::vector::length(g2_elements);
    <b>assert</b>!(num_g1 == num_g2, 1);
    <b>let</b> g1_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> g2_handles = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_g2) {
        std::vector::push_back(&<b>mut</b> g1_handles, std::vector::borrow(g1_elements, i).handle);
        std::vector::push_back(&<b>mut</b> g2_handles, std::vector::borrow(g2_elements, i).handle);
        i = i + 1;
    };

    <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
        handle: <a href="curves.md#0x1_curves_multi_pairing_internal">multi_pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handles, g2_handles)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_from_u64"></a>

## Function `scalar_from_u64`

Scalar basics.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64">scalar_from_u64</a>&lt;G&gt;(value: u64): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64">scalar_from_u64</a>&lt;G&gt;(value: u64): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_neg"></a>

## Function `scalar_neg`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;G&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;G&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(_scalar_1.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_add"></a>

## Function `scalar_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add">scalar_add</a>&lt;G&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add">scalar_add</a>&lt;G&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(_scalar_1.handle, _scalar_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_mul"></a>

## Function `scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul">scalar_mul</a>&lt;G&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul">scalar_mul</a>&lt;G&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(_scalar_1.handle, _scalar_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_inv"></a>

## Function `scalar_inv`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): Option&lt;<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(scalar.handle);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; { handle };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_eq"></a>

## Function `scalar_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;G&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): bool {
    <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(scalar_1.handle, scalar_2.handle)
}
</code></pre>



</details>

<a name="0x1_curves_scalar_from_bytes"></a>

## Function `scalar_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;G&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;G&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;&gt; {
    <b>let</b> (succeeded, handle) = <a href="curves.md#0x1_curves_scalar_from_bytes_internal">scalar_from_bytes_internal</a>&lt;G&gt;(*bytes);
    <b>if</b> (succeeded) {
        <b>let</b> scalar = <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
            handle
        };
        std::option::some(scalar)
    } <b>else</b> {
        std::option::none()
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_identity"></a>

## Function `point_identity`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_identity">point_identity</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_identity">point_identity</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_point_identity_internal">point_identity_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_generator"></a>

## Function `point_generator`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_generator">point_generator</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_generator">point_generator</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_point_generator_internal">point_generator_internal</a>&lt;G&gt;()
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_add"></a>

## Function `point_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add">point_add</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add">point_add</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_point_add_internal">point_add_internal</a>&lt;G&gt;(point_1.handle, point_2.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_mul"></a>

## Function `point_mul`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_mul">point_mul</a>&lt;G&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;, _point: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_mul">point_mul</a>&lt;G&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;, _point: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_point_mul_internal">point_mul_internal</a>&lt;G&gt;(_scalar.handle, _point.handle)
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes"></a>

## Function `scalar_to_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;G&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>&lt;G&gt;(scalar.handle)
}
</code></pre>



</details>

<a name="0x1_curves_point_to_bytes"></a>

## Function `point_to_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes">point_to_bytes</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes">point_to_bytes</a>&lt;G&gt;(point: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_point_to_bytes_internal">point_to_bytes_internal</a>&lt;G&gt;(point.handle)
}
</code></pre>



</details>

<a name="0x1_curves_element_from_bytes"></a>

## Function `element_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_from_bytes">element_from_bytes</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_element_from_bytes">element_from_bytes</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_element_from_bytes_internal">element_from_bytes_internal</a>&lt;G&gt;(bytes)
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_eq"></a>

## Function `point_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq">point_eq</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq">point_eq</a>&lt;G&gt;(point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;, point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;): bool {
    <a href="curves.md#0x1_curves_point_eq_internal">point_eq_internal</a>&lt;G&gt;(point_1.handle, point_2.handle)
}
</code></pre>



</details>

<a name="0x1_curves_element_from_bytes_internal"></a>

## Function `element_from_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_element_from_bytes_internal">element_from_bytes_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_element_from_bytes_internal">element_from_bytes_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u8;
</code></pre>



</details>

<a name="0x1_curves_scalar_from_u64_internal"></a>

## Function `scalar_from_u64_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value: u64): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>&lt;G&gt;(value: u64): u8;
</code></pre>



</details>

<a name="0x1_curves_scalar_from_bytes_internal"></a>

## Function `scalar_from_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes_internal">scalar_from_bytes_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes_internal">scalar_from_bytes_internal</a>&lt;G&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (bool, u8);
</code></pre>



</details>

<a name="0x1_curves_scalar_neg_internal"></a>

## Function `scalar_neg_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(handle: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>&lt;G&gt;(handle: u8): u8;
</code></pre>



</details>

<a name="0x1_curves_scalar_add_internal"></a>

## Function `scalar_add_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): u8;
</code></pre>



</details>

<a name="0x1_curves_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): u8;
</code></pre>



</details>

<a name="0x1_curves_scalar_inv_internal"></a>

## Function `scalar_inv_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(handle: u8): (bool, u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>&lt;G&gt;(handle: u8): (bool, u8);
</code></pre>



</details>

<a name="0x1_curves_scalar_eq_internal"></a>

## Function `scalar_eq_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): bool;
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes_internal"></a>

## Function `scalar_to_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>&lt;G&gt;(h: u8): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>&lt;G&gt;(h: u8): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_curves_pairing_internal"></a>

## Function `pairing_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handle: u8, g2_handle: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handle: u8, g2_handle: u8): u8;
</code></pre>



</details>

<a name="0x1_curves_multi_pairing_internal"></a>

## Function `multi_pairing_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_multi_pairing_internal">multi_pairing_internal</a>&lt;G1, G2, Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_multi_pairing_internal">multi_pairing_internal</a>&lt;G1,G2,Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u8;
</code></pre>



</details>

<a name="0x1_curves_point_add_internal"></a>

## Function `point_add_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_add_internal">point_add_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add_internal">point_add_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): u8;
</code></pre>



</details>

<a name="0x1_curves_point_eq_internal"></a>

## Function `point_eq_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_eq_internal">point_eq_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq_internal">point_eq_internal</a>&lt;G&gt;(handle_1: u8, handle_2: u8): bool;
</code></pre>



</details>

<a name="0x1_curves_point_identity_internal"></a>

## Function `point_identity_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_identity_internal">point_identity_internal</a>&lt;G&gt;(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_identity_internal">point_identity_internal</a>&lt;G&gt;(): u8;
</code></pre>



</details>

<a name="0x1_curves_point_generator_internal"></a>

## Function `point_generator_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_generator_internal">point_generator_internal</a>&lt;G&gt;(): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_generator_internal">point_generator_internal</a>&lt;G&gt;(): u8;
</code></pre>



</details>

<a name="0x1_curves_point_mul_internal"></a>

## Function `point_mul_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_mul_internal">point_mul_internal</a>&lt;G&gt;(scalar_handle: u8, point_handle: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_mul_internal">point_mul_internal</a>&lt;G&gt;(scalar_handle: u8, point_handle: u8): u8;
</code></pre>



</details>

<a name="0x1_curves_point_to_bytes_internal"></a>

## Function `point_to_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes_internal">point_to_bytes_internal</a>&lt;G&gt;(handle: u8): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes_internal">point_to_bytes_internal</a>&lt;G&gt;(handle: u8): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
