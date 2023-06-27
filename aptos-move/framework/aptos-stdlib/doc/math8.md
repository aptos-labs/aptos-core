
<a name="0x1_math8"></a>

# Module `0x1::math8`

Standard math utilities missing in the Move Language.


-  [Constants](#@Constants_0)
-  [Function `max`](#0x1_math8_max)
-  [Function `min`](#0x1_math8_min)
-  [Function `average`](#0x1_math8_average)
-  [Function `mul_div`](#0x1_math8_mul_div)
-  [Function `clamp`](#0x1_math8_clamp)
-  [Function `pow`](#0x1_math8_pow)
-  [Function `floor_log2`](#0x1_math8_floor_log2)
-  [Function `log2`](#0x1_math8_log2)
-  [Function `log2_modified`](#0x1_math8_log2_modified)
-  [Function `sqrt`](#0x1_math8_sqrt)
-  [Function `ceil_div`](#0x1_math8_ceil_div)
-  [Specification](#@Specification_1)
    -  [Function `max`](#@Specification_1_max)
    -  [Function `min`](#@Specification_1_min)
    -  [Function `average`](#@Specification_1_average)
    -  [Function `clamp`](#@Specification_1_clamp)
    -  [Function `pow`](#@Specification_1_pow)
    -  [Function `floor_log2`](#@Specification_1_floor_log2)
    -  [Function `log2_modified`](#@Specification_1_log2_modified)
    -  [Function `sqrt`](#@Specification_1_sqrt)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_math8_EDIVISION_BY_ZERO"></a>



<pre><code><b>const</b> <a href="math8.md#0x1_math8_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>: u64 = 1;
</code></pre>



<a name="0x1_math8_EINVALID_ARG_FLOOR_LOG2"></a>

Abort value when an invalid argument is provided.


<pre><code><b>const</b> <a href="math8.md#0x1_math8_EINVALID_ARG_FLOOR_LOG2">EINVALID_ARG_FLOOR_LOG2</a>: u64 = 1;
</code></pre>



<a name="0x1_math8_max"></a>

## Function `max`

Return the largest of two numbers.


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_max">max</a>(a: u8, b: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_max">max</a>(a: u8, b: u8): u8 {
    <b>if</b> (a &gt;= b) a <b>else</b> b
}
</code></pre>



</details>

<a name="0x1_math8_min"></a>

## Function `min`

Return the smallest of two numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u8, b: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u8, b: u8): u8 {
    <b>if</b> (a &lt; b) a <b>else</b> b
}
</code></pre>



</details>

<a name="0x1_math8_average"></a>

## Function `average`

Return the average of two.


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_average">average</a>(a: u8, b: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_average">average</a>(a: u8, b: u8): u8 {
    <b>if</b> (a &lt; b) {
        a + (b - a) / 2
    } <b>else</b> {
        b + (a - b) / 2
    }
}
</code></pre>



</details>

<a name="0x1_math8_mul_div"></a>

## Function `mul_div`

Returns a * b / c going through u128 to prevent intermediate overflow


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_mul_div">mul_div</a>(a: u8, b: u8, c: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math8.md#0x1_math8_mul_div">mul_div</a>(a: u8, b: u8, c: u8): u8 {
    (((a <b>as</b> u16) * (b <b>as</b> u16) / (c <b>as</b> u16)) <b>as</b> u8)
}
</code></pre>



</details>

<a name="0x1_math8_clamp"></a>

## Function `clamp`

Return x clamped to the interval [lower, upper].


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_clamp">clamp</a>(x: u8, lower: u8, upper: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_clamp">clamp</a>(x: u8, lower: u8, upper: u8): u8 {
    <b>min</b>(upper, <a href="math8.md#0x1_math8_max">max</a>(lower, x))
}
</code></pre>



</details>

<a name="0x1_math8_pow"></a>

## Function `pow`

Return the value of n raised to power e


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_pow">pow</a>(n: u8, e: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_pow">pow</a>(n: u8, e: u8): u8 {
    <b>if</b> (e == 0) {
        1
    } <b>else</b> {
        <b>let</b> p = 1;
        <b>while</b> (e &gt; 1) {
            <b>if</b> (e % 2 == 1) {
                p = p * n;
            };
            e = e / 2;
            n = n * n;
        };
        p * n
    }
}
</code></pre>



</details>

<a name="0x1_math8_floor_log2"></a>

## Function `floor_log2`

Returns floor(lg2(x))


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_floor_log2">floor_log2</a>(x: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_floor_log2">floor_log2</a>(x: u8): u8 {
    <b>let</b> res = 0;
    <b>assert</b>!(x != 0, std::error::invalid_argument(<a href="math8.md#0x1_math8_EINVALID_ARG_FLOOR_LOG2">EINVALID_ARG_FLOOR_LOG2</a>));
    // Effectively the position of the most significant set bit
    <b>let</b> n = 4;
    <b>while</b> (n &gt; 0) {
        <b>if</b> (x &gt;= (1 &lt;&lt; n)) {
            x = x &gt;&gt; n;
            res = res + n;
        };
        n = n &gt;&gt; 1;
    };
    res
}
</code></pre>



</details>

<a name="0x1_math8_log2"></a>

## Function `log2`



<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_log2">log2</a>(x: u8): <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_FixedPoint32">fixed_point32::FixedPoint32</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_log2">log2</a>(x: u8): FixedPoint32 {
    <b>let</b> integer_part = <a href="math8.md#0x1_math8_floor_log2">floor_log2</a>(x);
    // Normalize x <b>to</b> [1, 2) in fixed point 32.
    <b>let</b> y = (<b>if</b> (x &gt;= 1 &lt;&lt; 32) {
        x &gt;&gt; (integer_part - 32)
    } <b>else</b> {
        x &lt;&lt; (32 - integer_part)
    } <b>as</b> u128);
    <b>let</b> frac = 0;
    <b>let</b> delta = 1 &lt;&lt; 31;
    <b>while</b> (delta != 0) {
        // log x = 1/2 log x^2
        // x in [1, 2)
        y = (y * y) &gt;&gt; 32;
        // x is now in [1, 4)
        // <b>if</b> x in [2, 4) then log x = 1 + log (x / 2)
        <b>if</b> (y &gt;= (2 &lt;&lt; 32)) { frac = frac + delta; y = y &gt;&gt; 1; };
        delta = delta &gt;&gt; 1;
    };
    <a href="../../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_raw_value">fixed_point32::create_from_raw_value</a> (((integer_part <b>as</b> u64) &lt;&lt; 32) + frac)
}
</code></pre>



</details>

<a name="0x1_math8_log2_modified"></a>

## Function `log2_modified`



<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_log2_modified">log2_modified</a>(x: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_log2_modified">log2_modified</a>(x: u8): u8 {
    <b>let</b> integer_part = <a href="math8.md#0x1_math8_floor_log2">floor_log2</a>(x);
    // Normalize x <b>to</b> [1, 2) in fixed point 32.
    <b>let</b> y = (<b>if</b> (x &gt;= 1 &lt;&lt; 4) {
        x &gt;&gt; (integer_part - 4)
    } <b>else</b> {
        x &lt;&lt; (4 - integer_part)
    } <b>as</b> u16);
    <b>let</b> frac = 0;
    <b>let</b> delta = 1 &lt;&lt; 4;
    <b>while</b> (delta != 0) {
        // log x = 1/2 log x^2
        // x in [1, 2)
        <b>spec</b> {
            <b>assume</b> y * y &lt;= MAX_U16;
        };
        y = (y * y) &gt;&gt; 4;
        // x is now in [1, 4)
        // <b>if</b> x in [2, 4) then log x = 1 + log (x / 2)
        <b>if</b> (y &gt;= (2 &lt;&lt; 4)) { frac = frac + delta; y = y &gt;&gt; 1; };
        delta = delta &gt;&gt; 1;
    };
    <b>let</b> t = (integer_part <b>as</b> u8) &lt;&lt; 4;
    t + frac
}
</code></pre>



</details>

<a name="0x1_math8_sqrt"></a>

## Function `sqrt`

Returns square root of x, precisely floor(sqrt(x))


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_sqrt">sqrt</a>(x: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_sqrt">sqrt</a>(x: u8): u8 {
    <b>if</b> (x == 0) <b>return</b> 0;
    // Note the plus 1 in the expression. Let n = floor_lg2(x) we have x in [2^n, 2^(n+1)&gt; and thus the answer in
    // the half-open interval [2^(n/2), 2^((n+1)/2)&gt;. For even n we can write this <b>as</b> [2^(n/2), <a href="math8.md#0x1_math8_sqrt">sqrt</a>(2) 2^(n/2)&gt;
    // for odd n [2^((n+1)/2)/<a href="math8.md#0x1_math8_sqrt">sqrt</a>(2), 2^((n+1)/2&gt;. For even n the left end point is integer for odd the right
    // end point is integer. If we <b>choose</b> <b>as</b> our first approximation the integer end point we have <b>as</b> maximum
    // relative <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> either (<a href="math8.md#0x1_math8_sqrt">sqrt</a>(2) - 1) or (1 - 1/<a href="math8.md#0x1_math8_sqrt">sqrt</a>(2)) both are smaller then 1/2.
    <b>let</b> res = 1 &lt;&lt; ((<a href="math8.md#0x1_math8_floor_log2">floor_log2</a>(x) + 1) &gt;&gt; 1);
    // We <b>use</b> standard newton-rhapson iteration <b>to</b> improve the initial approximation.
    // The <a href="../../move-stdlib/doc/error.md#0x1_error">error</a> term evolves <b>as</b> delta_i+1 = delta_i^2 / 2 (quadratic convergence).
    // It turns out that after 4 iterations the delta is smaller than 2^-32 and thus below the treshold.
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    res = (res + x / res) &gt;&gt; 1;
    <b>min</b>(res, x / res)
}
</code></pre>



</details>

<a name="0x1_math8_ceil_div"></a>

## Function `ceil_div`



<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_ceil_div">ceil_div</a>(x: u8, y: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="math8.md#0x1_math8_ceil_div">ceil_div</a>(x: u8, y: u8): u8 {
    // <a href="math8.md#0x1_math8_ceil_div">ceil_div</a>(x, y) = floor((x + y - 1) / y) = floor((x - 1) / y) + 1
    // (x + y - 1) could spuriously overflow. so we <b>use</b> the later version
    <b>if</b> (x == 0) {
        <b>assert</b>!(y != 0, <a href="math8.md#0x1_math8_EDIVISION_BY_ZERO">EDIVISION_BY_ZERO</a>);
        0
    }
    <b>else</b> (x - 1) / y + 1
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification


<a name="@Specification_1_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_max">max</a>(a: u8, b: u8): u8
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a &gt;= b ==&gt; result == a;
<b>ensures</b> a &lt; b ==&gt; result == b;
</code></pre>



<a name="@Specification_1_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u8, b: u8): u8
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a &lt; b ==&gt; result == a;
<b>ensures</b> a &gt;= b ==&gt; result == b;
</code></pre>



<a name="@Specification_1_average"></a>

### Function `average`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_average">average</a>(a: u8, b: u8): u8
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == (a + b) / 2;
</code></pre>



<a name="@Specification_1_clamp"></a>

### Function `clamp`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_clamp">clamp</a>(x: u8, lower: u8, upper: u8): u8
</code></pre>




<pre><code><b>requires</b> (lower &lt;= upper);
<b>ensures</b> (lower &lt;=x && x &lt;= upper) ==&gt; result == x;
<b>ensures</b> (x &lt; lower) ==&gt; result == lower;
<b>ensures</b> (upper &lt; x) ==&gt; result == upper;
</code></pre>



<a name="@Specification_1_pow"></a>

### Function `pow`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_pow">pow</a>(n: u8, e: u8): u8
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>pragma</b> unroll = 3;
<b>aborts_if</b> <a href="math8.md#0x1_math8_spec_pow">spec_pow</a>(n, e) &gt; MAX_U8;
<b>ensures</b> result == <a href="math8.md#0x1_math8_spec_pow">spec_pow</a>(n, e);
</code></pre>



<a name="@Specification_1_floor_log2"></a>

### Function `floor_log2`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_floor_log2">floor_log2</a>(x: u8): u8
</code></pre>




<pre><code><b>pragma</b> unroll=2;
<b>pragma</b> opaque;
<b>aborts_if</b> x == 0;
<b>ensures</b> <a href="math8.md#0x1_math8_spec_pow">spec_pow</a>(2, result) &lt;= x;
<b>ensures</b> x &lt; <a href="math8.md#0x1_math8_spec_pow">spec_pow</a>(2, result+1);
</code></pre>




<a name="0x1_math8_spec_pow"></a>


<pre><code><b>fun</b> <a href="math8.md#0x1_math8_spec_pow">spec_pow</a>(n: u8, e: u8): u8 {
   <b>if</b> (e == 0) {
       1
   }
   <b>else</b> {
       <b>if</b> (e == 1) {
           n
       }
       <b>else</b> {
           <b>if</b> (e == 2) {
               n*n
           }
           <b>else</b> {
               <b>if</b> (e == 3) {
                   n*n*n
               }
               <b>else</b> {
                   <b>if</b> (e == 4) {
                       n*n*n*n
                   }
                   <b>else</b> {
                       <b>if</b> (e == 5) {
                           n*n*n*n*n
                       }
                       <b>else</b> {
                           <b>if</b> (e == 6) {
                               n*n*n*n*n*n
                           }
                           <b>else</b> {
                               <b>if</b> (e == 7) {
                                   n*n*n*n*n*n*n
                               }
                               <b>else</b> {
                                   n*n*n*n*n*n*n*n
                               }
                           }
                       }
                   }
               }
           }
       }
   }
}
</code></pre>



<a name="@Specification_1_log2_modified"></a>

### Function `log2_modified`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_log2_modified">log2_modified</a>(x: u8): u8
</code></pre>




<pre><code><b>pragma</b> unroll=3;
<b>aborts_if</b> <b>true</b>;
</code></pre>



<a name="@Specification_1_sqrt"></a>

### Function `sqrt`


<pre><code><b>public</b> <b>fun</b> <a href="math8.md#0x1_math8_sqrt">sqrt</a>(x: u8): u8
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
