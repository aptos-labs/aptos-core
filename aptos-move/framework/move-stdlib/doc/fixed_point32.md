
<a id="0x1_fixed_point32"></a>

# Module `0x1::fixed_point32`

Defines a fixed&#45;point numeric type with a 32&#45;bit integer part and<br/> a 32&#45;bit fractional part.


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

Define a fixed&#45;point numeric type with 32 fractional bits.<br/> This is just a u64 integer but it is wrapped in a struct to<br/> make a unique type. This is a binary representation, so decimal<br/> values may not be exactly representable, but it provides more<br/> than 9 decimal digits of precision both before and after the<br/> decimal point (18 digits total). For comparison, double precision<br/> floating&#45;point has less than 16 decimal digits of precision, so<br/> be careful about using floating&#45;point to convert these values to<br/> decimal.


<pre><code>struct FixedPoint32 has copy, drop, store<br/></code></pre>



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



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_fixed_point32_EDENOMINATOR"></a>

The denominator provided was zero


<pre><code>const EDENOMINATOR: u64 &#61; 65537;<br/></code></pre>



<a id="0x1_fixed_point32_EDIVISION"></a>

The quotient value would be too large to be held in a <code>u64</code>


<pre><code>const EDIVISION: u64 &#61; 131074;<br/></code></pre>



<a id="0x1_fixed_point32_EDIVISION_BY_ZERO"></a>

A division by zero was encountered


<pre><code>const EDIVISION_BY_ZERO: u64 &#61; 65540;<br/></code></pre>



<a id="0x1_fixed_point32_EMULTIPLICATION"></a>

The multiplied value would be too large to be held in a <code>u64</code>


<pre><code>const EMULTIPLICATION: u64 &#61; 131075;<br/></code></pre>



<a id="0x1_fixed_point32_ERATIO_OUT_OF_RANGE"></a>

The computed ratio when converting to a <code>FixedPoint32</code> would be unrepresentable


<pre><code>const ERATIO_OUT_OF_RANGE: u64 &#61; 131077;<br/></code></pre>



<a id="0x1_fixed_point32_multiply_u64"></a>

## Function `multiply_u64`

Multiply a u64 integer by a fixed&#45;point number, truncating any<br/> fractional part of the product. This will abort if the product<br/> overflows.


<pre><code>public fun multiply_u64(val: u64, multiplier: fixed_point32::FixedPoint32): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multiply_u64(val: u64, multiplier: FixedPoint32): u64 &#123;<br/>    // The product of two 64 bit values has 128 bits, so perform the<br/>    // multiplication with u128 types and keep the full 128 bit product<br/>    // to avoid losing accuracy.<br/>    let unscaled_product &#61; (val as u128) &#42; (multiplier.value as u128);<br/>    // The unscaled product has 32 fractional bits (from the multiplier)<br/>    // so rescale it by shifting away the low bits.<br/>    let product &#61; unscaled_product &gt;&gt; 32;<br/>    // Check whether the value is too large.<br/>    assert!(product &lt;&#61; MAX_U64, EMULTIPLICATION);<br/>    (product as u64)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_divide_u64"></a>

## Function `divide_u64`

Divide a u64 integer by a fixed&#45;point number, truncating any<br/> fractional part of the quotient. This will abort if the divisor<br/> is zero or if the quotient overflows.


<pre><code>public fun divide_u64(val: u64, divisor: fixed_point32::FixedPoint32): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun divide_u64(val: u64, divisor: FixedPoint32): u64 &#123;<br/>    // Check for division by zero.<br/>    assert!(divisor.value !&#61; 0, EDIVISION_BY_ZERO);<br/>    // First convert to 128 bits and then shift left to<br/>    // add 32 fractional zero bits to the dividend.<br/>    let scaled_value &#61; (val as u128) &lt;&lt; 32;<br/>    let quotient &#61; scaled_value / (divisor.value as u128);<br/>    // Check whether the value is too large.<br/>    assert!(quotient &lt;&#61; MAX_U64, EDIVISION);<br/>    // the value may be too large, which will cause the cast to fail<br/>    // with an arithmetic error.<br/>    (quotient as u64)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_create_from_rational"></a>

