
<a name="0x1_curves"></a>

# Module `0x1::curves`



-  [Struct `Scalar`](#0x1_curves_Scalar)
-  [Struct `Point`](#0x1_curves_Point)
-  [Struct `BLS12_381_G1`](#0x1_curves_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_curves_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_curves_BLS12_381_Gt)
-  [Constants](#@Constants_0)
-  [Function `get_scalar_handle`](#0x1_curves_get_scalar_handle)
-  [Function `get_point_handle`](#0x1_curves_get_point_handle)
-  [Function `pairing`](#0x1_curves_pairing)
-  [Function `scalar_zero`](#0x1_curves_scalar_zero)
-  [Function `scalar_one`](#0x1_curves_scalar_one)
-  [Function `scalar_from_u64`](#0x1_curves_scalar_from_u64)
-  [Function `scalar_neg`](#0x1_curves_scalar_neg)
-  [Function `scalar_add`](#0x1_curves_scalar_add)
-  [Function `scalar_mul`](#0x1_curves_scalar_mul)
-  [Function `scalar_inv`](#0x1_curves_scalar_inv)
-  [Function `scalar_eq`](#0x1_curves_scalar_eq)
-  [Function `bytes_into_scalar`](#0x1_curves_bytes_into_scalar)
-  [Function `point_identity`](#0x1_curves_point_identity)
-  [Function `point_generator`](#0x1_curves_point_generator)
-  [Function `point_add`](#0x1_curves_point_add)
-  [Function `point_mul`](#0x1_curves_point_mul)
-  [Function `scalar_to_bytes`](#0x1_curves_scalar_to_bytes)
-  [Function `point_to_bytes`](#0x1_curves_point_to_bytes)
-  [Function `bytes_into_point`](#0x1_curves_bytes_into_point)
-  [Function `point_eq`](#0x1_curves_point_eq)
-  [Function `get_group_id`](#0x1_curves_get_group_id)
-  [Function `get_pairing_id`](#0x1_curves_get_pairing_id)
-  [Function `bytes_into_point_internal`](#0x1_curves_bytes_into_point_internal)
-  [Function `bytes_into_scalar_internal`](#0x1_curves_bytes_into_scalar_internal)
-  [Function `scalar_zero_internal`](#0x1_curves_scalar_zero_internal)
-  [Function `scalar_one_internal`](#0x1_curves_scalar_one_internal)
-  [Function `scalar_from_u64_internal`](#0x1_curves_scalar_from_u64_internal)
-  [Function `scalar_neg_internal`](#0x1_curves_scalar_neg_internal)
-  [Function `scalar_add_internal`](#0x1_curves_scalar_add_internal)
-  [Function `scalar_mul_internal`](#0x1_curves_scalar_mul_internal)
-  [Function `scalar_inv_internal`](#0x1_curves_scalar_inv_internal)
-  [Function `scalar_eq_internal`](#0x1_curves_scalar_eq_internal)
-  [Function `scalar_to_bytes_internal`](#0x1_curves_scalar_to_bytes_internal)
-  [Function `pairing_internal`](#0x1_curves_pairing_internal)
-  [Function `point_add_internal`](#0x1_curves_point_add_internal)
-  [Function `point_eq_internal`](#0x1_curves_point_eq_internal)
-  [Function `point_identity_internal`](#0x1_curves_point_identity_internal)
-  [Function `point_generator_internal`](#0x1_curves_point_generator_internal)
-  [Function `point_mul_internal`](#0x1_curves_point_mul_internal)
-  [Function `point_to_bytes_internal`](#0x1_curves_point_to_bytes_internal)


<pre><code><b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a name="0x1_curves_Scalar"></a>

## Struct `Scalar`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;Group&gt; <b>has</b> drop
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

<a name="0x1_curves_Point"></a>

## Struct `Point`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Point">Point</a>&lt;Group&gt; <b>has</b> drop
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_curves_GID_BLS12_381_G1"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_GID_BLS12_381_G1">GID_BLS12_381_G1</a>: u64 = 1;
</code></pre>



<a name="0x1_curves_GID_BLS12_381_G2"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_GID_BLS12_381_G2">GID_BLS12_381_G2</a>: u64 = 2;
</code></pre>



<a name="0x1_curves_GID_BLS12_381_Gt"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_GID_BLS12_381_Gt">GID_BLS12_381_Gt</a>: u64 = 3;
</code></pre>



<a name="0x1_curves_GID_UNKNOWN"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_GID_UNKNOWN">GID_UNKNOWN</a>: u64 = 0;
</code></pre>



<a name="0x1_curves_PID_BLS12_381"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_PID_BLS12_381">PID_BLS12_381</a>: u64 = 1;
</code></pre>



<a name="0x1_curves_PID_UNKNOWN"></a>



<pre><code><b>const</b> <a href="curves.md#0x1_curves_PID_UNKNOWN">PID_UNKNOWN</a>: u64 = 0;
</code></pre>



<a name="0x1_curves_get_scalar_handle"></a>

## Function `get_scalar_handle`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_get_scalar_handle">get_scalar_handle</a>&lt;G&gt;(s: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_get_scalar_handle">get_scalar_handle</a>&lt;G&gt;(s: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;): u64 {
    s.handle
}
</code></pre>



</details>

<a name="0x1_curves_get_point_handle"></a>

## Function `get_point_handle`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_get_point_handle">get_point_handle</a>&lt;G&gt;(p: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_get_point_handle">get_point_handle</a>&lt;G&gt;(p: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G&gt;): u64 {
    p.handle
}
</code></pre>



</details>

<a name="0x1_curves_pairing"></a>

## Function `pairing`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1, G2, Gt&gt;(point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1,G2,Gt&gt;(point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G1&gt;, point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
        handle: <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>(point_1.handle, point_2.handle, <a href="curves.md#0x1_curves_get_pairing_id">get_pairing_id</a>&lt;G1,G2,Gt&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_zero"></a>

## Function `scalar_zero`

Scalar basics.


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_zero">scalar_zero</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_zero">scalar_zero</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_zero_internal">scalar_zero_internal</a>(<a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_one"></a>

## Function `scalar_one`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_one">scalar_one</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_one">scalar_one</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_one_internal">scalar_one_internal</a>(<a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_from_u64"></a>

## Function `scalar_from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64">scalar_from_u64</a>&lt;G&gt;(value: u64): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64">scalar_from_u64</a>&lt;G&gt;(value: u64): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>(value, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_neg"></a>

## Function `scalar_neg`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>(<a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_add"></a>

## Function `scalar_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add">scalar_add</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add">scalar_add</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>(_scalar_1.handle, _scalar_2.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_mul"></a>

## Function `scalar_mul`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul">scalar_mul</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul">scalar_mul</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>(_scalar_1.handle, _scalar_2.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_inv"></a>

## Function `scalar_inv`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;T&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;T&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>(scalar.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_eq"></a>

## Function `scalar_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;T&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;T&gt;(scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;, scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): bool {
    <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>(scalar_1.handle, scalar_2.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
}
</code></pre>



</details>

<a name="0x1_curves_bytes_into_scalar"></a>

## Function `bytes_into_scalar`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_bytes_into_scalar">bytes_into_scalar</a>&lt;T&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_bytes_into_scalar">bytes_into_scalar</a>&lt;T&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_bytes_into_scalar_internal">bytes_into_scalar_internal</a>(bytes, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_identity"></a>

## Function `point_identity`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_identity">point_identity</a>&lt;T&gt;(): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_identity">point_identity</a>&lt;T&gt;(): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_point_identity_internal">point_identity_internal</a>(<a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_generator"></a>

## Function `point_generator`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_generator">point_generator</a>&lt;T&gt;(): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_generator">point_generator</a>&lt;T&gt;(): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_point_generator_internal">point_generator_internal</a>(<a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_add"></a>

## Function `point_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add">point_add</a>&lt;T&gt;(point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;, point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add">point_add</a>&lt;T&gt;(point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;, point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_point_add_internal">point_add_internal</a>(point_1.handle, point_2.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
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
        handle: <a href="curves.md#0x1_curves_point_mul_internal">point_mul_internal</a>(_scalar.handle, _point.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes"></a>

## Function `scalar_to_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;T&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;T&gt;(scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>(scalar.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
}
</code></pre>



</details>

<a name="0x1_curves_point_to_bytes"></a>

## Function `point_to_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes">point_to_bytes</a>&lt;T&gt;(point: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes">point_to_bytes</a>&lt;T&gt;(point: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="curves.md#0x1_curves_point_to_bytes_internal">point_to_bytes_internal</a>(point.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
}
</code></pre>



</details>

<a name="0x1_curves_bytes_into_point"></a>

## Function `bytes_into_point`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_bytes_into_point">bytes_into_point</a>&lt;T&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_bytes_into_point">bytes_into_point</a>&lt;T&gt;(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        handle: <a href="curves.md#0x1_curves_bytes_into_point_internal">bytes_into_point_internal</a>(bytes, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_eq"></a>

## Function `point_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq">point_eq</a>&lt;T&gt;(point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;, point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq">point_eq</a>&lt;T&gt;(point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;, point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;): bool {
    <a href="curves.md#0x1_curves_point_eq_internal">point_eq_internal</a>(point_1.handle, point_2.handle, <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;T&gt;())
}
</code></pre>



</details>

<a name="0x1_curves_get_group_id"></a>

## Function `get_group_id`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G&gt;(): u64 {
    <b>let</b> typ = type_of&lt;G&gt;();
    <b>if</b> (typ == type_of&lt;<a href="curves.md#0x1_curves_BLS12_381_G1">BLS12_381_G1</a>&gt;()) {
        <a href="curves.md#0x1_curves_GID_BLS12_381_G1">GID_BLS12_381_G1</a>
    } <b>else</b> <b>if</b> (typ == type_of&lt;<a href="curves.md#0x1_curves_BLS12_381_G2">BLS12_381_G2</a>&gt;()) {
        <a href="curves.md#0x1_curves_GID_BLS12_381_G2">GID_BLS12_381_G2</a>
    } <b>else</b> <b>if</b> (typ == type_of&lt;<a href="curves.md#0x1_curves_BLS12_381_Gt">BLS12_381_Gt</a>&gt;()) {
        <a href="curves.md#0x1_curves_GID_BLS12_381_Gt">GID_BLS12_381_Gt</a>
    } <b>else</b> {
        <a href="curves.md#0x1_curves_GID_UNKNOWN">GID_UNKNOWN</a>
    }
}
</code></pre>



</details>

<a name="0x1_curves_get_pairing_id"></a>

## Function `get_pairing_id`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_get_pairing_id">get_pairing_id</a>&lt;G1, G2, Gt&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_get_pairing_id">get_pairing_id</a>&lt;G1,G2,Gt&gt;(): u64 {
    <b>if</b> (<a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G1&gt;() == <a href="curves.md#0x1_curves_GID_BLS12_381_G1">GID_BLS12_381_G1</a> && <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;G2&gt;() == <a href="curves.md#0x1_curves_GID_BLS12_381_G2">GID_BLS12_381_G2</a> && <a href="curves.md#0x1_curves_get_group_id">get_group_id</a>&lt;Gt&gt;() == <a href="curves.md#0x1_curves_GID_BLS12_381_Gt">GID_BLS12_381_Gt</a>) {
        <a href="curves.md#0x1_curves_PID_BLS12_381">PID_BLS12_381</a>
    } <b>else</b> {
        <a href="curves.md#0x1_curves_PID_UNKNOWN">PID_UNKNOWN</a>
    }
}
</code></pre>



</details>

<a name="0x1_curves_bytes_into_point_internal"></a>

## Function `bytes_into_point_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_bytes_into_point_internal">bytes_into_point_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_bytes_into_point_internal">bytes_into_point_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_bytes_into_scalar_internal"></a>

## Function `bytes_into_scalar_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_bytes_into_scalar_internal">bytes_into_scalar_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_bytes_into_scalar_internal">bytes_into_scalar_internal</a>(bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_zero_internal"></a>

## Function `scalar_zero_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_zero_internal">scalar_zero_internal</a>(gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_zero_internal">scalar_zero_internal</a>(gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_one_internal"></a>

## Function `scalar_one_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_one_internal">scalar_one_internal</a>(gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_one_internal">scalar_one_internal</a>(gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_from_u64_internal"></a>

## Function `scalar_from_u64_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>(value: u64, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_u64_internal">scalar_from_u64_internal</a>(value: u64, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_neg_internal"></a>

## Function `scalar_neg_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>(gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg_internal">scalar_neg_internal</a>(gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_add_internal"></a>

## Function `scalar_add_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>(handle_1: u64, handle_2: u64, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_add_internal">scalar_add_internal</a>(handle_1: u64, handle_2: u64, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_mul_internal"></a>

## Function `scalar_mul_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>(handle_1: u64, handle_2: u64, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_mul_internal">scalar_mul_internal</a>(handle_1: u64, handle_2: u64, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_inv_internal"></a>

## Function `scalar_inv_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>(handle: u64, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv_internal">scalar_inv_internal</a>(handle: u64, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_scalar_eq_internal"></a>

## Function `scalar_eq_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>(handle_1: u64, handle_2: u64, gid: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq_internal">scalar_eq_internal</a>(handle_1: u64, handle_2: u64, gid: u64): bool;
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes_internal"></a>

## Function `scalar_to_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>(h: u64, gid: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes_internal">scalar_to_bytes_internal</a>(h: u64, gid: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_curves_pairing_internal"></a>

## Function `pairing_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>(p1_handle: u64, p2_handle: u64, pairing_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing_internal">pairing_internal</a>(p1_handle: u64, p2_handle: u64, pairing_id: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_point_add_internal"></a>

## Function `point_add_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_add_internal">point_add_internal</a>(handle_1: u64, handle_2: u64, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add_internal">point_add_internal</a>(handle_1: u64, handle_2: u64, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_point_eq_internal"></a>

## Function `point_eq_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_eq_internal">point_eq_internal</a>(handle_1: u64, handle_2: u64, gid: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq_internal">point_eq_internal</a>(handle_1: u64, handle_2: u64, gid: u64): bool;
</code></pre>



</details>

<a name="0x1_curves_point_identity_internal"></a>

## Function `point_identity_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_identity_internal">point_identity_internal</a>(gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_identity_internal">point_identity_internal</a>(gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_point_generator_internal"></a>

## Function `point_generator_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_generator_internal">point_generator_internal</a>(gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_generator_internal">point_generator_internal</a>(gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_point_mul_internal"></a>

## Function `point_mul_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_mul_internal">point_mul_internal</a>(scalar_handle: u64, point_handle: u64, gid: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_mul_internal">point_mul_internal</a>(scalar_handle: u64, point_handle: u64, gid: u64): u64;
</code></pre>



</details>

<a name="0x1_curves_point_to_bytes_internal"></a>

## Function `point_to_bytes_internal`



<pre><code><b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes_internal">point_to_bytes_internal</a>(handle: u64, gid: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes_internal">point_to_bytes_internal</a>(handle: u64, gid: u64): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
