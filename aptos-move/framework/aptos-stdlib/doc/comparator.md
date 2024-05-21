
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


<pre><code>use 0x1::bcs;<br/></code></pre>



<a id="0x1_comparator_Result"></a>

## Struct `Result`



<pre><code>struct Result has drop<br/></code></pre>



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



<pre><code>const EQUAL: u8 &#61; 0;<br/></code></pre>



<a id="0x1_comparator_GREATER"></a>



<pre><code>const GREATER: u8 &#61; 2;<br/></code></pre>



<a id="0x1_comparator_SMALLER"></a>



<pre><code>const SMALLER: u8 &#61; 1;<br/></code></pre>



<a id="0x1_comparator_is_equal"></a>

## Function `is_equal`



<pre><code>public fun is_equal(result: &amp;comparator::Result): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_equal(result: &amp;Result): bool &#123;<br/>    result.inner &#61;&#61; EQUAL<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_comparator_is_smaller_than"></a>

## Function `is_smaller_than`



<pre><code>public fun is_smaller_than(result: &amp;comparator::Result): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_smaller_than(result: &amp;Result): bool &#123;<br/>    result.inner &#61;&#61; SMALLER<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_comparator_is_greater_than"></a>

## Function `is_greater_than`



<pre><code>public fun is_greater_than(result: &amp;comparator::Result): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_greater_than(result: &amp;Result): bool &#123;<br/>    result.inner &#61;&#61; GREATER<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_comparator_compare"></a>

## Function `compare`



<pre><code>public fun compare&lt;T&gt;(left: &amp;T, right: &amp;T): comparator::Result<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun compare&lt;T&gt;(left: &amp;T, right: &amp;T): Result &#123;<br/>    let left_bytes &#61; bcs::to_bytes(left);<br/>    let right_bytes &#61; bcs::to_bytes(right);<br/><br/>    compare_u8_vector(left_bytes, right_bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_comparator_compare_u8_vector"></a>

## Function `compare_u8_vector`



<pre><code>public fun compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): comparator::Result<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): Result &#123;<br/>    let left_length &#61; vector::length(&amp;left);<br/>    let right_length &#61; vector::length(&amp;right);<br/><br/>    let idx &#61; 0;<br/><br/>    while (idx &lt; left_length &amp;&amp; idx &lt; right_length) &#123;<br/>        let left_byte &#61; &#42;vector::borrow(&amp;left, idx);<br/>        let right_byte &#61; &#42;vector::borrow(&amp;right, idx);<br/><br/>        if (left_byte &lt; right_byte) &#123;<br/>            return Result &#123; inner: SMALLER &#125;<br/>        &#125; else if (left_byte &gt; right_byte) &#123;<br/>            return Result &#123; inner: GREATER &#125;<br/>        &#125;;<br/>        idx &#61; idx &#43; 1;<br/>    &#125;;<br/><br/>    if (left_length &lt; right_length) &#123;<br/>        Result &#123; inner: SMALLER &#125;<br/>    &#125; else if (left_length &gt; right_length) &#123;<br/>        Result &#123; inner: GREATER &#125;<br/>    &#125; else &#123;<br/>        Result &#123; inner: EQUAL &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Result"></a>

### Struct `Result`


<pre><code>struct Result has drop<br/></code></pre>



<dl>
<dt>
<code>inner: u8</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant inner &#61;&#61; EQUAL &#124;&#124; inner &#61;&#61; SMALLER &#124;&#124; inner &#61;&#61; GREATER;<br/></code></pre>



<a id="@Specification_1_is_equal"></a>

### Function `is_equal`


<pre><code>public fun is_equal(result: &amp;comparator::Result): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>let res &#61; result;<br/>ensures result &#61;&#61; (res.inner &#61;&#61; EQUAL);<br/></code></pre>



<a id="@Specification_1_is_smaller_than"></a>

### Function `is_smaller_than`


<pre><code>public fun is_smaller_than(result: &amp;comparator::Result): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>let res &#61; result;<br/>ensures result &#61;&#61; (res.inner &#61;&#61; SMALLER);<br/></code></pre>



<a id="@Specification_1_is_greater_than"></a>

### Function `is_greater_than`


<pre><code>public fun is_greater_than(result: &amp;comparator::Result): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>let res &#61; result;<br/>ensures result &#61;&#61; (res.inner &#61;&#61; GREATER);<br/></code></pre>



<a id="@Specification_1_compare"></a>

### Function `compare`


<pre><code>public fun compare&lt;T&gt;(left: &amp;T, right: &amp;T): comparator::Result<br/></code></pre>




<pre><code>let left_bytes &#61; bcs::to_bytes(left);<br/>let right_bytes &#61; bcs::to_bytes(right);<br/>ensures result &#61;&#61; spec_compare_u8_vector(left_bytes, right_bytes);<br/></code></pre>




<a id="0x1_comparator_spec_compare_u8_vector"></a>


<pre><code>fun spec_compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): Result;<br/></code></pre>



<a id="@Specification_1_compare_u8_vector"></a>

### Function `compare_u8_vector`


<pre><code>public fun compare_u8_vector(left: vector&lt;u8&gt;, right: vector&lt;u8&gt;): comparator::Result<br/></code></pre>




<pre><code>pragma unroll &#61; 5;<br/>pragma opaque;<br/>aborts_if false;<br/>let left_length &#61; len(left);<br/>let right_length &#61; len(right);<br/>ensures (result.inner &#61;&#61; EQUAL) &#61;&#61;&gt; (<br/>    (left_length &#61;&#61; right_length) &amp;&amp;<br/>        (forall i: u64 where i &lt; left_length: left[i] &#61;&#61; right[i])<br/>);<br/>ensures (result.inner &#61;&#61; SMALLER) &#61;&#61;&gt; (<br/>    (exists i: u64 where i &lt; left_length:<br/>        (i &lt; right_length) &amp;&amp;<br/>            (left[i] &lt; right[i]) &amp;&amp;<br/>            (forall j: u64 where j &lt; i: left[j] &#61;&#61; right[j])<br/>    ) &#124;&#124;<br/>        (left_length &lt; right_length)<br/>);<br/>ensures (result.inner &#61;&#61; GREATER) &#61;&#61;&gt; (<br/>    (exists i: u64 where i &lt; left_length:<br/>        (i &lt; right_length) &amp;&amp;<br/>            (left[i] &gt; right[i]) &amp;&amp;<br/>            (forall j: u64 where j &lt; i: left[j] &#61;&#61; right[j])<br/>    ) &#124;&#124;<br/>        (left_length &gt; right_length)<br/>);<br/>ensures [abstract] result &#61;&#61; spec_compare_u8_vector(left, right);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
