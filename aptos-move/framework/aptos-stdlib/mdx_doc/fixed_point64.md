
<a id="0x1_fixed_point64"></a>

# Module `0x1::fixed_point64`

Defines a fixed&#45;point numeric type with a 64&#45;bit integer part and
a 64&#45;bit fractional part.


-  [Struct `FixedPoint64`](#0x1_fixed_point64_FixedPoint64)
-  [Constants](#@Constants_0)
-  [Function `sub`](#0x1_fixed_point64_sub)
-  [Function `add`](#0x1_fixed_point64_add)
-  [Function `multiply_u128`](#0x1_fixed_point64_multiply_u128)
-  [Function `divide_u128`](#0x1_fixed_point64_divide_u128)
-  [Function `create_from_rational`](#0x1_fixed_point64_create_from_rational)
-  [Function `create_from_raw_value`](#0x1_fixed_point64_create_from_raw_value)
-  [Function `get_raw_value`](#0x1_fixed_point64_get_raw_value)
-  [Function `is_zero`](#0x1_fixed_point64_is_zero)
-  [Function `min`](#0x1_fixed_point64_min)
-  [Function `max`](#0x1_fixed_point64_max)
-  [Function `less_or_equal`](#0x1_fixed_point64_less_or_equal)
-  [Function `less`](#0x1_fixed_point64_less)
-  [Function `greater_or_equal`](#0x1_fixed_point64_greater_or_equal)
-  [Function `greater`](#0x1_fixed_point64_greater)
-  [Function `equal`](#0x1_fixed_point64_equal)
-  [Function `almost_equal`](#0x1_fixed_point64_almost_equal)
-  [Function `create_from_u128`](#0x1_fixed_point64_create_from_u128)
-  [Function `floor`](#0x1_fixed_point64_floor)
-  [Function `ceil`](#0x1_fixed_point64_ceil)
-  [Function `round`](#0x1_fixed_point64_round)
-  [Specification](#@Specification_1)
    -  [Function `sub`](#@Specification_1_sub)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `multiply_u128`](#@Specification_1_multiply_u128)
    -  [Function `divide_u128`](#@Specification_1_divide_u128)
    -  [Function `create_from_rational`](#@Specification_1_create_from_rational)
    -  [Function `create_from_raw_value`](#@Specification_1_create_from_raw_value)
    -  [Function `min`](#@Specification_1_min)
    -  [Function `max`](#@Specification_1_max)
    -  [Function `less_or_equal`](#@Specification_1_less_or_equal)
    -  [Function `less`](#@Specification_1_less)
    -  [Function `greater_or_equal`](#@Specification_1_greater_or_equal)
    -  [Function `greater`](#@Specification_1_greater)
    -  [Function `equal`](#@Specification_1_equal)
    -  [Function `almost_equal`](#@Specification_1_almost_equal)
    -  [Function `create_from_u128`](#@Specification_1_create_from_u128)
    -  [Function `floor`](#@Specification_1_floor)
    -  [Function `ceil`](#@Specification_1_ceil)
    -  [Function `round`](#@Specification_1_round)


<pre><code></code></pre>



<a id="0x1_fixed_point64_FixedPoint64"></a>

## Struct `FixedPoint64`

Define a fixed&#45;point numeric type with 64 fractional bits.
This is just a u128 integer but it is wrapped in a struct to
make a unique type. This is a binary representation, so decimal
values may not be exactly representable, but it provides more
than 9 decimal digits of precision both before and after the
decimal point (18 digits total). For comparison, double precision
floating&#45;point has less than 16 decimal digits of precision, so
be careful about using floating&#45;point to convert these values to
decimal.


<pre><code><b>struct</b> <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_fixed_point64_MAX_U128"></a>



<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>: u256 &#61; 340282366920938463463374607431768211455;<br /></code></pre>



<a id="0x1_fixed_point64_EDENOMINATOR"></a>

The denominator provided was zero


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EDENOMINATOR">EDENOMINATOR</a>: u64 &#61; 65537;<br /></code></pre>



<a id="0x1_fixed_point64_EDIVISION"></a>

The quotient value would be too large to be held in a <code>u128</code>


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION">EDIVISION</a>: u64 &#61; 131074;<br /></code></pre>



<a id="0x1_fixed_point64_EDIVISION_BY_ZERO"></a>

A division by zero was encountered


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>: u64 &#61; 65540;<br /></code></pre>



<a id="0x1_fixed_point64_EMULTIPLICATION"></a>

The multiplied value would be too large to be held in a <code>u128</code>


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EMULTIPLICATION">EMULTIPLICATION</a>: u64 &#61; 131075;<br /></code></pre>



<a id="0x1_fixed_point64_ENEGATIVE_RESULT"></a>

Abort code on calculation result is negative.


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_ENEGATIVE_RESULT">ENEGATIVE_RESULT</a>: u64 &#61; 65542;<br /></code></pre>



<a id="0x1_fixed_point64_ERATIO_OUT_OF_RANGE"></a>

The computed ratio when converting to a <code><a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a></code> would be unrepresentable


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>: u64 &#61; 131077;<br /></code></pre>



<a id="0x1_fixed_point64_sub"></a>

## Function `sub`

Returns x &#45; y. x must be not less than y.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_sub">sub</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_sub">sub</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    <b>let</b> x_raw &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(x);<br />    <b>let</b> y_raw &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(y);<br />    <b>assert</b>!(x_raw &gt;&#61; y_raw, <a href="fixed_point64.md#0x1_fixed_point64_ENEGATIVE_RESULT">ENEGATIVE_RESULT</a>);<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(x_raw &#45; y_raw)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_add"></a>

## Function `add`

Returns x &#43; y. The result cannot be greater than MAX_U128.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_add">add</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_add">add</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    <b>let</b> x_raw &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(x);<br />    <b>let</b> y_raw &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(y);<br />    <b>let</b> result &#61; (x_raw <b>as</b> u256) &#43; (y_raw <b>as</b> u256);<br />    <b>assert</b>!(result &lt;&#61; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>((result <b>as</b> u128))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_multiply_u128"></a>

## Function `multiply_u128`

Multiply a u128 integer by a fixed&#45;point number, truncating any
fractional part of the product. This will abort if the product
overflows.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_multiply_u128">multiply_u128</a>(val: u128, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_multiply_u128">multiply_u128</a>(val: u128, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />    // The product of two 128 bit values <b>has</b> 256 bits, so perform the<br />    // multiplication <b>with</b> u256 types and keep the full 256 bit product<br />    // <b>to</b> avoid losing accuracy.<br />    <b>let</b> unscaled_product &#61; (val <b>as</b> u256) &#42; (multiplier.value <b>as</b> u256);<br />    // The unscaled product <b>has</b> 64 fractional bits (from the multiplier)<br />    // so rescale it by shifting away the low bits.<br />    <b>let</b> product &#61; unscaled_product &gt;&gt; 64;<br />    // Check whether the value is too large.<br />    <b>assert</b>!(product &lt;&#61; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_EMULTIPLICATION">EMULTIPLICATION</a>);<br />    (product <b>as</b> u128)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_divide_u128"></a>

## Function `divide_u128`

Divide a u128 integer by a fixed&#45;point number, truncating any
fractional part of the quotient. This will abort if the divisor
is zero or if the quotient overflows.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_divide_u128">divide_u128</a>(val: u128, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_divide_u128">divide_u128</a>(val: u128, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />    // Check for division by zero.<br />    <b>assert</b>!(divisor.value !&#61; 0, <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>);<br />    // First convert <b>to</b> 256 bits and then shift left <b>to</b><br />    // add 64 fractional zero bits <b>to</b> the dividend.<br />    <b>let</b> scaled_value &#61; (val <b>as</b> u256) &lt;&lt; 64;<br />    <b>let</b> quotient &#61; scaled_value / (divisor.value <b>as</b> u256);<br />    // Check whether the value is too large.<br />    <b>assert</b>!(quotient &lt;&#61; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION">EDIVISION</a>);<br />    // the value may be too large, which will cause the cast <b>to</b> fail<br />    // <b>with</b> an arithmetic <a href="../../move-stdlib/doc/error.md#0x1_error">error</a>.<br />    (quotient <b>as</b> u128)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_create_from_rational"></a>

## Function `create_from_rational`

Create a fixed&#45;point value from a rational number specified by its
numerator and denominator. Calling this function should be preferred
for using <code><a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">Self::create_from_raw_value</a></code> which is also available.
This will abort if the denominator is zero. It will also
abort if the numerator is nonzero and the ratio is not in the range
2^&#45;64 .. 2^64&#45;1. When specifying decimal fractions, be careful about
rounding errors: if you round to display N digits after the decimal
point, you can use a denominator of 10^N to avoid numbers where the
very small imprecision in the binary representation could change the
rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_rational">create_from_rational</a>(numerator: u128, denominator: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_rational">create_from_rational</a>(numerator: u128, denominator: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    // If the denominator is zero, this will <b>abort</b>.<br />    // Scale the numerator <b>to</b> have 64 fractional bits, so that the quotient will have 64<br />    // fractional bits.<br />    <b>let</b> scaled_numerator &#61; (numerator <b>as</b> u256) &lt;&lt; 64;<br />    <b>assert</b>!(denominator !&#61; 0, <a href="fixed_point64.md#0x1_fixed_point64_EDENOMINATOR">EDENOMINATOR</a>);<br />    <b>let</b> quotient &#61; scaled_numerator / (denominator <b>as</b> u256);<br />    <b>assert</b>!(quotient !&#61; 0 &#124;&#124; numerator &#61;&#61; 0, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    // Return the quotient <b>as</b> a fixed&#45;point number. We first need <b>to</b> check whether the cast<br />    // can succeed.<br />    <b>assert</b>!(quotient &lt;&#61; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123; value: (quotient <b>as</b> u128) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_create_from_raw_value"></a>

## Function `create_from_raw_value`

Create a fixedpoint value from a raw value.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(value: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(value: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123; value &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_get_raw_value"></a>

## Function `get_raw_value`

Accessor for the raw u128 value. Other less common operations, such as
adding or subtracting FixedPoint64 values, can be done using the raw
values directly.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />    num.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_is_zero"></a>

## Function `is_zero`

Returns true if the ratio is zero.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_is_zero">is_zero</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_is_zero">is_zero</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    num.value &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_min"></a>

## Function `min`

Returns the smaller of the two FixedPoint64 numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    <b>if</b> (num1.value &lt; num2.value) &#123;<br />        num1<br />    &#125; <b>else</b> &#123;<br />        num2<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_max"></a>

## Function `max`

Returns the larger of the two FixedPoint64 numbers.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_max">max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_max">max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    <b>if</b> (num1.value &gt; num2.value) &#123;<br />        num1<br />    &#125; <b>else</b> &#123;<br />        num2<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_less_or_equal"></a>

## Function `less_or_equal`

Returns true if num1 &lt;&#61; num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less_or_equal">less_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less_or_equal">less_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    num1.value &lt;&#61; num2.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_less"></a>

## Function `less`

Returns true if num1 &lt; num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less">less</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less">less</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    num1.value &lt; num2.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_greater_or_equal"></a>

## Function `greater_or_equal`

Returns true if num1 &gt;&#61; num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater_or_equal">greater_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater_or_equal">greater_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    num1.value &gt;&#61; num2.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_greater"></a>

## Function `greater`

Returns true if num1 &gt; num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater">greater</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater">greater</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    num1.value &gt; num2.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_equal"></a>

## Function `equal`

Returns true if num1 &#61; num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_equal">equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_equal">equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    num1.value &#61;&#61; num2.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_almost_equal"></a>

## Function `almost_equal`

Returns true if num1 almost equals to num2, which means abs(num1&#45;num2) &lt;&#61; precision


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_almost_equal">almost_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_almost_equal">almost_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />    <b>if</b> (num1.value &gt; num2.value) &#123;<br />        (num1.value &#45; num2.value &lt;&#61; precision.value)<br />    &#125; <b>else</b> &#123;<br />        (num2.value &#45; num1.value &lt;&#61; precision.value)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_create_from_u128"></a>

## Function `create_from_u128`

Create a fixedpoint value from a u128 value.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_u128">create_from_u128</a>(val: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_u128">create_from_u128</a>(val: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />    <b>let</b> value &#61; (val <b>as</b> u256) &lt;&lt; 64;<br />    <b>assert</b>!(value &lt;&#61; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;value: (value <b>as</b> u128)&#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_floor"></a>

## Function `floor`

Returns the largest integer less than or equal to a given number.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />    num.value &gt;&gt; 64<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_ceil"></a>

## Function `ceil`

Rounds up the given FixedPoint64 to the next largest integer.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />    <b>let</b> floored_num &#61; <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(num) &lt;&lt; 64;<br />    <b>if</b> (num.value &#61;&#61; floored_num) &#123;<br />        <b>return</b> floored_num &gt;&gt; 64<br />    &#125;;<br />    <b>let</b> val &#61; ((floored_num <b>as</b> u256) &#43; (1 &lt;&lt; 64));<br />    (val &gt;&gt; 64 <b>as</b> u128)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point64_round"></a>

## Function `round`

Returns the value of a FixedPoint64 to the nearest integer.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_round">round</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_round">round</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />    <b>let</b> floored_num &#61; <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(num) &lt;&lt; 64;<br />    <b>let</b> boundary &#61; floored_num &#43; ((1 &lt;&lt; 64) / 2);<br />    <b>if</b> (num.value &lt; boundary) &#123;<br />        floored_num &gt;&gt; 64<br />    &#125; <b>else</b> &#123;<br />        <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(num)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_sub">sub</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> x.value &lt; y.value <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ENEGATIVE_RESULT">ENEGATIVE_RESULT</a>;<br /><b>ensures</b> result.value &#61;&#61; x.value &#45; y.value;<br /></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_add">add</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> (x.value <b>as</b> u256) &#43; (y.value <b>as</b> u256) &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;<br /><b>ensures</b> result.value &#61;&#61; x.value &#43; y.value;<br /></code></pre>



<a id="@Specification_1_multiply_u128"></a>

### Function `multiply_u128`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_multiply_u128">multiply_u128</a>(val: u128, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_MultiplyAbortsIf">MultiplyAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_multiply_u128">spec_multiply_u128</a>(val, multiplier);<br /></code></pre>




<a id="0x1_fixed_point64_MultiplyAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_MultiplyAbortsIf">MultiplyAbortsIf</a> &#123;<br />val: num;<br />multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>;<br /><b>aborts_if</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_multiply_u128">spec_multiply_u128</a>(val, multiplier) &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EMULTIPLICATION">EMULTIPLICATION</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point64_spec_multiply_u128"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_multiply_u128">spec_multiply_u128</a>(val: num, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): num &#123;<br />   (val &#42; multiplier.value) &gt;&gt; 64<br />&#125;<br /></code></pre>



<a id="@Specification_1_divide_u128"></a>

### Function `divide_u128`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_divide_u128">divide_u128</a>(val: u128, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_DivideAbortsIf">DivideAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_divide_u128">spec_divide_u128</a>(val, divisor);<br /></code></pre>




<a id="0x1_fixed_point64_DivideAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_DivideAbortsIf">DivideAbortsIf</a> &#123;<br />val: num;<br />divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>;<br /><b>aborts_if</b> divisor.value &#61;&#61; 0 <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>;<br /><b>aborts_if</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_divide_u128">spec_divide_u128</a>(val, divisor) &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION">EDIVISION</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point64_spec_divide_u128"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_divide_u128">spec_divide_u128</a>(val: num, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): num &#123;<br />   (val &lt;&lt; 64) / divisor.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_from_rational"></a>

### Function `create_from_rational`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_rational">create_from_rational</a>(numerator: u128, denominator: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>pragma</b> verify_duration_estimate &#61; 1000;<br /><b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromRationalAbortsIf">CreateFromRationalAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_rational">spec_create_from_rational</a>(numerator, denominator);<br /></code></pre>




<a id="0x1_fixed_point64_CreateFromRationalAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromRationalAbortsIf">CreateFromRationalAbortsIf</a> &#123;<br />numerator: u128;<br />denominator: u128;<br /><b>let</b> scaled_numerator &#61; (numerator <b>as</b> u256)&lt;&lt; 64;<br /><b>let</b> scaled_denominator &#61; (denominator <b>as</b> u256);<br /><b>let</b> quotient &#61; scaled_numerator / scaled_denominator;<br /><b>aborts_if</b> scaled_denominator &#61;&#61; 0 <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EDENOMINATOR">EDENOMINATOR</a>;<br /><b>aborts_if</b> quotient &#61;&#61; 0 &amp;&amp; scaled_numerator !&#61; 0 <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;<br /><b>aborts_if</b> quotient &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point64_spec_create_from_rational"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_rational">spec_create_from_rational</a>(numerator: num, denominator: num): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />   <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>&#123;value: (numerator &lt;&lt; 128) / (denominator &lt;&lt; 64)&#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_from_raw_value"></a>

### Function `create_from_raw_value`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(value: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result.value &#61;&#61; value;<br /></code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_min">spec_min</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_min"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_min">spec_min</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />   <b>if</b> (num1.value &lt; num2.value) &#123;<br />       num1<br />   &#125; <b>else</b> &#123;<br />       num2<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_max">max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_max">spec_max</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_max"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_max">spec_max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />   <b>if</b> (num1.value &gt; num2.value) &#123;<br />       num1<br />   &#125; <b>else</b> &#123;<br />       num2<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_less_or_equal"></a>

### Function `less_or_equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less_or_equal">less_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">spec_less_or_equal</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_less_or_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">spec_less_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />   num1.value &lt;&#61; num2.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_less"></a>

### Function `less`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less">less</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_less">spec_less</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_less"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_less">spec_less</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />   num1.value &lt; num2.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_greater_or_equal"></a>

### Function `greater_or_equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater_or_equal">greater_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_greater_or_equal">spec_greater_or_equal</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_greater_or_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_greater_or_equal">spec_greater_or_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />   num1.value &gt;&#61; num2.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_greater"></a>

### Function `greater`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater">greater</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_greater">spec_greater</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_greater"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_greater">spec_greater</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />   num1.value &gt; num2.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_equal"></a>

### Function `equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_equal">equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_equal">spec_equal</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point64_spec_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_equal">spec_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />   num1.value &#61;&#61; num2.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_almost_equal"></a>

### Function `almost_equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_almost_equal">almost_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_almost_equal">spec_almost_equal</a>(num1, num2, precision);<br /></code></pre>




<a id="0x1_fixed_point64_spec_almost_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_almost_equal">spec_almost_equal</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool &#123;<br />   <b>if</b> (num1.value &gt; num2.value) &#123;<br />       (num1.value &#45; num2.value &lt;&#61; precision.value)<br />   &#125; <b>else</b> &#123;<br />       (num2.value &#45; num1.value &lt;&#61; precision.value)<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_from_u128"></a>

### Function `create_from_u128`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_u128">create_from_u128</a>(val: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromU64AbortsIf">CreateFromU64AbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">spec_create_from_u128</a>(val);<br /></code></pre>




<a id="0x1_fixed_point64_CreateFromU64AbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromU64AbortsIf">CreateFromU64AbortsIf</a> &#123;<br />val: num;<br /><b>let</b> scaled_value &#61; (val <b>as</b> u256) &lt;&lt; 64;<br /><b>aborts_if</b> scaled_value &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point64_spec_create_from_u128"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">spec_create_from_u128</a>(val: num): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;<br />   <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> &#123;value: val &lt;&lt; 64&#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_floor"></a>

### Function `floor`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_floor">spec_floor</a>(num);<br /></code></pre>




<a id="0x1_fixed_point64_spec_floor"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_floor">spec_floor</a>(val: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />   <b>let</b> fractional &#61; val.value % (1 &lt;&lt; 64);<br />   <b>if</b> (fractional &#61;&#61; 0) &#123;<br />       val.value &gt;&gt; 64<br />   &#125; <b>else</b> &#123;<br />       (val.value &#45; fractional) &gt;&gt; 64<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_ceil"></a>

### Function `ceil`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1000;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_ceil">spec_ceil</a>(num);<br /></code></pre>




<a id="0x1_fixed_point64_spec_ceil"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_ceil">spec_ceil</a>(val: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />   <b>let</b> fractional &#61; val.value % (1 &lt;&lt; 64);<br />   <b>let</b> one &#61; 1 &lt;&lt; 64;<br />   <b>if</b> (fractional &#61;&#61; 0) &#123;<br />       val.value &gt;&gt; 64<br />   &#125; <b>else</b> &#123;<br />       (val.value &#45; fractional &#43; one) &gt;&gt; 64<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_round"></a>

### Function `round`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_round">round</a>(num: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point64.md#0x1_fixed_point64_spec_round">spec_round</a>(num);<br /></code></pre>




<a id="0x1_fixed_point64_spec_round"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_round">spec_round</a>(val: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 &#123;<br />   <b>let</b> fractional &#61; val.value % (1 &lt;&lt; 64);<br />   <b>let</b> boundary &#61; (1 &lt;&lt; 64) / 2;<br />   <b>let</b> one &#61; 1 &lt;&lt; 64;<br />   <b>if</b> (fractional &lt; boundary) &#123;<br />       (val.value &#45; fractional) &gt;&gt; 64<br />   &#125; <b>else</b> &#123;<br />       (val.value &#45; fractional &#43; one) &gt;&gt; 64<br />   &#125;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
