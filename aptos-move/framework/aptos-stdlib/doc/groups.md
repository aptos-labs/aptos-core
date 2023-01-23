
<a name="0x1_groups"></a>

# Module `0x1::groups`



-  [Struct `BLS12_381_G1`](#0x1_groups_BLS12_381_G1)
-  [Struct `BLS12_381_G2`](#0x1_groups_BLS12_381_G2)
-  [Struct `BLS12_381_Gt`](#0x1_groups_BLS12_381_Gt)
-  [Struct `Ristretto255`](#0x1_groups_Ristretto255)
-  [Struct `Element`](#0x1_groups_Element)
-  [Function `add`](#0x1_groups_add)
-  [Function `pairing`](#0x1_groups_pairing)
-  [Function `eq`](#0x1_groups_eq)
-  [Function `generator`](#0x1_groups_generator)
-  [Function `add_internal`](#0x1_groups_add_internal)
-  [Function `eq_internal`](#0x1_groups_eq_internal)
-  [Function `generator_internal`](#0x1_groups_generator_internal)
-  [Function `pairing_product_internal`](#0x1_groups_pairing_product_internal)


<pre><code></code></pre>



<a name="0x1_groups_BLS12_381_G1"></a>

## Struct `BLS12_381_G1`



<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_G1">BLS12_381_G1</a>
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

<a name="0x1_groups_BLS12_381_G2"></a>

## Struct `BLS12_381_G2`



<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_G2">BLS12_381_G2</a>
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

<a name="0x1_groups_BLS12_381_Gt"></a>

## Struct `BLS12_381_Gt`



<pre><code><b>struct</b> <a href="groups.md#0x1_groups_BLS12_381_Gt">BLS12_381_Gt</a>
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

<a name="0x1_groups_Ristretto255"></a>

## Struct `Ristretto255`



<pre><code><b>struct</b> <a href="groups.md#0x1_groups_Ristretto255">Ristretto255</a>
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

<a name="0x1_groups_Element"></a>

## Struct `Element`

This struct represents an element of the group represented by the type argument <code>G</code>.


<pre><code><b>struct</b> <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; <b>has</b> <b>copy</b>, drop
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

<a name="0x1_groups_add"></a>

## Function `add`

Check if <code>P == Q</code> for group elements <code>P</code> and <code>Q</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_add">add</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_add">add</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): bool {
    <a href="groups.md#0x1_groups_add_internal">add_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
}
</code></pre>



</details>

<a name="0x1_groups_pairing"></a>

## Function `pairing`

Perform a pairing.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing">pairing</a>&lt;G1, G2, Gt&gt;(element_1: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G1&gt;, element_2: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G2&gt;): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing">pairing</a>&lt;G1,G2,Gt&gt;(element_1: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G1&gt;, element_2: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G2&gt;): <a href="groups.md#0x1_groups_Element">Element</a>&lt;Gt&gt; {
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;Gt&gt; {
        handle: <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[element_1.handle], <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[element_2.handle])
    }
}
</code></pre>



</details>

<a name="0x1_groups_eq"></a>

## Function `eq`

Check if <code>P == Q</code> for group elements <code>P</code> and <code>Q</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_eq">eq</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_eq">eq</a>&lt;G&gt;(element_p: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;, element_q: &<a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt;): bool {
    <a href="groups.md#0x1_groups_eq_internal">eq_internal</a>&lt;G&gt;(element_p.handle, element_q.handle)
}
</code></pre>



</details>

<a name="0x1_groups_generator"></a>

## Function `generator`

Get a generator of group <code>G</code>.


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_generator">generator</a>&lt;G&gt;(): <a href="groups.md#0x1_groups_Element">groups::Element</a>&lt;G&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="groups.md#0x1_groups_generator">generator</a>&lt;G&gt;(): <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
    <a href="groups.md#0x1_groups_Element">Element</a>&lt;G&gt; {
        handle: <a href="groups.md#0x1_groups_generator_internal">generator_internal</a>&lt;G&gt;(),
    }
}
</code></pre>



</details>

<a name="0x1_groups_add_internal"></a>

## Function `add_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_add_internal">add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_add_internal">add_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_groups_eq_internal"></a>

## Function `eq_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_eq_internal">eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_eq_internal">eq_internal</a>&lt;G&gt;(handle_1: u64, handle_2: u64): bool;
</code></pre>



</details>

<a name="0x1_groups_generator_internal"></a>

## Function `generator_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_generator_internal">generator_internal</a>&lt;G&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_generator_internal">generator_internal</a>&lt;G&gt;(): u64;
</code></pre>



</details>

<a name="0x1_groups_pairing_product_internal"></a>

## Function `pairing_product_internal`



<pre><code><b>fun</b> <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1, G2, Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="groups.md#0x1_groups_pairing_product_internal">pairing_product_internal</a>&lt;G1,G2,Gt&gt;(g1_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, g2_handles: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): u64;
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
