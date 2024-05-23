
<a id="0x1_math128"></a>

# Module `0x1::math128`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `max`](#0x1_math128_max)
-  [Function `min`](#0x1_math128_min)
-  [Function `average`](#0x1_math128_average)
-  [Function `gcd`](#0x1_math128_gcd)
-  [Function `mul_div`](#0x1_math128_mul_div)
-  [Function `clamp`](#0x1_math128_clamp)
-  [Function `pow`](#0x1_math128_pow)
-  [Function `floor_log2`](#0x1_math128_floor_log2)
-  [Function `log2`](#0x1_math128_log2)
-  [Function `log2_64`](#0x1_math128_log2_64)
-  [Function `sqrt`](#0x1_math128_sqrt)
-  [Function `ceil_div`](#0x1_math128_ceil_div)
-  [Specification](#@Specification_1)
    -  [Function `max`](#@Specification_1_max)
    -  [Function `min`](#@Specification_1_min)
    -  [Function `average`](#@Specification_1_average)
    -  [Function `clamp`](#@Specification_1_clamp)
    -  [Function `pow`](#@Specification_1_pow)
    -  [Function `floor_log2`](#@Specification_1_floor_log2)
    -  [Function `sqrt`](#@Specification_1_sqrt)


<pre><code>use 0x1::error;<br/>use 0x1::fixed_point32;<br/>use 0x1::fixed_point64;<br/></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math128_EINVALID_ARG_FLOOR_LOG2"></a>

Cannot log2 the value 0


<pre><code>const EINVALID_ARG_FLOOR_LOG2: u64 &#61; 1;<br/></code></pre>



<a id="0x1_math128_max"></a>

## Function `max`

Return the largest of two numbers.


<pre><code>public fun max(a: u128, b: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max(a: u128, b: u128): u128 &#123;<br/>    if (a &gt;&#61; b) a else b<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_min"></a>

## Function `min`

Return the smallest of two numbers.


<pre><code>public fun min(a: u128, b: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun min(a: u128, b: u128): u128 &#123;<br/>    if (a &lt; b) a else b<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_average"></a>

## Function `average`

Return the average of two.


<pre><code>public fun average(a: u128, b: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun average(a: u128, b: u128): u128 &#123;<br/>    if (a &lt; b) &#123;<br/>        a &#43; (b &#45; a) / 2<br/>    &#125; else &#123;<br/>        b &#43; (a &#45; b) / 2<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_gcd"></a>

## Function `gcd`

Return greatest common divisor of <code>a</code> &amp; <code>b</code>, via the Euclidean algorithm.


<pre><code>public fun gcd(a: u128, b: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun gcd(a: u128, b: u128): u128 &#123;<br/>    let (large, small) &#61; if (a &gt; b) (a, b) else (b, a);<br/>    while (small !&#61; 0) &#123;<br/>        let tmp &#61; small;<br/>        small &#61; large % small;<br/>        large &#61; tmp;<br/>    &#125;;<br/>    large<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_mul_div"></a>

## Function `mul_div`

Returns a &#42; b / c going through u256 to prevent intermediate overflow


<pre><code>public fun mul_div(a: u128, b: u128, c: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun mul_div(a: u128, b: u128, c: u128): u128 &#123;<br/>    // Inline functions cannot take constants, as then every module using it needs the constant<br/>    assert!(c !&#61; 0, std::error::invalid_argument(4));<br/>    (((a as u256) &#42; (b as u256) / (c as u256)) as u128)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_clamp"></a>

## Function `clamp`

Return x clamped to the interval [lower, upper].


<pre><code>public fun clamp(x: u128, lower: u128, upper: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun clamp(x: u128, lower: u128, upper: u128): u128 &#123;<br/>    min(upper, max(lower, x))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_pow"></a>

## Function `pow`

Return the value of n raised to power e


<pre><code>public fun pow(n: u128, e: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pow(n: u128, e: u128): u128 &#123;<br/>    if (e &#61;&#61; 0) &#123;<br/>        1<br/>    &#125; else &#123;<br/>        let p &#61; 1;<br/>        while (e &gt; 1) &#123;<br/>            if (e % 2 &#61;&#61; 1) &#123;<br/>                p &#61; p &#42; n;<br/>            &#125;;<br/>            e &#61; e / 2;<br/>            n &#61; n &#42; n;<br/>        &#125;;<br/>        p &#42; n<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_floor_log2"></a>

## Function `floor_log2`

Returns floor(log2(x))


<pre><code>public fun floor_log2(x: u128): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun floor_log2(x: u128): u8 &#123;<br/>    let res &#61; 0;<br/>    assert!(x !&#61; 0, std::error::invalid_argument(EINVALID_ARG_FLOOR_LOG2));<br/>    // Effectively the position of the most significant set bit<br/>    let n &#61; 64;<br/>    while (n &gt; 0) &#123;<br/>        if (x &gt;&#61; (1 &lt;&lt; n)) &#123;<br/>            x &#61; x &gt;&gt; n;<br/>            res &#61; res &#43; n;<br/>        &#125;;<br/>        n &#61; n &gt;&gt; 1;<br/>    &#125;;<br/>    res<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_log2"></a>

## Function `log2`



<pre><code>public fun log2(x: u128): fixed_point32::FixedPoint32<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun log2(x: u128): FixedPoint32 &#123;<br/>    let integer_part &#61; floor_log2(x);<br/>    // Normalize x to [1, 2) in fixed point 32.<br/>    if (x &gt;&#61; 1 &lt;&lt; 32) &#123;<br/>        x &#61; x &gt;&gt; (integer_part &#45; 32);<br/>    &#125; else &#123;<br/>        x &#61; x &lt;&lt; (32 &#45; integer_part);<br/>    &#125;;<br/>    let frac &#61; 0;<br/>    let delta &#61; 1 &lt;&lt; 31;<br/>    while (delta !&#61; 0) &#123;<br/>        // log x &#61; 1/2 log x^2<br/>        // x in [1, 2)<br/>        x &#61; (x &#42; x) &gt;&gt; 32;<br/>        // x is now in [1, 4)<br/>        // if x in [2, 4) then log x &#61; 1 &#43; log (x / 2)<br/>        if (x &gt;&#61; (2 &lt;&lt; 32)) &#123; frac &#61; frac &#43; delta; x &#61; x &gt;&gt; 1; &#125;;<br/>        delta &#61; delta &gt;&gt; 1;<br/>    &#125;;<br/>    fixed_point32::create_from_raw_value (((integer_part as u64) &lt;&lt; 32) &#43; frac)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_log2_64"></a>

## Function `log2_64`



<pre><code>public fun log2_64(x: u128): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun log2_64(x: u128): FixedPoint64 &#123;<br/>    let integer_part &#61; floor_log2(x);<br/>    // Normalize x to [1, 2) in fixed point 63. To ensure x is smaller then 1&lt;&lt;64<br/>    if (x &gt;&#61; 1 &lt;&lt; 63) &#123;<br/>        x &#61; x &gt;&gt; (integer_part &#45; 63);<br/>    &#125; else &#123;<br/>        x &#61; x &lt;&lt; (63 &#45; integer_part);<br/>    &#125;;<br/>    let frac &#61; 0;<br/>    let delta &#61; 1 &lt;&lt; 63;<br/>    while (delta !&#61; 0) &#123;<br/>        // log x &#61; 1/2 log x^2<br/>        // x in [1, 2)<br/>        x &#61; (x &#42; x) &gt;&gt; 63;<br/>        // x is now in [1, 4)<br/>        // if x in [2, 4) then log x &#61; 1 &#43; log (x / 2)<br/>        if (x &gt;&#61; (2 &lt;&lt; 63)) &#123; frac &#61; frac &#43; delta; x &#61; x &gt;&gt; 1; &#125;;<br/>        delta &#61; delta &gt;&gt; 1;<br/>    &#125;;<br/>    fixed_point64::create_from_raw_value (((integer_part as u128) &lt;&lt; 64) &#43; frac)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_sqrt"></a>

## Function `sqrt`

Returns square root of x, precisely floor(sqrt(x))


<pre><code>public fun sqrt(x: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sqrt(x: u128): u128 &#123;<br/>    if (x &#61;&#61; 0) return 0;<br/>    // Note the plus 1 in the expression. Let n &#61; floor_lg2(x) we have x in [2^n, 2^&#123;n&#43;1&#125;) and thus the answer in<br/>    // the half&#45;open interval [2^(n/2), 2^&#123;(n&#43;1)/2&#125;). For even n we can write this as [2^(n/2), sqrt(2) 2^&#123;n/2&#125;)<br/>    // for odd n [2^((n&#43;1)/2)/sqrt(2), 2^((n&#43;1)/2). For even n the left end point is integer for odd the right<br/>    // end point is integer. If we choose as our first approximation the integer end point we have as maximum<br/>    // relative error either (sqrt(2) &#45; 1) or (1 &#45; 1/sqrt(2)) both are smaller then 1/2.<br/>    let res &#61; 1 &lt;&lt; ((floor_log2(x) &#43; 1) &gt;&gt; 1);<br/>    // We use standard newton&#45;rhapson iteration to improve the initial approximation.<br/>    // The error term evolves as delta_i&#43;1 &#61; delta_i^2 / 2 (quadratic convergence).<br/>    // It turns out that after 5 iterations the delta is smaller than 2^&#45;64 and thus below the treshold.<br/>    res &#61; (res &#43; x / res) &gt;&gt; 1;<br/>    res &#61; (res &#43; x / res) &gt;&gt; 1;<br/>    res &#61; (res &#43; x / res) &gt;&gt; 1;<br/>    res &#61; (res &#43; x / res) &gt;&gt; 1;<br/>    res &#61; (res &#43; x / res) &gt;&gt; 1;<br/>    min(res, x / res)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_math128_ceil_div"></a>

## Function `ceil_div`



<pre><code>public fun ceil_div(x: u128, y: u128): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun ceil_div(x: u128, y: u128): u128 &#123;<br/>    // ceil_div(x, y) &#61; floor((x &#43; y &#45; 1) / y) &#61; floor((x &#45; 1) / y) &#43; 1<br/>    // (x &#43; y &#45; 1) could spuriously overflow. so we use the later version<br/>    if (x &#61;&#61; 0) &#123;<br/>        // Inline functions cannot take constants, as then every module using it needs the constant<br/>        assert!(y !&#61; 0, std::error::invalid_argument(4));<br/>        0<br/>    &#125;<br/>    else (x &#45; 1) / y &#43; 1<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_max"></a>

### Function `max`


<pre><code>public fun max(a: u128, b: u128): u128<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures a &gt;&#61; b &#61;&#61;&gt; result &#61;&#61; a;<br/>ensures a &lt; b &#61;&#61;&gt; result &#61;&#61; b;<br/></code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code>public fun min(a: u128, b: u128): u128<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures a &lt; b &#61;&#61;&gt; result &#61;&#61; a;<br/>ensures a &gt;&#61; b &#61;&#61;&gt; result &#61;&#61; b;<br/></code></pre>



<a id="@Specification_1_average"></a>

### Function `average`


<pre><code>public fun average(a: u128, b: u128): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; (a &#43; b) / 2;<br/></code></pre>



<a id="@Specification_1_clamp"></a>

### Function `clamp`


<pre><code>public fun clamp(x: u128, lower: u128, upper: u128): u128<br/></code></pre>




<pre><code>requires (lower &lt;&#61; upper);<br/>aborts_if false;<br/>ensures (lower &lt;&#61;x &amp;&amp; x &lt;&#61; upper) &#61;&#61;&gt; result &#61;&#61; x;<br/>ensures (x &lt; lower) &#61;&#61;&gt; result &#61;&#61; lower;<br/>ensures (upper &lt; x) &#61;&#61;&gt; result &#61;&#61; upper;<br/></code></pre>



<a id="@Specification_1_pow"></a>

### Function `pow`


<pre><code>public fun pow(n: u128, e: u128): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] spec_pow(n, e) &gt; MAX_U128;<br/>ensures [abstract] result &#61;&#61; spec_pow(n, e);<br/></code></pre>



<a id="@Specification_1_floor_log2"></a>

### Function `floor_log2`


<pre><code>public fun floor_log2(x: u128): u8<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] x &#61;&#61; 0;<br/>ensures [abstract] spec_pow(2, result) &lt;&#61; x;<br/>ensures [abstract] x &lt; spec_pow(2, result&#43;1);<br/></code></pre>



<a id="@Specification_1_sqrt"></a>

### Function `sqrt`


<pre><code>public fun sqrt(x: u128): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] x &gt; 0 &#61;&#61;&gt; result &#42; result &lt;&#61; x;<br/>ensures [abstract] x &gt; 0 &#61;&#61;&gt; x &lt; (result&#43;1) &#42; (result&#43;1);<br/></code></pre>




<a id="0x1_math128_spec_pow"></a>


<pre><code>fun spec_pow(n: u128, e: u128): u128 &#123;<br/>   if (e &#61;&#61; 0) &#123;<br/>       1<br/>   &#125;<br/>   else &#123;<br/>       n &#42; spec_pow(n, e&#45;1)<br/>   &#125;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
