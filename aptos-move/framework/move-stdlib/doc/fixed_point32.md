
<a id="0x1_fixed_point32"></a>

# Module `0x1::fixed_point32`

Defines a fixed-point numeric type with a 32-bit integer part and
a 32-bit fractional part.


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

Define a fixed-point numeric type with 32 fractional bits.
This is just a u64 integer but it is wrapped in a struct to
make a unique type. This is a binary representation, so decimal
values may not be exactly representable, but it provides more
than 9 decimal digits of precision both before and after the
decimal point (18 digits total). For comparison, double precision
floating-point has less than 16 decimal digits of precision, so
be careful about using floating-point to convert these values to
decimal.


<pre><code>struct FixedPoint32 has copy, drop, store
</code></pre>



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



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_fixed_point32_EDENOMINATOR"></a>

The denominator provided was zero


<pre><code>const EDENOMINATOR: u64 &#61; 65537;
</code></pre>



<a id="0x1_fixed_point32_EDIVISION"></a>

The quotient value would be too large to be held in a <code>u64</code>


<pre><code>const EDIVISION: u64 &#61; 131074;
</code></pre>



<a id="0x1_fixed_point32_EDIVISION_BY_ZERO"></a>

A division by zero was encountered


<pre><code>const EDIVISION_BY_ZERO: u64 &#61; 65540;
</code></pre>



<a id="0x1_fixed_point32_EMULTIPLICATION"></a>

The multiplied value would be too large to be held in a <code>u64</code>


<pre><code>const EMULTIPLICATION: u64 &#61; 131075;
</code></pre>



<a id="0x1_fixed_point32_ERATIO_OUT_OF_RANGE"></a>

The computed ratio when converting to a <code>FixedPoint32</code> would be unrepresentable


<pre><code>const ERATIO_OUT_OF_RANGE: u64 &#61; 131077;
</code></pre>



<a id="0x1_fixed_point32_multiply_u64"></a>

## Function `multiply_u64`

Multiply a u64 integer by a fixed-point number, truncating any
fractional part of the product. This will abort if the product
overflows.


