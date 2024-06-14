
<a id="0x1_fixed_point32"></a>

# Module `0x1::fixed_point32`

Defines a fixed&#45;point numeric type with a 32&#45;bit integer part and
a 32&#45;bit fractional part.


-  [Struct `FixedPoint32`](#0x1_fixed_point32_FixedPoint32)
-  [Constants](#@Constants_0)
-  [Function `multiply_u64`](#0x1_fixed_point32_multiply_u64)
-  [Function `divide_u64`](#0x1_fixed_point32_divide_u64)
-  [Function `create_from_rational`](#0x1_fixed_point32_create_from_rational)
-  [Function `create_from_raw_value`](#0x1_fixed_point32_create_from_raw_value)
-  [Function `get_raw_value`](#0x1_fixed_point32_get_raw_value)
-  [Function `is_zero`](#0x1_fixed_point32_is_zero)
-  [Function `min`](#0x1_fixed_point32_min)
-  [Function `max`](#0x1_fixed_point32_max)
-  [Function `create_from_u64`](#0x1_fixed_point32_create_from_u64)
-  [Function `floor`](#0x1_fixed_point32_floor)
-  [Function `ceil`](#0x1_fixed_point32_ceil)
-  [Function `round`](#0x1_fixed_point32_round)
-  [Specification](#@Specification_1)
    -  [Function `multiply_u64`](#@Specification_1_multiply_u64)
    -  [Function `divide_u64`](#@Specification_1_divide_u64)
    -  [Function `create_from_rational`](#@Specification_1_create_from_rational)
    -  [Function `create_from_raw_value`](#@Specification_1_create_from_raw_value)
    -  [Function `min`](#@Specification_1_min)
    -  [Function `max`](#@Specification_1_max)
    -  [Function `create_from_u64`](#@Specification_1_create_from_u64)
    -  [Function `floor`](#@Specification_1_floor)
    -  [Function `ceil`](#@Specification_1_ceil)
    -  [Function `round`](#@Specification_1_round)


<pre><code></code></pre>



<a id="0x1_fixed_point32_FixedPoint32"></a>

## Struct `FixedPoint32`

Define a fixed&#45;point numeric type with 32 fractional bits.
This is just a u64 integer but it is wrapped in a struct to
make a unique type. This is a binary representation, so decimal
values may not be exactly representable, but it provides more
than 9 decimal digits of precision both before and after the
decimal point (18 digits total). For comparison, double precision
floating&#45;point has less than 16 decimal digits of precision, so
be careful about using floating&#45;point to convert these values to
decimal.


<pre><code><b>struct</b> <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_fixed_point32_MAX_U64"></a>



<pre><code><b>const</b> <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a>: u128 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_fixed_point32_EDENOMINATOR"></a>

The denominator provided was zero


<pre><code><b>const</b> <a href="fixed_point32.md#0x1_fixed_point32_EDENOMINATOR">EDENOMINATOR</a>: u64 &#61; 65537;<br /></code></pre>



<a id="0x1_fixed_point32_EDIVISION"></a>

The quotient value would be too large to be held in a <code>u64</code>


<pre><code><b>const</b> <a href="fixed_point32.md#0x1_fixed_point32_EDIVISION">EDIVISION</a>: u64 &#61; 131074;<br /></code></pre>



<a id="0x1_fixed_point32_EDIVISION_BY_ZERO"></a>

A division by zero was encountered


<pre><code><b>const</b> <a href="fixed_point32.md#0x1_fixed_point32_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>: u64 &#61; 65540;<br /></code></pre>



<a id="0x1_fixed_point32_EMULTIPLICATION"></a>

The multiplied value would be too large to be held in a <code>u64</code>


<pre><code><b>const</b> <a href="fixed_point32.md#0x1_fixed_point32_EMULTIPLICATION">EMULTIPLICATION</a>: u64 &#61; 131075;<br /></code></pre>



<a id="0x1_fixed_point32_ERATIO_OUT_OF_RANGE"></a>

The computed ratio when converting to a <code><a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a></code> would be unrepresentable


<pre><code><b>const</b> <a href="fixed_point32.md#0x1_fixed_point32_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>: u64 &#61; 131077;<br /></code></pre>



<a id="0x1_fixed_point32_multiply_u64"></a>

## Function `multiply_u64`

Multiply a u64 integer by a fixed&#45;point number, truncating any
fractional part of the product. This will abort if the product
overflows.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_multiply_u64">multiply_u64</a>(val: u64, multiplier: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_multiply_u64">multiply_u64</a>(val: u64, multiplier: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />    // The product of two 64 bit values <b>has</b> 128 bits, so perform the<br />    // multiplication <b>with</b> u128 types and keep the full 128 bit product<br />    // <b>to</b> avoid losing accuracy.<br />    <b>let</b> unscaled_product &#61; (val <b>as</b> u128) &#42; (multiplier.value <b>as</b> u128);<br />    // The unscaled product <b>has</b> 32 fractional bits (from the multiplier)<br />    // so rescale it by shifting away the low bits.<br />    <b>let</b> product &#61; unscaled_product &gt;&gt; 32;<br />    // Check whether the value is too large.<br />    <b>assert</b>!(product &lt;&#61; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a>, <a href="fixed_point32.md#0x1_fixed_point32_EMULTIPLICATION">EMULTIPLICATION</a>);<br />    (product <b>as</b> u64)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_divide_u64"></a>

## Function `divide_u64`

Divide a u64 integer by a fixed&#45;point number, truncating any
fractional part of the quotient. This will abort if the divisor
is zero or if the quotient overflows.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_divide_u64">divide_u64</a>(val: u64, divisor: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_divide_u64">divide_u64</a>(val: u64, divisor: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />    // Check for division by zero.<br />    <b>assert</b>!(divisor.value !&#61; 0, <a href="fixed_point32.md#0x1_fixed_point32_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>);<br />    // First convert <b>to</b> 128 bits and then shift left <b>to</b><br />    // add 32 fractional zero bits <b>to</b> the dividend.<br />    <b>let</b> scaled_value &#61; (val <b>as</b> u128) &lt;&lt; 32;<br />    <b>let</b> quotient &#61; scaled_value / (divisor.value <b>as</b> u128);<br />    // Check whether the value is too large.<br />    <b>assert</b>!(quotient &lt;&#61; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a>, <a href="fixed_point32.md#0x1_fixed_point32_EDIVISION">EDIVISION</a>);<br />    // the value may be too large, which will cause the cast <b>to</b> fail<br />    // <b>with</b> an arithmetic <a href="error.md#0x1_error">error</a>.<br />    (quotient <b>as</b> u64)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_create_from_rational"></a>

## Function `create_from_rational`

Create a fixed&#45;point value from a rational number specified by its
numerator and denominator. Calling this function should be preferred
for using <code><a href="fixed_point32.md#0x1_fixed_point32_create_from_raw_value">Self::create_from_raw_value</a></code> which is also available.
This will abort if the denominator is zero. It will also
abort if the numerator is nonzero and the ratio is not in the range
2^&#45;32 .. 2^32&#45;1. When specifying decimal fractions, be careful about
rounding errors: if you round to display N digits after the decimal
point, you can use a denominator of 10^N to avoid numbers where the
very small imprecision in the binary representation could change the
rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_rational">create_from_rational</a>(numerator: u64, denominator: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_rational">create_from_rational</a>(numerator: u64, denominator: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />    // If the denominator is zero, this will <b>abort</b>.<br />    // Scale the numerator <b>to</b> have 64 fractional bits and the denominator<br />    // <b>to</b> have 32 fractional bits, so that the quotient will have 32<br />    // fractional bits.<br />    <b>let</b> scaled_numerator &#61; (numerator <b>as</b> u128) &lt;&lt; 64;<br />    <b>let</b> scaled_denominator &#61; (denominator <b>as</b> u128) &lt;&lt; 32;<br />    <b>assert</b>!(scaled_denominator !&#61; 0, <a href="fixed_point32.md#0x1_fixed_point32_EDENOMINATOR">EDENOMINATOR</a>);<br />    <b>let</b> quotient &#61; scaled_numerator / scaled_denominator;<br />    <b>assert</b>!(quotient !&#61; 0 &#124;&#124; numerator &#61;&#61; 0, <a href="fixed_point32.md#0x1_fixed_point32_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    // Return the quotient <b>as</b> a fixed&#45;point number. We first need <b>to</b> check whether the cast<br />    // can succeed.<br />    <b>assert</b>!(quotient &lt;&#61; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a>, <a href="fixed_point32.md#0x1_fixed_point32_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123; value: (quotient <b>as</b> u64) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_create_from_raw_value"></a>

## Function `create_from_raw_value`

Create a fixedpoint value from a raw value.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_raw_value">create_from_raw_value</a>(value: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_raw_value">create_from_raw_value</a>(value: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />    <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123; value &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_get_raw_value"></a>

## Function `get_raw_value`

Accessor for the raw u64 value. Other less common operations, such as
adding or subtracting FixedPoint32 values, can be done using the raw
values directly.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_get_raw_value">get_raw_value</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_get_raw_value">get_raw_value</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />    num.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_is_zero"></a>

## Function `is_zero`

Returns true if the ratio is zero.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_is_zero">is_zero</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_is_zero">is_zero</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): bool &#123;<br />    num.value &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_min"></a>

## Function `min`

Returns the smaller of the two FixedPoint32 numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />    <b>if</b> (num1.value &lt; num2.value) &#123;<br />        num1<br />    &#125; <b>else</b> &#123;<br />        num2<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_max"></a>

## Function `max`

Returns the larger of the two FixedPoint32 numbers.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_max">max</a>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_max">max</a>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />    <b>if</b> (num1.value &gt; num2.value) &#123;<br />        num1<br />    &#125; <b>else</b> &#123;<br />        num2<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_create_from_u64"></a>

## Function `create_from_u64`

Create a fixedpoint value from a u64 value.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_u64">create_from_u64</a>(val: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_u64">create_from_u64</a>(val: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />    <b>let</b> value &#61; (val <b>as</b> u128) &lt;&lt; 32;<br />    <b>assert</b>!(value &lt;&#61; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a>, <a href="fixed_point32.md#0x1_fixed_point32_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);<br />    <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;value: (value <b>as</b> u64)&#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_floor"></a>

## Function `floor`

Returns the largest integer less than or equal to a given number.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_floor">floor</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_floor">floor</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />    num.value &gt;&gt; 32<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_ceil"></a>

## Function `ceil`

Rounds up the given FixedPoint32 to the next largest integer.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_ceil">ceil</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_ceil">ceil</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />    <b>let</b> floored_num &#61; <a href="fixed_point32.md#0x1_fixed_point32_floor">floor</a>(num) &lt;&lt; 32;<br />    <b>if</b> (num.value &#61;&#61; floored_num) &#123;<br />        <b>return</b> floored_num &gt;&gt; 32<br />    &#125;;<br />    <b>let</b> val &#61; ((floored_num <b>as</b> u128) &#43; (1 &lt;&lt; 32));<br />    (val &gt;&gt; 32 <b>as</b> u64)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_fixed_point32_round"></a>

## Function `round`

Returns the value of a FixedPoint32 to the nearest integer.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_round">round</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_round">round</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />    <b>let</b> floored_num &#61; <a href="fixed_point32.md#0x1_fixed_point32_floor">floor</a>(num) &lt;&lt; 32;<br />    <b>let</b> boundary &#61; floored_num &#43; ((1 &lt;&lt; 32) / 2);<br />    <b>if</b> (num.value &lt; boundary) &#123;<br />        floored_num &gt;&gt; 32<br />    &#125; <b>else</b> &#123;<br />        <a href="fixed_point32.md#0x1_fixed_point32_ceil">ceil</a>(num)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_multiply_u64"></a>

### Function `multiply_u64`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_multiply_u64">multiply_u64</a>(val: u64, multiplier: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point32.md#0x1_fixed_point32_MultiplyAbortsIf">MultiplyAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_multiply_u64">spec_multiply_u64</a>(val, multiplier);<br /></code></pre>




<a id="0x1_fixed_point32_MultiplyAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point32.md#0x1_fixed_point32_MultiplyAbortsIf">MultiplyAbortsIf</a> &#123;<br />val: num;<br />multiplier: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>;<br /><b>aborts_if</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_multiply_u64">spec_multiply_u64</a>(val, multiplier) &gt; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a> <b>with</b> <a href="fixed_point32.md#0x1_fixed_point32_EMULTIPLICATION">EMULTIPLICATION</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point32_spec_multiply_u64"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_multiply_u64">spec_multiply_u64</a>(val: num, multiplier: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): num &#123;<br />   (val &#42; multiplier.value) &gt;&gt; 32<br />&#125;<br /></code></pre>



<a id="@Specification_1_divide_u64"></a>

### Function `divide_u64`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_divide_u64">divide_u64</a>(val: u64, divisor: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point32.md#0x1_fixed_point32_DivideAbortsIf">DivideAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_divide_u64">spec_divide_u64</a>(val, divisor);<br /></code></pre>




<a id="0x1_fixed_point32_DivideAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point32.md#0x1_fixed_point32_DivideAbortsIf">DivideAbortsIf</a> &#123;<br />val: num;<br />divisor: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>;<br /><b>aborts_if</b> divisor.value &#61;&#61; 0 <b>with</b> <a href="fixed_point32.md#0x1_fixed_point32_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>;<br /><b>aborts_if</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_divide_u64">spec_divide_u64</a>(val, divisor) &gt; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a> <b>with</b> <a href="fixed_point32.md#0x1_fixed_point32_EDIVISION">EDIVISION</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point32_spec_divide_u64"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_divide_u64">spec_divide_u64</a>(val: num, divisor: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): num &#123;<br />   (val &lt;&lt; 32) / divisor.value<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_from_rational"></a>

### Function `create_from_rational`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_rational">create_from_rational</a>(numerator: u64, denominator: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point32.md#0x1_fixed_point32_CreateFromRationalAbortsIf">CreateFromRationalAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_create_from_rational">spec_create_from_rational</a>(numerator, denominator);<br /></code></pre>




<a id="0x1_fixed_point32_CreateFromRationalAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point32.md#0x1_fixed_point32_CreateFromRationalAbortsIf">CreateFromRationalAbortsIf</a> &#123;<br />numerator: u64;<br />denominator: u64;<br /><b>let</b> scaled_numerator &#61; (numerator <b>as</b> u128)&lt;&lt; 64;<br /><b>let</b> scaled_denominator &#61; (denominator <b>as</b> u128) &lt;&lt; 32;<br /><b>let</b> quotient &#61; scaled_numerator / scaled_denominator;<br /><b>aborts_if</b> scaled_denominator &#61;&#61; 0 <b>with</b> <a href="fixed_point32.md#0x1_fixed_point32_EDENOMINATOR">EDENOMINATOR</a>;<br /><b>aborts_if</b> quotient &#61;&#61; 0 &amp;&amp; scaled_numerator !&#61; 0 <b>with</b> <a href="fixed_point32.md#0x1_fixed_point32_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;<br /><b>aborts_if</b> quotient &gt; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a> <b>with</b> <a href="fixed_point32.md#0x1_fixed_point32_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point32_spec_create_from_rational"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_create_from_rational">spec_create_from_rational</a>(numerator: num, denominator: num): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />   <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>&#123;value: (numerator &lt;&lt; 64) / (denominator &lt;&lt; 32)&#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_from_raw_value"></a>

### Function `create_from_raw_value`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_raw_value">create_from_raw_value</a>(value: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result.value &#61;&#61; value;<br /></code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_min">spec_min</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point32_spec_min"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_min">spec_min</a>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />   <b>if</b> (num1.value &lt; num2.value) &#123;<br />       num1<br />   &#125; <b>else</b> &#123;<br />       num2<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_max">max</a>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_max">spec_max</a>(num1, num2);<br /></code></pre>




<a id="0x1_fixed_point32_spec_max"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_max">spec_max</a>(num1: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>, num2: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />   <b>if</b> (num1.value &gt; num2.value) &#123;<br />       num1<br />   &#125; <b>else</b> &#123;<br />       num2<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_from_u64"></a>

### Function `create_from_u64`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_create_from_u64">create_from_u64</a>(val: u64): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="fixed_point32.md#0x1_fixed_point32_CreateFromU64AbortsIf">CreateFromU64AbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_create_from_u64">spec_create_from_u64</a>(val);<br /></code></pre>




<a id="0x1_fixed_point32_CreateFromU64AbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point32.md#0x1_fixed_point32_CreateFromU64AbortsIf">CreateFromU64AbortsIf</a> &#123;<br />val: num;<br /><b>let</b> scaled_value &#61; (val <b>as</b> u128) &lt;&lt; 32;<br /><b>aborts_if</b> scaled_value &gt; <a href="fixed_point32.md#0x1_fixed_point32_MAX_U64">MAX_U64</a>;<br />&#125;<br /></code></pre>




<a id="0x1_fixed_point32_spec_create_from_u64"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_create_from_u64">spec_create_from_u64</a>(val: num): <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;<br />   <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a> &#123;value: val &lt;&lt; 32&#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_floor"></a>

### Function `floor`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_floor">floor</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_floor">spec_floor</a>(num);<br /></code></pre>




<a id="0x1_fixed_point32_spec_floor"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_floor">spec_floor</a>(val: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />   <b>let</b> fractional &#61; val.value % (1 &lt;&lt; 32);<br />   <b>if</b> (fractional &#61;&#61; 0) &#123;<br />       val.value &gt;&gt; 32<br />   &#125; <b>else</b> &#123;<br />       (val.value &#45; fractional) &gt;&gt; 32<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_ceil"></a>

### Function `ceil`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_ceil">ceil</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_ceil">spec_ceil</a>(num);<br /></code></pre>




<a id="0x1_fixed_point32_spec_ceil"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_ceil">spec_ceil</a>(val: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />   <b>let</b> fractional &#61; val.value % (1 &lt;&lt; 32);<br />   <b>let</b> one &#61; 1 &lt;&lt; 32;<br />   <b>if</b> (fractional &#61;&#61; 0) &#123;<br />       val.value &gt;&gt; 32<br />   &#125; <b>else</b> &#123;<br />       (val.value &#45; fractional &#43; one) &gt;&gt; 32<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_round"></a>

### Function `round`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_round">round</a>(num: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="fixed_point32.md#0x1_fixed_point32_spec_round">spec_round</a>(num);<br /></code></pre>




<a id="0x1_fixed_point32_spec_round"></a>


<pre><code><b>fun</b> <a href="fixed_point32.md#0x1_fixed_point32_spec_round">spec_round</a>(val: <a href="fixed_point32.md#0x1_fixed_point32_FixedPoint32">FixedPoint32</a>): u64 &#123;<br />   <b>let</b> fractional &#61; val.value % (1 &lt;&lt; 32);<br />   <b>let</b> boundary &#61; (1 &lt;&lt; 32) / 2;<br />   <b>let</b> one &#61; 1 &lt;&lt; 32;<br />   <b>if</b> (fractional &lt; boundary) &#123;<br />       (val.value &#45; fractional) &gt;&gt; 32<br />   &#125; <b>else</b> &#123;<br />       (val.value &#45; fractional &#43; one) &gt;&gt; 32<br />   &#125;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
