
<a id="0x1_fixed_point64"></a>

# Module `0x1::fixed_point64`

Defines a fixed-point numeric type with a 64-bit integer part and
a 64-bit fractional part.


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

Define a fixed-point numeric type with 64 fractional bits.
This is just a u128 integer but it is wrapped in a struct to
make a unique type. This is a binary representation, so decimal
values may not be exactly representable, but it provides more
than 9 decimal digits of precision both before and after the
decimal point (18 digits total). For comparison, double precision
floating-point has less than 16 decimal digits of precision, so
be careful about using floating-point to convert these values to
decimal.


<pre><code><b>struct</b> <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



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



<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>: u256 = 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_fixed_point64_EDENOMINATOR"></a>

The denominator provided was zero


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EDENOMINATOR">EDENOMINATOR</a>: u64 = 65537;
</code></pre>



<a id="0x1_fixed_point64_EDIVISION"></a>

The quotient value would be too large to be held in a <code>u128</code>


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION">EDIVISION</a>: u64 = 131074;
</code></pre>



<a id="0x1_fixed_point64_EDIVISION_BY_ZERO"></a>

A division by zero was encountered


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>: u64 = 65540;
</code></pre>



<a id="0x1_fixed_point64_EMULTIPLICATION"></a>

The multiplied value would be too large to be held in a <code>u128</code>


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_EMULTIPLICATION">EMULTIPLICATION</a>: u64 = 131075;
</code></pre>



<a id="0x1_fixed_point64_ERATIO_OUT_OF_RANGE"></a>

The computed ratio when converting to a <code><a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a></code> would be unrepresentable


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>: u64 = 131077;
</code></pre>



<a id="0x1_fixed_point64_ENEGATIVE_RESULT"></a>

Abort code on calculation result is negative.


<pre><code><b>const</b> <a href="fixed_point64.md#0x1_fixed_point64_ENEGATIVE_RESULT">ENEGATIVE_RESULT</a>: u64 = 65542;
</code></pre>



<a id="0x1_fixed_point64_sub"></a>

## Function `sub`

Returns self - y. self must be not less than y.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_sub">sub</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_sub">sub</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    <b>let</b> x_raw = self.<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>();
    <b>let</b> y_raw = y.<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>();
    <b>assert</b>!(x_raw &gt;= y_raw, <a href="fixed_point64.md#0x1_fixed_point64_ENEGATIVE_RESULT">ENEGATIVE_RESULT</a>);
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(x_raw - y_raw)
}
</code></pre>



</details>

<a id="0x1_fixed_point64_add"></a>

## Function `add`

