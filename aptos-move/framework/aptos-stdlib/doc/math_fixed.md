
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


<pre><code>use 0x1::error;<br/>use 0x1::fixed_point32;<br/>use 0x1::math128;<br/></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math_fixed_EOVERFLOW_EXP"></a>

Abort code on overflow


<pre><code>const EOVERFLOW_EXP: u64 &#61; 1;<br/></code></pre>



<a id="0x1_math_fixed_LN2"></a>

Natural log 2 in 32 bit fixed point


<pre><code>const LN2: u128 &#61; 2977044472;<br/></code></pre>



<a id="0x1_math_fixed_LN2_X_32"></a>



<pre><code>const LN2_X_32: u64 &#61; 95265423104;<br/></code></pre>



<a id="0x1_math_fixed_sqrt"></a>

## Function `sqrt`

Square root of fixed point number


<pre><code>public fun sqrt(x: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sqrt(x: FixedPoint32): FixedPoint32 &#123;<br/>    let y &#61; (fixed_point32::get_raw_value(x) as u128);<br/>    fixed_point32::create_from_raw_value((math128::sqrt(y &lt;&lt; 32) as u64))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_exp"></a>

## Function `exp`

Exponent function with a precission of 9 digits.


<pre><code>public fun exp(x: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun exp(x: FixedPoint32): FixedPoint32 &#123;<br/>    let raw_value &#61; (fixed_point32::get_raw_value(x) as u128);<br/>    fixed_point32::create_from_raw_value((exp_raw(raw_value) as u64))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_log2_plus_32"></a>

## Function `log2_plus_32`

Because log2 is negative for values &lt; 1 we instead return log2(x) &#43; 32 which<br/> is positive for all values of x.


<pre><code>public fun log2_plus_32(x: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun log2_plus_32(x: FixedPoint32): FixedPoint32 &#123;<br/>    let raw_value &#61; (fixed_point32::get_raw_value(x) as u128);<br/>    math128::log2(raw_value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_ln_plus_32ln2"></a>

## Function `ln_plus_32ln2`



<pre><code>public fun ln_plus_32ln2(x: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ln_plus_32ln2(x: FixedPoint32): FixedPoint32 &#123;<br/>    let raw_value &#61; (fixed_point32::get_raw_value(x) as u128);<br/>    let x &#61; (fixed_point32::get_raw_value(math128::log2(raw_value)) as u128);<br/>    fixed_point32::create_from_raw_value((x &#42; LN2 &gt;&gt; 32 as u64))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_pow"></a>

## Function `pow`

Integer power of a fixed point number


<pre><code>public fun pow(x: fixed_point32::FixedPoint32, n: u64): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pow(x: FixedPoint32, n: u64): FixedPoint32 &#123;<br/>    let raw_value &#61; (fixed_point32::get_raw_value(x) as u128);<br/>    fixed_point32::create_from_raw_value((pow_raw(raw_value, (n as u128)) as u64))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_mul_div"></a>

## Function `mul_div`

Specialized function for x &#42; y / z that omits intermediate shifting


<pre><code>public fun mul_div(x: fixed_point32::FixedPoint32, y: fixed_point32::FixedPoint32, z: fixed_point32::FixedPoint32): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mul_div(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 &#123;<br/>    let a &#61; fixed_point32::get_raw_value(x);<br/>    let b &#61; fixed_point32::get_raw_value(y);<br/>    let c &#61; fixed_point32::get_raw_value(z);<br/>    fixed_point32::create_from_raw_value (math64::mul_div(a, b, c))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_exp_raw"></a>

## Function `exp_raw`



<pre><code>fun exp_raw(x: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun exp_raw(x: u128): u128 &#123;<br/>    // exp(x / 2^32) &#61; 2^(x / (2^32 &#42; ln(2))) &#61; 2^(floor(x / (2^32 &#42; ln(2))) &#43; frac(x / (2^32 &#42; ln(2))))<br/>    let shift_long &#61; x / LN2;<br/>    assert!(shift_long &lt;&#61; 31, std::error::invalid_state(EOVERFLOW_EXP));<br/>    let shift &#61; (shift_long as u8);<br/>    let remainder &#61; x % LN2;<br/>    // At this point we want to calculate 2^(remainder / ln2) &lt;&lt; shift<br/>    // ln2 &#61; 595528 &#42; 4999 which means<br/>    let bigfactor &#61; 595528;<br/>    let exponent &#61; remainder / bigfactor;<br/>    let x &#61; remainder % bigfactor;<br/>    // 2^(remainder / ln2) &#61; (2^(1/4999))^exponent &#42; exp(x / 2^32)<br/>    let roottwo &#61; 4295562865;  // fixed point representation of 2^(1/4999)<br/>    // This has an error of 5000 / 4 10^9 roughly 6 digits of precission<br/>    let power &#61; pow_raw(roottwo, exponent);<br/>    let eps_correction &#61; 1241009291;<br/>    power &#61; power &#43; ((power &#42; eps_correction &#42; exponent) &gt;&gt; 64);<br/>    // x is fixed point number smaller than 595528/2^32 &lt; 0.00014 so we need only 2 tayler steps<br/>    // to get the 6 digits of precission<br/>    let taylor1 &#61; (power &#42; x) &gt;&gt; (32 &#45; shift);<br/>    let taylor2 &#61; (taylor1 &#42; x) &gt;&gt; 32;<br/>    let taylor3 &#61; (taylor2 &#42; x) &gt;&gt; 32;<br/>    (power &lt;&lt; shift) &#43; taylor1 &#43; taylor2 / 2 &#43; taylor3 / 6<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed_pow_raw"></a>

## Function `pow_raw`



<pre><code>fun pow_raw(x: u128, n: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pow_raw(x: u128, n: u128): u128 &#123;<br/>    let res: u256 &#61; 1 &lt;&lt; 64;<br/>    x &#61; x &lt;&lt; 32;<br/>    while (n !&#61; 0) &#123;<br/>        if (n &amp; 1 !&#61; 0) &#123;<br/>            res &#61; (res &#42; (x as u256)) &gt;&gt; 64;<br/>        &#125;;<br/>        n &#61; n &gt;&gt; 1;<br/>        x &#61; ((((x as u256) &#42; (x as u256)) &gt;&gt; 64) as u128);<br/>    &#125;;<br/>    ((res &gt;&gt; 32) as u128)<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
