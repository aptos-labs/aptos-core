
<a id="0x1_math_fixed64"></a>

# Module `0x1::math_fixed64`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `sqrt`](#0x1_math_fixed64_sqrt)
-  [Function `exp`](#0x1_math_fixed64_exp)
-  [Function `log2_plus_64`](#0x1_math_fixed64_log2_plus_64)
-  [Function `ln_plus_32ln2`](#0x1_math_fixed64_ln_plus_32ln2)
-  [Function `pow`](#0x1_math_fixed64_pow)
-  [Function `mul_div`](#0x1_math_fixed64_mul_div)
-  [Function `exp_raw`](#0x1_math_fixed64_exp_raw)
-  [Function `pow_raw`](#0x1_math_fixed64_pow_raw)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="math128.md#0x1_math128">0x1::math128</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math_fixed64_EOVERFLOW_EXP"></a>

Abort code on overflow


<pre><code><b>const</b> <a href="math_fixed64.md#0x1_math_fixed64_EOVERFLOW_EXP">EOVERFLOW_EXP</a>: u64 = 1;
</code></pre>



<a id="0x1_math_fixed64_LN2"></a>

Natural log 2 in 32 bit fixed point


<pre><code><b>const</b> <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>: u256 = 12786308645202655660;
</code></pre>



<a id="0x1_math_fixed64_sqrt"></a>

## Function `sqrt`

Square root of fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_sqrt">sqrt</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_sqrt">sqrt</a>(x: FixedPoint64): FixedPoint64 {
    <b>let</b> y = x.get_raw_value();
    <b>let</b> z = (<a href="math128.md#0x1_math128_sqrt">math128::sqrt</a>(y) &lt;&lt; 32 <b>as</b> u256);
    z = (z + ((y <b>as</b> u256) &lt;&lt; 64) / z) &gt;&gt; 1;
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>((z <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_math_fixed64_exp"></a>

## Function `exp`

Exponent function with a precission of 9 digits.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x: FixedPoint64): FixedPoint64 {
    <b>let</b> raw_value = (x.get_raw_value() <b>as</b> u256);
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>((<a href="math_fixed64.md#0x1_math_fixed64_exp_raw">exp_raw</a>(raw_value) <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_math_fixed64_log2_plus_64"></a>

## Function `log2_plus_64`

Because log2 is negative for values < 1 we instead return log2(x) + 64 which
is positive for all values of x.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_log2_plus_64">log2_plus_64</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_log2_plus_64">log2_plus_64</a>(x: FixedPoint64): FixedPoint64 {
    <b>let</b> raw_value = (x.get_raw_value());
    <a href="math128.md#0x1_math128_log2_64">math128::log2_64</a>(raw_value)
}
</code></pre>



</details>

<a id="0x1_math_fixed64_ln_plus_32ln2"></a>

## Function `ln_plus_32ln2`



<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_ln_plus_32ln2">ln_plus_32ln2</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_ln_plus_32ln2">ln_plus_32ln2</a>(x: FixedPoint64): FixedPoint64 {
    <b>let</b> raw_value = x.get_raw_value();
    <b>let</b> x = (<a href="math128.md#0x1_math128_log2_64">math128::log2_64</a>(raw_value).get_raw_value() <b>as</b> u256);
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>(((x * <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>) &gt;&gt; 64 <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_math_fixed64_pow"></a>

## Function `pow`

Integer power of a fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow">pow</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, n: u64): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow">pow</a>(x: FixedPoint64, n: u64): FixedPoint64 {
    <b>let</b> raw_value = (x.get_raw_value() <b>as</b> u256);
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>((<a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(raw_value, (n <b>as</b> u128)) <b>as</b> u128))
}
</code></pre>



</details>

<a id="0x1_math_fixed64_mul_div"></a>

## Function `mul_div`

Specialized function for x * y / z that omits intermediate shifting


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_mul_div">mul_div</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, z: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_mul_div">mul_div</a>(x: FixedPoint64, y: FixedPoint64, z: FixedPoint64): FixedPoint64 {
    <b>let</b> a = x.get_raw_value();
    <b>let</b> b = y.get_raw_value();
    <b>let</b> c = z.get_raw_value();
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a> (<a href="math128.md#0x1_math128_mul_div">math128::mul_div</a>(a, b, c))
}
</code></pre>



</details>

<a id="0x1_math_fixed64_exp_raw"></a>

## Function `exp_raw`



<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp_raw">exp_raw</a>(x: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp_raw">exp_raw</a>(x: u256): u256 {
    // <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x / 2^64) = 2^(x / (2^64 * ln(2))) = 2^(floor(x / (2^64 * ln(2))) + frac(x / (2^64 * ln(2))))
    <b>let</b> shift_long = x / <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>;
    <b>assert</b>!(shift_long &lt;= 63, std::error::invalid_state(<a href="math_fixed64.md#0x1_math_fixed64_EOVERFLOW_EXP">EOVERFLOW_EXP</a>));
    <b>let</b> shift = (shift_long <b>as</b> u8);
    <b>let</b> remainder = x % <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>;
    // At this point we want <b>to</b> calculate 2^(remainder / ln2) &lt;&lt; shift
    // ln2 = 580 * 22045359733108027
    <b>let</b> bigfactor = 22045359733108027;
    <b>let</b> exponent = remainder / bigfactor;
    <b>let</b> x = remainder % bigfactor;
    // 2^(remainder / ln2) = (2^(1/580))^exponent * <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x / 2^64)
    <b>let</b> roottwo = 18468802611690918839;  // fixed point representation of 2^(1/580)
    // 2^(1/580) = roottwo(1 - eps), so the number we seek is roottwo^exponent (1 - eps * exponent)
    <b>let</b> power = <a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(roottwo, (exponent <b>as</b> u128));
    <b>let</b> eps_correction = 219071715585908898;
    power -= ((power * eps_correction * exponent) &gt;&gt; 128);
    // x is fixed point number smaller than bigfactor/2^64 &lt; 0.0011 so we need only 5 tayler steps
    // <b>to</b> get the 15 digits of precission
    <b>let</b> taylor1 = (power * x) &gt;&gt; (64 - shift);
    <b>let</b> taylor2 = (taylor1 * x) &gt;&gt; 64;
    <b>let</b> taylor3 = (taylor2 * x) &gt;&gt; 64;
    <b>let</b> taylor4 = (taylor3 * x) &gt;&gt; 64;
    <b>let</b> taylor5 = (taylor4 * x) &gt;&gt; 64;
    <b>let</b> taylor6 = (taylor5 * x) &gt;&gt; 64;
    (power &lt;&lt; shift) + taylor1 + taylor2 / 2 + taylor3 / 6 + taylor4 / 24 + taylor5 / 120 + taylor6 / 720
}
</code></pre>



</details>

<a id="0x1_math_fixed64_pow_raw"></a>

## Function `pow_raw`



<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(x: u256, n: u128): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(x: u256, n: u128): u256 {
    <b>let</b> res: u256 = 1 &lt;&lt; 64;
    <b>while</b> (n != 0) {
        <b>if</b> (n & 1 != 0) {
            res = (res * x) &gt;&gt; 64;
        };
        n &gt;&gt;= 1;
        x = (x * x) &gt;&gt; 64;
    };
    res
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