Returns self + y. The result cannot be greater than MAX_U128.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_add">add</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_add">add</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    <b>let</b> x_raw = self.<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>();
    <b>let</b> y_raw = y.<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>();
    <b>let</b> result = (x_raw <b>as</b> u256) + (y_raw <b>as</b> u256);
    <b>assert</b>!(result &lt;= <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>((result <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_fixed_point64_multiply_u128"></a>

## Function `multiply_u128`

Multiply a u128 integer by a fixed-point number, truncating any
fractional part of the product. This will abort if the product
overflows.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_multiply_u128">multiply_u128</a>(val: u128, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_multiply_u128">multiply_u128</a>(val: u128, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
    // The product of two 128 bit values <b>has</b> 256 bits, so perform the
    // multiplication <b>with</b> u256 types and keep the full 256 bit product
    // <b>to</b> avoid losing accuracy.
    <b>let</b> unscaled_product = (val <b>as</b> u256) * (multiplier.value <b>as</b> u256);
    // The unscaled product <b>has</b> 64 fractional bits (from the multiplier)
    // so rescale it by shifting away the low bits.
    <b>let</b> product = unscaled_product &gt;&gt; 64;
    // Check whether the value is too large.
    <b>assert</b>!(product &lt;= <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_EMULTIPLICATION">EMULTIPLICATION</a>);
    (product <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_fixed_point64_divide_u128"></a>

## Function `divide_u128`

Divide a u128 integer by a fixed-point number, truncating any
fractional part of the quotient. This will abort if the divisor
is zero or if the quotient overflows.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_divide_u128">divide_u128</a>(val: u128, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_divide_u128">divide_u128</a>(val: u128, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
    // Check for division by zero.
    <b>assert</b>!(divisor.value != 0, <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>);
    // First convert <b>to</b> 256 bits and then shift left <b>to</b>
    // add 64 fractional zero bits <b>to</b> the dividend.
    <b>let</b> scaled_value = (val <b>as</b> u256) &lt;&lt; 64;
    <b>let</b> quotient = scaled_value / (divisor.value <b>as</b> u256);
    // Check whether the value is too large.
    <b>assert</b>!(quotient &lt;= <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION">EDIVISION</a>);
    // the value may be too large, which will cause the cast <b>to</b> fail
    // <b>with</b> an arithmetic <a href="../../move-stdlib/doc/error.md#0x1_error">error</a>.
    (quotient <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_fixed_point64_create_from_rational"></a>

## Function `create_from_rational`

Create a fixed-point value from a rational number specified by its
numerator and denominator. Calling this function should be preferred
for using <code><a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">Self::create_from_raw_value</a></code> which is also available.
This will abort if the denominator is zero. It will also
abort if the numerator is nonzero and the ratio is not in the range
2^-64 .. 2^64-1. When specifying decimal fractions, be careful about
rounding errors: if you round to display N digits after the decimal
point, you can use a denominator of 10^N to avoid numbers where the
very small imprecision in the binary representation could change the
rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_rational">create_from_rational</a>(numerator: u128, denominator: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_rational">create_from_rational</a>(numerator: u128, denominator: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    // If the denominator is zero, this will <b>abort</b>.
    // Scale the numerator <b>to</b> have 64 fractional bits, so that the quotient will have 64
    // fractional bits.
    <b>let</b> scaled_numerator = (numerator <b>as</b> u256) &lt;&lt; 64;
    <b>assert</b>!(denominator != 0, <a href="fixed_point64.md#0x1_fixed_point64_EDENOMINATOR">EDENOMINATOR</a>);
    <b>let</b> quotient = scaled_numerator / (denominator <b>as</b> u256);
    <b>assert</b>!(quotient != 0 || numerator == 0, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);
    // Return the quotient <b>as</b> a fixed-point number. We first need <b>to</b> check whether the cast
    // can succeed.
    <b>assert</b>!(quotient &lt;= <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);
    <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> { value: (quotient <b>as</b> u128) }
}
</code></pre>



</details>

<a id="0x1_fixed_point64_create_from_raw_value"></a>

## Function `create_from_raw_value`

Create a fixedpoint value from a raw value.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(value: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(value: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> { value }
}
</code></pre>



</details>

<a id="0x1_fixed_point64_get_raw_value"></a>

## Function `get_raw_value`

Accessor for the raw u128 value. Other less common operations, such as
adding or subtracting FixedPoint64 values, can be done using the raw
values directly.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">get_raw_value</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
    self.value
}
</code></pre>



</details>

<a id="0x1_fixed_point64_is_zero"></a>

## Function `is_zero`

Returns true if the ratio is zero.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_is_zero">is_zero</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_is_zero">is_zero</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    self.value == 0
}
</code></pre>



</details>

<a id="0x1_fixed_point64_min"></a>

## Function `min`

Returns the smaller of the two FixedPoint64 numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    <b>if</b> (num1.value &lt; num2.value) {
        num1
    } <b>else</b> {
        num2
    }
}
</code></pre>



</details>

<a id="0x1_fixed_point64_max"></a>

## Function `max`

Returns the larger of the two FixedPoint64 numbers.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_max">max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_max">max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    <b>if</b> (num1.value &gt; num2.value) {
        num1
    } <b>else</b> {
        num2
    }
}
</code></pre>



</details>

<a id="0x1_fixed_point64_less_or_equal"></a>

## Function `less_or_equal`

Returns true if self <= num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less_or_equal">less_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less_or_equal">less_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    self.value &lt;= num2.value
}
</code></pre>



</details>

<a id="0x1_fixed_point64_less"></a>

## Function `less`

Returns true if self < num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less">less</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less">less</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    self.value &lt; num2.value
}
</code></pre>



</details>

<a id="0x1_fixed_point64_greater_or_equal"></a>

## Function `greater_or_equal`

Returns true if self >= num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater_or_equal">greater_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater_or_equal">greater_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    self.value &gt;= num2.value
}
</code></pre>



</details>

<a id="0x1_fixed_point64_greater"></a>

## Function `greater`

Returns true if self > num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater">greater</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater">greater</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    self.value &gt; num2.value
}
</code></pre>



</details>

<a id="0x1_fixed_point64_equal"></a>

## Function `equal`

Returns true if self = num2


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_equal">equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_equal">equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    self.value == num2.value
}
</code></pre>



</details>

<a id="0x1_fixed_point64_almost_equal"></a>

## Function `almost_equal`

Returns true if self almost equals to num2, which means abs(num1-num2) <= precision


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_almost_equal">almost_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_almost_equal">almost_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
    <b>if</b> (self.value &gt; num2.value) {
        (self.value - num2.value &lt;= precision.value)
    } <b>else</b> {
        (num2.value - self.value &lt;= precision.value)
    }
}
</code></pre>



