
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /></code></pre>



<a id="0x1_comparator_Result"></a>

## Struct `Result`



<pre><code><b>struct</b> <a href="comparator.md#0x1_comparator_Result">Result</a> <b>has</b> drop<br /></code></pre>



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



<pre><code><b>const</b> <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>: u8 &#61; 0;<br /></code></pre>



<a id="0x1_comparator_GREATER"></a>



<pre><code><b>const</b> <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>: u8 &#61; 2;<br /></code></pre>



<a id="0x1_comparator_SMALLER"></a>



<pre><code><b>const</b> <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>: u8 &#61; 1;<br /></code></pre>



<a id="0x1_comparator_is_equal"></a>

## Function `is_equal`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_equal">is_equal</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_equal">is_equal</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">Result</a>): bool &#123;<br />    result.inner &#61;&#61; <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_comparator_is_smaller_than"></a>

## Function `is_smaller_than`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_smaller_than">is_smaller_than</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_smaller_than">is_smaller_than</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">Result</a>): bool &#123;<br />    result.inner &#61;&#61; <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_comparator_is_greater_than"></a>

## Function `is_greater_than`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_greater_than">is_greater_than</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_greater_than">is_greater_than</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">Result</a>): bool &#123;<br />    result.inner &#61;&#61; <a href="comparator.md#0x1_comparator_GREATER">GREATER</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_comparator_compare"></a>

## Function `compare`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare">compare</a>&lt;T&gt;(left: &amp;T, right: &amp;T): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare">compare</a>&lt;T&gt;(left: &amp;T, right: &amp;T): <a href="comparator.md#0x1_comparator_Result">Result</a> &#123;<br />    <b>let</b> left_bytes &#61; <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(left);<br />    <b>let</b> right_bytes &#61; <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(right);<br /><br />    <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left_bytes, right_bytes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_comparator_compare_u8_vector"></a>

## Function `compare_u8_vector`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">Result</a> &#123;<br />    <b>let</b> left_length &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;left);<br />    <b>let</b> right_length &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;right);<br /><br />    <b>let</b> idx &#61; 0;<br /><br />    <b>while</b> (idx &lt; left_length &amp;&amp; idx &lt; right_length) &#123;<br />        <b>let</b> left_byte &#61; &#42;<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;left, idx);<br />        <b>let</b> right_byte &#61; &#42;<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;right, idx);<br /><br />        <b>if</b> (left_byte &lt; right_byte) &#123;<br />            <b>return</b> <a href="comparator.md#0x1_comparator_Result">Result</a> &#123; inner: <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a> &#125;<br />        &#125; <b>else</b> <b>if</b> (left_byte &gt; right_byte) &#123;<br />            <b>return</b> <a href="comparator.md#0x1_comparator_Result">Result</a> &#123; inner: <a href="comparator.md#0x1_comparator_GREATER">GREATER</a> &#125;<br />        &#125;;<br />        idx &#61; idx &#43; 1;<br />    &#125;;<br /><br />    <b>if</b> (left_length &lt; right_length) &#123;<br />        <a href="comparator.md#0x1_comparator_Result">Result</a> &#123; inner: <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a> &#125;<br />    &#125; <b>else</b> <b>if</b> (left_length &gt; right_length) &#123;<br />        <a href="comparator.md#0x1_comparator_Result">Result</a> &#123; inner: <a href="comparator.md#0x1_comparator_GREATER">GREATER</a> &#125;<br />    &#125; <b>else</b> &#123;<br />        <a href="comparator.md#0x1_comparator_Result">Result</a> &#123; inner: <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a> &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Result"></a>

### Struct `Result`


<pre><code><b>struct</b> <a href="comparator.md#0x1_comparator_Result">Result</a> <b>has</b> drop<br /></code></pre>



<dl>
<dt>
<code>inner: u8</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> inner &#61;&#61; <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a> &#124;&#124; inner &#61;&#61; <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a> &#124;&#124; inner &#61;&#61; <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>;<br /></code></pre>



<a id="@Specification_1_is_equal"></a>

### Function `is_equal`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_equal">is_equal</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>let</b> res &#61; result;<br /><b>ensures</b> result &#61;&#61; (res.inner &#61;&#61; <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>);<br /></code></pre>



<a id="@Specification_1_is_smaller_than"></a>

### Function `is_smaller_than`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_smaller_than">is_smaller_than</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>let</b> res &#61; result;<br /><b>ensures</b> result &#61;&#61; (res.inner &#61;&#61; <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>);<br /></code></pre>



<a id="@Specification_1_is_greater_than"></a>

### Function `is_greater_than`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_greater_than">is_greater_than</a>(result: &amp;<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>let</b> res &#61; result;<br /><b>ensures</b> result &#61;&#61; (res.inner &#61;&#61; <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>);<br /></code></pre>



<a id="@Specification_1_compare"></a>

### Function `compare`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare">compare</a>&lt;T&gt;(left: &amp;T, right: &amp;T): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a><br /></code></pre>




<pre><code><b>let</b> left_bytes &#61; <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(left);<br /><b>let</b> right_bytes &#61; <a href="../../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(right);<br /><b>ensures</b> result &#61;&#61; <a href="comparator.md#0x1_comparator_spec_compare_u8_vector">spec_compare_u8_vector</a>(left_bytes, right_bytes);<br /></code></pre>




<a id="0x1_comparator_spec_compare_u8_vector"></a>


<pre><code><b>fun</b> <a href="comparator.md#0x1_comparator_spec_compare_u8_vector">spec_compare_u8_vector</a>(left: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">Result</a>;<br /></code></pre>



<a id="@Specification_1_compare_u8_vector"></a>

### Function `compare_u8_vector`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a><br /></code></pre>




<pre><code><b>pragma</b> unroll &#61; 5;<br /><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>let</b> left_length &#61; len(left);<br /><b>let</b> right_length &#61; len(right);<br /><b>ensures</b> (result.inner &#61;&#61; <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>) &#61;&#61;&gt; (<br />    (left_length &#61;&#61; right_length) &amp;&amp;<br />        (<b>forall</b> i: u64 <b>where</b> i &lt; left_length: left[i] &#61;&#61; right[i])<br />);<br /><b>ensures</b> (result.inner &#61;&#61; <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>) &#61;&#61;&gt; (<br />    (<b>exists</b> i: u64 <b>where</b> i &lt; left_length:<br />        (i &lt; right_length) &amp;&amp;<br />            (left[i] &lt; right[i]) &amp;&amp;<br />            (<b>forall</b> j: u64 <b>where</b> j &lt; i: left[j] &#61;&#61; right[j])<br />    ) &#124;&#124;<br />        (left_length &lt; right_length)<br />);<br /><b>ensures</b> (result.inner &#61;&#61; <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>) &#61;&#61;&gt; (<br />    (<b>exists</b> i: u64 <b>where</b> i &lt; left_length:<br />        (i &lt; right_length) &amp;&amp;<br />            (left[i] &gt; right[i]) &amp;&amp;<br />            (<b>forall</b> j: u64 <b>where</b> j &lt; i: left[j] &#61;&#61; right[j])<br />    ) &#124;&#124;<br />        (left_length &gt; right_length)<br />);<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="comparator.md#0x1_comparator_spec_compare_u8_vector">spec_compare_u8_vector</a>(left, right);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
