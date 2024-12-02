
<a id="0x1_arbitrary_precision"></a>

# Module `0x1::arbitrary_precision`

Types and functions for unsigned arbitrary precision numbers.


-  [Struct `Number`](#0x1_arbitrary_precision_Number)
-  [Constants](#@Constants_0)
-  [Function `product`](#0x1_arbitrary_precision_product)
-  [Function `mul_u64_assign`](#0x1_arbitrary_precision_mul_u64_assign)
-  [Function `shift_by_chunk_assign`](#0x1_arbitrary_precision_shift_by_chunk_assign)
-  [Function `shift_by_bit_assign`](#0x1_arbitrary_precision_shift_by_bit_assign)
-  [Function `shift_up_by_bit`](#0x1_arbitrary_precision_shift_up_by_bit)
-  [Function `shift_up_by_bit_assign`](#0x1_arbitrary_precision_shift_up_by_bit_assign)
-  [Function `shift_down_by_bit`](#0x1_arbitrary_precision_shift_down_by_bit)
-  [Function `shift_down_by_bit_assign`](#0x1_arbitrary_precision_shift_down_by_bit_assign)
-  [Function `mul_assign`](#0x1_arbitrary_precision_mul_assign)
-  [Function `add_assign`](#0x1_arbitrary_precision_add_assign)
-  [Function `sum`](#0x1_arbitrary_precision_sum)
-  [Function `sub`](#0x1_arbitrary_precision_sub)
-  [Function `log2_floor`](#0x1_arbitrary_precision_log2_floor)
-  [Function `round`](#0x1_arbitrary_precision_round)
-  [Function `exp2`](#0x1_arbitrary_precision_exp2)
-  [Function `from_bin_repr`](#0x1_arbitrary_precision_from_bin_repr)
-  [Function `split_by_point`](#0x1_arbitrary_precision_split_by_point)
-  [Function `is_zero`](#0x1_arbitrary_precision_is_zero)
-  [Function `ceil`](#0x1_arbitrary_precision_ceil)
-  [Function `from_u64`](#0x1_arbitrary_precision_from_u64)
-  [Function `from_u128`](#0x1_arbitrary_precision_from_u128)
-  [Function `from_fixed_point64`](#0x1_arbitrary_precision_from_fixed_point64)
-  [Function `get_chunk`](#0x1_arbitrary_precision_get_chunk)
-  [Function `cmp`](#0x1_arbitrary_precision_cmp)
-  [Function `greater_than`](#0x1_arbitrary_precision_greater_than)
-  [Function `less_than`](#0x1_arbitrary_precision_less_than)
-  [Function `eq`](#0x1_arbitrary_precision_eq)
-  [Function `get_integer_chunk`](#0x1_arbitrary_precision_get_integer_chunk)
-  [Function `get_fractional_chunk`](#0x1_arbitrary_precision_get_fractional_chunk)
-  [Function `as_u128`](#0x1_arbitrary_precision_as_u128)
-  [Function `as_u64`](#0x1_arbitrary_precision_as_u64)
-  [Function `min_assign`](#0x1_arbitrary_precision_min_assign)
-  [Function `floor_assign`](#0x1_arbitrary_precision_floor_assign)
-  [Function `div_ceil`](#0x1_arbitrary_precision_div_ceil)
-  [Function `default`](#0x1_arbitrary_precision_default)
-  [Function `trim_zeros`](#0x1_arbitrary_precision_trim_zeros)


<pre><code><b>use</b> <a href="fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_arbitrary_precision_Number"></a>

## Struct `Number`

With <code>n</code> chunks, it represents the number:
<code>chunks[0]*R^(exp_plus_anchor-<a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>+0) + ... + chunks[n-1]*R^(exp_plus_anchor-<a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>+n-1)</code>,
where <code>R = 2^64</code>.


<pre><code><b>struct</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> <b>has</b> <b>copy</b>, drop
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
<code>exp_plus_anchor: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_arbitrary_precision_ANCHOR"></a>



<pre><code><b>const</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>: u64 = 9223372036854775808;
</code></pre>



<a id="0x1_arbitrary_precision_U64_MASK"></a>



<pre><code><b>const</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_U64_MASK">U64_MASK</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_arbitrary_precision_product"></a>

## Function `product`

Compute <code>v[0]*...*v[n-1]</code> for a list of numbers <code>v</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_product">product</a>(v: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>&gt;): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_product">product</a>(v: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>&gt;): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> accumulator = <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(1);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(v, |item|{
        <b>let</b> item: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> = item;
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_assign">mul_assign</a>(&<b>mut</b> accumulator, item);
    });
    accumulator
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_mul_u64_assign"></a>

## Function `mul_u64_assign`

Update <code>x</code> as <code>x * y</code>. <code>y</code> is a u64.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_u64_assign">mul_u64_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_u64_assign">mul_u64_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: u64) {
    <b>let</b> other = (y <b>as</b> u128);
    <b>let</b> carry = 0;
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_mut">vector::for_each_mut</a>(&<b>mut</b> x.chunks, |chunk|{
        <b>let</b> chunk: &<b>mut</b> u64 = chunk;
        <b>let</b> new_val = other * (*chunk <b>as</b> u128) + carry;
        *chunk = ((new_val & <a href="arbitrary_precision.md#0x1_arbitrary_precision_U64_MASK">U64_MASK</a>) <b>as</b> u64);
        carry = new_val &gt;&gt; 64;
    });
    <b>if</b> (carry &gt; 0) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> x.chunks, (carry <b>as</b> u64));
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_shift_by_chunk_assign"></a>

## Function `shift_by_chunk_assign`

Equivalent of <code>self &lt;&lt; c</code> at chunk level, where <code>c + <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> == degree_diff_plus_anchor</code>.
<code>c</code> can also be negative, which means <code>self &gt;&gt; (-c)</code> at chunk level.


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_chunk_assign">shift_by_chunk_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, offset_plus_anchor: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_chunk_assign">shift_by_chunk_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, offset_plus_anchor: u64) {
    self.exp_plus_anchor = (((self.exp_plus_anchor <b>as</b> u128) + (offset_plus_anchor <b>as</b> u128) - (<a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> <b>as</b> u128)) <b>as</b> u64);
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_shift_by_bit_assign"></a>

## Function `shift_by_bit_assign`

Equivalent of <code>self &lt;&lt; b</code> at bit level, where <code>b + <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> == bit_offset_plus_anchor</code>.
<code>b</code> can also be negative, which means <code>self &gt;&gt; (-b)</code> at bit level.


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_bit_assign">shift_by_bit_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, bit_offset_plus_anchor: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_bit_assign">shift_by_bit_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, bit_offset_plus_anchor: u64) {
    <b>let</b> equivalent_multiplier = 1 &lt;&lt; ((bit_offset_plus_anchor % 64) <b>as</b> u8);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_u64_assign">mul_u64_assign</a>(self, equivalent_multiplier);
    <b>let</b> chunk_offset_plus_anchor = <b>if</b> (bit_offset_plus_anchor &lt; <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>) {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - (<a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - bit_offset_plus_anchor + 63) / 64
    } <b>else</b> {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> + (bit_offset_plus_anchor - <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>) / 64
    };
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_chunk_assign">shift_by_chunk_assign</a>(self, chunk_offset_plus_anchor);
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_shift_up_by_bit"></a>

## Function `shift_up_by_bit`

Compute <code>x &lt;&lt; k</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit">shift_up_by_bit</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, k: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit">shift_up_by_bit</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, k: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(&<b>mut</b> x, k);
    x
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_shift_up_by_bit_assign"></a>

## Function `shift_up_by_bit_assign`

Update <code>x</code> to be <code>x &lt;&lt; k</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, k: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, k: u64) {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_bit_assign">shift_by_bit_assign</a>(x, k + <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>);
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_shift_down_by_bit"></a>

## Function `shift_down_by_bit`

Compute <code>x &gt;&gt; k</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit">shift_down_by_bit</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, num_bits: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit">shift_down_by_bit</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, num_bits: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit_assign">shift_down_by_bit_assign</a>(&<b>mut</b> x, num_bits);
    x
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_shift_down_by_bit_assign"></a>

## Function `shift_down_by_bit_assign`

Update <code>x</code> to be <code>x &gt;&gt; k</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit_assign">shift_down_by_bit_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, num_bits: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit_assign">shift_down_by_bit_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, num_bits: u64) {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_bit_assign">shift_by_bit_assign</a>(self, <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - num_bits);
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_mul_assign"></a>

## Function `mul_assign`

Update <code>x</code> to be <code>x * y</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_assign">mul_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_assign">mul_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>) {
    <b>let</b> sub_results = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> { chunks, exp_plus_anchor } = y;
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(&chunks, |i, chunk|{
        <b>let</b> chunk = *chunk;
        <b>let</b> self_clone = *x;
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_mul_u64_assign">mul_u64_assign</a>(&<b>mut</b> self_clone, chunk);
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_chunk_assign">shift_by_chunk_assign</a>(&<b>mut</b> self_clone, exp_plus_anchor + i);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> sub_results, self_clone);
    });
    *x = <a href="arbitrary_precision.md#0x1_arbitrary_precision_sum">sum</a>(sub_results);
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_add_assign"></a>

## Function `add_assign`

Update <code>x</code> to be <code>x + y</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>) {
    <b>let</b> x_degree_lmt_plus_anchor = x.exp_plus_anchor + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks);
    <b>let</b> y_degree_lmt_plus_anchor = y.exp_plus_anchor + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&y.chunks);
    <b>let</b> degree_high_plus_anchor = max(x_degree_lmt_plus_anchor, y_degree_lmt_plus_anchor);
    <b>let</b> degree_low_plus_anchor = <b>min</b>(x.exp_plus_anchor, y.exp_plus_anchor);
    <b>let</b> new_chunks = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> carry = 0;
    <b>let</b> i = degree_low_plus_anchor;
    <b>while</b> (i &lt; degree_high_plus_anchor) {
        <b>let</b> chunk_0 = <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(x, i);
        <b>let</b> chunk_1 = <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(&y, i);
        <b>let</b> new_val = (chunk_0 <b>as</b> u128) + (chunk_1 <b>as</b> u128) + carry;
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_chunks, ((new_val & <a href="arbitrary_precision.md#0x1_arbitrary_precision_U64_MASK">U64_MASK</a>) <b>as</b> u64));
        carry = new_val &gt;&gt; 64;
        i = i + 1;
    };
    <b>if</b> (carry &gt; 0) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_chunks, (carry <b>as</b> u64));
    };

    *x = <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: new_chunks,
        exp_plus_anchor: degree_low_plus_anchor,
    };
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_sum"></a>

## Function `sum`

Compute <code>v[0]+...+v[n-1]</code> for a list of <code>n</code> values <code>v[]</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_sum">sum</a>(v: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>&gt;): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_sum">sum</a>(v: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>&gt;): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> accumulator = <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(0);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(v, |item|{
        <b>let</b> item: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> = item;
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(&<b>mut</b> accumulator, item);
    });
    accumulator
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_sub"></a>

## Function `sub`

Compute <code>a - b</code>. Abort if <code>a &lt; b</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_sub">sub</a>(a: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, b: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_sub">sub</a>(a: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, b: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> a_degree_lmt_plus_anchor = a.exp_plus_anchor + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&a.chunks);
    <b>let</b> b_degree_lmt_plus_anchor = b.exp_plus_anchor + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&b.chunks);
    <b>let</b> degree_high_plus_anchor = max(a_degree_lmt_plus_anchor, b_degree_lmt_plus_anchor);
    <b>let</b> degree_low_plus_anchor = <b>min</b>(a.exp_plus_anchor, b.exp_plus_anchor);
    <b>let</b> i = degree_low_plus_anchor;
    <b>let</b> borrowed = 0;
    <b>let</b> new_chunks = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>while</b> (i &lt; degree_high_plus_anchor) {
        <b>let</b> chunk_a = (<a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(&a, i) <b>as</b> u128);
        <b>let</b> chunk_b = (<a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(&b, i) <b>as</b> u128);
        <b>let</b> new_chunk = chunk_a + (1 &lt;&lt; 64) - chunk_b - borrowed;
        borrowed = 1 - (new_chunk &gt;&gt; 64);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_chunks, ((new_chunk & <a href="arbitrary_precision.md#0x1_arbitrary_precision_U64_MASK">U64_MASK</a>) <b>as</b> u64));
        i = i + 1;
    };
    <b>assert</b>!(borrowed == 0, 9990);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: new_chunks,
        exp_plus_anchor: degree_low_plus_anchor,
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_log2_floor"></a>

## Function `log2_floor`

find <code>p</code> such that <code>2^p &lt;= x &lt; 2^{p+1}</code>.
If <code>p &gt;= 0</code>, return <code>(p, 0)</code>; otherwise, return <code>(0, -p)</code>.
Abort if <code>x = 0</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_log2_floor">log2_floor</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_log2_floor">log2_floor</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): (u64, u64) {
    <b>let</b> n = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; n) {
        <b>let</b> chunk = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&x.chunks, n-1-i);
        <b>if</b> (chunk &gt; 0) {
            <b>let</b> bit_offset = (<a href="math64.md#0x1_math64_floor_log2">math64::floor_log2</a>(chunk) <b>as</b> u64);
            <b>if</b> (n-1-i+x.exp_plus_anchor &gt;= <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>) {
                <b>let</b> chunk_offset = n - 1 - i + x.exp_plus_anchor - <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>;
                <b>return</b> (chunk_offset * 64 + bit_offset, 0)
            } <b>else</b> {
                <b>let</b> minus_chunk_offset = <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - (n - 1 - i + x.exp_plus_anchor);
                <b>return</b> (0, minus_chunk_offset * 64 - bit_offset)
            }
        };
        i = i + 1;
    };
    <b>abort</b>(999)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_round"></a>

## Function `round`

Round <code>x</code> to the nearest multiplier of <code>unit</code>.
<code>unit</code> must be a power of 2.
<code>(k+1/2)*unit</code> will be rounded to <code>(k+1)*unit</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_round">round</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, unit: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_round">round</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, unit: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> (shift_up, shift_down) = <a href="arbitrary_precision.md#0x1_arbitrary_precision_log2_floor">log2_floor</a>(&unit);
    // Ensure increment is a power of 2.
    <b>let</b> offset_plus_anchor = <b>if</b> (shift_down == 0) {
        shift_up + <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>
    } <b>else</b> {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - shift_down
    };
    <b>let</b> neg_offset_plus_anchor = (((<a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> <b>as</b> u128) * 2 - (offset_plus_anchor <b>as</b> u128)) <b>as</b> u64);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_bit_assign">shift_by_bit_assign</a>(&<b>mut</b> x, neg_offset_plus_anchor);
    <b>let</b> (int, frac) = <a href="arbitrary_precision.md#0x1_arbitrary_precision_split_by_point">split_by_point</a>(x);
    <b>let</b> half = <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(0, 1);
    <b>let</b> carry_or_not = <b>if</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_less_than">less_than</a>(&frac, &half)) {
        0
    } <b>else</b> {
        1
    };
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(&<b>mut</b> int, <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(carry_or_not));
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_by_bit_assign">shift_by_bit_assign</a>(&<b>mut</b> int, offset_plus_anchor);
    int
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_exp2"></a>

## Function `exp2`

Compute <code>2^k</code> for an integer <code>k</code>.
To specify a non-negative <code>k</code>, set <code>maybe_k=k, maybe_neg_k=0</code>.
To specify a negative <code>k</code>, set <code>maybe_k=0, maybe_neg_k=-k</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(maybe_k: u64, maybe_neg_k: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(maybe_k: u64, maybe_neg_k: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>assert</b>!(maybe_k == 0 || maybe_neg_k == 0, 9991);
    <b>let</b> ret = <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(1);
    <b>if</b> (maybe_neg_k == 0) {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(&<b>mut</b> ret, maybe_k);
    } <b>else</b> {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit_assign">shift_down_by_bit_assign</a>(&<b>mut</b> ret, maybe_neg_k);
    };
    ret
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_from_bin_repr"></a>

## Function `from_bin_repr`

Construct from a binary representation.
A binary representation is a byte array that contains only '0', '1' and at most 1 '.'.
E.g.,
- b"" => 0
- b"." => 0
- b"10.01" => 2.25
- b".001" => 1/8
- b"111." => 7
- b"01111" => 15


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_bin_repr">from_bin_repr</a>(repr: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_bin_repr">from_bin_repr</a>(repr: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> is_int_part = <b>true</b>;
    <b>let</b> frac_digits = 0;
    <b>let</b> res = <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(0);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(repr, |chr|{
        <b>if</b> (is_int_part) {
            <b>if</b> (chr == 46) {
                is_int_part = <b>false</b>;
            } <b>else</b> <b>if</b> (chr == 48) {
                <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(&<b>mut</b> res, 1);
            } <b>else</b> <b>if</b> (chr == 49) {
                <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(&<b>mut</b> res, 1);
                <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(&<b>mut</b> res, <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(1));
            } <b>else</b> {
                <b>abort</b>(9990)
            }
        } <b>else</b> {
            frac_digits = frac_digits + 1;
            <b>if</b> (chr == 48) {
                // Nothing <b>to</b> do.
            } <b>else</b> <b>if</b> (chr == 49) {
                <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(&<b>mut</b> res, <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(0, frac_digits));
            } <b>else</b> {
                <b>abort</b>(9991)
            }
        }
    });
    res
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_split_by_point"></a>

## Function `split_by_point`

Given <code>x=a.b</code>, return <code>a</code> and <code>0.b</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_split_by_point">split_by_point</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): (<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_split_by_point">split_by_point</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): (<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>) {
    <b>if</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> &lt; x.exp_plus_anchor) {
        <b>return</b> (x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(0))
    };
    <b>let</b> chunk_0_pos = <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - x.exp_plus_anchor;
    <b>let</b> num_chunks = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks);
    <b>if</b> (chunk_0_pos &gt;= num_chunks) {
        <b>return</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(0), x)
    };

    <b>let</b> int = <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&x.chunks, chunk_0_pos, num_chunks),
        exp_plus_anchor: <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>,
    };

    <b>let</b> frac = <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&x.chunks, 0, chunk_0_pos),
        exp_plus_anchor: <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - chunk_0_pos,
    };

    (int, frac)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_is_zero"></a>

## Function `is_zero`

Check if <code>x = 0</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_is_zero">is_zero</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_is_zero">is_zero</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): bool {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_all">vector::all</a>(&x.chunks, |chunk|{ <b>let</b> chunk: u64 = *chunk; chunk == 0})
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_ceil"></a>

## Function `ceil`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_ceil">ceil</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_ceil">ceil</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> half = <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(0, 1);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_add_assign">add_assign</a>(&<b>mut</b> x, half);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_round">round</a>(x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(1))
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_from_u64"></a>

## Function `from_u64`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(val: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(val: u64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[val],
        exp_plus_anchor: <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>,
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_from_u128"></a>

## Function `from_u128`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u128">from_u128</a>(val: u128): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u128">from_u128</a>(val: u128): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> chunk_0 = ((val & <a href="arbitrary_precision.md#0x1_arbitrary_precision_U64_MASK">U64_MASK</a>) <b>as</b> u64);
    <b>let</b> chunk_1 = ((val &gt;&gt; 64) <b>as</b> u64);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[chunk_0, chunk_1,],
        exp_plus_anchor: <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>,
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_from_fixed_point64"></a>

## Function `from_fixed_point64`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_fixed_point64">from_fixed_point64</a>(val: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_fixed_point64">from_fixed_point64</a>(val: FixedPoint64): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>let</b> raw = <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(val);
    <b>let</b> chunk_0 = ((raw & <a href="arbitrary_precision.md#0x1_arbitrary_precision_U64_MASK">U64_MASK</a>) <b>as</b> u64);
    <b>let</b> chunk_1 = ((raw &gt;&gt; 64) <b>as</b> u64);
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[chunk_0, chunk_1],
        exp_plus_anchor: <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - 1,
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_get_chunk"></a>

## Function `get_chunk`



<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, degree_plus_anchor: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, degree_plus_anchor: u64): u64 {
    <b>if</b> (degree_plus_anchor &lt; x.exp_plus_anchor) <b>return</b> 0;
    <b>let</b> pos_in_arr = degree_plus_anchor - x.exp_plus_anchor;
    <b>if</b> (pos_in_arr &gt;= <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks)) <b>return</b> 0;
    *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&x.chunks, pos_in_arr)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_cmp"></a>

## Function `cmp`



<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_cmp">cmp</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_cmp">cmp</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): u64 {
    <b>let</b> x_degree_lmt_plus_anchor = x.exp_plus_anchor + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks);
    <b>let</b> y_degree_lmt_plus_anchor = y.exp_plus_anchor + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&y.chunks);
    <b>let</b> degree_high_plus_anchor = max(x_degree_lmt_plus_anchor, y_degree_lmt_plus_anchor);
    <b>let</b> degree_low_plus_anchor = <b>min</b>(x.exp_plus_anchor, y.exp_plus_anchor);
    <b>let</b> i = degree_high_plus_anchor;
    <b>while</b> (i &gt;= degree_low_plus_anchor) {
        <b>let</b> chunk_x = <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(x, i);
        <b>let</b> chunk_y = <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(y, i);
        <b>if</b> (chunk_x &lt; chunk_y) <b>return</b> 9;
        <b>if</b> (chunk_x &gt; chunk_y) <b>return</b> 11;
        i = i - 1;
    };

    10
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_greater_than"></a>

## Function `greater_than`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_greater_than">greater_than</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_greater_than">greater_than</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): bool {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_cmp">cmp</a>(x, y) &gt; 10
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_less_than"></a>

## Function `less_than`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_less_than">less_than</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_less_than">less_than</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): bool {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_cmp">cmp</a>(x, y) &lt; 10
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_eq"></a>

## Function `eq`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_eq">eq</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_eq">eq</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, y: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): bool {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_cmp">cmp</a>(x, y) == 10
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_get_integer_chunk"></a>

## Function `get_integer_chunk`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_integer_chunk">get_integer_chunk</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_integer_chunk">get_integer_chunk</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, idx: u64): u64 {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> + idx)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_get_fractional_chunk"></a>

## Function `get_fractional_chunk`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_fractional_chunk">get_fractional_chunk</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_fractional_chunk">get_fractional_chunk</a>(x: &<a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, idx: u64): u64 {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a> - idx)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_as_u128"></a>

## Function `as_u128`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_as_u128">as_u128</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_as_u128">as_u128</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): u128 {
    <b>let</b> chunk_0 = (<a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(&x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>) <b>as</b> u128);
    <b>let</b> chunk_1 = (<a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(&x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>+1) <b>as</b> u128);
    chunk_0 + (chunk_1 &lt;&lt; 64)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_as_u64"></a>

## Function `as_u64`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_as_u64">as_u64</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_as_u64">as_u64</a>(x: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): u64 {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_get_chunk">get_chunk</a>(&x, <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>)
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_min_assign"></a>

## Function `min_assign`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_min_assign">min_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, other: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_min_assign">min_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, other: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>) {
    <b>if</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_less_than">less_than</a>(&other, self)) {
        *self = other;
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_floor_assign"></a>

## Function `floor_assign`



<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_floor_assign">floor_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_floor_assign">floor_assign</a>(self: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>) {
    <b>let</b> (int, _) = <a href="arbitrary_precision.md#0x1_arbitrary_precision_split_by_point">split_by_point</a>(*self);
    *self = int;
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_div_ceil"></a>

## Function `div_ceil`

Return integer <code>q</code> such that <code>q*d &gt;= n &gt; (q-1)*d</code>.


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_div_ceil">div_ceil</a>(n: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, d: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_div_ceil">div_ceil</a>(n: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>, d: <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <b>if</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_is_zero">is_zero</a>(&n)) {
        <b>return</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(0)
    };
    <b>let</b> one = <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(1);

    <b>let</b> (d_up, d_down) = <a href="arbitrary_precision.md#0x1_arbitrary_precision_log2_floor">log2_floor</a>(&d);
    <b>let</b> (n_up, n_down) = <a href="arbitrary_precision.md#0x1_arbitrary_precision_log2_floor">log2_floor</a>(&n);

    <b>let</b> hi = <b>if</b> (d_down == 0 && n_down == 0) {
        <b>if</b> (n_up &gt;= d_up) {
            <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(n_up - d_up, 0)
        } <b>else</b> {
            <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(0, d_up - n_up)
        }
    } <b>else</b> <b>if</b> (d_down == 0 && n_up == 0) {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(0, d_up + n_down)
    } <b>else</b> <b>if</b> (d_up == 0 && n_down == 0) {
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(d_down + n_up, 0)
    } <b>else</b> {
        <b>if</b> (n_down &gt;= d_down) {
            <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(0, n_down - d_down)
        } <b>else</b> {
            <a href="arbitrary_precision.md#0x1_arbitrary_precision_exp2">exp2</a>(d_down - n_down, 0)
        }
    };
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit_assign">shift_up_by_bit_assign</a>(&<b>mut</b> hi, 1);
    <b>let</b> lo = <a href="arbitrary_precision.md#0x1_arbitrary_precision_from_u64">from_u64</a>(0);

    // Binary search for the quotient.
    // Invariant: `hi*d &gt;= n &gt; lo*d`.
    <b>while</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_greater_than">greater_than</a>(&<a href="arbitrary_precision.md#0x1_arbitrary_precision_sub">sub</a>(hi, lo), &one)) {
        <b>let</b> md = <a href="arbitrary_precision.md#0x1_arbitrary_precision_sum">sum</a>(<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[lo, hi]);
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit_assign">shift_down_by_bit_assign</a>(&<b>mut</b> md, 1);
        <a href="arbitrary_precision.md#0x1_arbitrary_precision_trim_zeros">trim_zeros</a>(&<b>mut</b> md);
        <b>let</b> prod = <a href="arbitrary_precision.md#0x1_arbitrary_precision_product">product</a>(<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[md, d]);
        <b>if</b> (<a href="arbitrary_precision.md#0x1_arbitrary_precision_greater_than">greater_than</a>(&n, &prod)) {
            lo = md;
        } <b>else</b> {
            hi = md;
        }
    };
    hi
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_default"></a>

## Function `default`



<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_default">default</a>(): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_default">default</a>(): <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
    <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a> {
        chunks: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        exp_plus_anchor: <a href="arbitrary_precision.md#0x1_arbitrary_precision_ANCHOR">ANCHOR</a>,
    }
}
</code></pre>



</details>

<a id="0x1_arbitrary_precision_trim_zeros"></a>

## Function `trim_zeros`



<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_trim_zeros">trim_zeros</a>(x: &<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_trim_zeros">trim_zeros</a>(x:&<b>mut</b> <a href="arbitrary_precision.md#0x1_arbitrary_precision_Number">Number</a>) {
    <b>let</b> n = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks);
    <b>let</b> i = n;
    <b>while</b> (i &gt; 0 && *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&x.chunks, i-1) == 0) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> x.chunks);
    };
    <b>let</b> k = 0;
    <b>let</b> n = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&x.chunks);
    <b>while</b> (k &lt; n && *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&x.chunks, k) == 0) {
        k = k + 1;
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> x.chunks);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; k) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> x.chunks);
        i = i + 1;
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> x.chunks);
    x.exp_plus_anchor = x.exp_plus_anchor + k;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