</details>

<a id="0x1_fixed_point64_create_from_u128"></a>

## Function `create_from_u128`

Create a fixedpoint value from a u128 value.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_u128">create_from_u128</a>(val: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_u128">create_from_u128</a>(val: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
    <b>let</b> value = (val <b>as</b> u256) &lt;&lt; 64;
    <b>assert</b>!(value &lt;= <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>, <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>);
    <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {value: (value <b>as</b> u128)}
}
</code></pre>



</details>

<a id="0x1_fixed_point64_floor"></a>

## Function `floor`

Returns the largest integer less than or equal to a given number.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
    self.value &gt;&gt; 64
}
</code></pre>



</details>

<a id="0x1_fixed_point64_ceil"></a>

## Function `ceil`

Rounds up the given FixedPoint64 to the next largest integer.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
    <b>let</b> floored_num = self.<a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>() &lt;&lt; 64;
    <b>if</b> (self.value == floored_num) {
        <b>return</b> floored_num &gt;&gt; 64
    };
    <b>let</b> val = ((floored_num <b>as</b> u256) + (1 &lt;&lt; 64));
    (val &gt;&gt; 64 <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_fixed_point64_round"></a>

## Function `round`

Returns the value of a FixedPoint64 to the nearest integer.


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_round">round</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_round">round</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
    <b>let</b> floored_num = self.<a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>() &lt;&lt; 64;
    <b>let</b> boundary = floored_num + ((1 &lt;&lt; 64) / 2);
    <b>if</b> (self.value &lt; boundary) {
        floored_num &gt;&gt; 64
    } <b>else</b> {
        self.<a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>()
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code><b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_sub">sub</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> self.value &lt; y.value <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ENEGATIVE_RESULT">ENEGATIVE_RESULT</a>;
<b>ensures</b> result.value == self.value - y.value;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_add">add</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> (self.value <b>as</b> u256) + (y.value <b>as</b> u256) &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;
<b>ensures</b> result.value == self.value + y.value;
</code></pre>



<a id="@Specification_1_multiply_u128"></a>

### Function `multiply_u128`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_multiply_u128">multiply_u128</a>(val: u128, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_MultiplyAbortsIf">MultiplyAbortsIf</a>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_multiply_u128">spec_multiply_u128</a>(val, multiplier);
</code></pre>




<a id="0x1_fixed_point64_MultiplyAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_MultiplyAbortsIf">MultiplyAbortsIf</a> {
    val: num;
    multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>;
    <b>aborts_if</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_multiply_u128">spec_multiply_u128</a>(val, multiplier) &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EMULTIPLICATION">EMULTIPLICATION</a>;
}
</code></pre>




<a id="0x1_fixed_point64_spec_multiply_u128"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_multiply_u128">spec_multiply_u128</a>(val: num, multiplier: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): num {
   (val * multiplier.value) &gt;&gt; 64
}
</code></pre>



<a id="@Specification_1_divide_u128"></a>

### Function `divide_u128`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_divide_u128">divide_u128</a>(val: u128, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_DivideAbortsIf">DivideAbortsIf</a>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_divide_u128">spec_divide_u128</a>(val, divisor);
</code></pre>




<a id="0x1_fixed_point64_DivideAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_DivideAbortsIf">DivideAbortsIf</a> {
    val: num;
    divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>;
    <b>aborts_if</b> divisor.value == 0 <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>;
    <b>aborts_if</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_divide_u128">spec_divide_u128</a>(val, divisor) &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EDIVISION">EDIVISION</a>;
}
</code></pre>




<a id="0x1_fixed_point64_spec_divide_u128"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_divide_u128">spec_divide_u128</a>(val: num, divisor: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): num {
   (val &lt;&lt; 64) / divisor.value
}
</code></pre>



