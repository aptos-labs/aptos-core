
<a id="0x1_fixed_point64"></a>

# Module `0x1::fixed_point64`

Defines a fixed&#45;point numeric type with a 64&#45;bit integer part and<br/> a 64&#45;bit fractional part.


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

Define a fixed&#45;point numeric type with 64 fractional bits.<br/> This is just a u128 integer but it is wrapped in a struct to<br/> make a unique type. This is a binary representation, so decimal<br/> values may not be exactly representable, but it provides more<br/> than 9 decimal digits of precision both before and after the<br/> decimal point (18 digits total). For comparison, double precision<br/> floating&#45;point has less than 16 decimal digits of precision, so<br/> be careful about using floating&#45;point to convert these values to<br/> decimal.


<pre><code>struct FixedPoint64 has copy, drop, store<br/></code></pre>



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



<pre><code>const MAX_U128: u256 &#61; 340282366920938463463374607431768211455;<br/></code></pre>



<a id="0x1_fixed_point64_EDENOMINATOR"></a>

The denominator provided was zero


<pre><code>const EDENOMINATOR: u64 &#61; 65537;<br/></code></pre>



<a id="0x1_fixed_point64_EDIVISION"></a>

The quotient value would be too large to be held in a <code>u128</code>


<pre><code>const EDIVISION: u64 &#61; 131074;<br/></code></pre>



<a id="0x1_fixed_point64_EDIVISION_BY_ZERO"></a>

A division by zero was encountered


<pre><code>const EDIVISION_BY_ZERO: u64 &#61; 65540;<br/></code></pre>



<a id="0x1_fixed_point64_EMULTIPLICATION"></a>

The multiplied value would be too large to be held in a <code>u128</code>


<pre><code>const EMULTIPLICATION: u64 &#61; 131075;<br/></code></pre>



<a id="0x1_fixed_point64_ENEGATIVE_RESULT"></a>

Abort code on calculation result is negative.


<pre><code>const ENEGATIVE_RESULT: u64 &#61; 65542;<br/></code></pre>



<a id="0x1_fixed_point64_ERATIO_OUT_OF_RANGE"></a>

The computed ratio when converting to a <code>FixedPoint64</code> would be unrepresentable


<pre><code>const ERATIO_OUT_OF_RANGE: u64 &#61; 131077;<br/></code></pre>



<a id="0x1_fixed_point64_sub"></a>

## Function `sub`

Returns x &#45; y. x must be not less than y.


