
<a id="0x1_fixed_decimal"></a>

# Module `0x1::fixed_decimal`

Fixed-point decimal implementation, useful for financial applications where, for example, prices
need to be tracked between assets with disparate market values or decimal amounts.

Fixed-point decimal value are represented as a simple <code>u128</code> without a type wrapper, to optimize
performance. This enables, for example, prices to be arranged in total order within a sorted
collection, using simple arithmetic comparators for m-ary search tree traversal or similar.

This implementation provides enough precision such that an integer value of 1 multiplied by
the largest possible fixed-point decimal value (<code><a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a></code>) will result in the
largest possible power of of 10 that can fit in a <code>u64</code> (<code><a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u64">MAX_U64_DECIMAL_u64</a></code>). Conversely,
<code><a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u64">MAX_U64_DECIMAL_u64</a></code> multiplied by the smallest possible fixed-point decimal value (1 encoded
as a <code>u128</code>) will have a result of 1. For more, see <code><a href="fixed_decimal.md#0x1_fixed_decimal_scale_int">scale_int</a>()</code> and assocated tests.


-  [Constants](#@Constants_0)
-  [Function `get_MAX_U64_DECIMAL`](#0x1_fixed_decimal_get_MAX_U64_DECIMAL)
-  [Function `get_MAX_U64_DECIMAL_inline`](#0x1_fixed_decimal_get_MAX_U64_DECIMAL_inline)
-  [Function `get_MAX_DECIMAL_FIXED`](#0x1_fixed_decimal_get_MAX_DECIMAL_FIXED)
-  [Function `get_MAX_DECIMAL_FIXED_inline`](#0x1_fixed_decimal_get_MAX_DECIMAL_FIXED_inline)
-  [Function `from_int`](#0x1_fixed_decimal_from_int)
-  [Function `from_ratio`](#0x1_fixed_decimal_from_ratio)
-  [Function `add`](#0x1_fixed_decimal_add)
-  [Function `subtract`](#0x1_fixed_decimal_subtract)
-  [Function `multiply`](#0x1_fixed_decimal_multiply)
-  [Function `divide`](#0x1_fixed_decimal_divide)
-  [Function `scale_int`](#0x1_fixed_decimal_scale_int)
-  [Function `from_ratio_optimistic`](#0x1_fixed_decimal_from_ratio_optimistic)
-  [Function `scale_int_optimistic`](#0x1_fixed_decimal_scale_int_optimistic)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_fixed_decimal_E_DIVIDE_BY_ZERO"></a>

Dividing by zero is not permitted.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>: u64 = 6;
</code></pre>



<a id="0x1_fixed_decimal_E_FIXED_TOO_LARGE"></a>

Decimal fixed point input exceeded the maximum allowed value.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE">E_FIXED_TOO_LARGE</a>: u64 = 1;
</code></pre>



<a id="0x1_fixed_decimal_E_FIXED_TOO_LARGE_LHS"></a>

Decimal fixed point input on left hand side exceeded the maximum allowed value.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_LHS">E_FIXED_TOO_LARGE_LHS</a>: u64 = 3;
</code></pre>



<a id="0x1_fixed_decimal_E_FIXED_TOO_LARGE_RHS"></a>

Decimal fixed point input on right hand side exceeded the maximum allowed value.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_RHS">E_FIXED_TOO_LARGE_RHS</a>: u64 = 4;
</code></pre>



<a id="0x1_fixed_decimal_E_INT_TOO_LARGE"></a>

Integer input exceeded the largest power of 10 that can fit in a <code>u64</code>.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_INT_TOO_LARGE">E_INT_TOO_LARGE</a>: u64 = 0;
</code></pre>



<a id="0x1_fixed_decimal_E_OVERFLOW"></a>

The operation overflowed.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_OVERFLOW">E_OVERFLOW</a>: u64 = 2;
</code></pre>



<a id="0x1_fixed_decimal_E_UNDERFLOW"></a>

The operation underflowed.


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_E_UNDERFLOW">E_UNDERFLOW</a>: u64 = 5;
</code></pre>



<a id="0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128"></a>

Largest power of 10 that can fit in a <code>u64</code>, squared. In Python:

```python
import math
print(f"{(10 ** (int(math.log10(int('1' * 64, 2))))) ** 2:_}")
```


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>: u128 = 100000000000000000000000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_MAX_DECIMAL_FIXED_u256"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u256">MAX_DECIMAL_FIXED_u256</a>: u256 = 100000000000000000000000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_MAX_U64_DECIMAL_u128"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u128">MAX_U64_DECIMAL_u128</a>: u128 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_MAX_U64_DECIMAL_u256"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u256">MAX_U64_DECIMAL_u256</a>: u256 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_MAX_U64_DECIMAL_u64"></a>

Largest power of 10 that can fit in a <code>u64</code>. In Python:

```python
import math
print(f"{10 ** (int(math.log10(int('1' * 64, 2)))):_}")
```


<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u64">MAX_U64_DECIMAL_u64</a>: u64 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_SCALE_FACTOR_u128"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u128">SCALE_FACTOR_u128</a>: u128 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_SCALE_FACTOR_u256"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a>: u256 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_UNITY_u128"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_UNITY_u128">UNITY_u128</a>: u128 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_UNITY_u256"></a>



<pre><code><b>const</b> <a href="fixed_decimal.md#0x1_fixed_decimal_UNITY_u256">UNITY_u256</a>: u256 = 10000000000000000000;
</code></pre>



<a id="0x1_fixed_decimal_get_MAX_U64_DECIMAL"></a>

## Function `get_MAX_U64_DECIMAL`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_U64_DECIMAL">get_MAX_U64_DECIMAL</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_U64_DECIMAL">get_MAX_U64_DECIMAL</a>(): u64 { <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u64">MAX_U64_DECIMAL_u64</a> }
</code></pre>



</details>

<a id="0x1_fixed_decimal_get_MAX_U64_DECIMAL_inline"></a>

## Function `get_MAX_U64_DECIMAL_inline`



<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_U64_DECIMAL_inline">get_MAX_U64_DECIMAL_inline</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_U64_DECIMAL_inline">get_MAX_U64_DECIMAL_inline</a>(): u64 { 10_000_000_000_000_000_000 }
</code></pre>



</details>

<a id="0x1_fixed_decimal_get_MAX_DECIMAL_FIXED"></a>

## Function `get_MAX_DECIMAL_FIXED`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_DECIMAL_FIXED">get_MAX_DECIMAL_FIXED</a>(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_DECIMAL_FIXED">get_MAX_DECIMAL_FIXED</a>(): u128 { <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a> }
</code></pre>



</details>

<a id="0x1_fixed_decimal_get_MAX_DECIMAL_FIXED_inline"></a>

## Function `get_MAX_DECIMAL_FIXED_inline`



<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_DECIMAL_FIXED_inline">get_MAX_DECIMAL_FIXED_inline</a>(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_get_MAX_DECIMAL_FIXED_inline">get_MAX_DECIMAL_FIXED_inline</a>(): u128 {
    100_000_000_000_000_000_000_000_000_000_000_000_000
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_from_int"></a>

## Function `from_int`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_from_int">from_int</a>(int: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_from_int">from_int</a>(int: u64): u128 {
    <b>assert</b>!(int &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u64">MAX_U64_DECIMAL_u64</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_INT_TOO_LARGE">E_INT_TOO_LARGE</a>);
    (int <b>as</b> u128) * (<a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u128">SCALE_FACTOR_u128</a>)
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_from_ratio"></a>

## Function `from_ratio`

Inputs do not necessarily need to be within max representable <code>u64</code> value bounds. See tests.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_from_ratio">from_ratio</a>(numerator: u64, denominator: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_from_ratio">from_ratio</a>(numerator: u64, denominator: u64): u128 {
    <b>assert</b>!(denominator != 0, <a href="fixed_decimal.md#0x1_fixed_decimal_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> result = (numerator <b>as</b> u256) * (<a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a>) / (denominator <b>as</b> u256);
    <b>assert</b>!(result &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u256">MAX_DECIMAL_FIXED_u256</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_OVERFLOW">E_OVERFLOW</a>);
    (result <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_add"></a>

## Function `add`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_add">add</a>(fixed_l: u128, fixed_r: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_add">add</a>(fixed_l: u128, fixed_r: u128): u128 {
    <b>assert</b>!(fixed_l &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_LHS">E_FIXED_TOO_LARGE_LHS</a>);
    <b>assert</b>!(fixed_r &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_RHS">E_FIXED_TOO_LARGE_RHS</a>);
    <b>let</b> result = fixed_l + fixed_r;
    <b>assert</b>!(result &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_OVERFLOW">E_OVERFLOW</a>);
    result
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_subtract"></a>

## Function `subtract`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_subtract">subtract</a>(fixed_l: u128, fixed_r: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_subtract">subtract</a>(fixed_l: u128, fixed_r: u128): u128 {
    <b>assert</b>!(fixed_l &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_LHS">E_FIXED_TOO_LARGE_LHS</a>);
    <b>assert</b>!(fixed_r &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_RHS">E_FIXED_TOO_LARGE_RHS</a>);
    <b>assert</b>!(fixed_l &gt;= fixed_r, <a href="fixed_decimal.md#0x1_fixed_decimal_E_UNDERFLOW">E_UNDERFLOW</a>);
    fixed_l - fixed_r
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_multiply"></a>

## Function `multiply`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_multiply">multiply</a>(fixed_l: u128, fixed_r: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_multiply">multiply</a>(fixed_l: u128, fixed_r: u128): u128 {
    <b>assert</b>!(fixed_l &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_LHS">E_FIXED_TOO_LARGE_LHS</a>);
    <b>assert</b>!(fixed_r &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_RHS">E_FIXED_TOO_LARGE_RHS</a>);
    <b>let</b> result = (fixed_l <b>as</b> u256) * (fixed_r <b>as</b> u256) / (<a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a>);
    <b>assert</b>!(result &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u256">MAX_DECIMAL_FIXED_u256</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_OVERFLOW">E_OVERFLOW</a>);
    (result <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_divide"></a>

## Function `divide`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_divide">divide</a>(fixed_l: u128, fixed_r: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_divide">divide</a>(fixed_l: u128, fixed_r: u128): u128 {
    <b>assert</b>!(fixed_l &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_LHS">E_FIXED_TOO_LARGE_LHS</a>);
    <b>assert</b>!(fixed_r &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE_RHS">E_FIXED_TOO_LARGE_RHS</a>);
    <b>assert</b>!(fixed_r != 0, <a href="fixed_decimal.md#0x1_fixed_decimal_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>let</b> result = (fixed_l <b>as</b> u256) * <a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a> / (fixed_r <b>as</b> u256);
    <b>assert</b>!(result &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u256">MAX_DECIMAL_FIXED_u256</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_OVERFLOW">E_OVERFLOW</a>);
    (result <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_scale_int"></a>

## Function `scale_int`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_scale_int">scale_int</a>(int: u64, fixed: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_scale_int">scale_int</a>(int: u64, fixed: u128): u64 {
    <b>assert</b>!(int &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u64">MAX_U64_DECIMAL_u64</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_INT_TOO_LARGE">E_INT_TOO_LARGE</a>);
    <b>assert</b>!(fixed &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u128">MAX_DECIMAL_FIXED_u128</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_FIXED_TOO_LARGE">E_FIXED_TOO_LARGE</a>);
    <b>let</b> result = ((int <b>as</b> u256) * (fixed <b>as</b> u256)) / <a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a>;
    <b>assert</b>!(result &lt;= <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u256">MAX_U64_DECIMAL_u256</a>, <a href="fixed_decimal.md#0x1_fixed_decimal_E_OVERFLOW">E_OVERFLOW</a>);
    (result <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_from_ratio_optimistic"></a>

## Function `from_ratio_optimistic`

For when division by zero will not happen, but overflow might. A performance optimization
that enables low-cost checks from calling functions.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_from_ratio_optimistic">from_ratio_optimistic</a>(numerator: u64, denominator: u64): (u256, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_from_ratio_optimistic">from_ratio_optimistic</a>(numerator: u64, denominator: u64): (u256, bool) {
    <b>let</b> result = (numerator <b>as</b> u256) * (<a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a>) / (denominator <b>as</b> u256);
    (
        result, // Value before casting back <b>to</b> `u128`.
        // True <b>if</b> result overflows a fixed decimal.
        result &gt; <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_DECIMAL_FIXED_u256">MAX_DECIMAL_FIXED_u256</a>,
    )
}
</code></pre>



</details>

<a id="0x1_fixed_decimal_scale_int_optimistic"></a>

## Function `scale_int_optimistic`

For when integer and fixed decimal inputs are valid, but the result might overflow or
truncate. A performance optimization that enables low-cost checks from calling functions.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_scale_int_optimistic">scale_int_optimistic</a>(int: u64, fixed: u128): (u256, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="fixed_decimal.md#0x1_fixed_decimal_scale_int_optimistic">scale_int_optimistic</a>(int: u64, fixed: u128): (u256, bool) {
    <b>let</b> result = ((int <b>as</b> u256) * (fixed <b>as</b> u256)) / <a href="fixed_decimal.md#0x1_fixed_decimal_SCALE_FACTOR_u256">SCALE_FACTOR_u256</a>;
    (
        result, // Value before casting back <b>to</b> `u64`.
        // True <b>if</b> result exceeds maximum representable power of ten for a `u64`.
        result &gt; <a href="fixed_decimal.md#0x1_fixed_decimal_MAX_U64_DECIMAL_u256">MAX_U64_DECIMAL_u256</a>,
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
