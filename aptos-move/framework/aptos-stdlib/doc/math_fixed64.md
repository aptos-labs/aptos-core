
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


<pre><code>use 0x1::error;<br/>use 0x1::fixed_point64;<br/>use 0x1::math128;<br/></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math_fixed64_EOVERFLOW_EXP"></a>

Abort code on overflow


<pre><code>const EOVERFLOW_EXP: u64 &#61; 1;<br/></code></pre>



<a id="0x1_math_fixed64_LN2"></a>

Natural log 2 in 32 bit fixed point


<pre><code>const LN2: u256 &#61; 12786308645202655660;<br/></code></pre>



<a id="0x1_math_fixed64_sqrt"></a>

## Function `sqrt`

Square root of fixed point number


<pre><code>public fun sqrt(x: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sqrt(x: FixedPoint64): FixedPoint64 &#123;<br/>    let y &#61; fixed_point64::get_raw_value(x);<br/>    let z &#61; (math128::sqrt(y) &lt;&lt; 32 as u256);<br/>    z &#61; (z &#43; ((y as u256) &lt;&lt; 64) / z) &gt;&gt; 1;<br/>    fixed_point64::create_from_raw_value((z as u128))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_exp"></a>

## Function `exp`

Exponent function with a precission of 9 digits.


<pre><code>public fun exp(x: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun exp(x: FixedPoint64): FixedPoint64 &#123;<br/>    let raw_value &#61; (fixed_point64::get_raw_value(x) as u256);<br/>    fixed_point64::create_from_raw_value((exp_raw(raw_value) as u128))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_log2_plus_64"></a>

## Function `log2_plus_64`

Because log2 is negative for values < 1 we instead return log2(x) + 64 which
is positive for all values of x.


<pre><code>public fun log2_plus_64(x: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun log2_plus_64(x: FixedPoint64): FixedPoint64 &#123;<br/>    let raw_value &#61; (fixed_point64::get_raw_value(x) as u128);<br/>    math128::log2_64(raw_value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_ln_plus_32ln2"></a>

## Function `ln_plus_32ln2`



<pre><code>public fun ln_plus_32ln2(x: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ln_plus_32ln2(x: FixedPoint64): FixedPoint64 &#123;<br/>    let raw_value &#61; fixed_point64::get_raw_value(x);<br/>    let x &#61; (fixed_point64::get_raw_value(math128::log2_64(raw_value)) as u256);<br/>    fixed_point64::create_from_raw_value(((x &#42; LN2) &gt;&gt; 64 as u128))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_pow"></a>

## Function `pow`

Integer power of a fixed point number


<pre><code>public fun pow(x: fixed_point64::FixedPoint64, n: u64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pow(x: FixedPoint64, n: u64): FixedPoint64 &#123;<br/>    let raw_value &#61; (fixed_point64::get_raw_value(x) as u256);<br/>    fixed_point64::create_from_raw_value((pow_raw(raw_value, (n as u128)) as u128))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_mul_div"></a>

## Function `mul_div`

Specialized function for x * y / z that omits intermediate shifting


<pre><code>public fun mul_div(x: fixed_point64::FixedPoint64, y: fixed_point64::FixedPoint64, z: fixed_point64::FixedPoint64): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mul_div(x: FixedPoint64, y: FixedPoint64, z: FixedPoint64): FixedPoint64 &#123;<br/>    let a &#61; fixed_point64::get_raw_value(x);<br/>    let b &#61; fixed_point64::get_raw_value(y);<br/>    let c &#61; fixed_point64::get_raw_value(z);<br/>    fixed_point64::create_from_raw_value (math128::mul_div(a, b, c))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_exp_raw"></a>

## Function `exp_raw`



<pre><code>fun exp_raw(x: u256): u256<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun exp_raw(x: u256): u256 &#123;<br/>    // exp(x / 2^64) &#61; 2^(x / (2^64 &#42; ln(2))) &#61; 2^(floor(x / (2^64 &#42; ln(2))) &#43; frac(x / (2^64 &#42; ln(2))))<br/>    let shift_long &#61; x / LN2;<br/>    assert!(shift_long &lt;&#61; 63, std::error::invalid_state(EOVERFLOW_EXP));<br/>    let shift &#61; (shift_long as u8);<br/>    let remainder &#61; x % LN2;<br/>    // At this point we want to calculate 2^(remainder / ln2) &lt;&lt; shift<br/>    // ln2 &#61; 580 &#42; 22045359733108027<br/>    let bigfactor &#61; 22045359733108027;<br/>    let exponent &#61; remainder / bigfactor;<br/>    let x &#61; remainder % bigfactor;<br/>    // 2^(remainder / ln2) &#61; (2^(1/580))^exponent &#42; exp(x / 2^64)<br/>    let roottwo &#61; 18468802611690918839;  // fixed point representation of 2^(1/580)<br/>    // 2^(1/580) &#61; roottwo(1 &#45; eps), so the number we seek is roottwo^exponent (1 &#45; eps &#42; exponent)<br/>    let power &#61; pow_raw(roottwo, (exponent as u128));<br/>    let eps_correction &#61; 219071715585908898;<br/>    power &#61; power &#45; ((power &#42; eps_correction &#42; exponent) &gt;&gt; 128);<br/>    // x is fixed point number smaller than bigfactor/2^64 &lt; 0.0011 so we need only 5 tayler steps<br/>    // to get the 15 digits of precission<br/>    let taylor1 &#61; (power &#42; x) &gt;&gt; (64 &#45; shift);<br/>    let taylor2 &#61; (taylor1 &#42; x) &gt;&gt; 64;<br/>    let taylor3 &#61; (taylor2 &#42; x) &gt;&gt; 64;<br/>    let taylor4 &#61; (taylor3 &#42; x) &gt;&gt; 64;<br/>    let taylor5 &#61; (taylor4 &#42; x) &gt;&gt; 64;<br/>    let taylor6 &#61; (taylor5 &#42; x) &gt;&gt; 64;<br/>    (power &lt;&lt; shift) &#43; taylor1 &#43; taylor2 / 2 &#43; taylor3 / 6 &#43; taylor4 / 24 &#43; taylor5 / 120 &#43; taylor6 / 720<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math_fixed64_pow_raw"></a>

## Function `pow_raw`



<pre><code>fun pow_raw(x: u256, n: u128): u256<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun pow_raw(x: u256, n: u128): u256 &#123;<br/>    let res: u256 &#61; 1 &lt;&lt; 64;<br/>    while (n !&#61; 0) &#123;<br/>        if (n &amp; 1 !&#61; 0) &#123;<br/>            res &#61; (res &#42; x) &gt;&gt; 64;<br/>        &#125;;<br/>        n &#61; n &gt;&gt; 1;<br/>        x &#61; (x &#42; x) &gt;&gt; 64;<br/>    &#125;;<br/>    res<br/>&#125;<br/></code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