<pre><code>public fun sub(x: fixed_point64::FixedPoint64, y: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sub(x: FixedPoint64, y: FixedPoint64): FixedPoint64 &#123;<br/>    let x_raw &#61; get_raw_value(x);<br/>    let y_raw &#61; get_raw_value(y);<br/>    assert!(x_raw &gt;&#61; y_raw, ENEGATIVE_RESULT);<br/>    create_from_raw_value(x_raw &#45; y_raw)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_add"></a>

## Function `add`

Returns x &#43; y. The result cannot be greater than MAX_U128.


<pre><code>public fun add(x: fixed_point64::FixedPoint64, y: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add(x: FixedPoint64, y: FixedPoint64): FixedPoint64 &#123;<br/>    let x_raw &#61; get_raw_value(x);<br/>    let y_raw &#61; get_raw_value(y);<br/>    let result &#61; (x_raw as u256) &#43; (y_raw as u256);<br/>    assert!(result &lt;&#61; MAX_U128, ERATIO_OUT_OF_RANGE);<br/>    create_from_raw_value((result as u128))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_multiply_u128"></a>

## Function `multiply_u128`

Multiply a u128 integer by a fixed&#45;point number, truncating any<br/> fractional part of the product. This will abort if the product<br/> overflows.


<pre><code>public fun multiply_u128(val: u128, multiplier: fixed_point64::FixedPoint64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multiply_u128(val: u128, multiplier: FixedPoint64): u128 &#123;<br/>    // The product of two 128 bit values has 256 bits, so perform the<br/>    // multiplication with u256 types and keep the full 256 bit product<br/>    // to avoid losing accuracy.<br/>    let unscaled_product &#61; (val as u256) &#42; (multiplier.value as u256);<br/>    // The unscaled product has 64 fractional bits (from the multiplier)<br/>    // so rescale it by shifting away the low bits.<br/>    let product &#61; unscaled_product &gt;&gt; 64;<br/>    // Check whether the value is too large.<br/>    assert!(product &lt;&#61; MAX_U128, EMULTIPLICATION);<br/>    (product as u128)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_divide_u128"></a>

## Function `divide_u128`

Divide a u128 integer by a fixed&#45;point number, truncating any<br/> fractional part of the quotient. This will abort if the divisor<br/> is zero or if the quotient overflows.


<pre><code>public fun divide_u128(val: u128, divisor: fixed_point64::FixedPoint64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun divide_u128(val: u128, divisor: FixedPoint64): u128 &#123;<br/>    // Check for division by zero.<br/>    assert!(divisor.value !&#61; 0, EDIVISION_BY_ZERO);<br/>    // First convert to 256 bits and then shift left to<br/>    // add 64 fractional zero bits to the dividend.<br/>    let scaled_value &#61; (val as u256) &lt;&lt; 64;<br/>    let quotient &#61; scaled_value / (divisor.value as u256);<br/>    // Check whether the value is too large.<br/>    assert!(quotient &lt;&#61; MAX_U128, EDIVISION);<br/>    // the value may be too large, which will cause the cast to fail<br/>    // with an arithmetic error.<br/>    (quotient as u128)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_create_from_rational"></a>

## Function `create_from_rational`

Create a fixed&#45;point value from a rational number specified by its<br/> numerator and denominator. Calling this function should be preferred<br/> for using <code>Self::create_from_raw_value</code> which is also available.<br/> This will abort if the denominator is zero. It will also<br/> abort if the numerator is nonzero and the ratio is not in the range<br/> 2^&#45;64 .. 2^64&#45;1. When specifying decimal fractions, be careful about<br/> rounding errors: if you round to display N digits after the decimal<br/> point, you can use a denominator of 10^N to avoid numbers where the<br/> very small imprecision in the binary representation could change the<br/> rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.


<pre><code>public fun create_from_rational(numerator: u128, denominator: u128): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_rational(numerator: u128, denominator: u128): FixedPoint64 &#123;<br/>    // If the denominator is zero, this will abort.<br/>    // Scale the numerator to have 64 fractional bits, so that the quotient will have 64<br/>    // fractional bits.<br/>    let scaled_numerator &#61; (numerator as u256) &lt;&lt; 64;<br/>    assert!(denominator !&#61; 0, EDENOMINATOR);<br/>    let quotient &#61; scaled_numerator / (denominator as u256);<br/>    assert!(quotient !&#61; 0 &#124;&#124; numerator &#61;&#61; 0, ERATIO_OUT_OF_RANGE);<br/>    // Return the quotient as a fixed&#45;point number. We first need to check whether the cast<br/>    // can succeed.<br/>    assert!(quotient &lt;&#61; MAX_U128, ERATIO_OUT_OF_RANGE);<br/>    FixedPoint64 &#123; value: (quotient as u128) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_create_from_raw_value"></a>

## Function `create_from_raw_value`

Create a fixedpoint value from a raw value.


<pre><code>public fun create_from_raw_value(value: u128): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_raw_value(value: u128): FixedPoint64 &#123;<br/>    FixedPoint64 &#123; value &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_get_raw_value"></a>

## Function `get_raw_value`

Accessor for the raw u128 value. Other less common operations, such as<br/> adding or subtracting FixedPoint64 values, can be done using the raw<br/> values directly.


<pre><code>public fun get_raw_value(num: fixed_point64::FixedPoint64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_raw_value(num: FixedPoint64): u128 &#123;<br/>    num.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_is_zero"></a>

## Function `is_zero`

Returns true if the ratio is zero.


<pre><code>public fun is_zero(num: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_zero(num: FixedPoint64): bool &#123;<br/>    num.value &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_min"></a>

## Function `min`

Returns the smaller of the two FixedPoint64 numbers.


<pre><code>public fun min(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun min(num1: FixedPoint64, num2: FixedPoint64): FixedPoint64 &#123;<br/>    if (num1.value &lt; num2.value) &#123;<br/>        num1<br/>    &#125; else &#123;<br/>        num2<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_max"></a>

## Function `max`

Returns the larger of the two FixedPoint64 numbers.


<pre><code>public fun max(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max(num1: FixedPoint64, num2: FixedPoint64): FixedPoint64 &#123;<br/>    if (num1.value &gt; num2.value) &#123;<br/>        num1<br/>    &#125; else &#123;<br/>        num2<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_less_or_equal"></a>

## Function `less_or_equal`

Returns true if num1 &lt;&#61; num2


<pre><code>public fun less_or_equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun less_or_equal(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>    num1.value &lt;&#61; num2.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_less"></a>

## Function `less`

Returns true if num1 &lt; num2


<pre><code>public fun less(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun less(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>    num1.value &lt; num2.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_greater_or_equal"></a>

## Function `greater_or_equal`

Returns true if num1 &gt;&#61; num2


<pre><code>public fun greater_or_equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun greater_or_equal(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>    num1.value &gt;&#61; num2.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_greater"></a>

## Function `greater`

Returns true if num1 &gt; num2


<pre><code>public fun greater(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun greater(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>    num1.value &gt; num2.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_equal"></a>

## Function `equal`

Returns true if num1 &#61; num2


<pre><code>public fun equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun equal(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>    num1.value &#61;&#61; num2.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_almost_equal"></a>

## Function `almost_equal`

Returns true if num1 almost equals to num2, which means abs(num1&#45;num2) &lt;&#61; precision


<pre><code>public fun almost_equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64, precision: fixed_point64::FixedPoint64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun almost_equal(num1: FixedPoint64, num2: FixedPoint64, precision: FixedPoint64): bool &#123;<br/>    if (num1.value &gt; num2.value) &#123;<br/>        (num1.value &#45; num2.value &lt;&#61; precision.value)<br/>    &#125; else &#123;<br/>        (num2.value &#45; num1.value &lt;&#61; precision.value)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_create_from_u128"></a>

## Function `create_from_u128`

Create a fixedpoint value from a u128 value.


<pre><code>public fun create_from_u128(val: u128): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_u128(val: u128): FixedPoint64 &#123;<br/>    let value &#61; (val as u256) &lt;&lt; 64;<br/>    assert!(value &lt;&#61; MAX_U128, ERATIO_OUT_OF_RANGE);<br/>    FixedPoint64 &#123;value: (value as u128)&#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_floor"></a>

## Function `floor`

Returns the largest integer less than or equal to a given number.


<pre><code>public fun floor(num: fixed_point64::FixedPoint64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun floor(num: FixedPoint64): u128 &#123;<br/>    num.value &gt;&gt; 64<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_ceil"></a>

## Function `ceil`

Rounds up the given FixedPoint64 to the next largest integer.


<pre><code>public fun ceil(num: fixed_point64::FixedPoint64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ceil(num: FixedPoint64): u128 &#123;<br/>    let floored_num &#61; floor(num) &lt;&lt; 64;<br/>    if (num.value &#61;&#61; floored_num) &#123;<br/>        return floored_num &gt;&gt; 64<br/>    &#125;;<br/>    let val &#61; ((floored_num as u256) &#43; (1 &lt;&lt; 64));<br/>    (val &gt;&gt; 64 as u128)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_fixed_point64_round"></a>

## Function `round`

Returns the value of a FixedPoint64 to the nearest integer.


<pre><code>public fun round(num: fixed_point64::FixedPoint64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun round(num: FixedPoint64): u128 &#123;<br/>    let floored_num &#61; floor(num) &lt;&lt; 64;<br/>    let boundary &#61; floored_num &#43; ((1 &lt;&lt; 64) / 2);<br/>    if (num.value &lt; boundary) &#123;<br/>        floored_num &gt;&gt; 64<br/>    &#125; else &#123;<br/>        ceil(num)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code>public fun sub(x: fixed_point64::FixedPoint64, y: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if x.value &lt; y.value with ENEGATIVE_RESULT;<br/>ensures result.value &#61;&#61; x.value &#45; y.value;<br/></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add(x: fixed_point64::FixedPoint64, y: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if (x.value as u256) &#43; (y.value as u256) &gt; MAX_U128 with ERATIO_OUT_OF_RANGE;<br/>ensures result.value &#61;&#61; x.value &#43; y.value;<br/></code></pre>



<a id="@Specification_1_multiply_u128"></a>

### Function `multiply_u128`


<pre><code>public fun multiply_u128(val: u128, multiplier: fixed_point64::FixedPoint64): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>include MultiplyAbortsIf;<br/>ensures result &#61;&#61; spec_multiply_u128(val, multiplier);<br/></code></pre>




<a id="0x1_fixed_point64_MultiplyAbortsIf"></a>


<pre><code>schema MultiplyAbortsIf &#123;<br/>val: num;<br/>multiplier: FixedPoint64;<br/>aborts_if spec_multiply_u128(val, multiplier) &gt; MAX_U128 with EMULTIPLICATION;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point64_spec_multiply_u128"></a>


<pre><code>fun spec_multiply_u128(val: num, multiplier: FixedPoint64): num &#123;<br/>   (val &#42; multiplier.value) &gt;&gt; 64<br/>&#125;<br/></code></pre>



<a id="@Specification_1_divide_u128"></a>

### Function `divide_u128`


<pre><code>public fun divide_u128(val: u128, divisor: fixed_point64::FixedPoint64): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>include DivideAbortsIf;<br/>ensures result &#61;&#61; spec_divide_u128(val, divisor);<br/></code></pre>




<a id="0x1_fixed_point64_DivideAbortsIf"></a>


<pre><code>schema DivideAbortsIf &#123;<br/>val: num;<br/>divisor: FixedPoint64;<br/>aborts_if divisor.value &#61;&#61; 0 with EDIVISION_BY_ZERO;<br/>aborts_if spec_divide_u128(val, divisor) &gt; MAX_U128 with EDIVISION;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point64_spec_divide_u128"></a>


<pre><code>fun spec_divide_u128(val: num, divisor: FixedPoint64): num &#123;<br/>   (val &lt;&lt; 64) / divisor.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_from_rational"></a>

### Function `create_from_rational`


<pre><code>public fun create_from_rational(numerator: u128, denominator: u128): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma verify_duration_estimate &#61; 1000;<br/>include CreateFromRationalAbortsIf;<br/>ensures result &#61;&#61; spec_create_from_rational(numerator, denominator);<br/></code></pre>




<a id="0x1_fixed_point64_CreateFromRationalAbortsIf"></a>


<pre><code>schema CreateFromRationalAbortsIf &#123;<br/>numerator: u128;<br/>denominator: u128;<br/>let scaled_numerator &#61; (numerator as u256)&lt;&lt; 64;<br/>let scaled_denominator &#61; (denominator as u256);<br/>let quotient &#61; scaled_numerator / scaled_denominator;<br/>aborts_if scaled_denominator &#61;&#61; 0 with EDENOMINATOR;<br/>aborts_if quotient &#61;&#61; 0 &amp;&amp; scaled_numerator !&#61; 0 with ERATIO_OUT_OF_RANGE;<br/>aborts_if quotient &gt; MAX_U128 with ERATIO_OUT_OF_RANGE;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point64_spec_create_from_rational"></a>


<pre><code>fun spec_create_from_rational(numerator: num, denominator: num): FixedPoint64 &#123;<br/>   FixedPoint64&#123;value: (numerator &lt;&lt; 128) / (denominator &lt;&lt; 64)&#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_from_raw_value"></a>

### Function `create_from_raw_value`


<pre><code>public fun create_from_raw_value(value: u128): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result.value &#61;&#61; value;<br/></code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code>public fun min(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_min(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_min"></a>


<pre><code>fun spec_min(num1: FixedPoint64, num2: FixedPoint64): FixedPoint64 &#123;<br/>   if (num1.value &lt; num2.value) &#123;<br/>       num1<br/>   &#125; else &#123;<br/>       num2<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_max"></a>

### Function `max`


<pre><code>public fun max(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_max(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_max"></a>


<pre><code>fun spec_max(num1: FixedPoint64, num2: FixedPoint64): FixedPoint64 &#123;<br/>   if (num1.value &gt; num2.value) &#123;<br/>       num1<br/>   &#125; else &#123;<br/>       num2<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_less_or_equal"></a>

### Function `less_or_equal`


<pre><code>public fun less_or_equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_less_or_equal(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_less_or_equal"></a>


<pre><code>fun spec_less_or_equal(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>   num1.value &lt;&#61; num2.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_less"></a>

### Function `less`


<pre><code>public fun less(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_less(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_less"></a>


<pre><code>fun spec_less(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>   num1.value &lt; num2.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_greater_or_equal"></a>

### Function `greater_or_equal`


<pre><code>public fun greater_or_equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_greater_or_equal(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_greater_or_equal"></a>


<pre><code>fun spec_greater_or_equal(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>   num1.value &gt;&#61; num2.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_greater"></a>

### Function `greater`


<pre><code>public fun greater(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_greater(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_greater"></a>


<pre><code>fun spec_greater(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>   num1.value &gt; num2.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_equal"></a>

### Function `equal`


<pre><code>public fun equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_equal(num1, num2);<br/></code></pre>




<a id="0x1_fixed_point64_spec_equal"></a>


<pre><code>fun spec_equal(num1: FixedPoint64, num2: FixedPoint64): bool &#123;<br/>   num1.value &#61;&#61; num2.value<br/>&#125;<br/></code></pre>



<a id="@Specification_1_almost_equal"></a>

### Function `almost_equal`


<pre><code>public fun almost_equal(num1: fixed_point64::FixedPoint64, num2: fixed_point64::FixedPoint64, precision: fixed_point64::FixedPoint64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_almost_equal(num1, num2, precision);<br/></code></pre>




<a id="0x1_fixed_point64_spec_almost_equal"></a>


<pre><code>fun spec_almost_equal(num1: FixedPoint64, num2: FixedPoint64, precision: FixedPoint64): bool &#123;<br/>   if (num1.value &gt; num2.value) &#123;<br/>       (num1.value &#45; num2.value &lt;&#61; precision.value)<br/>   &#125; else &#123;<br/>       (num2.value &#45; num1.value &lt;&#61; precision.value)<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_from_u128"></a>

### Function `create_from_u128`


<pre><code>public fun create_from_u128(val: u128): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma opaque;<br/>include CreateFromU64AbortsIf;<br/>ensures result &#61;&#61; spec_create_from_u128(val);<br/></code></pre>




<a id="0x1_fixed_point64_CreateFromU64AbortsIf"></a>


<pre><code>schema CreateFromU64AbortsIf &#123;<br/>val: num;<br/>let scaled_value &#61; (val as u256) &lt;&lt; 64;<br/>aborts_if scaled_value &gt; MAX_U128;<br/>&#125;<br/></code></pre>




<a id="0x1_fixed_point64_spec_create_from_u128"></a>


<pre><code>fun spec_create_from_u128(val: num): FixedPoint64 &#123;<br/>   FixedPoint64 &#123;value: val &lt;&lt; 64&#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_floor"></a>

### Function `floor`


<pre><code>public fun floor(num: fixed_point64::FixedPoint64): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_floor(num);<br/></code></pre>




<a id="0x1_fixed_point64_spec_floor"></a>


<pre><code>fun spec_floor(val: FixedPoint64): u128 &#123;<br/>   let fractional &#61; val.value % (1 &lt;&lt; 64);<br/>   if (fractional &#61;&#61; 0) &#123;<br/>       val.value &gt;&gt; 64<br/>   &#125; else &#123;<br/>       (val.value &#45; fractional) &gt;&gt; 64<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_ceil"></a>

### Function `ceil`


<pre><code>public fun ceil(num: fixed_point64::FixedPoint64): u128<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;<br/>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_ceil(num);<br/></code></pre>




<a id="0x1_fixed_point64_spec_ceil"></a>


<pre><code>fun spec_ceil(val: FixedPoint64): u128 &#123;<br/>   let fractional &#61; val.value % (1 &lt;&lt; 64);<br/>   let one &#61; 1 &lt;&lt; 64;<br/>   if (fractional &#61;&#61; 0) &#123;<br/>       val.value &gt;&gt; 64<br/>   &#125; else &#123;<br/>       (val.value &#45; fractional &#43; one) &gt;&gt; 64<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_round"></a>

### Function `round`


<pre><code>public fun round(num: fixed_point64::FixedPoint64): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_round(num);<br/></code></pre>




<a id="0x1_fixed_point64_spec_round"></a>


<pre><code>fun spec_round(val: FixedPoint64): u128 &#123;<br/>   let fractional &#61; val.value % (1 &lt;&lt; 64);<br/>   let boundary &#61; (1 &lt;&lt; 64) / 2;<br/>   let one &#61; 1 &lt;&lt; 64;<br/>   if (fractional &lt; boundary) &#123;<br/>       (val.value &#45; fractional) &gt;&gt; 64<br/>   &#125; else &#123;<br/>       (val.value &#45; fractional &#43; one) &gt;&gt; 64<br/>   &#125;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
