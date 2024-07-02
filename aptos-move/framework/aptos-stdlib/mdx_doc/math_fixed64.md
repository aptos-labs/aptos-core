
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;<br /><b>use</b> <a href="math128.md#0x1_math128">0x1::math128</a>;<br /></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math_fixed64_EOVERFLOW_EXP"></a>

Abort code on overflow


<pre><code><b>const</b> <a href="math_fixed64.md#0x1_math_fixed64_EOVERFLOW_EXP">EOVERFLOW_EXP</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_math_fixed64_LN2"></a>

Natural log 2 in 32 bit fixed point


<pre><code><b>const</b> <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>: u256 &#61; 12786308645202655660;<br /></code></pre>



<a id="0x1_math_fixed64_sqrt"></a>

## Function `sqrt`

Square root of fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_sqrt">sqrt</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_sqrt">sqrt</a>(x: FixedPoint64): FixedPoint64 &#123;<br />    <b>let</b> y &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(x);<br />    <b>let</b> z &#61; (<a href="math128.md#0x1_math128_sqrt">math128::sqrt</a>(y) &lt;&lt; 32 <b>as</b> u256);<br />    z &#61; (z &#43; ((y <b>as</b> u256) &lt;&lt; 64) / z) &gt;&gt; 1;<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>((z <b>as</b> u128))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_exp"></a>

## Function `exp`

Exponent function with a precission of 9 digits.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x: FixedPoint64): FixedPoint64 &#123;<br />    <b>let</b> raw_value &#61; (<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(x) <b>as</b> u256);<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>((<a href="math_fixed64.md#0x1_math_fixed64_exp_raw">exp_raw</a>(raw_value) <b>as</b> u128))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_log2_plus_64"></a>

## Function `log2_plus_64`

Because log2 is negative for values &lt; 1 we instead return log2(x) &#43; 64 which
is positive for all values of x.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_log2_plus_64">log2_plus_64</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_log2_plus_64">log2_plus_64</a>(x: FixedPoint64): FixedPoint64 &#123;<br />    <b>let</b> raw_value &#61; (<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(x) <b>as</b> u128);<br />    <a href="math128.md#0x1_math128_log2_64">math128::log2_64</a>(raw_value)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_ln_plus_32ln2"></a>

## Function `ln_plus_32ln2`



<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_ln_plus_32ln2">ln_plus_32ln2</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_ln_plus_32ln2">ln_plus_32ln2</a>(x: FixedPoint64): FixedPoint64 &#123;<br />    <b>let</b> raw_value &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(x);<br />    <b>let</b> x &#61; (<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(<a href="math128.md#0x1_math128_log2_64">math128::log2_64</a>(raw_value)) <b>as</b> u256);<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>(((x &#42; <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>) &gt;&gt; 64 <b>as</b> u128))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_pow"></a>

## Function `pow`

Integer power of a fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow">pow</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, n: u64): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow">pow</a>(x: FixedPoint64, n: u64): FixedPoint64 &#123;<br />    <b>let</b> raw_value &#61; (<a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(x) <b>as</b> u256);<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>((<a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(raw_value, (n <b>as</b> u128)) <b>as</b> u128))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_mul_div"></a>

## Function `mul_div`

Specialized function for x &#42; y / z that omits intermediate shifting


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_mul_div">mul_div</a>(x: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, y: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, z: <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_mul_div">mul_div</a>(x: FixedPoint64, y: FixedPoint64, z: FixedPoint64): FixedPoint64 &#123;<br />    <b>let</b> a &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(x);<br />    <b>let</b> b &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(y);<br />    <b>let</b> c &#61; <a href="fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(z);<br />    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a> (<a href="math128.md#0x1_math128_mul_div">math128::mul_div</a>(a, b, c))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_exp_raw"></a>

## Function `exp_raw`



<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp_raw">exp_raw</a>(x: u256): u256<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_exp_raw">exp_raw</a>(x: u256): u256 &#123;<br />    // <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x / 2^64) &#61; 2^(x / (2^64 &#42; ln(2))) &#61; 2^(floor(x / (2^64 &#42; ln(2))) &#43; frac(x / (2^64 &#42; ln(2))))<br />    <b>let</b> shift_long &#61; x / <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>;<br />    <b>assert</b>!(shift_long &lt;&#61; 63, std::error::invalid_state(<a href="math_fixed64.md#0x1_math_fixed64_EOVERFLOW_EXP">EOVERFLOW_EXP</a>));<br />    <b>let</b> shift &#61; (shift_long <b>as</b> u8);<br />    <b>let</b> remainder &#61; x % <a href="math_fixed64.md#0x1_math_fixed64_LN2">LN2</a>;<br />    // At this point we want <b>to</b> calculate 2^(remainder / ln2) &lt;&lt; shift<br />    // ln2 &#61; 580 &#42; 22045359733108027<br />    <b>let</b> bigfactor &#61; 22045359733108027;<br />    <b>let</b> exponent &#61; remainder / bigfactor;<br />    <b>let</b> x &#61; remainder % bigfactor;<br />    // 2^(remainder / ln2) &#61; (2^(1/580))^exponent &#42; <a href="math_fixed64.md#0x1_math_fixed64_exp">exp</a>(x / 2^64)<br />    <b>let</b> roottwo &#61; 18468802611690918839;  // fixed point representation of 2^(1/580)<br />    // 2^(1/580) &#61; roottwo(1 &#45; eps), so the number we seek is roottwo^exponent (1 &#45; eps &#42; exponent)<br />    <b>let</b> power &#61; <a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(roottwo, (exponent <b>as</b> u128));<br />    <b>let</b> eps_correction &#61; 219071715585908898;<br />    power &#61; power &#45; ((power &#42; eps_correction &#42; exponent) &gt;&gt; 128);<br />    // x is fixed point number smaller than bigfactor/2^64 &lt; 0.0011 so we need only 5 tayler steps<br />    // <b>to</b> get the 15 digits of precission<br />    <b>let</b> taylor1 &#61; (power &#42; x) &gt;&gt; (64 &#45; shift);<br />    <b>let</b> taylor2 &#61; (taylor1 &#42; x) &gt;&gt; 64;<br />    <b>let</b> taylor3 &#61; (taylor2 &#42; x) &gt;&gt; 64;<br />    <b>let</b> taylor4 &#61; (taylor3 &#42; x) &gt;&gt; 64;<br />    <b>let</b> taylor5 &#61; (taylor4 &#42; x) &gt;&gt; 64;<br />    <b>let</b> taylor6 &#61; (taylor5 &#42; x) &gt;&gt; 64;<br />    (power &lt;&lt; shift) &#43; taylor1 &#43; taylor2 / 2 &#43; taylor3 / 6 &#43; taylor4 / 24 &#43; taylor5 / 120 &#43; taylor6 / 720<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed64_pow_raw"></a>

## Function `pow_raw`



<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(x: u256, n: u128): u256<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed64.md#0x1_math_fixed64_pow_raw">pow_raw</a>(x: u256, n: u128): u256 &#123;<br />    <b>let</b> res: u256 &#61; 1 &lt;&lt; 64;<br />    <b>while</b> (n !&#61; 0) &#123;<br />        <b>if</b> (n &amp; 1 !&#61; 0) &#123;<br />            res &#61; (res &#42; x) &gt;&gt; 64;<br />        &#125;;<br />        n &#61; n &gt;&gt; 1;<br />        x &#61; (x &#42; x) &gt;&gt; 64;<br />    &#125;;<br />    res<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