## Function `create_from_rational`

Create a fixed&#45;point value from a rational number specified by its<br/> numerator and denominator. Calling this function should be preferred<br/> for using <code>Self::create_from_raw_value</code> which is also available.<br/> This will abort if the denominator is zero. It will also<br/> abort if the numerator is nonzero and the ratio is not in the range<br/> 2^&#45;32 .. 2^32&#45;1. When specifying decimal fractions, be careful about<br/> rounding errors: if you round to display N digits after the decimal<br/> point, you can use a denominator of 10^N to avoid numbers where the<br/> very small imprecision in the binary representation could change the<br/> rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.


<pre><code>public fun create_from_rational(numerator: u64, denominator: u64): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_rational(numerator: u64, denominator: u64): FixedPoint32 &#123;<br/>    // If the denominator is zero, this will abort.<br/>    // Scale the numerator to have 64 fractional bits and the denominator<br/>    // to have 32 fractional bits, so that the quotient will have 32<br/>    // fractional bits.<br/>    let scaled_numerator &#61; (numerator as u128) &lt;&lt; 64;<br/>    let scaled_denominator &#61; (denominator as u128) &lt;&lt; 32;<br/>    assert!(scaled_denominator !&#61; 0, EDENOMINATOR);<br/>    let quotient &#61; scaled_numerator / scaled_denominator;<br/>    assert!(quotient !&#61; 0 &#124;&#124; numerator &#61;&#61; 0, ERATIO_OUT_OF_RANGE);<br/>    // Return the quotient as a fixed&#45;point number. We first need to check whether the cast<br/>    // can succeed.<br/>    assert!(quotient &lt;&#61; MAX_U64, ERATIO_OUT_OF_RANGE);<br/>    FixedPoint32 &#123; value: (quotient as u64) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_create_from_raw_value"></a>

## Function `create_from_raw_value`

Create a fixedpoint value from a raw value.


<pre><code>public fun create_from_raw_value(value: u64): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_raw_value(value: u64): FixedPoint32 &#123;<br/>    FixedPoint32 &#123; value &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_get_raw_value"></a>

## Function `get_raw_value`

Accessor for the raw u64 value. Other less common operations, such as<br/> adding or subtracting FixedPoint32 values, can be done using the raw<br/> values directly.


<pre><code>public fun get_raw_value(num: fixed_point32::FixedPoint32): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_raw_value(num: FixedPoint32): u64 &#123;<br/>    num.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_is_zero"></a>

## Function `is_zero`

Returns true if the ratio is zero.


<pre><code>public fun is_zero(num: fixed_point32::FixedPoint32): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_zero(num: FixedPoint32): bool &#123;<br/>    num.value &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_min"></a>

## Function `min`

Returns the smaller of the two FixedPoint32 numbers.


