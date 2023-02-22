
<a name="0x1_math_fixed"></a>

# Module `0x1::math_fixed`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `sqrt`](#0x1_math_fixed_sqrt)
-  [Function `exp`](#0x1_math_fixed_exp)
-  [Function `pow`](#0x1_math_fixed_pow)
-  [Function `mul_div`](#0x1_math_fixed_mul_div)
-  [Function `exp_raw`](#0x1_math_fixed_exp_raw)
-  [Function `pow_raw`](#0x1_math_fixed_pow_raw)
-  [Function `assert_approx_the_same`](#0x1_math_fixed_assert_approx_the_same)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
<b>use</b> <a href="math128.md#0x1_math128">0x1::math128</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_math_fixed_EOVERFLOW"></a>

Abort code on overflow


<pre><code><b>const</b> <a href="math_fixed.md#0x1_math_fixed_EOVERFLOW">EOVERFLOW</a>: u64 = 1;
</code></pre>



<a name="0x1_math_fixed_LN2"></a>

Natural log 2 in 32 bit fixed point


<pre><code><b>const</b> <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a>: u128 = 2977044472;
</code></pre>



<a name="0x1_math_fixed_sqrt"></a>

## Function `sqrt`

Square root of fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_sqrt">sqrt</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_sqrt">sqrt</a>(x: FixedPoint32): FixedPoint32 {
    <b>let</b> y = (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);
    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((<a href="math128.md#0x1_math128_sqrt">math128::sqrt</a>(y &lt;&lt; 32) <b>as</b> u64))
}
</code></pre>



</details>

<a name="0x1_math_fixed_exp"></a>

## Function `exp`

Exponent function with a precission of 6 digits.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x: FixedPoint32): FixedPoint32 {
    <b>let</b> raw_value = (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);
    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((<a href="math_fixed.md#0x1_math_fixed_exp_raw">exp_raw</a>(raw_value) <b>as</b> u64))
}
</code></pre>



</details>

<a name="0x1_math_fixed_pow"></a>

## Function `pow`

Integer power of a fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow">pow</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, n: u64): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow">pow</a>(x: FixedPoint32, n: u64): FixedPoint32 {
    <b>let</b> raw_value = (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);
    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((<a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(raw_value, (n <b>as</b> u128)) <b>as</b> u64))
}
</code></pre>



</details>

<a name="0x1_math_fixed_mul_div"></a>

## Function `mul_div`

Specialized function for x * y / z that omits intermediate shifting


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_mul_div">mul_div</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, y: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, z: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_mul_div">mul_div</a>(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 {
    <b>let</b> a = (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);
    <b>let</b> b = (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(y) <b>as</b> u128);
    <b>let</b> c = (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(z) <b>as</b> u128);
    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a> (((a * b / c) <b>as</b> u64))
}
</code></pre>



</details>

<a name="0x1_math_fixed_exp_raw"></a>

## Function `exp_raw`



<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp_raw">exp_raw</a>(x: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp_raw">exp_raw</a>(x: u128): u128 {
    // <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x / 2^32) = 2^(x / (2^32 * ln(2))) = 2^(floor(x / (2^32 * ln(2))) + frac(x / (2^32 * ln(2))))
    <b>let</b> shift_long = x / <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a>;
    <b>assert</b>!(shift_long &lt;= 31, std::error::invalid_state(<a href="math_fixed.md#0x1_math_fixed_EOVERFLOW">EOVERFLOW</a>));
    <b>let</b> shift = (shift_long <b>as</b> u8);
    <b>let</b> remainder = x % <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a>;
    // At this point we want <b>to</b> calculate 2^(remainder / ln2) &lt;&lt; shift
    // ln2 = 595528 * 4999 which means
    <b>let</b> bigfactor = 595528;
    <b>let</b> exponent = remainder / bigfactor;
    <b>let</b> x = remainder % bigfactor;
    // 2^(remainder / ln2) = (2^(1/4999))^exponent * <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x / 2^32)
    <b>let</b> roottwo = 4295562865;  // fixed point representation of 2^(1/4999)
    // This <b>has</b> an <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> of 5000 / 4 10^9 roughly 6 digits of precission
    <b>let</b> power = <a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(roottwo, exponent);
    // x is fixed point number smaller than 595528/2^32 &lt; 0.00014 so we need only 2 tayler steps
    // <b>to</b> get the 6 digits of precission
    <b>let</b> taylor1 = (power * x) &gt;&gt; (32 - shift);
    <b>let</b> taylor2 = (taylor1 * x) &gt;&gt; 32;
    (power &lt;&lt; shift) + taylor1 + taylor2 / 2
}
</code></pre>



</details>

<a name="0x1_math_fixed_pow_raw"></a>

## Function `pow_raw`



<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(x: u128, n: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(x: u128, n: u128): u128 {
    <b>let</b> res: u128 = 1 &lt;&lt; 32;
    <b>while</b> (n != 0) {
        <b>if</b> (n & 1 != 0) {
            res = (res * x) &gt;&gt; 32;
        };
        n = n &gt;&gt; 1;
        x = (x * x) &gt;&gt; 32;
    };
    res
}
</code></pre>



</details>

<a name="0x1_math_fixed_assert_approx_the_same"></a>

## Function `assert_approx_the_same`

For functions that approximate a value it's useful to test a value is close
to the most correct value up to 10^5 digits of precision


<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_assert_approx_the_same">assert_approx_the_same</a>(x: u128, y: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_assert_approx_the_same">assert_approx_the_same</a>(x: u128, y: u128) {
    <b>if</b> (x &lt; y) {
        <b>let</b> tmp = x;
        x = y;
        y = tmp;
    };
    <b>assert</b>!((x - y) * 100000 / x == 0, 0);
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
