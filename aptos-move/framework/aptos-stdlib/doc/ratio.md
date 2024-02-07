
<a id="0x1_ratio"></a>

# Module `0x1::ratio`



-  [Struct `Ratio`](#0x1_ratio_Ratio)
-  [Constants](#@Constants_0)
-  [Function `from_terms`](#0x1_ratio_from_terms)
-  [Function `inverse`](#0x1_ratio_inverse)
-  [Function `from_int`](#0x1_ratio_from_int)
-  [Function `is_zero`](#0x1_ratio_is_zero)
-  [Function `is_unity`](#0x1_ratio_is_unity)
-  [Function `is_infinity`](#0x1_ratio_is_infinity)
-  [Function `is_nan`](#0x1_ratio_is_nan)
-  [Function `is_special`](#0x1_ratio_is_special)
-  [Function `to_terms`](#0x1_ratio_to_terms)
-  [Function `identical`](#0x1_ratio_identical)
-  [Function `to_quotient_and_remainder`](#0x1_ratio_to_quotient_and_remainder)
-  [Function `to_quotient_and_remainder_unchecked`](#0x1_ratio_to_quotient_and_remainder_unchecked)
-  [Function `reduce`](#0x1_ratio_reduce)
-  [Function `reduce_unchecked`](#0x1_ratio_reduce_unchecked)
-  [Function `less_than`](#0x1_ratio_less_than)
-  [Function `less_than_or_equal`](#0x1_ratio_less_than_or_equal)
-  [Function `equal`](#0x1_ratio_equal)
-  [Function `greater_than_or_equal`](#0x1_ratio_greater_than_or_equal)
-  [Function `greater_than`](#0x1_ratio_greater_than)
-  [Function `less_than_unchecked`](#0x1_ratio_less_than_unchecked)
-  [Function `less_than_or_equal_unchecked`](#0x1_ratio_less_than_or_equal_unchecked)
-  [Function `equal_unchecked`](#0x1_ratio_equal_unchecked)
-  [Function `greater_than_or_equal_unchecked`](#0x1_ratio_greater_than_or_equal_unchecked)
-  [Function `greater_than_unchecked`](#0x1_ratio_greater_than_unchecked)
-  [Function `sort`](#0x1_ratio_sort)
-  [Function `sort_unchecked`](#0x1_ratio_sort_unchecked)
-  [Function `times_int_as_int`](#0x1_ratio_times_int_as_int)
-  [Function `times_int_as_int_unchecked`](#0x1_ratio_times_int_as_int_unchecked)
-  [Function `multiply`](#0x1_ratio_multiply)
-  [Function `multiply_unchecked`](#0x1_ratio_multiply_unchecked)
-  [Function `divide`](#0x1_ratio_divide)
-  [Function `divide_unchecked`](#0x1_ratio_divide_unchecked)
-  [Function `add`](#0x1_ratio_add)
-  [Function `add_unchecked`](#0x1_ratio_add_unchecked)
-  [Function `subtract`](#0x1_ratio_subtract)
-  [Function `subtract_unchecked`](#0x1_ratio_subtract_unchecked)


<pre><code></code></pre>



<a id="0x1_ratio_Ratio"></a>

## Struct `Ratio`

Special cases defined as follows:
- infinity := n:0 (n nonzero)
- zero := 0:d (d nonzero)
- NaN := 0:0


<pre><code><b>struct</b> <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>n: u64</code>
</dt>
<dd>
 Numerator.
</dd>
<dt>
<code>d: u64</code>
</dt>
<dd>
 Denominator.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ratio_E_DIVIDE_BY_ZERO"></a>

Attempting to divide by zero.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>: u64 = 10;
</code></pre>



<a id="0x1_ratio_E_INFINITY"></a>

Input was inifinity.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_INFINITY">E_INFINITY</a>: u64 = 3;
</code></pre>



<a id="0x1_ratio_E_INFINITY_DIVIDED_BY_INFINITY"></a>

May not divide infinity by infinity.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_INFINITY_DIVIDED_BY_INFINITY">E_INFINITY_DIVIDED_BY_INFINITY</a>: u64 = 11;
</code></pre>



<a id="0x1_ratio_E_NAN"></a>

Input was not a number.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_NAN">E_NAN</a>: u64 = 0;
</code></pre>



<a id="0x1_ratio_E_NAN_LHS"></a>

Input on the left hand side of the comparator was not a number.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>: u64 = 1;
</code></pre>



<a id="0x1_ratio_E_NAN_RHS"></a>

Input on the right hand side of the comparator was not a number.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>: u64 = 2;
</code></pre>



<a id="0x1_ratio_E_OVERFLOW"></a>

Result overflows a u64.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_OVERFLOW">E_OVERFLOW</a>: u64 = 4;
</code></pre>



<a id="0x1_ratio_E_OVERFLOW_DENOMINATOR"></a>

Result denominator overflows a u64.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_OVERFLOW_DENOMINATOR">E_OVERFLOW_DENOMINATOR</a>: u64 = 7;
</code></pre>



<a id="0x1_ratio_E_OVERFLOW_NUMERATOR"></a>

Result numerator overflows a u64.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_OVERFLOW_NUMERATOR">E_OVERFLOW_NUMERATOR</a>: u64 = 6;
</code></pre>



<a id="0x1_ratio_E_SUBTRACT_INFINITY"></a>

Attempting to subtract infinity.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_SUBTRACT_INFINITY">E_SUBTRACT_INFINITY</a>: u64 = 9;
</code></pre>



<a id="0x1_ratio_E_UNDERFLOW_NUMERATOR"></a>

Result underflows numerator.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_UNDERFLOW_NUMERATOR">E_UNDERFLOW_NUMERATOR</a>: u64 = 8;
</code></pre>



<a id="0x1_ratio_E_ZERO_TIMES_INFINITY"></a>

Zero times infinity is undefined.


<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_E_ZERO_TIMES_INFINITY">E_ZERO_TIMES_INFINITY</a>: u64 = 5;
</code></pre>



<a id="0x1_ratio_U64_MAX"></a>



<pre><code><b>const</b> <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_ratio_from_terms"></a>

## Function `from_terms`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_from_terms">from_terms</a>(x: u64, y: u64): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_from_terms">from_terms</a>(x: u64, y: u64): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: x, d: y } }
</code></pre>



</details>

<a id="0x1_ratio_inverse"></a>

## Function `inverse`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_inverse">inverse</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_inverse">inverse</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: r.d, d: r.n } }
</code></pre>



</details>

<a id="0x1_ratio_from_int"></a>

## Function `from_int`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_from_int">from_int</a>(i: u64): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_from_int">from_int</a>(i: u64): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: i, d: 1 } }
</code></pre>



</details>

<a id="0x1_ratio_is_zero"></a>

## Function `is_zero`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_zero">is_zero</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_zero">is_zero</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool { r.n == 0 && r.d != 0 }
</code></pre>



</details>

<a id="0x1_ratio_is_unity"></a>

## Function `is_unity`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_unity">is_unity</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_unity">is_unity</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool { r.n != 0 && r.n == r.d }
</code></pre>



</details>

<a id="0x1_ratio_is_infinity"></a>

## Function `is_infinity`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool { r.n != 0 && r.d == 0 }
</code></pre>



</details>

<a id="0x1_ratio_is_nan"></a>

## Function `is_nan`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool { r.n == 0 && r.d == 0 }
</code></pre>



</details>

<a id="0x1_ratio_is_special"></a>

## Function `is_special`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_special">is_special</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_is_special">is_special</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool { r.n == 0 || r.d == 0 }
</code></pre>



</details>

<a id="0x1_ratio_to_terms"></a>

## Function `to_terms`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_to_terms">to_terms</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_to_terms">to_terms</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): (u64, u64) { (r.n, r.d) }
</code></pre>



</details>

<a id="0x1_ratio_identical"></a>

## Function `identical`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_identical">identical</a>(a: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, b: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_identical">identical</a>(a: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, b: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool { a == b }
</code></pre>



</details>

<a id="0x1_ratio_to_quotient_and_remainder"></a>

## Function `to_quotient_and_remainder`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_to_quotient_and_remainder">to_quotient_and_remainder</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_to_quotient_and_remainder">to_quotient_and_remainder</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): (u64, u64) {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN">E_NAN</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r), <a href="ratio.md#0x1_ratio_E_INFINITY">E_INFINITY</a>);
    <a href="ratio.md#0x1_ratio_to_quotient_and_remainder_unchecked">to_quotient_and_remainder_unchecked</a>(r)
}
</code></pre>



</details>

<a id="0x1_ratio_to_quotient_and_remainder_unchecked"></a>

## Function `to_quotient_and_remainder_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_to_quotient_and_remainder_unchecked">to_quotient_and_remainder_unchecked</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_to_quotient_and_remainder_unchecked">to_quotient_and_remainder_unchecked</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): (u64, u64) {
    (r.n / r.d, r.n % r.d)
}
</code></pre>



</details>

<a id="0x1_ratio_reduce"></a>

## Function `reduce`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_reduce">reduce</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_reduce">reduce</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN">E_NAN</a>);
    <a href="ratio.md#0x1_ratio_reduce_unchecked">reduce_unchecked</a>(r)
}
</code></pre>



</details>

<a id="0x1_ratio_reduce_unchecked"></a>

## Function `reduce_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_reduce_unchecked">reduce_unchecked</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_reduce_unchecked">reduce_unchecked</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>let</b> gcd = <a href="math64.md#0x1_math64_gcd">math64::gcd</a>(r.n, r.d);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: r.n / gcd, d: r.d / gcd }
}
</code></pre>



</details>

<a id="0x1_ratio_less_than"></a>

## Function `less_than`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than">less_than</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than">less_than</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <a href="ratio.md#0x1_ratio_less_than_unchecked">less_than_unchecked</a>(l, r)
}
</code></pre>



</details>

<a id="0x1_ratio_less_than_or_equal"></a>

## Function `less_than_or_equal`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than_or_equal">less_than_or_equal</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than_or_equal">less_than_or_equal</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <a href="ratio.md#0x1_ratio_less_than_or_equal_unchecked">less_than_or_equal_unchecked</a>(l, r)
}
</code></pre>



</details>

<a id="0x1_ratio_equal"></a>

## Function `equal`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_equal">equal</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_equal">equal</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <a href="ratio.md#0x1_ratio_equal_unchecked">equal_unchecked</a>(l, r)
}
</code></pre>



</details>

<a id="0x1_ratio_greater_than_or_equal"></a>

## Function `greater_than_or_equal`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than_or_equal">greater_than_or_equal</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than_or_equal">greater_than_or_equal</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <a href="ratio.md#0x1_ratio_greater_than_or_equal_unchecked">greater_than_or_equal_unchecked</a>(l, r)
}
</code></pre>



</details>

<a id="0x1_ratio_greater_than"></a>

## Function `greater_than`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than">greater_than</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than">greater_than</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <a href="ratio.md#0x1_ratio_greater_than_unchecked">greater_than_unchecked</a>(l, r)
}
</code></pre>



</details>

<a id="0x1_ratio_less_than_unchecked"></a>

## Function `less_than_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than_unchecked">less_than_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than_unchecked">less_than_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
   ((l.n <b>as</b> u128) * (r.d <b>as</b> u128)) &lt; ((r.n <b>as</b> u128) * (l.d <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_ratio_less_than_or_equal_unchecked"></a>

## Function `less_than_or_equal_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than_or_equal_unchecked">less_than_or_equal_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_less_than_or_equal_unchecked">less_than_or_equal_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
   ((l.n <b>as</b> u128) * (r.d <b>as</b> u128)) &lt;= ((r.n <b>as</b> u128) * (l.d <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_ratio_equal_unchecked"></a>

## Function `equal_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_equal_unchecked">equal_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_equal_unchecked">equal_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
   ((l.n <b>as</b> u128) * (r.d <b>as</b> u128)) == ((r.n <b>as</b> u128) * (l.d <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_ratio_greater_than_or_equal_unchecked"></a>

## Function `greater_than_or_equal_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than_or_equal_unchecked">greater_than_or_equal_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than_or_equal_unchecked">greater_than_or_equal_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
   ((l.n <b>as</b> u128) * (r.d <b>as</b> u128)) &gt;= ((r.n <b>as</b> u128) * (l.d <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_ratio_greater_than_unchecked"></a>

## Function `greater_than_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than_unchecked">greater_than_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_greater_than_unchecked">greater_than_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): bool {
   ((l.n <b>as</b> u128) * (r.d <b>as</b> u128)) &gt; ((r.n <b>as</b> u128) * (l.d <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_ratio_sort"></a>

## Function `sort`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_sort">sort</a>(x: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): (<a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ratio.md#0x1_ratio_sort">sort</a>(x: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): (<a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>) {
    <b>if</b> (<a href="ratio.md#0x1_ratio_less_than">less_than</a>(x, y)) (x, y) <b>else</b> (y, x)
}
</code></pre>



</details>

<a id="0x1_ratio_sort_unchecked"></a>

## Function `sort_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_sort_unchecked">sort_unchecked</a>(x: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): (<a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="ratio.md#0x1_ratio_sort_unchecked">sort_unchecked</a>(x: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): (<a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>) {
    <b>if</b> (<a href="ratio.md#0x1_ratio_less_than_unchecked">less_than_unchecked</a>(x, y)) (x, y) <b>else</b> (y, x)
}
</code></pre>



</details>

<a id="0x1_ratio_times_int_as_int"></a>

## Function `times_int_as_int`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_times_int_as_int">times_int_as_int</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, i: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_times_int_as_int">times_int_as_int</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, i: u64): u64 {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN">E_NAN</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r), <a href="ratio.md#0x1_ratio_E_INFINITY">E_INFINITY</a>);
    <b>let</b> result = ((r.n <b>as</b> u128) * (i <b>as</b> u128)) / (r.d <b>as</b> u128);
    <b>assert</b>!(result &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW">E_OVERFLOW</a>);
    (result <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_ratio_times_int_as_int_unchecked"></a>

## Function `times_int_as_int_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_times_int_as_int_unchecked">times_int_as_int_unchecked</a>(r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, i: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_times_int_as_int_unchecked">times_int_as_int_unchecked</a>(r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, i: u64): u64 {
    (((r.n <b>as</b> u128) * (i <b>as</b> u128)) / (r.d <b>as</b> u128) <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_ratio_multiply"></a>

## Function `multiply`

inf * inf = inf
inf * n = inf
inf * 0 = error


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_multiply">multiply</a>(x: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_multiply">multiply</a>(x: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>let</b> (l, r) = <a href="ratio.md#0x1_ratio_sort">sort</a>(x, y);
    <b>let</b> zero_times_infinity = <a href="ratio.md#0x1_ratio_is_zero">is_zero</a>(l) && <a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r);
    <b>assert</b>!(!zero_times_infinity, <a href="ratio.md#0x1_ratio_E_ZERO_TIMES_INFINITY">E_ZERO_TIMES_INFINITY</a>);
    <b>let</b> n = (l.n <b>as</b> u128) * (r.n <b>as</b> u128);
    <b>let</b> d = (l.d <b>as</b> u128) * (r.d <b>as</b> u128);
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <b>let</b> n_reduced = n / gcd;
    <b>assert</b>!(n_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_NUMERATOR">E_OVERFLOW_NUMERATOR</a>);
    <b>let</b> d_reduced = d / gcd;
    <b>assert</b>!(d_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_DENOMINATOR">E_OVERFLOW_DENOMINATOR</a>);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n_reduced <b>as</b> u64), d: (d_reduced <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_multiply_unchecked"></a>

## Function `multiply_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_multiply_unchecked">multiply_unchecked</a>(x: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_multiply_unchecked">multiply_unchecked</a>(x: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, y: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>let</b> n = (x.n <b>as</b> u128) * (y.n <b>as</b> u128);
    <b>let</b> d = (x.d <b>as</b> u128) * (y.d <b>as</b> u128);
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: ((n / gcd) <b>as</b> u64), d: ((d / gcd) <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_divide"></a>

## Function `divide`

inf / inf = error
inf / N = inf
N / inf = 0
0 / N = 0
0 / inf = 0
_ / 0 = error


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_divide">divide</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_divide">divide</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_zero">is_zero</a>(r), <a href="ratio.md#0x1_ratio_E_DIVIDE_BY_ZERO">E_DIVIDE_BY_ZERO</a>);
    <b>if</b> (<a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(l)) {
        <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r), <a href="ratio.md#0x1_ratio_E_INFINITY_DIVIDED_BY_INFINITY">E_INFINITY_DIVIDED_BY_INFINITY</a>);
        <b>return</b> <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: 1, d: 0 }
    };
    <b>let</b> n = ((l.n <b>as</b> u128) * (r.d <b>as</b> u128));
    <b>let</b> d = ((r.n <b>as</b> u128) * (l.d <b>as</b> u128));
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <b>let</b> n_reduced = n / gcd;
    <b>assert</b>!(n_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_NUMERATOR">E_OVERFLOW_NUMERATOR</a>);
    <b>let</b> d_reduced = d / gcd;
    <b>assert</b>!(d_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_DENOMINATOR">E_OVERFLOW_DENOMINATOR</a>);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n_reduced <b>as</b> u64), d: (d_reduced <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_divide_unchecked"></a>

## Function `divide_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_divide_unchecked">divide_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_divide_unchecked">divide_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>let</b> n = ((l.n <b>as</b> u128) * (r.d <b>as</b> u128));
    <b>let</b> d = ((r.n <b>as</b> u128) * (l.d <b>as</b> u128));
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n / gcd <b>as</b> u64), d: (d / gcd <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_add"></a>

## Function `add`

inf + inf = inf
inf + n = inf
inf + 0 = inf


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_add">add</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_add">add</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <b>if</b> (l.d == 0 || r.d == 0) <b>return</b> <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: 1, d: 0 };
    <b>let</b> l_d = (l.d <b>as</b> u128);
    <b>let</b> r_d = (r.d <b>as</b> u128);
    <b>let</b> n = ((l.n <b>as</b> u128) * r_d) + ((r.n <b>as</b> u128) * l_d);
    <b>let</b> d = (l_d * r_d);
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <b>let</b> n_reduced = n / gcd;
    <b>assert</b>!(n_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_NUMERATOR">E_OVERFLOW_NUMERATOR</a>);
    <b>let</b> d_reduced = d / gcd;
    <b>assert</b>!(d_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_DENOMINATOR">E_OVERFLOW_DENOMINATOR</a>);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n_reduced <b>as</b> u64), d: (d_reduced <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_add_unchecked"></a>

## Function `add_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_add_unchecked">add_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_add_unchecked">add_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>let</b> l_d = (l.d <b>as</b> u128);
    <b>let</b> r_d = (r.d <b>as</b> u128);
    <b>let</b> n = ((l.n <b>as</b> u128) * r_d) + ((r.n <b>as</b> u128) * l_d);
    <b>let</b> d = (l_d * r_d);
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n / gcd <b>as</b> u64), d: (d / gcd <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_subtract"></a>

## Function `subtract`

Infinity minus n = infinity
Infinity minus 0 = infinity
_ - inf = error


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_subtract">subtract</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_subtract">subtract</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(l), <a href="ratio.md#0x1_ratio_E_NAN_LHS">E_NAN_LHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_nan">is_nan</a>(r), <a href="ratio.md#0x1_ratio_E_NAN_RHS">E_NAN_RHS</a>);
    <b>assert</b>!(!<a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(r), <a href="ratio.md#0x1_ratio_E_SUBTRACT_INFINITY">E_SUBTRACT_INFINITY</a>);
    <b>if</b> (<a href="ratio.md#0x1_ratio_is_infinity">is_infinity</a>(l)) <b>return</b> <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: 1, d: 0 };
    <b>let</b> l_d = (l.d <b>as</b> u128);
    <b>let</b> r_d = (r.d <b>as</b> u128);
    <b>let</b> a = ((l.n <b>as</b> u128) * r_d);
    <b>let</b> b = ((r.n <b>as</b> u128) * l_d);
    <b>assert</b>!(a &gt;= b, <a href="ratio.md#0x1_ratio_E_UNDERFLOW_NUMERATOR">E_UNDERFLOW_NUMERATOR</a>);
    <b>let</b> n = a - b;
    <b>let</b> d = (l_d * r_d);
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <b>let</b> n_reduced = n / gcd;
    <b>assert</b>!(n_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_NUMERATOR">E_OVERFLOW_NUMERATOR</a>);
    <b>let</b> d_reduced = d / gcd;
    <b>assert</b>!(d_reduced &lt;= <a href="ratio.md#0x1_ratio_U64_MAX">U64_MAX</a>, <a href="ratio.md#0x1_ratio_E_OVERFLOW_DENOMINATOR">E_OVERFLOW_DENOMINATOR</a>);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n_reduced <b>as</b> u64), d: (d_reduced <b>as</b> u64) }
}
</code></pre>



</details>

<a id="0x1_ratio_subtract_unchecked"></a>

## Function `subtract_unchecked`



<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_subtract_unchecked">subtract_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">ratio::Ratio</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ratio.md#0x1_ratio_subtract_unchecked">subtract_unchecked</a>(l: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>, r: <a href="ratio.md#0x1_ratio_Ratio">Ratio</a>): <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> {
    <b>let</b> l_d = (l.d <b>as</b> u128);
    <b>let</b> r_d = (r.d <b>as</b> u128);
    <b>let</b> n = ((l.n <b>as</b> u128) * r_d) - ((r.n <b>as</b> u128) * l_d);
    <b>let</b> d = (l_d * r_d);
    <b>let</b> gcd = <a href="math128.md#0x1_math128_gcd">math128::gcd</a>(n, d);
    <a href="ratio.md#0x1_ratio_Ratio">Ratio</a> { n: (n / gcd <b>as</b> u64), d: (d / gcd <b>as</b> u64) }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
