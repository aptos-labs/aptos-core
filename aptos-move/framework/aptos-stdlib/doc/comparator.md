
<a id="0x1_comparator"></a>

# Module `0x1::comparator`

Provides a framework for comparing two elements


-  [Struct `Result`](#0x1_comparator_Result)
-  [Constants](#@Constants_0)
-  [Function `is_equal`](#0x1_comparator_is_equal)
-  [Function `is_smaller_than`](#0x1_comparator_is_smaller_than)
-  [Function `is_greater_than`](#0x1_comparator_is_greater_than)
-  [Function `compare`](#0x1_comparator_compare)
-  [Function `compare_u8_vector`](#0x1_comparator_compare_u8_vector)
-  [Specification](#@Specification_1)
    -  [Struct `Result`](#@Specification_1_Result)
    -  [Function `is_equal`](#@Specification_1_is_equal)
    -  [Function `is_smaller_than`](#@Specification_1_is_smaller_than)
    -  [Function `is_greater_than`](#@Specification_1_is_greater_than)
    -  [Function `compare`](#@Specification_1_compare)
    -  [Function `compare_u8_vector`](#@Specification_1_compare_u8_vector)


<pre><code>use 0x1::bcs;
</code></pre>



<a id="0x1_comparator_Result"></a>

## Struct `Result`



<pre><code>struct Result has drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_comparator_EQUAL"></a>



<pre><code>const EQUAL: u8 &#61; 0;
</code></pre>



<a id="0x1_comparator_GREATER"></a>



<pre><code>const GREATER: u8 &#61; 2;
</code></pre>



<a id="0x1_comparator_SMALLER"></a>



<pre><code>const SMALLER: u8 &#61; 1;
</code></pre>



<a id="0x1_comparator_is_equal"></a>

## Function `is_equal`



<pre><code>public fun is_equal(result: &amp;comparator::Result): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_equal(result: &amp;Result): bool &#123;
    result.inner &#61;&#61; EQUAL
&#125;
</code></pre>



</details>

<a id="0x1_comparator_is_smaller_than"></a>

## Function `is_smaller_than`



<pre><code>public fun is_smaller_than(result: &amp;comparator::Result): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_smaller_than(result: &amp;Result): bool &#123;
    result.inner &#61;&#61; SMALLER
&#125;
</code></pre>



</details>

<a id="0x1_comparator_is_greater_than"></a>

## Function `is_greater_than`



<pre><code>public fun is_greater_than(result: &amp;comparator::Result): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_greater_than(result: &amp;Result): bool &#123;
    result.inner &#61;&#61; GREATER
&#125;
</code></pre>



</details>

<a id="0x1_comparator_compare"></a>

## Function `compare`



<pre><code>public fun compare&lt;T&gt;(left: &amp;T, right: &amp;T): comparator::Result
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun compare&lt;T&gt;(left: &amp;T, right: &amp;T): Result &#123;
    let left_bytes &#61; bcs::to_bytes(left);
    let right_bytes &#61; bcs::to_bytes(right);

    compare_u8_vector(left_bytes, right_bytes)
&#125;
</code></pre>



</details>

<a id="0x1_comparator_compare_u8_vector"></a>

## Function `compare_u8_vector`



<pre><code>public fun compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): comparator::Result
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): Result &#123;
    let left_length &#61; vector::length(&amp;left);
    let right_length &#61; vector::length(&amp;right);

    let idx &#61; 0;

    while (idx &lt; left_length &amp;&amp; idx &lt; right_length) &#123;
        let left_byte &#61; &#42;vector::borrow(&amp;left, idx);
        let right_byte &#61; &#42;vector::borrow(&amp;right, idx);

        if (left_byte &lt; right_byte) &#123;
            return Result &#123; inner: SMALLER &#125;
        &#125; else if (left_byte &gt; right_byte) &#123;
            return Result &#123; inner: GREATER &#125;
        &#125;;
        idx &#61; idx &#43; 1;
    &#125;;

    if (left_length &lt; right_length) &#123;
        Result &#123; inner: SMALLER &#125;
    &#125; else if (left_length &gt; right_length) &#123;
        Result &#123; inner: GREATER &#125;
    &#125; else &#123;
        Result &#123; inner: EQUAL &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Result"></a>

### Struct `Result`


<pre><code>struct Result has drop
</code></pre>



<dl>
<dt>
<code>inner: u8</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant inner &#61;&#61; EQUAL &#124;&#124; inner &#61;&#61; SMALLER &#124;&#124; inner &#61;&#61; GREATER;
</code></pre>



<a id="@Specification_1_is_equal"></a>

### Function `is_equal`


<pre><code>public fun is_equal(result: &amp;comparator::Result): bool
</code></pre>




<pre><code>aborts_if false;
let res &#61; result;
ensures result &#61;&#61; (res.inner &#61;&#61; EQUAL);
</code></pre>



<a id="@Specification_1_is_smaller_than"></a>

### Function `is_smaller_than`


<pre><code>public fun is_smaller_than(result: &amp;comparator::Result): bool
</code></pre>




<pre><code>aborts_if false;
let res &#61; result;
ensures result &#61;&#61; (res.inner &#61;&#61; SMALLER);
</code></pre>



<a id="@Specification_1_is_greater_than"></a>

### Function `is_greater_than`


<pre><code>public fun is_greater_than(result: &amp;comparator::Result): bool
</code></pre>




<pre><code>aborts_if false;
let res &#61; result;
ensures result &#61;&#61; (res.inner &#61;&#61; GREATER);
</code></pre>



<a id="@Specification_1_compare"></a>

### Function `compare`


<pre><code>public fun compare&lt;T&gt;(left: &amp;T, right: &amp;T): comparator::Result
</code></pre>




<pre><code>let left_bytes &#61; bcs::to_bytes(left);
let right_bytes &#61; bcs::to_bytes(right);
ensures result &#61;&#61; spec_compare_u8_vector(left_bytes, right_bytes);
</code></pre>




<a id="0x1_comparator_spec_compare_u8_vector"></a>


<pre><code>fun spec_compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): Result;
</code></pre>



<a id="@Specification_1_compare_u8_vector"></a>

### Function `compare_u8_vector`


<pre><code>public fun compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): comparator::Result
</code></pre>




<pre><code>pragma unroll &#61; 5;
pragma opaque;
aborts_if false;
let left_length &#61; len(left);
let right_length &#61; len(right);
ensures (result.inner &#61;&#61; EQUAL) &#61;&#61;&gt; (
    (left_length &#61;&#61; right_length) &amp;&amp;
        (forall i: u64 where i &lt; left_length: left[i] &#61;&#61; right[i])
);
ensures (result.inner &#61;&#61; SMALLER) &#61;&#61;&gt; (
    (exists i: u64 where i &lt; left_length:
        (i &lt; right_length) &amp;&amp;
            (left[i] &lt; right[i]) &amp;&amp;
            (forall j: u64 where j &lt; i: left[j] &#61;&#61; right[j])
    ) &#124;&#124;
        (left_length &lt; right_length)
);
ensures (result.inner &#61;&#61; GREATER) &#61;&#61;&gt; (
    (exists i: u64 where i &lt; left_length:
        (i &lt; right_length) &amp;&amp;
            (left[i] &gt; right[i]) &amp;&amp;
            (forall j: u64 where j &lt; i: left[j] &#61;&#61; right[j])
    ) &#124;&#124;
        (left_length &gt; right_length)
);
ensures [abstract] result &#61;&#61; spec_compare_u8_vector(left, right);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
