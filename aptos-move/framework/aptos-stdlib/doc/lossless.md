
<a id="0x1_lossless"></a>

# Module `0x1::lossless`



-  [Struct `Number`](#0x1_lossless_Number)
-  [Function `product`](#0x1_lossless_product)
-  [Function `add_assign`](#0x1_lossless_add_assign)
-  [Function `mul_assign`](#0x1_lossless_mul_assign)
-  [Function `sum`](#0x1_lossless_sum)
-  [Function `neg`](#0x1_lossless_neg)
-  [Function `neg_assign`](#0x1_lossless_neg_assign)
-  [Function `sub`](#0x1_lossless_sub)
-  [Function `round`](#0x1_lossless_round)
-  [Function `is_power_of_2`](#0x1_lossless_is_power_of_2)
-  [Function `ceil`](#0x1_lossless_ceil)
-  [Function `from_u64`](#0x1_lossless_from_u64)
-  [Function `from_u128`](#0x1_lossless_from_u128)
-  [Function `from_fixed_point64`](#0x1_lossless_from_fixed_point64)
-  [Function `cmp`](#0x1_lossless_cmp)
-  [Function `greater_than`](#0x1_lossless_greater_than)
-  [Function `less_than`](#0x1_lossless_less_than)
-  [Function `get_integer_chunk`](#0x1_lossless_get_integer_chunk)
-  [Function `as_u128`](#0x1_lossless_as_u128)
-  [Function `as_u64`](#0x1_lossless_as_u64)
-  [Function `min_assign`](#0x1_lossless_min_assign)
-  [Function `floor_assign`](#0x1_lossless_floor_assign)
-  [Function `div_ceil`](#0x1_lossless_div_ceil)
-  [Function `default`](#0x1_lossless_default)


<pre><code><b>use</b> <a href="fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
</code></pre>



<a id="0x1_lossless_Number"></a>

## Struct `Number`

The actual value is <code>x[0]*R^{p-2^32} + ... + x[n-1]*R^{p-2^32+n-1}</code>,
where <code>n</code> is the length of vector <code>x</code>.


<pre><code><b>struct</b> <a href="lossless.md#0x1_lossless_Number">Number</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>p: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lossless_product"></a>

## Function `product`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_product">product</a>(items: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>&gt;): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_product">product</a>(items: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lossless.md#0x1_lossless_Number">Number</a>&gt;): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_add_assign"></a>

## Function `add_assign`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_add_assign">add_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, other: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_add_assign">add_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">Number</a>, other: <a href="lossless.md#0x1_lossless_Number">Number</a>) {

}
</code></pre>



</details>

<a id="0x1_lossless_mul_assign"></a>

## Function `mul_assign`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_mul_assign">mul_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, other: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_mul_assign">mul_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">Number</a>, other: <a href="lossless.md#0x1_lossless_Number">Number</a>) {

}
</code></pre>



</details>

<a id="0x1_lossless_sum"></a>

## Function `sum`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_sum">sum</a>(items: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>&gt;): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_sum">sum</a>(items: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="lossless.md#0x1_lossless_Number">Number</a>&gt;): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_neg"></a>

## Function `neg`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_neg">neg</a>(item: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_neg">neg</a>(item: <a href="lossless.md#0x1_lossless_Number">Number</a>): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_neg_assign"></a>

## Function `neg_assign`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_neg_assign">neg_assign</a>(item: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_neg_assign">neg_assign</a>(item: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">Number</a>) {

}
</code></pre>



</details>

<a id="0x1_lossless_sub"></a>

## Function `sub`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_sub">sub</a>(a: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, b: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_sub">sub</a>(a: <a href="lossless.md#0x1_lossless_Number">Number</a>, b: <a href="lossless.md#0x1_lossless_Number">Number</a>): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_round"></a>

## Function `round`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_round">round</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, increment: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_round">round</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>, increment: <a href="lossless.md#0x1_lossless_Number">Number</a>): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_is_power_of_2"></a>

## Function `is_power_of_2`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_is_power_of_2">is_power_of_2</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_is_power_of_2">is_power_of_2</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>): bool {
    <b>false</b>
}
</code></pre>



</details>

<a id="0x1_lossless_ceil"></a>

## Function `ceil`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_ceil">ceil</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_ceil">ceil</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_from_u64"></a>

## Function `from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_from_u64">from_u64</a>(val: u64): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_from_u64">from_u64</a>(val: u64): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_from_u128"></a>

## Function `from_u128`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_from_u128">from_u128</a>(val: u128): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_from_u128">from_u128</a>(val: u128): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_from_fixed_point64"></a>

## Function `from_fixed_point64`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_from_fixed_point64">from_fixed_point64</a>(val: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_from_fixed_point64">from_fixed_point64</a>(val: FixedPoint64): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_cmp"></a>

## Function `cmp`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_cmp">cmp</a>(x: &<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, y: &<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_cmp">cmp</a>(x: &<a href="lossless.md#0x1_lossless_Number">Number</a>, y: &<a href="lossless.md#0x1_lossless_Number">Number</a>): u64 {
    0
}
</code></pre>



</details>

<a id="0x1_lossless_greater_than"></a>

## Function `greater_than`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_greater_than">greater_than</a>(x: &<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, y: &<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_greater_than">greater_than</a>(x: &<a href="lossless.md#0x1_lossless_Number">Number</a>, y: &<a href="lossless.md#0x1_lossless_Number">Number</a>): bool {
    <b>false</b>
}
</code></pre>



</details>

<a id="0x1_lossless_less_than"></a>

## Function `less_than`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_less_than">less_than</a>(x: &<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, y: &<a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_less_than">less_than</a>(x: &<a href="lossless.md#0x1_lossless_Number">Number</a>, y: &<a href="lossless.md#0x1_lossless_Number">Number</a>): bool {
    <b>false</b>
}
</code></pre>



</details>

<a id="0x1_lossless_get_integer_chunk"></a>

## Function `get_integer_chunk`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_get_integer_chunk">get_integer_chunk</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_get_integer_chunk">get_integer_chunk</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>, idx: u64): u64 {
    0
}
</code></pre>



</details>

<a id="0x1_lossless_as_u128"></a>

## Function `as_u128`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_as_u128">as_u128</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_as_u128">as_u128</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>): u128 {
    0
}
</code></pre>



</details>

<a id="0x1_lossless_as_u64"></a>

## Function `as_u64`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_as_u64">as_u64</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_as_u64">as_u64</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>): u64 {
    0
}
</code></pre>



</details>

<a id="0x1_lossless_min_assign"></a>

## Function `min_assign`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_min_assign">min_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, other: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_min_assign">min_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">Number</a>, other: <a href="lossless.md#0x1_lossless_Number">Number</a>) {

}
</code></pre>



</details>

<a id="0x1_lossless_floor_assign"></a>

## Function `floor_assign`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_floor_assign">floor_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_floor_assign">floor_assign</a>(self: &<b>mut</b> <a href="lossless.md#0x1_lossless_Number">Number</a>) {
}
</code></pre>



</details>

<a id="0x1_lossless_div_ceil"></a>

## Function `div_ceil`



<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_div_ceil">div_ceil</a>(x: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>, y: <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lossless.md#0x1_lossless_div_ceil">div_ceil</a>(x: <a href="lossless.md#0x1_lossless_Number">Number</a>, y: <a href="lossless.md#0x1_lossless_Number">Number</a>): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_default">default</a>()
}
</code></pre>



</details>

<a id="0x1_lossless_default"></a>

## Function `default`



<pre><code><b>fun</b> <a href="lossless.md#0x1_lossless_default">default</a>(): <a href="lossless.md#0x1_lossless_Number">lossless::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="lossless.md#0x1_lossless_default">default</a>(): <a href="lossless.md#0x1_lossless_Number">Number</a> {
    <a href="lossless.md#0x1_lossless_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        p: 0,
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
