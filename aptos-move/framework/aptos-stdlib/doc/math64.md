
<a name="0x1_math64"></a>

# Module `0x1::math64`

Standard math utilities missing in the Move Language.


-  [Function `max`](#0x1_math64_max)
-  [Function `min`](#0x1_math64_min)
-  [Function `average`](#0x1_math64_average)
-  [Function `pow`](#0x1_math64_pow)
-  [Specification](#@Specification_0)
    -  [Function `max`](#@Specification_0_max)
    -  [Function `min`](#@Specification_0_min)
    -  [Function `average`](#@Specification_0_average)
    -  [Function `pow`](#@Specification_0_pow)


<pre><code></code></pre>



<a name="0x1_math64_max"></a>

## Function `max`

Return the largest of two numbers.


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_max">max</a>(a: u64, b: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_max">max</a>(a: u64, b: u64): u64 {
    <b>if</b> (a &gt;= b) a <b>else</b> b
}
</code></pre>



</details>

<a name="0x1_math64_min"></a>

## Function `min`

Return the smallest of two numbers.


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u64, b: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u64, b: u64): u64 {
    <b>if</b> (a &lt; b) a <b>else</b> b
}
</code></pre>



</details>

<a name="0x1_math64_average"></a>

## Function `average`

Return the average of two.


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_average">average</a>(a: u64, b: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_average">average</a>(a: u64, b: u64): u64 {
    <b>if</b> (a &lt; b) {
        a + (b - a) / 2
    } <b>else</b> {
        b + (a - b) / 2
    }
}
</code></pre>



</details>

<a name="0x1_math64_pow"></a>

## Function `pow`

Return the value of n raised to power e


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_pow">pow</a>(n: u64, e: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_pow">pow</a>(n: u64, e: u64): u64 {
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

<a name="@Specification_0"></a>

## Specification


<a name="@Specification_0_max"></a>

### Function `max`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_max">max</a>(a: u64, b: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a &gt;= b ==&gt; result == a;
<b>ensures</b> a &lt; b ==&gt; result == b;
</code></pre>



<a name="@Specification_0_min"></a>

### Function `min`


<pre><code><b>public</b> <b>fun</b> <b>min</b>(a: u64, b: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> a &lt; b ==&gt; result == a;
<b>ensures</b> a &gt;= b ==&gt; result == b;
</code></pre>



<a name="@Specification_0_average"></a>

### Function `average`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_average">average</a>(a: u64, b: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == (a + b) / 2;
</code></pre>



<a name="@Specification_0_pow"></a>

### Function `pow`


<pre><code><b>public</b> <b>fun</b> <a href="math64.md#0x1_math64_pow">pow</a>(n: u64, e: u64): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n, e) &gt; MAX_U64;
<b>ensures</b> [abstract] result == <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n, e);
</code></pre>




<a name="0x1_math64_spec_pow"></a>


<pre><code><b>fun</b> <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(e: u64, n: u64): u64 {
   <b>if</b> (e == 0) {
       1
   }
   <b>else</b> {
       n * <a href="math64.md#0x1_math64_spec_pow">spec_pow</a>(n, e-1)
   }
}
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
