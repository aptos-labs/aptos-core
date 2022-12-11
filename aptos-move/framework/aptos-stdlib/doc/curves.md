
<a name="0x1_curves"></a>

# Module `0x1::curves`



-  [Struct `Scalar`](#0x1_curves_Scalar)
-  [Struct `Point`](#0x1_curves_Point)
-  [Struct `BLS12_381_G1`](#0x1_curves_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_curves_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_curves_BLS12_381_Gt)
-  [Function `pairing`](#0x1_curves_pairing)
-  [Function `scalar_zero`](#0x1_curves_scalar_zero)
-  [Function `scalar_one`](#0x1_curves_scalar_one)
-  [Function `scalar_neg`](#0x1_curves_scalar_neg)
-  [Function `scalar_add`](#0x1_curves_scalar_add)
-  [Function `scalar_mul`](#0x1_curves_scalar_mul)
-  [Function `scalar_inv`](#0x1_curves_scalar_inv)
-  [Function `scalar_eq`](#0x1_curves_scalar_eq)
-  [Function `point_zero`](#0x1_curves_point_zero)
-  [Function `point_identity`](#0x1_curves_point_identity)
-  [Function `point_add`](#0x1_curves_point_add)
-  [Function `point_mul`](#0x1_curves_point_mul)
-  [Function `scalar_to_bytes`](#0x1_curves_scalar_to_bytes)
-  [Function `point_to_bytes`](#0x1_curves_point_to_bytes)
-  [Function `scalar_from_bytes`](#0x1_curves_scalar_from_bytes)
-  [Function `point_from_bytes`](#0x1_curves_point_from_bytes)
-  [Function `point_eq`](#0x1_curves_point_eq)


<pre><code></code></pre>



<a name="0x1_curves_Scalar"></a>

## Struct `Scalar`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_curves_Point"></a>

## Struct `Point`



<pre><code><b>struct</b> <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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

<a name="0x1_curves_pairing"></a>

## Function `pairing`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1, G2, Gt&gt;(_p1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G1&gt;, _p2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_pairing">pairing</a>&lt;G1,G2,Gt&gt;(_p1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G1&gt;, _p2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;G2&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;Gt&gt; {
        bytes: b"asdf"
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
        bytes: b"asdf"
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


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_one">scalar_one</a>&lt;G&gt;(): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;G&gt;;
</code></pre>



</details>

<a name="0x1_curves_scalar_neg"></a>

## Function `scalar_neg`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_neg">scalar_neg</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    //todo
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        bytes: b"asdf"
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
    //todo
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        bytes: b"asdf"
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
    //todo
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        bytes: b"asdf"
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_inv"></a>

## Function `scalar_inv`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;T&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_inv">scalar_inv</a>&lt;T&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    //todo
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        bytes: b"asdf"
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_eq"></a>

## Function `scalar_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_eq">scalar_eq</a>&lt;T&gt;(_scalar_1: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;, _scalar_2: &<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): bool {
    //todo
    <b>false</b>
}
</code></pre>



</details>

<a name="0x1_curves_point_zero"></a>

## Function `point_zero`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_zero">point_zero</a>&lt;T&gt;(): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_zero">point_zero</a>&lt;T&gt;(): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        bytes: b"adf"
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
        bytes: b"adf"
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_add"></a>

## Function `point_add`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add">point_add</a>&lt;T&gt;(_point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;, _point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_add">point_add</a>&lt;T&gt;(_point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;, _point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        bytes: b"adf"
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
        bytes: b"asdf"
    }
}
</code></pre>



</details>

<a name="0x1_curves_scalar_to_bytes"></a>

## Function `scalar_to_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;T&gt;(_scalar: &<a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_to_bytes">scalar_to_bytes</a>&lt;T&gt;(_scalar :&<a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    b"adfd"
}
</code></pre>



</details>

<a name="0x1_curves_point_to_bytes"></a>

## Function `point_to_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes">point_to_bytes</a>&lt;T&gt;(_element: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_to_bytes">point_to_bytes</a>&lt;T&gt;(_element :&<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    b"asdf"
}
</code></pre>



</details>

<a name="0x1_curves_scalar_from_bytes"></a>

## Function `scalar_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;T&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Scalar">curves::Scalar</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_scalar_from_bytes">scalar_from_bytes</a>&lt;T&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Scalar">Scalar</a>&lt;T&gt; {
        bytes: *bytes
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_from_bytes"></a>

## Function `point_from_bytes`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_from_bytes">point_from_bytes</a>&lt;T&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_from_bytes">point_from_bytes</a>&lt;T&gt;(bytes: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
    <a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt; {
        bytes: *bytes
    }
}
</code></pre>



</details>

<a name="0x1_curves_point_eq"></a>

## Function `point_eq`



<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq">point_eq</a>&lt;T&gt;(_point_1: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;, _point_2: &<a href="curves.md#0x1_curves_Point">curves::Point</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="curves.md#0x1_curves_point_eq">point_eq</a>&lt;T&gt;(_point_1: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;, _point_2: &<a href="curves.md#0x1_curves_Point">Point</a>&lt;T&gt;): bool {
    //todo
    <b>false</b>
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