<pre><code>public fun multiply_u64(val: u64, multiplier: fixed_point32::FixedPoint32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multiply_u64(val: u64, multiplier: FixedPoint32): u64 &#123;
    // The product of two 64 bit values has 128 bits, so perform the
    // multiplication with u128 types and keep the full 128 bit product
    // to avoid losing accuracy.
    let unscaled_product &#61; (val as u128) &#42; (multiplier.value as u128);
    // The unscaled product has 32 fractional bits (from the multiplier)
    // so rescale it by shifting away the low bits.
    let product &#61; unscaled_product &gt;&gt; 32;
    // Check whether the value is too large.
    assert!(product &lt;&#61; MAX_U64, EMULTIPLICATION);
    (product as u64)
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_divide_u64"></a>

## Function `divide_u64`

Divide a u64 integer by a fixed-point number, truncating any
fractional part of the quotient. This will abort if the divisor
is zero or if the quotient overflows.


<pre><code>public fun divide_u64(val: u64, divisor: fixed_point32::FixedPoint32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun divide_u64(val: u64, divisor: FixedPoint32): u64 &#123;
    // Check for division by zero.
    assert!(divisor.value !&#61; 0, EDIVISION_BY_ZERO);
    // First convert to 128 bits and then shift left to
    // add 32 fractional zero bits to the dividend.
    let scaled_value &#61; (val as u128) &lt;&lt; 32;
    let quotient &#61; scaled_value / (divisor.value as u128);
    // Check whether the value is too large.
    assert!(quotient &lt;&#61; MAX_U64, EDIVISION);
    // the value may be too large, which will cause the cast to fail
    // with an arithmetic error.
    (quotient as u64)
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_create_from_rational"></a>

## Function `create_from_rational`

Create a fixed-point value from a rational number specified by its
numerator and denominator. Calling this function should be preferred
for using <code>Self::create_from_raw_value</code> which is also available.
This will abort if the denominator is zero. It will also
abort if the numerator is nonzero and the ratio is not in the range
2^-32 .. 2^32-1. When specifying decimal fractions, be careful about
rounding errors: if you round to display N digits after the decimal
point, you can use a denominator of 10^N to avoid numbers where the
very small imprecision in the binary representation could change the
rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.


<pre><code>public fun create_from_rational(numerator: u64, denominator: u64): fixed_point32::FixedPoint32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_rational(numerator: u64, denominator: u64): FixedPoint32 &#123;
    // If the denominator is zero, this will abort.
    // Scale the numerator to have 64 fractional bits and the denominator
    // to have 32 fractional bits, so that the quotient will have 32
    // fractional bits.
    let scaled_numerator &#61; (numerator as u128) &lt;&lt; 64;
    let scaled_denominator &#61; (denominator as u128) &lt;&lt; 32;
    assert!(scaled_denominator !&#61; 0, EDENOMINATOR);
    let quotient &#61; scaled_numerator / scaled_denominator;
    assert!(quotient !&#61; 0 &#124;&#124; numerator &#61;&#61; 0, ERATIO_OUT_OF_RANGE);
    // Return the quotient as a fixed&#45;point number. We first need to check whether the cast
    // can succeed.
    assert!(quotient &lt;&#61; MAX_U64, ERATIO_OUT_OF_RANGE);
    FixedPoint32 &#123; value: (quotient as u64) &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_create_from_raw_value"></a>

## Function `create_from_raw_value`

Create a fixedpoint value from a raw value.


<pre><code>public fun create_from_raw_value(value: u64): fixed_point32::FixedPoint32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_raw_value(value: u64): FixedPoint32 &#123;
    FixedPoint32 &#123; value &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_get_raw_value"></a>

## Function `get_raw_value`

Accessor for the raw u64 value. Other less common operations, such as
adding or subtracting FixedPoint32 values, can be done using the raw
values directly.


<pre><code>public fun get_raw_value(num: fixed_point32::FixedPoint32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_raw_value(num: FixedPoint32): u64 &#123;
    num.value
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_is_zero"></a>

## Function `is_zero`

Returns true if the ratio is zero.


<pre><code>public fun is_zero(num: fixed_point32::FixedPoint32): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_zero(num: FixedPoint32): bool &#123;
    num.value &#61;&#61; 0
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_min"></a>

## Function `min`

Returns the smaller of the two FixedPoint32 numbers.


<pre><code>public fun min(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun min(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;
    if (num1.value &lt; num2.value) &#123;
        num1
    &#125; else &#123;
        num2
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_max"></a>

## Function `max`

Returns the larger of the two FixedPoint32 numbers.


<pre><code>public fun max(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;
    if (num1.value &gt; num2.value) &#123;
        num1
    &#125; else &#123;
        num2
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_create_from_u64"></a>

## Function `create_from_u64`

Create a fixedpoint value from a u64 value.


<pre><code>public fun create_from_u64(val: u64): fixed_point32::FixedPoint32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_from_u64(val: u64): FixedPoint32 &#123;
    let value &#61; (val as u128) &lt;&lt; 32;
    assert!(value &lt;&#61; MAX_U64, ERATIO_OUT_OF_RANGE);
    FixedPoint32 &#123;value: (value as u64)&#125;
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_floor"></a>

## Function `floor`

Returns the largest integer less than or equal to a given number.


<pre><code>public fun floor(num: fixed_point32::FixedPoint32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun floor(num: FixedPoint32): u64 &#123;
    num.value &gt;&gt; 32
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_ceil"></a>

## Function `ceil`

Rounds up the given FixedPoint32 to the next largest integer.


<pre><code>public fun ceil(num: fixed_point32::FixedPoint32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ceil(num: FixedPoint32): u64 &#123;
    let floored_num &#61; floor(num) &lt;&lt; 32;
    if (num.value &#61;&#61; floored_num) &#123;
        return floored_num &gt;&gt; 32
    &#125;;
    let val &#61; ((floored_num as u128) &#43; (1 &lt;&lt; 32));
    (val &gt;&gt; 32 as u64)
&#125;
</code></pre>



</details>

<a id="0x1_fixed_point32_round"></a>

## Function `round`

Returns the value of a FixedPoint32 to the nearest integer.


<pre><code>public fun round(num: fixed_point32::FixedPoint32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun round(num: FixedPoint32): u64 &#123;
    let floored_num &#61; floor(num) &lt;&lt; 32;
    let boundary &#61; floored_num &#43; ((1 &lt;&lt; 32) / 2);
    if (num.value &lt; boundary) &#123;
        floored_num &gt;&gt; 32
    &#125; else &#123;
        ceil(num)
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code>pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_multiply_u64"></a>

### Function `multiply_u64`


<pre><code>public fun multiply_u64(val: u64, multiplier: fixed_point32::FixedPoint32): u64
</code></pre>




<pre><code>pragma opaque;
include MultiplyAbortsIf;
ensures result &#61;&#61; spec_multiply_u64(val, multiplier);
</code></pre>




<a id="0x1_fixed_point32_MultiplyAbortsIf"></a>


<pre><code>schema MultiplyAbortsIf &#123;
    val: num;
    multiplier: FixedPoint32;
    aborts_if spec_multiply_u64(val, multiplier) &gt; MAX_U64 with EMULTIPLICATION;
&#125;
</code></pre>




<a id="0x1_fixed_point32_spec_multiply_u64"></a>


<pre><code>fun spec_multiply_u64(val: num, multiplier: FixedPoint32): num &#123;
   (val &#42; multiplier.value) &gt;&gt; 32
&#125;
</code></pre>



<a id="@Specification_1_divide_u64"></a>

### Function `divide_u64`


<pre><code>public fun divide_u64(val: u64, divisor: fixed_point32::FixedPoint32): u64
</code></pre>




<pre><code>pragma opaque;
include DivideAbortsIf;
ensures result &#61;&#61; spec_divide_u64(val, divisor);
</code></pre>




<a id="0x1_fixed_point32_DivideAbortsIf"></a>


<pre><code>schema DivideAbortsIf &#123;
    val: num;
    divisor: FixedPoint32;
    aborts_if divisor.value &#61;&#61; 0 with EDIVISION_BY_ZERO;
    aborts_if spec_divide_u64(val, divisor) &gt; MAX_U64 with EDIVISION;
&#125;
</code></pre>




<a id="0x1_fixed_point32_spec_divide_u64"></a>


<pre><code>fun spec_divide_u64(val: num, divisor: FixedPoint32): num &#123;
   (val &lt;&lt; 32) / divisor.value
&#125;
</code></pre>



<a id="@Specification_1_create_from_rational"></a>

### Function `create_from_rational`


<pre><code>public fun create_from_rational(numerator: u64, denominator: u64): fixed_point32::FixedPoint32
</code></pre>




<pre><code>pragma opaque;
include CreateFromRationalAbortsIf;
ensures result &#61;&#61; spec_create_from_rational(numerator, denominator);
</code></pre>




<a id="0x1_fixed_point32_CreateFromRationalAbortsIf"></a>


<pre><code>schema CreateFromRationalAbortsIf &#123;
    numerator: u64;
    denominator: u64;
    let scaled_numerator &#61; (numerator as u128)&lt;&lt; 64;
    let scaled_denominator &#61; (denominator as u128) &lt;&lt; 32;
    let quotient &#61; scaled_numerator / scaled_denominator;
    aborts_if scaled_denominator &#61;&#61; 0 with EDENOMINATOR;
    aborts_if quotient &#61;&#61; 0 &amp;&amp; scaled_numerator !&#61; 0 with ERATIO_OUT_OF_RANGE;
    aborts_if quotient &gt; MAX_U64 with ERATIO_OUT_OF_RANGE;
&#125;
</code></pre>




<a id="0x1_fixed_point32_spec_create_from_rational"></a>


<pre><code>fun spec_create_from_rational(numerator: num, denominator: num): FixedPoint32 &#123;
   FixedPoint32&#123;value: (numerator &lt;&lt; 64) / (denominator &lt;&lt; 32)&#125;
&#125;
</code></pre>



<a id="@Specification_1_create_from_raw_value"></a>

### Function `create_from_raw_value`


<pre><code>public fun create_from_raw_value(value: u64): fixed_point32::FixedPoint32
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result.value &#61;&#61; value;
</code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code>public fun min(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_min(num1, num2);
</code></pre>




<a id="0x1_fixed_point32_spec_min"></a>


<pre><code>fun spec_min(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;
   if (num1.value &lt; num2.value) &#123;
       num1
   &#125; else &#123;
       num2
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_max"></a>

### Function `max`


<pre><code>public fun max(num1: fixed_point32::FixedPoint32, num2: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_max(num1, num2);
</code></pre>




<a id="0x1_fixed_point32_spec_max"></a>


<pre><code>fun spec_max(num1: FixedPoint32, num2: FixedPoint32): FixedPoint32 &#123;
   if (num1.value &gt; num2.value) &#123;
       num1
   &#125; else &#123;
       num2
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_create_from_u64"></a>

### Function `create_from_u64`


<pre><code>public fun create_from_u64(val: u64): fixed_point32::FixedPoint32
</code></pre>




<pre><code>pragma opaque;
include CreateFromU64AbortsIf;
ensures result &#61;&#61; spec_create_from_u64(val);
</code></pre>




<a id="0x1_fixed_point32_CreateFromU64AbortsIf"></a>


<pre><code>schema CreateFromU64AbortsIf &#123;
    val: num;
    let scaled_value &#61; (val as u128) &lt;&lt; 32;
    aborts_if scaled_value &gt; MAX_U64;
&#125;
</code></pre>




<a id="0x1_fixed_point32_spec_create_from_u64"></a>


<pre><code>fun spec_create_from_u64(val: num): FixedPoint32 &#123;
   FixedPoint32 &#123;value: val &lt;&lt; 32&#125;
&#125;
</code></pre>



<a id="@Specification_1_floor"></a>

### Function `floor`


<pre><code>public fun floor(num: fixed_point32::FixedPoint32): u64
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_floor(num);
</code></pre>




<a id="0x1_fixed_point32_spec_floor"></a>


<pre><code>fun spec_floor(val: FixedPoint32): u64 &#123;
   let fractional &#61; val.value % (1 &lt;&lt; 32);
   if (fractional &#61;&#61; 0) &#123;
       val.value &gt;&gt; 32
   &#125; else &#123;
       (val.value &#45; fractional) &gt;&gt; 32
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_ceil"></a>

### Function `ceil`


<pre><code>public fun ceil(num: fixed_point32::FixedPoint32): u64
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_ceil(num);
</code></pre>




<a id="0x1_fixed_point32_spec_ceil"></a>


<pre><code>fun spec_ceil(val: FixedPoint32): u64 &#123;
   let fractional &#61; val.value % (1 &lt;&lt; 32);
   let one &#61; 1 &lt;&lt; 32;
   if (fractional &#61;&#61; 0) &#123;
       val.value &gt;&gt; 32
   &#125; else &#123;
       (val.value &#45; fractional &#43; one) &gt;&gt; 32
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_round"></a>

### Function `round`


<pre><code>public fun round(num: fixed_point32::FixedPoint32): u64
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_round(num);
</code></pre>




<a id="0x1_fixed_point32_spec_round"></a>


<pre><code>fun spec_round(val: FixedPoint32): u64 &#123;
   let fractional &#61; val.value % (1 &lt;&lt; 32);
   let boundary &#61; (1 &lt;&lt; 32) / 2;
   let one &#61; 1 &lt;&lt; 32;
   if (fractional &lt; boundary) &#123;
       (val.value &#45; fractional) &gt;&gt; 32
   &#125; else &#123;
       (val.value &#45; fractional &#43; one) &gt;&gt; 32
   &#125;
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
