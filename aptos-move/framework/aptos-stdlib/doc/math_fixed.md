
<a id="0x1_math_fixed"></a>

# Module `0x1::math_fixed`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `sqrt`](#0x1_math_fixed_sqrt)
-  [Function `exp`](#0x1_math_fixed_exp)
-  [Function `log2_plus_32`](#0x1_math_fixed_log2_plus_32)
-  [Function `ln_plus_32ln2`](#0x1_math_fixed_ln_plus_32ln2)
-  [Function `pow`](#0x1_math_fixed_pow)
-  [Function `mul_div`](#0x1_math_fixed_mul_div)
-  [Function `exp_raw`](#0x1_math_fixed_exp_raw)
-  [Function `pow_raw`](#0x1_math_fixed_pow_raw)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;<br /><b>use</b> <a href="math128.md#0x1_math128">0x1::math128</a>;<br /></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math_fixed_EOVERFLOW_EXP"></a>

Abort code on overflow


<pre><code><b>const</b> <a href="math_fixed.md#0x1_math_fixed_EOVERFLOW_EXP">EOVERFLOW_EXP</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_math_fixed_LN2"></a>

Natural log 2 in 32 bit fixed point


<pre><code><b>const</b> <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a>: u128 &#61; 2977044472;<br /></code></pre>



<a id="0x1_math_fixed_LN2_X_32"></a>



<pre><code><b>const</b> <a href="math_fixed.md#0x1_math_fixed_LN2_X_32">LN2_X_32</a>: u64 &#61; 95265423104;<br /></code></pre>



<a id="0x1_math_fixed_sqrt"></a>

## Function `sqrt`

Square root of fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_sqrt">sqrt</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_sqrt">sqrt</a>(x: FixedPoint32): FixedPoint32 &#123;<br />    <b>let</b> y &#61; (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);<br />    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((<a href="math128.md#0x1_math128_sqrt">math128::sqrt</a>(y &lt;&lt; 32) <b>as</b> u64))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_exp"></a>

## Function `exp`

Exponent function with a precission of 9 digits.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x: FixedPoint32): FixedPoint32 &#123;<br />    <b>let</b> raw_value &#61; (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);<br />    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((<a href="math_fixed.md#0x1_math_fixed_exp_raw">exp_raw</a>(raw_value) <b>as</b> u64))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_log2_plus_32"></a>

## Function `log2_plus_32`

Because log2 is negative for values &lt; 1 we instead return log2(x) &#43; 32 which
is positive for all values of x.


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_log2_plus_32">log2_plus_32</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_log2_plus_32">log2_plus_32</a>(x: FixedPoint32): FixedPoint32 &#123;<br />    <b>let</b> raw_value &#61; (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);<br />    <a href="math128.md#0x1_math128_log2">math128::log2</a>(raw_value)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_ln_plus_32ln2"></a>

## Function `ln_plus_32ln2`



<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_ln_plus_32ln2">ln_plus_32ln2</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_ln_plus_32ln2">ln_plus_32ln2</a>(x: FixedPoint32): FixedPoint32 &#123;<br />    <b>let</b> raw_value &#61; (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);<br />    <b>let</b> x &#61; (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(<a href="math128.md#0x1_math128_log2">math128::log2</a>(raw_value)) <b>as</b> u128);<br />    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((x &#42; <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a> &gt;&gt; 32 <b>as</b> u64))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_pow"></a>

## Function `pow`

Integer power of a fixed point number


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow">pow</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, n: u64): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow">pow</a>(x: FixedPoint32, n: u64): FixedPoint32 &#123;<br />    <b>let</b> raw_value &#61; (<a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x) <b>as</b> u128);<br />    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a>((<a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(raw_value, (n <b>as</b> u128)) <b>as</b> u64))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_mul_div"></a>

## Function `mul_div`

Specialized function for x &#42; y / z that omits intermediate shifting


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_mul_div">mul_div</a>(x: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, y: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>, z: <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math_fixed.md#0x1_math_fixed_mul_div">mul_div</a>(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 &#123;<br />    <b>let</b> a &#61; <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(x);<br />    <b>let</b> b &#61; <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(y);<br />    <b>let</b> c &#61; <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_get_raw_value">fixed_point32::get_raw_value</a>(z);<br />    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a> (<a href="math64.md#0x1_math64_mul_div">math64::mul_div</a>(a, b, c))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_exp_raw"></a>

## Function `exp_raw`



<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp_raw">exp_raw</a>(x: u128): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_exp_raw">exp_raw</a>(x: u128): u128 &#123;<br />    // <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x / 2^32) &#61; 2^(x / (2^32 &#42; ln(2))) &#61; 2^(floor(x / (2^32 &#42; ln(2))) &#43; frac(x / (2^32 &#42; ln(2))))<br />    <b>let</b> shift_long &#61; x / <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a>;<br />    <b>assert</b>!(shift_long &lt;&#61; 31, std::error::invalid_state(<a href="math_fixed.md#0x1_math_fixed_EOVERFLOW_EXP">EOVERFLOW_EXP</a>));<br />    <b>let</b> shift &#61; (shift_long <b>as</b> u8);<br />    <b>let</b> remainder &#61; x % <a href="math_fixed.md#0x1_math_fixed_LN2">LN2</a>;<br />    // At this point we want <b>to</b> calculate 2^(remainder / ln2) &lt;&lt; shift<br />    // ln2 &#61; 595528 &#42; 4999 which means<br />    <b>let</b> bigfactor &#61; 595528;<br />    <b>let</b> exponent &#61; remainder / bigfactor;<br />    <b>let</b> x &#61; remainder % bigfactor;<br />    // 2^(remainder / ln2) &#61; (2^(1/4999))^exponent &#42; <a href="math_fixed.md#0x1_math_fixed_exp">exp</a>(x / 2^32)<br />    <b>let</b> roottwo &#61; 4295562865;  // fixed point representation of 2^(1/4999)<br />    // This <b>has</b> an <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> of 5000 / 4 10^9 roughly 6 digits of precission<br />    <b>let</b> power &#61; <a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(roottwo, exponent);<br />    <b>let</b> eps_correction &#61; 1241009291;<br />    power &#61; power &#43; ((power &#42; eps_correction &#42; exponent) &gt;&gt; 64);<br />    // x is fixed point number smaller than 595528/2^32 &lt; 0.00014 so we need only 2 tayler steps<br />    // <b>to</b> get the 6 digits of precission<br />    <b>let</b> taylor1 &#61; (power &#42; x) &gt;&gt; (32 &#45; shift);<br />    <b>let</b> taylor2 &#61; (taylor1 &#42; x) &gt;&gt; 32;<br />    <b>let</b> taylor3 &#61; (taylor2 &#42; x) &gt;&gt; 32;<br />    (power &lt;&lt; shift) &#43; taylor1 &#43; taylor2 / 2 &#43; taylor3 / 6<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math_fixed_pow_raw"></a>

## Function `pow_raw`



<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(x: u128, n: u128): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="math_fixed.md#0x1_math_fixed_pow_raw">pow_raw</a>(x: u128, n: u128): u128 &#123;<br />    <b>let</b> res: u256 &#61; 1 &lt;&lt; 64;<br />    x &#61; x &lt;&lt; 32;<br />    <b>while</b> (n !&#61; 0) &#123;<br />        <b>if</b> (n &amp; 1 !&#61; 0) &#123;<br />            res &#61; (res &#42; (x <b>as</b> u256)) &gt;&gt; 64;<br />        &#125;;<br />        n &#61; n &gt;&gt; 1;<br />        x &#61; ((((x <b>as</b> u256) &#42; (x <b>as</b> u256)) &gt;&gt; 64) <b>as</b> u128);<br />    &#125;;<br />    ((res &gt;&gt; 32) <b>as</b> u128)<br />&#125;<br /></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