<a id="@Specification_1_create_from_rational"></a>

### Function `create_from_rational`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_rational">create_from_rational</a>(numerator: u128, denominator: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> verify_duration_estimate = 1000;
<b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromRationalAbortsIf">CreateFromRationalAbortsIf</a>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_rational">spec_create_from_rational</a>(numerator, denominator);
</code></pre>




<a id="0x1_fixed_point64_CreateFromRationalAbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromRationalAbortsIf">CreateFromRationalAbortsIf</a> {
    numerator: u128;
    denominator: u128;
    <b>let</b> scaled_numerator = (numerator <b>as</b> u256)&lt;&lt; 64;
    <b>let</b> scaled_denominator = (denominator <b>as</b> u256);
    <b>let</b> quotient = scaled_numerator / scaled_denominator;
    <b>aborts_if</b> scaled_denominator == 0 <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_EDENOMINATOR">EDENOMINATOR</a>;
    <b>aborts_if</b> quotient == 0 && scaled_numerator != 0 <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;
    <b>aborts_if</b> quotient &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a> <b>with</b> <a href="fixed_point64.md#0x1_fixed_point64_ERATIO_OUT_OF_RANGE">ERATIO_OUT_OF_RANGE</a>;
}
</code></pre>




<a id="0x1_fixed_point64_spec_create_from_rational"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_rational">spec_create_from_rational</a>(numerator: num, denominator: num): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
   <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>{value: (numerator &lt;&lt; 128) / (denominator &lt;&lt; 64)}
}
</code></pre>



<a id="@Specification_1_create_from_raw_value"></a>

### Function `create_from_raw_value`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">create_from_raw_value</a>(value: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.value == value;
</code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_min">spec_min</a>(num1, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_min"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_min">spec_min</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
   <b>if</b> (num1.value &lt; num2.value) {
       num1
   } <b>else</b> {
       num2
   }
}
</code></pre>



<a id="@Specification_1_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_max">max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_max">spec_max</a>(num1, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_max"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_max">spec_max</a>(num1: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
   <b>if</b> (num1.value &gt; num2.value) {
       num1
   } <b>else</b> {
       num2
   }
}
</code></pre>



<a id="@Specification_1_less_or_equal"></a>

### Function `less_or_equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less_or_equal">less_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">spec_less_or_equal</a>(self, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_less_or_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">spec_less_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
   self.value &lt;= num2.value
}
</code></pre>



<a id="@Specification_1_less"></a>

### Function `less`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_less">less</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_less">spec_less</a>(self, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_less"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_less">spec_less</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
   self.value &lt; num2.value
}
</code></pre>



<a id="@Specification_1_greater_or_equal"></a>

### Function `greater_or_equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater_or_equal">greater_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_greater_or_equal">spec_greater_or_equal</a>(self, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_greater_or_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_greater_or_equal">spec_greater_or_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
   self.value &gt;= num2.value
}
</code></pre>



<a id="@Specification_1_greater"></a>

