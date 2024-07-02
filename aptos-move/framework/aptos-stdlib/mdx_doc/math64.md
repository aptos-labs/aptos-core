
<a id="0x1_math64"></a>

# Module `0x1::math64`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `max`](#0x1_math64_max)
-  [Function `min`](#0x1_math64_min)
-  [Function `average`](#0x1_math64_average)
-  [Function `gcd`](#0x1_math64_gcd)
-  [Function `mul_div`](#0x1_math64_mul_div)
-  [Function `clamp`](#0x1_math64_clamp)
-  [Function `pow`](#0x1_math64_pow)
-  [Function `floor_log2`](#0x1_math64_floor_log2)
-  [Function `log2`](#0x1_math64_log2)
-  [Function `sqrt`](#0x1_math64_sqrt)
-  [Function `ceil_div`](#0x1_math64_ceil_div)
-  [Specification](#@Specification_1)
    -  [Function `max`](#@Specification_1_max)
    -  [Function `min`](#@Specification_1_min)
    -  [Function `average`](#@Specification_1_average)
    -  [Function `clamp`](#@Specification_1_clamp)
    -  [Function `pow`](#@Specification_1_pow)
    -  [Function `floor_log2`](#@Specification_1_floor_log2)
    -  [Function `sqrt`](#@Specification_1_sqrt)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;<br /></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math64_EINVALID_ARG_FLOOR_LOG2"></a>

Cannot log2 the value 0


<pre><code><b>const</b> <a href="math64.md#0x1_math64_EINVALID_ARG_FLOOR_LOG2">EINVALID_ARG_FLOOR_LOG2</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_math64_max"></a>

## Function `max`

Return the largest of two numbers.


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_max">max</a>(a: u64, b: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_max">max</a>(a: u64, b: u64): u64 &#123;<br />    <b>if</b> (a &gt;&#61; b) a <b>else</b> b<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_min"></a>

## Function `min`

Return the smallest of two numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u64, b: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u64, b: u64): u64 &#123;<br />    <b>if</b> (a &lt; b) a <b>else</b> b<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_average"></a>

## Function `average`

Return the average of two.


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_average">average</a>(a: u64, b: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_average">average</a>(a: u64, b: u64): u64 &#123;<br />    <b>if</b> (a &lt; b) &#123;<br />        a &#43; (b &#45; a) / 2<br />    &#125; <b>else</b> &#123;<br />        b &#43; (a &#45; b) / 2<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_gcd"></a>

## Function `gcd`

Return greatest common divisor of <code>a</code> &amp; <code>b</code>, via the Euclidean algorithm.


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_gcd">gcd</a>(a: u64, b: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math64.md#0x1_math64_gcd">gcd</a>(a: u64, b: u64): u64 &#123;<br />    <b>let</b> (large, small) &#61; <b>if</b> (a &gt; b) (a, b) <b>else</b> (b, a);<br />    <b>while</b> (small !&#61; 0) &#123;<br />        <b>let</b> tmp &#61; small;<br />        small &#61; large % small;<br />        large &#61; tmp;<br />    &#125;;<br />    large<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_mul_div"></a>

## Function `mul_div`

Returns a &#42; b / c going through u128 to prevent intermediate overflow


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_mul_div">mul_div</a>(a: u64, b: u64, c: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math64.md#0x1_math64_mul_div">mul_div</a>(a: u64, b: u64, c: u64): u64 &#123;<br />    // Inline functions cannot take constants, <b>as</b> then every <b>module</b> using it needs the constant<br />    <b>assert</b>!(c !&#61; 0, std::error::invalid_argument(4));<br />    (((a <b>as</b> u128) &#42; (b <b>as</b> u128) / (c <b>as</b> u128)) <b>as</b> u64)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_clamp"></a>

## Function `clamp`

Return x clamped to the interval [lower, upper].


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_clamp">clamp</a>(x: u64, lower: u64, upper: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_clamp">clamp</a>(x: u64, lower: u64, upper: u64): u64 &#123;<br />    <b>min</b>(upper, <a href="math64.md#0x1_math64_max">max</a>(lower, x))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_pow"></a>

## Function `pow`

Return the value of n raised to power e


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_pow">pow</a>(n: u64, e: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_pow">pow</a>(n: u64, e: u64): u64 &#123;<br />    <b>if</b> (e &#61;&#61; 0) &#123;<br />        1<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> p &#61; 1;<br />        <b>while</b> (e &gt; 1) &#123;<br />            <b>if</b> (e % 2 &#61;&#61; 1) &#123;<br />                p &#61; p &#42; n;<br />            &#125;;<br />            e &#61; e / 2;<br />            n &#61; n &#42; n;<br />        &#125;;<br />        p &#42; n<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_floor_log2"></a>

## Function `floor_log2`

Returns floor(lg2(x))


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_floor_log2">floor_log2</a>(x: u64): u8<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_floor_log2">floor_log2</a>(x: u64): u8 &#123;<br />    <b>let</b> res &#61; 0;<br />    <b>assert</b>!(x !&#61; 0, std::error::invalid_argument(<a href="math64.md#0x1_math64_EINVALID_ARG_FLOOR_LOG2">EINVALID_ARG_FLOOR_LOG2</a>));<br />    // Effectively the position of the most significant set bit<br />    <b>let</b> n &#61; 32;<br />    <b>while</b> (n &gt; 0) &#123;<br />        <b>if</b> (x &gt;&#61; (1 &lt;&lt; n)) &#123;<br />            x &#61; x &gt;&gt; n;<br />            res &#61; res &#43; n;<br />        &#125;;<br />        n &#61; n &gt;&gt; 1;<br />    &#125;;<br />    res<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_log2"></a>

## Function `log2`



<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_log2">log2</a>(x: u64): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_log2">log2</a>(x: u64): FixedPoint32 &#123;<br />    <b>let</b> integer_part &#61; <a href="math64.md#0x1_math64_floor_log2">floor_log2</a>(x);<br />    // Normalize x <b>to</b> [1, 2) in fixed point 32.<br />    <b>let</b> y &#61; (<b>if</b> (x &gt;&#61; 1 &lt;&lt; 32) &#123;<br />        x &gt;&gt; (integer_part &#45; 32)<br />    &#125; <b>else</b> &#123;<br />        x &lt;&lt; (32 &#45; integer_part)<br />    &#125; <b>as</b> u128);<br />    <b>let</b> frac &#61; 0;<br />    <b>let</b> delta &#61; 1 &lt;&lt; 31;<br />    <b>while</b> (delta !&#61; 0) &#123;<br />        // log x &#61; 1/2 log x^2<br />        // x in [1, 2)<br />        y &#61; (y &#42; y) &gt;&gt; 32;<br />        // x is now in [1, 4)<br />        // <b>if</b> x in [2, 4) then log x &#61; 1 &#43; log (x / 2)<br />        <b>if</b> (y &gt;&#61; (2 &lt;&lt; 32)) &#123; frac &#61; frac &#43; delta; y &#61; y &gt;&gt; 1; &#125;;<br />        delta &#61; delta &gt;&gt; 1;<br />    &#125;;<br />    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a> (((integer_part <b>as</b> u64) &lt;&lt; 32) &#43; frac)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_sqrt"></a>

## Function `sqrt`

Returns square root of x, precisely floor(sqrt(x))


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_sqrt">sqrt</a>(x: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_sqrt">sqrt</a>(x: u64): u64 &#123;<br />    <b>if</b> (x &#61;&#61; 0) <b>return</b> 0;<br />    // Note the plus 1 in the expression. Let n &#61; floor_lg2(x) we have x in [2^n, 2^(n&#43;1)&gt; and thus the answer in<br />    // the half&#45;open interval [2^(n/2), 2^((n&#43;1)/2)&gt;. For even n we can write this <b>as</b> [2^(n/2), <a href="math64.md#0x1_math64_sqrt">sqrt</a>(2) 2^(n/2)&gt;<br />    // for odd n [2^((n&#43;1)/2)/<a href="math64.md#0x1_math64_sqrt">sqrt</a>(2), 2^((n&#43;1)/2&gt;. For even n the left end point is integer for odd the right<br />    // end point is integer. If we <b>choose</b> <b>as</b> our first approximation the integer end point we have <b>as</b> maximum<br />    // relative <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> either (<a href="math64.md#0x1_math64_sqrt">sqrt</a>(2) &#45; 1) or (1 &#45; 1/<a href="math64.md#0x1_math64_sqrt">sqrt</a>(2)) both are smaller then 1/2.<br />    <b>let</b> res &#61; 1 &lt;&lt; ((<a href="math64.md#0x1_math64_floor_log2">floor_log2</a>(x) &#43; 1) &gt;&gt; 1);<br />    // We <b>use</b> standard newton&#45;rhapson iteration <b>to</b> improve the initial approximation.<br />    // The <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> term evolves <b>as</b> delta_i&#43;1 &#61; delta_i^2 / 2 (quadratic convergence).<br />    // It turns out that after 4 iterations the delta is smaller than 2^&#45;32 and thus below the treshold.<br />    res &#61; (res &#43; x / res) &gt;&gt; 1;<br />    res &#61; (res &#43; x / res) &gt;&gt; 1;<br />    res &#61; (res &#43; x / res) &gt;&gt; 1;<br />    res &#61; (res &#43; x / res) &gt;&gt; 1;<br />    <b>min</b>(res, x / res)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_math64_ceil_div"></a>

## Function `ceil_div`



<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_ceil_div">ceil_div</a>(x: u64, y: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math64.md#0x1_math64_ceil_div">ceil_div</a>(x: u64, y: u64): u64 &#123;<br />    // <a href="math64.md#0x1_math64_ceil_div">ceil_div</a>(x, y) &#61; floor((x &#43; y &#45; 1) / y) &#61; floor((x &#45; 1) / y) &#43; 1<br />    // (x &#43; y &#45; 1) could spuriously overflow. so we <b>use</b> the later version<br />    <b>if</b> (x &#61;&#61; 0) &#123;<br />        // Inline functions cannot take constants, <b>as</b> then every <b>module</b> using it needs the constant<br />        <b>assert</b>!(y !&#61; 0, std::error::invalid_argument(4));<br />        0<br />    &#125;<br />    <b>else</b> (x &#45; 1) / y &#43; 1<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_max">max</a>(a: u64, b: u64): u64<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> a &gt;&#61; b &#61;&#61;&gt; result &#61;&#61; a;<br /><b>ensures</b> a &lt; b &#61;&#61;&gt; result &#61;&#61; b;<br /></code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u64, b: u64): u64<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> a &lt; b &#61;&#61;&gt; result &#61;&#61; a;<br /><b>ensures</b> a &gt;&#61; b &#61;&#61;&gt; result &#61;&#61; b;<br /></code></pre>



<a id="@Specification_1_average"></a>

### Function `average`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_average">average</a>(a: u64, b: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (a &#43; b) / 2;<br /></code></pre>



<a id="@Specification_1_clamp"></a>

### Function `clamp`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_clamp">clamp</a>(x: u64, lower: u64, upper: u64): u64<br /></code></pre>




<pre><code><b>requires</b> (lower &lt;&#61; upper);<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> (lower &lt;&#61;x &amp;&amp; x &lt;&#61; upper) &#61;&#61;&gt; result &#61;&#61; x;<br /><b>ensures</b> (x &lt; lower) &#61;&#61;&gt; result &#61;&#61; lower;<br /><b>ensures</b> (upper &lt; x) &#61;&#61;&gt; result &#61;&#61; upper;<br /></code></pre>



<a id="@Specification_1_pow"></a>

### Function `pow`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_pow">pow</a>(n: u64, e: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n, e) &gt; MAX_U64;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n, e);<br /></code></pre>



<a id="@Specification_1_floor_log2"></a>

### Function `floor_log2`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_floor_log2">floor_log2</a>(x: u64): u8<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] x &#61;&#61; 0;<br /><b>ensures</b> [abstract] <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(2, result) &lt;&#61; x;<br /><b>ensures</b> [abstract] x &lt; <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(2, result&#43;1);<br /></code></pre>



<a id="@Specification_1_sqrt"></a>

### Function `sqrt`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_sqrt">sqrt</a>(x: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] x &gt; 0 &#61;&#61;&gt; result &#42; result &lt;&#61; x;<br /><b>ensures</b> [abstract] x &gt; 0 &#61;&#61;&gt; x &lt; (result&#43;1) &#42; (result&#43;1);<br /></code></pre>




<a id="0x1_math64_spec_pow"></a>


<pre><code><b>fun</b> <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n: u64, e: u64): u64 &#123;<br />   <b>if</b> (e &#61;&#61; 0) &#123;<br />       1<br />   &#125;<br />   <b>else</b> &#123;<br />       n &#42; <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n, e&#45;1)<br />   &#125;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