<pre><code>public fun min(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun min(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;<br/>    if (num1.value &lt; num2.value) &#123;<br/>        num1<br/>    &#125; else &#123;<br/>        num2<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_max"></a>

## Function `max`

Returns the larger of the two FixedPoint32 numbers.


<pre><code>public fun max(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;<br/>    if (num1.value &gt; num2.value) &#123;<br/>        num1<br/>    &#125; else &#123;<br/>        num2<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_create_from_u64"></a>

## Function `create_from_u64`

Create a fixedpoint value from a u64 value.


<pre><code>public fun create_from_u64(val: u64): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_u64(val: u64): FixedPoint32 &#123;<br/>    let value &#61; (val as u128) &lt;&lt; 32;<br/>    assert!(value &lt;&#61; MAX_U64, ERATIO_OUT_OF_RANGE);<br/>    FixedPoint32 &#123;value: (value as u64)&#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_floor"></a>

## Function `floor`

Returns the largest integer less than or equal to a given number.


<pre><code>public fun floor(num: fixed_point32::FixedPoint32): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun floor(num: FixedPoint32): u64 &#123;<br/>    num.value &gt;&gt; 32<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_ceil"></a>

## Function `ceil`

Rounds up the given FixedPoint32 to the next largest integer.


<pre><code>public fun ceil(num: fixed_point32::FixedPoint32): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ceil(num: FixedPoint32): u64 &#123;<br/>    let floored_num &#61; floor(num) &lt;&lt; 32;<br/>    if (num.value &#61;&#61; floored_num) &#123;<br/>        return floored_num &gt;&gt; 32<br/>    &#125;;<br/>    let val &#61; ((floored_num as u128) &#43; (1 &lt;&lt; 32));<br/>    (val &gt;&gt; 32 as u64)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point32_round"></a>

## Function `round`

Returns the value of a FixedPoint32 to the nearest integer.


<pre><code>public fun round(num: fixed_point32::FixedPoint32): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun round(num: FixedPoint32): u64 &#123;<br/>    let floored_num &#61; floor(num) &lt;&lt; 32;<br/>    let boundary &#61; floored_num &#43; ((1 &lt;&lt; 32) / 2);<br/>    if (num.value &lt; boundary) &#123;<br/>        floored_num &gt;&gt; 32<br/>    &#125; else &#123;<br/>        ceil(num)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_multiply_u64"></a>

### Function `multiply_u64`


<pre><code>public fun multiply_u64(val: u64, multiplier: fixed_point32::FixedPoint32): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>include MultiplyAbortsIf;<br/>ensures result &#61;&#61; spec_multiply_u64(val, multiplier);<br/></code></pre>




<a id="0x1_fixed_point32_MultiplyAbortsIf"></a>


<pre><code>schema MultiplyAbortsIf &#123;<br/>val: num;<br/>multiplier: FixedPoint32;<br/>aborts_if spec_multiply_u64(val, multiplier) &gt; MAX_U64 with EMULTIPLICATION;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point32_spec_multiply_u64"></a>


<pre><code>fun spec_multiply_u64(val: num, multiplier: FixedPoint32): num &#123;<br/>   (val &#42; multiplier.value) &gt;&gt; 32<br/>&#125;<br/></code></pre>



<a id="@Specification_1_divide_u64"></a>

### Function `divide_u64`


<pre><code>public fun divide_u64(val: u64, divisor: fixed_point32::FixedPoint32): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>include DivideAbortsIf;<br/>ensures result &#61;&#61; spec_divide_u64(val, divisor);<br/></code></pre>




<a id="0x1_fixed_point32_DivideAbortsIf"></a>


<pre><code>schema DivideAbortsIf &#123;<br/>val: num;<br/>divisor: FixedPoint32;<br/>aborts_if divisor.value &#61;&#61; 0 with EDIVISION_BY_ZERO;<br/>aborts_if spec_divide_u64(val, divisor) &gt; MAX_U64 with EDIVISION;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point32_spec_divide_u64"></a>


<pre><code>fun spec_divide_u64(val: num, divisor: FixedPoint32): num &#123;<br/>   (val &lt;&lt; 32) / divisor.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_from_rational"></a>

### Function `create_from_rational`


<pre><code>public fun create_from_rational(numerator: u64, denominator: u64): fixed_point32::FixedPoint32<br/></code></pre>




<pre><code>pragma opaque;<br/>include CreateFromRationalAbortsIf;<br/>ensures result &#61;&#61; spec_create_from_rational(numerator, denominator);<br/></code></pre>




<a id="0x1_fixed_point32_CreateFromRationalAbortsIf"></a>


<pre><code>schema CreateFromRationalAbortsIf &#123;<br/>numerator: u64;<br/>denominator: u64;<br/>let scaled_numerator &#61; (numerator as u128)&lt;&lt; 64;<br/>let scaled_denominator &#61; (denominator as u128) &lt;&lt; 32;<br/>let quotient &#61; scaled_numerator / scaled_denominator;<br/>aborts_if scaled_denominator &#61;&#61; 0 with EDENOMINATOR;<br/>aborts_if quotient &#61;&#61; 0 &amp;&amp; scaled_numerator !&#61; 0 with ERATIO_OUT_OF_RANGE;<br/>aborts_if quotient &gt; MAX_U64 with ERATIO_OUT_OF_RANGE;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point32_spec_create_from_rational"></a>


<pre><code>fun spec_create_from_rational(numerator: num, denominator: num): FixedPoint32 &#123;<br/>   FixedPoint32&#123;value: (numerator &lt;&lt; 64) / (denominator &lt;&lt; 32)&#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_from_raw_value"></a>

### Function `create_from_raw_value`


<pre><code>public fun create_from_raw_value(value: u64): fixed_point32::FixedPoint32<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result.value &#61;&#61; value;<br/></code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code>public fun min(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_min(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point32_spec_min"></a>


<pre><code>fun spec_min(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;<br/>   if (num1.value &lt; num2.value) &#123;<br/>       num1<br/>   &#125; else &#123;<br/>       num2<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_max"></a>

### Function `max`


<pre><code>public fun max(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_max(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point32_spec_max"></a>


<pre><code>fun spec_max(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;<br/>   if (num1.value &gt; num2.value) &#123;<br/>       num1<br/>   &#125; else &#123;<br/>       num2<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_from_u64"></a>

### Function `create_from_u64`


<pre><code>public fun create_from_u64(val: u64): fixed_point32::FixedPoint32<br/></code></pre>




<pre><code>pragma opaque;<br/>include CreateFromU64AbortsIf;<br/>ensures result &#61;&#61; spec_create_from_u64(val);<br/></code></pre>




<a id="0x1_fixed_point32_CreateFromU64AbortsIf"></a>


<pre><code>schema CreateFromU64AbortsIf &#123;<br/>val: num;<br/>let scaled_value &#61; (val as u128) &lt;&lt; 32;<br/>aborts_if scaled_value &gt; MAX_U64;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point32_spec_create_from_u64"></a>


<pre><code>fun spec_create_from_u64(val: num): FixedPoint32 &#123;<br/>   FixedPoint32 &#123;value: val &lt;&lt; 32&#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_floor"></a>

### Function `floor`


<pre><code>public fun floor(num: fixed_point32::FixedPoint32): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_floor(num);<br/></code></pre>




<a id="0x1_fixed_point32_spec_floor"></a>


<pre><code>fun spec_floor(val: FixedPoint32): u64 &#123;<br/>   let fractional &#61; val.value % (1 &lt;&lt; 32);<br/>   if (fractional &#61;&#61; 0) &#123;<br/>       val.value &gt;&gt; 32<br/>   &#125; else &#123;<br/>       (val.value &#45; fractional) &gt;&gt; 32<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_ceil"></a>

### Function `ceil`


<pre><code>public fun ceil(num: fixed_point32::FixedPoint32): u64<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_ceil(num);<br/></code></pre>




<a id="0x1_fixed_point32_spec_ceil"></a>


<pre><code>fun spec_ceil(val: FixedPoint32): u64 &#123;<br/>   let fractional &#61; val.value % (1 &lt;&lt; 32);<br/>   let one &#61; 1 &lt;&lt; 32;<br/>   if (fractional &#61;&#61; 0) &#123;<br/>       val.value &gt;&gt; 32<br/>   &#125; else &#123;<br/>       (val.value &#45; fractional &#43; one) &gt;&gt; 32<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_round"></a>

### Function `round`


<pre><code>public fun round(num: fixed_point32::FixedPoint32): u64<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_round(num);<br/></code></pre>




<a id="0x1_fixed_point32_spec_round"></a>


<pre><code>fun spec_round(val: FixedPoint32): u64 &#123;<br/>   let fractional &#61; val.value % (1 &lt;&lt; 32);<br/>   let boundary &#61; (1 &lt;&lt; 32) / 2;<br/>   let one &#61; 1 &lt;&lt; 32;<br/>   if (fractional &lt; boundary) &#123;<br/>       (val.value &#45; fractional) &gt;&gt; 32<br/>   &#125; else &#123;<br/>       (val.value &#45; fractional &#43; one) &gt;&gt; 32<br/>   &#125;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