### Function `greater`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_greater">greater</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_greater">spec_greater</a>(self, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_greater"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_greater">spec_greater</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
   self.value &gt; num2.value
}
</code></pre>



<a id="@Specification_1_equal"></a>

### Function `equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_equal">equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_equal">spec_equal</a>(self, num2);
</code></pre>




<a id="0x1_fixed_point64_spec_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_equal">spec_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
   self.value == num2.value
}
</code></pre>



<a id="@Specification_1_almost_equal"></a>

### Function `almost_equal`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_almost_equal">almost_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_almost_equal">spec_almost_equal</a>(self, num2, precision);
</code></pre>




<a id="0x1_fixed_point64_spec_almost_equal"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_almost_equal">spec_almost_equal</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, num2: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>, precision: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): bool {
   <b>if</b> (self.value &gt; num2.value) {
       (self.value - num2.value &lt;= precision.value)
   } <b>else</b> {
       (num2.value - self.value &lt;= precision.value)
   }
}
</code></pre>



<a id="@Specification_1_create_from_u128"></a>

### Function `create_from_u128`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_create_from_u128">create_from_u128</a>(val: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromU64AbortsIf">CreateFromU64AbortsIf</a>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">spec_create_from_u128</a>(val);
</code></pre>




<a id="0x1_fixed_point64_CreateFromU64AbortsIf"></a>


<pre><code><b>schema</b> <a href="fixed_point64.md#0x1_fixed_point64_CreateFromU64AbortsIf">CreateFromU64AbortsIf</a> {
    val: num;
    <b>let</b> scaled_value = (val <b>as</b> u256) &lt;&lt; 64;
    <b>aborts_if</b> scaled_value &gt; <a href="fixed_point64.md#0x1_fixed_point64_MAX_U128">MAX_U128</a>;
}
</code></pre>




<a id="0x1_fixed_point64_spec_create_from_u128"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">spec_create_from_u128</a>(val: num): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {
   <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a> {value: val &lt;&lt; 64}
}
</code></pre>



<a id="@Specification_1_floor"></a>

### Function `floor`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_floor">floor</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_floor">spec_floor</a>(self);
</code></pre>




<a id="0x1_fixed_point64_spec_floor"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_floor">spec_floor</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
   <b>let</b> fractional = self.value % (1 &lt;&lt; 64);
   <b>if</b> (fractional == 0) {
       self.value &gt;&gt; 64
   } <b>else</b> {
       (self.value - fractional) &gt;&gt; 64
   }
}
</code></pre>



<a id="@Specification_1_ceil"></a>

### Function `ceil`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_ceil">ceil</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1000;
<b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_ceil">spec_ceil</a>(self);
</code></pre>




<a id="0x1_fixed_point64_spec_ceil"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_ceil">spec_ceil</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
   <b>let</b> fractional = self.value % (1 &lt;&lt; 64);
   <b>let</b> one = 1 &lt;&lt; 64;
   <b>if</b> (fractional == 0) {
       self.value &gt;&gt; 64
   } <b>else</b> {
       (self.value - fractional + one) &gt;&gt; 64
   }
}
</code></pre>



<a id="@Specification_1_round"></a>

### Function `round`


<pre><code><b>public</b> <b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_round">round</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="fixed_point64.md#0x1_fixed_point64_spec_round">spec_round</a>(self);
</code></pre>




<a id="0x1_fixed_point64_spec_round"></a>


<pre><code><b>fun</b> <a href="fixed_point64.md#0x1_fixed_point64_spec_round">spec_round</a>(self: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">FixedPoint64</a>): u128 {
   <b>let</b> fractional = self.value % (1 &lt;&lt; 64);
   <b>let</b> boundary = (1 &lt;&lt; 64) / 2;
   <b>let</b> one = 1 &lt;&lt; 64;
   <b>if</b> (fractional &lt; boundary) {
       (self.value - fractional) &gt;&gt; 64
   } <b>else</b> {
       (self.value - fractional + one) &gt;&gt; 64
   }
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
