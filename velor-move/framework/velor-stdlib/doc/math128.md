
<a id="0x1_math128"></a>

# Module `0x1::math128`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `max`](#0x1_math128_max)
-  [Function `min`](#0x1_math128_min)
-  [Function `average`](#0x1_math128_average)
-  [Function `gcd`](#0x1_math128_gcd)
-  [Function `lcm`](#0x1_math128_lcm)
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
<b>use</b> <a href="fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_math128_EINVALID_ARG_FLOOR_LOG2"></a>

Cannot log2 the value 0


<pre><code><b>const</b> <a href="math128.md#0x1_math128_EINVALID_ARG_FLOOR_LOG2">EINVALID_ARG_FLOOR_LOG2</a>: u64 = 1;
</code></pre>



<a id="0x1_math128_max"></a>

## Function `max`

Return the largest of two numbers.


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_max">max</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_max">max</a>(a: u128, b: u128): u128 {
    <b>if</b> (a &gt;= b) a <b>else</b> b
}
</code></pre>



</details>

<a id="0x1_math128_min"></a>

## Function `min`

Return the smallest of two numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u128, b: u128): u128 {
    <b>if</b> (a &lt; b) a <b>else</b> b
}
</code></pre>



</details>

<a id="0x1_math128_average"></a>

## Function `average`

Return the average of two.


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_average">average</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_average">average</a>(a: u128, b: u128): u128 {
    <b>if</b> (a &lt; b) {
        a + (b - a) / 2
    } <b>else</b> {
        b + (a - b) / 2
    }
}
</code></pre>



</details>

<a id="0x1_math128_gcd"></a>

## Function `gcd`

Return greatest common divisor of <code>a</code> & <code>b</code>, via the Euclidean algorithm.


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_gcd">gcd</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math128.md#0x1_math128_gcd">gcd</a>(a: u128, b: u128): u128 {
    <b>let</b> (large, small) = <b>if</b> (a &gt; b) (a, b) <b>else</b> (b, a);
    <b>while</b> (small != 0) {
        <b>let</b> tmp = small;
        small = large % small;
        large = tmp;
    };
    large
}
</code></pre>



</details>

<a id="0x1_math128_lcm"></a>

## Function `lcm`

Return least common multiple of <code>a</code> & <code>b</code>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_lcm">lcm</a>(a: u128, b: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math128.md#0x1_math128_lcm">lcm</a>(a: u128, b: u128): u128 {
    <b>if</b> (a == 0 || b == 0) {
        0
    } <b>else</b> {
        a / <a href="math128.md#0x1_math128_gcd">gcd</a>(a, b) * b
    }
}
</code></pre>



</details>

<a id="0x1_math128_mul_div"></a>

## Function `mul_div`

Returns a * b / c going through u256 to prevent intermediate overflow


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_mul_div">mul_div</a>(a: u128, b: u128, c: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math128.md#0x1_math128_mul_div">mul_div</a>(a: u128, b: u128, c: u128): u128 {
    // Inline functions cannot take constants, <b>as</b> then every <b>module</b> using it needs the constant
    <b>assert</b>!(c != 0, std::error::invalid_argument(4));
    (((a <b>as</b> u256) * (b <b>as</b> u256) / (c <b>as</b> u256)) <b>as</b> u128)
}
</code></pre>



</details>

<a id="0x1_math128_clamp"></a>

## Function `clamp`

Return x clamped to the interval [lower, upper].


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_clamp">clamp</a>(x: u128, lower: u128, upper: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_clamp">clamp</a>(x: u128, lower: u128, upper: u128): u128 {
    <b>min</b>(upper, <a href="math128.md#0x1_math128_max">max</a>(lower, x))
}
</code></pre>



</details>

<a id="0x1_math128_pow"></a>

## Function `pow`

Return the value of n raised to power e


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_pow">pow</a>(n: u128, e: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_pow">pow</a>(n: u128, e: u128): u128 {
    <b>if</b> (e == 0) {
        1
    } <b>else</b> {
        <b>let</b> p = 1;
        <b>while</b> (e &gt; 1) {
            <b>if</b> (e % 2 == 1) {
                p *= n;
            };
            e /= 2;
            n *= n;
        };
        p * n
    }
}
</code></pre>



</details>

<a id="0x1_math128_floor_log2"></a>

## Function `floor_log2`

Returns floor(log2(x))


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_floor_log2">floor_log2</a>(x: u128): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_floor_log2">floor_log2</a>(x: u128): u8 {
    <b>let</b> res = 0;
    <b>assert</b>!(x != 0, std::error::invalid_argument(<a href="math128.md#0x1_math128_EINVALID_ARG_FLOOR_LOG2">EINVALID_ARG_FLOOR_LOG2</a>));
    // Effectively the position of the most significant set bit
    <b>let</b> n = 64;
    <b>while</b> (n &gt; 0) {
        <b>if</b> (x &gt;= (1 &lt;&lt; n)) {
            x &gt;&gt;= n;
            res += n;
        };
        n &gt;&gt;= 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_math128_log2"></a>

## Function `log2`



<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_log2">log2</a>(x: u128): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_log2">log2</a>(x: u128): FixedPoint32 {
    <b>let</b> integer_part = <a href="math128.md#0x1_math128_floor_log2">floor_log2</a>(x);
    // Normalize x <b>to</b> [1, 2) in fixed point 32.
    <b>if</b> (x &gt;= 1 &lt;&lt; 32) {
        x &gt;&gt;= (integer_part - 32);
    } <b>else</b> {
        x &lt;&lt;= (32 - integer_part);
    };
    <b>let</b> frac = 0;
    <b>let</b> delta = 1 &lt;&lt; 31;
    <b>while</b> (delta != 0) {
        // log x = 1/2 log x^2
        // x in [1, 2)
        x = (x * x) &gt;&gt; 32;
        // x is now in [1, 4)
        // <b>if</b> x in [2, 4) then log x = 1 + log (x / 2)
        <b>if</b> (x &gt;= (2 &lt;&lt; 32)) { frac += delta; x &gt;&gt;= 1; };
        delta &gt;&gt;= 1;
    };
    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a> (((integer_part <b>as</b> u64) &lt;&lt; 32) + frac)
}
</code></pre>



</details>

<a id="0x1_math128_log2_64"></a>

## Function `log2_64`



<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_log2_64">log2_64</a>(x: u128): <a href="fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_log2_64">log2_64</a>(x: u128): FixedPoint64 {
    <b>let</b> integer_part = <a href="math128.md#0x1_math128_floor_log2">floor_log2</a>(x);
    // Normalize x <b>to</b> [1, 2) in fixed point 63. To ensure x is smaller then 1&lt;&lt;64
    <b>if</b> (x &gt;= 1 &lt;&lt; 63) {
        x &gt;&gt;= (integer_part - 63);
    } <b>else</b> {
        x &lt;&lt;= (63 - integer_part);
    };
    <b>let</b> frac = 0;
    <b>let</b> delta = 1 &lt;&lt; 63;
    <b>while</b> (delta != 0) {
        // log x = 1/2 log x^2
        // x in [1, 2)
        x = (x * x) &gt;&gt; 63;
        // x is now in [1, 4)
        // <b>if</b> x in [2, 4) then log x = 1 + log (x / 2)
        <b>if</b> (x &gt;= (2 &lt;&lt; 63)) { frac += delta; x &gt;&gt;= 1; };
        delta &gt;&gt;= 1;
    };
    <a href="fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a> (((integer_part <b>as</b> u128) &lt;&lt; 64) + frac)
}
</code></pre>



</details>

<a id="0x1_math128_sqrt"></a>

## Function `sqrt`

Returns square root of x, precisely floor(sqrt(x))


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_sqrt">sqrt</a>(x: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_sqrt">sqrt</a>(x: u128): u128 {
    <b>if</b> (x == 0) <b>return</b> 0;
    // Note the plus 1 in the expression. Let n = floor_lg2(x) we have x in [2^n, 2^{n+1}) and thus the answer in
    // the half-open interval [2^(n/2), 2^{(n+1)/2}). For even n we can write this <b>as</b> [2^(n/2), <a href="math128.md#0x1_math128_sqrt">sqrt</a>(2) 2^{n/2})
    // for odd n [2^((n+1)/2)/<a href="math128.md#0x1_math128_sqrt">sqrt</a>(2), 2^((n+1)/2). For even n the left end point is integer for odd the right
    // end point is integer. If we <b>choose</b> <b>as</b> our first approximation the integer end point we have <b>as</b> maximum
    // relative <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> either (<a href="math128.md#0x1_math128_sqrt">sqrt</a>(2) - 1) or (1 - 1/<a href="math128.md#0x1_math128_sqrt">sqrt</a>(2)) both are smaller then 1/2.
    <b>let</b> res = 1 &lt;&lt; ((<a href="math128.md#0x1_math128_floor_log2">floor_log2</a>(x) + 1) &gt;&gt; 1);
    // We <b>use</b> standard newton-rhapson iteration <b>to</b> improve the initial approximation.
    // The <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> term evolves <b>as</b> delta_i+1 = delta_i^2 / 2 (quadratic convergence).
    // It turns out that after 5 iterations the delta is smaller than 2^-64 and thus below the treshold.
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    <b>min</b>(res, x / res)
}
</code></pre>



</details>

<a id="0x1_math128_ceil_div"></a>

## Function `ceil_div`



<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_ceil_div">ceil_div</a>(x: u128, y: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math128.md#0x1_math128_ceil_div">ceil_div</a>(x: u128, y: u128): u128 {
    // <a href="math128.md#0x1_math128_ceil_div">ceil_div</a>(x, y) = floor((x + y - 1) / y) = floor((x - 1) / y) + 1
    // (x + y - 1) could spuriously overflow. so we <b>use</b> the later version
    <b>if</b> (x == 0) {
        // Inline functions cannot take constants, <b>as</b> then every <b>module</b> using it needs the constant
        <b>assert</b>!(y != 0, std::error::invalid_argument(4));
        0
    }
    <b>else</b> (x - 1) / y + 1
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_max">max</a>(a: u128, b: u128): u128
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a &gt;= b ==&gt; result == a;
<b>ensures</b> a &lt; b ==&gt; result == b;
</code></pre>



<a id="@Specification_1_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u128, b: u128): u128
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a &lt; b ==&gt; result == a;
<b>ensures</b> a &gt;= b ==&gt; result == b;
</code></pre>



<a id="@Specification_1_average"></a>

### Function `average`


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_average">average</a>(a: u128, b: u128): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == (a + b) / 2;
</code></pre>



<a id="@Specification_1_clamp"></a>

### Function `clamp`


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_clamp">clamp</a>(x: u128, lower: u128, upper: u128): u128
</code></pre>




<pre><code><b>requires</b> (lower &lt;= upper);
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> (lower &lt;=x && x &lt;= upper) ==&gt; result == x;
<b>ensures</b> (x &lt; lower) ==&gt; result == lower;
<b>ensures</b> (upper &lt; x) ==&gt; result == upper;
</code></pre>



<a id="@Specification_1_pow"></a>

### Function `pow`


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_pow">pow</a>(n: u128, e: u128): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <a href="math128.md#0x1_math128_spec_pow">spec_pow</a>(n, e) &gt; MAX_U128;
<b>ensures</b> [abstract] result == <a href="math128.md#0x1_math128_spec_pow">spec_pow</a>(n, e);
</code></pre>



<a id="@Specification_1_floor_log2"></a>

### Function `floor_log2`


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_floor_log2">floor_log2</a>(x: u128): u8
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] x == 0;
<b>ensures</b> [abstract] <a href="math128.md#0x1_math128_spec_pow">spec_pow</a>(2, result) &lt;= x;
<b>ensures</b> [abstract] x &lt; <a href="math128.md#0x1_math128_spec_pow">spec_pow</a>(2, result+1);
</code></pre>



<a id="@Specification_1_sqrt"></a>

### Function `sqrt`


<pre><code><b>public</b> <b>fun</b> <a href="math128.md#0x1_math128_sqrt">sqrt</a>(x: u128): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] x &gt; 0 ==&gt; result * result &lt;= x;
<b>ensures</b> [abstract] x &gt; 0 ==&gt; x &lt; (result+1) * (result+1);
</code></pre>




<a id="0x1_math128_spec_pow"></a>


<pre><code><b>fun</b> <a href="math128.md#0x1_math128_spec_pow">spec_pow</a>(n: u128, e: u128): u128 {
   <b>if</b> (e == 0) {
       1
   }
   <b>else</b> {
       n * <a href="math128.md#0x1_math128_spec_pow">spec_pow</a>(n, e-1)
   }
}
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
