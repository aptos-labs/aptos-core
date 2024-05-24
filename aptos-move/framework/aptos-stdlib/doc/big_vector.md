
<a id="0x1_big_vector"></a>

# Module `0x1::big_vector`



-  [Struct `BigVector`](#0x1_big_vector_BigVector)
-  [Constants](#@Constants_0)
-  [Function `empty`](#0x1_big_vector_empty)
-  [Function `singleton`](#0x1_big_vector_singleton)
-  [Function `destroy_empty`](#0x1_big_vector_destroy_empty)
-  [Function `destroy`](#0x1_big_vector_destroy)
-  [Function `borrow`](#0x1_big_vector_borrow)
-  [Function `borrow_mut`](#0x1_big_vector_borrow_mut)
-  [Function `append`](#0x1_big_vector_append)
-  [Function `push_back`](#0x1_big_vector_push_back)
-  [Function `pop_back`](#0x1_big_vector_pop_back)
-  [Function `remove`](#0x1_big_vector_remove)
-  [Function `swap_remove`](#0x1_big_vector_swap_remove)
-  [Function `swap`](#0x1_big_vector_swap)
-  [Function `reverse`](#0x1_big_vector_reverse)
-  [Function `index_of`](#0x1_big_vector_index_of)
-  [Function `contains`](#0x1_big_vector_contains)
-  [Function `to_vector`](#0x1_big_vector_to_vector)
-  [Function `length`](#0x1_big_vector_length)
-  [Function `is_empty`](#0x1_big_vector_is_empty)
-  [Specification](#@Specification_1)
    -  [Struct `BigVector`](#@Specification_1_BigVector)
    -  [Function `empty`](#@Specification_1_empty)
    -  [Function `singleton`](#@Specification_1_singleton)
    -  [Function `destroy_empty`](#@Specification_1_destroy_empty)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `borrow_mut`](#@Specification_1_borrow_mut)
    -  [Function `append`](#@Specification_1_append)
    -  [Function `push_back`](#@Specification_1_push_back)
    -  [Function `pop_back`](#@Specification_1_pop_back)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `swap_remove`](#@Specification_1_swap_remove)
    -  [Function `swap`](#@Specification_1_swap)
    -  [Function `reverse`](#@Specification_1_reverse)
    -  [Function `index_of`](#@Specification_1_index_of)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_big_vector_BigVector"></a>

## Struct `BigVector`

A scalable vector implementation based on tables where elements are grouped into buckets.
Each bucket has a capacity of <code>bucket_size</code> elements.


<pre><code><b>struct</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>buckets: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>end_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_big_vector_EINDEX_OUT_OF_BOUNDS"></a>

Vector index is out of bounds


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_big_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_big_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non&#45;empty vector


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_big_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_big_vector_empty"></a>

## Function `empty`

Regular Vector API
Create an empty vector.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_empty">empty</a>&lt;T: store&gt;(bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_empty">empty</a>&lt;T: store&gt;(bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; &#123;<br />    <b>assert</b>!(bucket_size &gt; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>));<br />    <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a> &#123;<br />        buckets: <a href="table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),<br />        end_index: 0,<br />        bucket_size,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in element.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_singleton">singleton</a>&lt;T: store&gt;(element: T, bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_singleton">singleton</a>&lt;T: store&gt;(element: T, bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; &#123;<br />    <b>let</b> v &#61; <a href="big_vector.md#0x1_big_vector_empty">empty</a>(bucket_size);<br />    <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(&amp;<b>mut</b> v, element);<br />    v<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) &#123;<br />    <b>assert</b>!(<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(&amp;v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>));<br />    <b>let</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a> &#123; buckets, end_index: _, bucket_size: _ &#125; &#61; v;<br />    <a href="table_with_length.md#0x1_table_with_length_destroy_empty">table_with_length::destroy_empty</a>(buckets);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_destroy"></a>

## Function `destroy`

Destroy the vector <code>v</code> if T has <code>drop</code>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy">destroy</a>&lt;T: drop&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy">destroy</a>&lt;T: drop&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) &#123;<br />    <b>let</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a> &#123; buckets, end_index, bucket_size: _ &#125; &#61; v;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (end_index &gt; 0) &#123;<br />        <b>let</b> num_elements &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;<a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> buckets, i));<br />        end_index &#61; end_index &#45; num_elements;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="table_with_length.md#0x1_table_with_length_destroy_empty">table_with_length::destroy_empty</a>(buckets);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow">borrow</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &amp;T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow">borrow</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): &amp;T &#123;<br />    <b>assert</b>!(i &lt; <a href="big_vector.md#0x1_big_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(<a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&amp;v.buckets, i / v.bucket_size), i % v.bucket_size)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &amp;<b>mut</b> T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): &amp;<b>mut</b> T &#123;<br />    <b>assert</b>!(i &lt; <a href="big_vector.md#0x1_big_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(<a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, i / v.bucket_size), i % v.bucket_size)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the elements in the other vector onto the lhs vector in the
same order as they occurred in other.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_append">append</a>&lt;T: store&gt;(lhs: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, other: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_append">append</a>&lt;T: store&gt;(lhs: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, other: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) &#123;<br />    <b>let</b> other_len &#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(&amp;other);<br />    <b>let</b> half_other_len &#61; other_len / 2;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; half_other_len) &#123;<br />        <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(lhs, <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>(&amp;<b>mut</b> other, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <b>while</b> (i &lt; other_len) &#123;<br />        <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(lhs, <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>(&amp;<b>mut</b> other));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>(other);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_push_back"></a>

## Function `push_back`

Add element <code>val</code> to the end of the vector <code>v</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, val: T) &#123;<br />    <b>let</b> num_buckets &#61; <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets);<br />    <b>if</b> (v.end_index &#61;&#61; num_buckets &#42; v.bucket_size) &#123;<br />        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&amp;<b>mut</b> v.buckets, num_buckets, <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(<a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, num_buckets), val);<br />    &#125; <b>else</b> &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(<a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, num_buckets &#45; 1), val);<br />    &#125;;<br />    v.end_index &#61; v.end_index &#43; 1;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>v</code>. It doesn&apos;t shrink the buckets even if they&apos;re empty.
Call <code>shrink_to_fit</code> explicity to deallocate empty buckets.
Aborts if <code>v</code> is empty.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): T &#123;<br />    <b>assert</b>!(!<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_vector.md#0x1_big_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));<br />    <b>let</b> num_buckets &#61; <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets);<br />    <b>let</b> last_bucket &#61; <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, num_buckets &#45; 1);<br />    <b>let</b> val &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(last_bucket);<br />    // Shrink the <a href="table.md#0x1_table">table</a> <b>if</b> the last <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> is empty.<br />    <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(last_bucket)) &#123;<br />        <b>move</b> last_bucket;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(<a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> v.buckets, num_buckets &#45; 1));<br />    &#125;;<br />    v.end_index &#61; v.end_index &#45; 1;<br />    val<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_remove"></a>

## Function `remove`

Remove the element at index i in the vector v and return the owned value that was previously stored at i in v.
All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_remove">remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_remove">remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): T &#123;<br />    <b>let</b> len &#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(v);<br />    <b>assert</b>!(i &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> num_buckets &#61; <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets);<br />    <b>let</b> cur_bucket_index &#61; i / v.bucket_size &#43; 1;<br />    <b>let</b> cur_bucket &#61; <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, cur_bucket_index &#45; 1);<br />    <b>let</b> res &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(cur_bucket, i % v.bucket_size);<br />    v.end_index &#61; v.end_index &#45; 1;<br />    <b>move</b> cur_bucket;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> cur_bucket_index &lt;&#61; num_buckets;<br />            <b>invariant</b> <a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(v.buckets) &#61;&#61; num_buckets;<br />        &#125;;<br />        (cur_bucket_index &lt; num_buckets)<br />    &#125;) &#123;<br />        // remove one element from the start of current <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a><br />        <b>let</b> cur_bucket &#61; <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, cur_bucket_index);<br />        <b>let</b> t &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(cur_bucket, 0);<br />        <b>move</b> cur_bucket;<br />        // and put it at the end of the last one<br />        <b>let</b> prev_bucket &#61; <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, cur_bucket_index &#45; 1);<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(prev_bucket, t);<br />        cur_bucket_index &#61; cur_bucket_index &#43; 1;<br />    &#125;;<br />    <b>spec</b> &#123;<br />        <b>assert</b> cur_bucket_index &#61;&#61; num_buckets;<br />    &#125;;<br /><br />    // Shrink the <a href="table.md#0x1_table">table</a> <b>if</b> the last <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> is empty.<br />    <b>let</b> last_bucket &#61; <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, num_buckets &#45; 1);<br />    <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(last_bucket)) &#123;<br />        <b>move</b> last_bucket;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(<a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> v.buckets, num_buckets &#45; 1));<br />    &#125;;<br /><br />    res<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>v</code> with the last element and then pop the vector.
This is O(1), but does not preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): T &#123;<br />    <b>assert</b>!(i &lt; <a href="big_vector.md#0x1_big_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> last_val &#61; <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>(v);<br />    // <b>if</b> the requested value is the last one, <b>return</b> it<br />    <b>if</b> (v.end_index &#61;&#61; i) &#123;<br />        <b>return</b> last_val<br />    &#125;;<br />    // because the lack of mem::swap, here we swap remove the requested value from the bucket<br />    // and append the last_val <b>to</b> the bucket then swap the last bucket val back<br />    <b>let</b> bucket &#61; <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, i / v.bucket_size);<br />    <b>let</b> bucket_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket);<br />    <b>let</b> val &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(bucket, i % v.bucket_size);<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(bucket, last_val);<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(bucket, i % v.bucket_size, bucket_len &#45; 1);<br />    val<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_swap"></a>

## Function `swap`

Swap the elements at the i&apos;th and j&apos;th indices in the vector v. Will abort if either of i or j are out of bounds
for v.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap">swap</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64, j: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap">swap</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64, j: u64) &#123;<br />    <b>assert</b>!(i &lt; <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &amp;&amp; j &lt; <a href="big_vector.md#0x1_big_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> i_bucket_index &#61; i / v.bucket_size;<br />    <b>let</b> j_bucket_index &#61; j / v.bucket_size;<br />    <b>let</b> i_vector_index &#61; i % v.bucket_size;<br />    <b>let</b> j_vector_index &#61; j % v.bucket_size;<br />    <b>if</b> (i_bucket_index &#61;&#61; j_bucket_index) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(<a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&amp;<b>mut</b> v.buckets, i_bucket_index), i_vector_index, j_vector_index);<br />        <b>return</b><br />    &#125;;<br />    // If i and j are in different buckets, take the buckets out first for easy mutation.<br />    <b>let</b> bucket_i &#61; <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> v.buckets, i_bucket_index);<br />    <b>let</b> bucket_j &#61; <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> v.buckets, j_bucket_index);<br />    // Get the elements from buckets by calling `swap_remove`.<br />    <b>let</b> element_i &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&amp;<b>mut</b> bucket_i, i_vector_index);<br />    <b>let</b> element_j &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&amp;<b>mut</b> bucket_j, j_vector_index);<br />    // Swap the elements and push back <b>to</b> the other bucket.<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> bucket_i, element_j);<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> bucket_j, element_i);<br />    <b>let</b> last_index_in_bucket_i &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bucket_i) &#45; 1;<br />    <b>let</b> last_index_in_bucket_j &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;bucket_j) &#45; 1;<br />    // Re&#45;position the swapped elements <b>to</b> the right index.<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(&amp;<b>mut</b> bucket_i, i_vector_index, last_index_in_bucket_i);<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(&amp;<b>mut</b> bucket_j, j_vector_index, last_index_in_bucket_j);<br />    // Add back the buckets.<br />    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&amp;<b>mut</b> v.buckets, i_bucket_index, bucket_i);<br />    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&amp;<b>mut</b> v.buckets, j_bucket_index, bucket_j);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_reverse"></a>

## Function `reverse`

Reverse the order of the elements in the vector v in&#45;place.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_reverse">reverse</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_reverse">reverse</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) &#123;<br />    <b>let</b> new_buckets &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <b>let</b> push_bucket &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <b>let</b> num_buckets &#61; <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets);<br />    <b>let</b> num_buckets_left &#61; num_buckets;<br /><br />    <b>while</b> (num_buckets_left &gt; 0) &#123;<br />        <b>let</b> pop_bucket &#61; <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&amp;<b>mut</b> v.buckets, num_buckets_left &#45; 1);<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_reverse">vector::for_each_reverse</a>(pop_bucket, &#124;val&#124; &#123;<br />            <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> push_bucket, val);<br />            <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;push_bucket) &#61;&#61; v.bucket_size) &#123;<br />                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> new_buckets, push_bucket);<br />                push_bucket &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />            &#125;;<br />        &#125;);<br />        num_buckets_left &#61; num_buckets_left &#45; 1;<br />    &#125;;<br /><br />    <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;push_bucket) &gt; 0) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> new_buckets, push_bucket);<br />    &#125; <b>else</b> &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(push_bucket);<br />    &#125;;<br /><br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&amp;<b>mut</b> new_buckets);<br />    <b>let</b> i &#61; 0;<br />    <b>assert</b>!(<a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets) &#61;&#61; 0, 0);<br />    <b>while</b> (i &lt; num_buckets) &#123;<br />        <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&amp;<b>mut</b> v.buckets, i, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> new_buckets));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(new_buckets);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_index_of"></a>

## Function `index_of`

Return the index of the first occurrence of an element in v that is equal to e. Returns (true, index) if such an
element was found, and (false, 0) otherwise.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: &amp;T): (bool, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, val: &amp;T): (bool, u64) &#123;<br />    <b>let</b> num_buckets &#61; <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets);<br />    <b>let</b> bucket_index &#61; 0;<br />    <b>while</b> (bucket_index &lt; num_buckets) &#123;<br />        <b>let</b> cur &#61; <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&amp;v.buckets, bucket_index);<br />        <b>let</b> (found, i) &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(cur, val);<br />        <b>if</b> (found) &#123;<br />            <b>return</b> (<b>true</b>, bucket_index &#42; v.bucket_size &#43; i)<br />        &#125;;<br />        bucket_index &#61; bucket_index &#43; 1;<br />    &#125;;<br />    (<b>false</b>, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_contains"></a>

## Function `contains`

Return if an element equal to e exists in the vector v.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_contains">contains</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: &amp;T): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_contains">contains</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, val: &amp;T): bool &#123;<br />    <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(v)) <b>return</b> <b>false</b>;<br />    <b>let</b> (exist, _) &#61; <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>(v, val);<br />    exist<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_to_vector"></a>

## Function `to_vector`

Convert a big vector to a native vector, which is supposed to be called mostly by view functions to get an
atomic view of the whole vector.
Disclaimer: This function may be costly as the big vector may be huge in size. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_to_vector">to_vector</a>&lt;T: <b>copy</b>&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_to_vector">to_vector</a>&lt;T: <b>copy</b>&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; &#123;<br />    <b>let</b> res &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <b>let</b> num_buckets &#61; <a href="table_with_length.md#0x1_table_with_length_length">table_with_length::length</a>(&amp;v.buckets);<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; num_buckets) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> res, &#42;<a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&amp;v.buckets, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    res<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_length">length</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_length">length</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): u64 &#123;<br />    v.end_index<br />&#125;<br /></code></pre>



</details>

<a id="0x1_big_vector_is_empty"></a>

## Function `is_empty`

Return <code><b>true</b></code> if the vector <code>v</code> has no elements and <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): bool &#123;<br />    <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_BigVector"></a>

### Struct `BigVector`


<pre><code><b>struct</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; <b>has</b> store<br /></code></pre>



<dl>
<dt>
<code>buckets: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>end_index: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> bucket_size !&#61; 0;<br /><b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; 0 &#61;&#61;&gt; end_index &#61;&#61; 0;<br /><b>invariant</b> end_index &#61;&#61; 0 &#61;&#61;&gt; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; 0;<br /><b>invariant</b> end_index &lt;&#61; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#42; bucket_size;<br /><b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; 0<br />    &#124;&#124; (<b>forall</b> i in 0..<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets)&#45;1: len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, i)) &#61;&#61; bucket_size);<br /><b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; 0<br />    &#124;&#124; len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#45;1 )) &lt;&#61; bucket_size;<br /><b>invariant</b> <b>forall</b> i in 0..<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets): <a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>(buckets, i);<br /><b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; (end_index &#43; bucket_size &#45; 1) / bucket_size;<br /><b>invariant</b> (<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; 0 &amp;&amp; end_index &#61;&#61; 0)<br />    &#124;&#124; (<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) !&#61; 0 &amp;&amp; ((<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#45; 1) &#42; bucket_size) &#43; (len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#45; 1))) &#61;&#61; end_index);<br /><b>invariant</b> <b>forall</b> i: u64 <b>where</b> i &gt;&#61; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets):  &#123;<br />    !<a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>(buckets, i)<br />&#125;;<br /><b>invariant</b> <b>forall</b> i: u64 <b>where</b> i &lt; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets):  &#123;<br />    <a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>(buckets, i)<br />&#125;;<br /><b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#61;&#61; 0<br />    &#124;&#124; (len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) &#45; 1)) &gt; 0);<br /></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_empty">empty</a>&lt;T: store&gt;(bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> bucket_size &#61;&#61; 0;<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_length">length</a>(result) &#61;&#61; 0;<br /><b>ensures</b> result.bucket_size &#61;&#61; bucket_size;<br /></code></pre>



<a id="@Specification_1_singleton"></a>

### Function `singleton`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_singleton">singleton</a>&lt;T: store&gt;(element: T, bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> bucket_size &#61;&#61; 0;<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_length">length</a>(result) &#61;&#61; 1;<br /><b>ensures</b> result.bucket_size &#61;&#61; bucket_size;<br /></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(v);<br /></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow">borrow</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &amp;T<br /></code></pre>




<pre><code><b>aborts_if</b> i &gt;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(v);<br /><b>ensures</b> result &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, i);<br /></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &amp;<b>mut</b> T<br /></code></pre>




<pre><code><b>aborts_if</b> i &gt;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(v);<br /><b>ensures</b> result &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, i);<br /></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_append">append</a>&lt;T: store&gt;(lhs: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, other: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: T)<br /></code></pre>




<pre><code><b>let</b> num_buckets &#61; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(v.buckets);<br /><b>include</b> <a href="big_vector.md#0x1_big_vector_PushbackAbortsIf">PushbackAbortsIf</a>&lt;T&gt;;<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &#61;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(<b>old</b>(v)) &#43; 1;<br /><b>ensures</b> v.end_index &#61;&#61; <b>old</b>(v.end_index) &#43; 1;<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, v.end_index&#45;1) &#61;&#61; val;<br /><b>ensures</b> <b>forall</b> i in 0..v.end_index&#45;1: <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, i) &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(v), i);<br /><b>ensures</b> v.bucket_size &#61;&#61; <b>old</b>(v).bucket_size;<br /></code></pre>




<a id="0x1_big_vector_PushbackAbortsIf"></a>


<pre><code><b>schema</b> <a href="big_vector.md#0x1_big_vector_PushbackAbortsIf">PushbackAbortsIf</a>&lt;T&gt; &#123;<br />v: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;;<br /><b>let</b> num_buckets &#61; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(v.buckets);<br /><b>aborts_if</b> num_buckets &#42; v.bucket_size &gt; MAX_U64;<br /><b>aborts_if</b> v.end_index &#43; 1 &gt; MAX_U64;<br />&#125;<br /></code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): T<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(v);<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &#61;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(<b>old</b>(v)) &#45; 1;<br /><b>ensures</b> result &#61;&#61; <b>old</b>(<a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, v.end_index&#45;1));<br /><b>ensures</b> <b>forall</b> i in 0..v.end_index: <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, i) &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(v), i);<br /></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_remove">remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>aborts_if</b> i &gt;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(v);<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &#61;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(<b>old</b>(v)) &#45; 1;<br /><b>ensures</b> result &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(v), i);<br /></code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap">swap</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64, j: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1000;<br /><b>aborts_if</b> i &gt;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &#124;&#124; j &gt;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(v);<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_length">length</a>(v) &#61;&#61; <a href="big_vector.md#0x1_big_vector_length">length</a>(<b>old</b>(v));<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, i) &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(v), j);<br /><b>ensures</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, j) &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(v), i);<br /><b>ensures</b> <b>forall</b> idx in 0..<a href="big_vector.md#0x1_big_vector_length">length</a>(v)<br />    <b>where</b> idx !&#61; i &amp;&amp; idx !&#61; j:<br />    <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(v, idx) &#61;&#61; <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(v), idx);<br /></code></pre>



<a id="@Specification_1_reverse"></a>

### Function `reverse`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_reverse">reverse</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>



<a id="@Specification_1_index_of"></a>

### Function `index_of`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>&lt;T&gt;(v: &amp;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: &amp;T): (bool, u64)<br /></code></pre>




<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>




<a id="0x1_big_vector_spec_table_len"></a>


<pre><code><b>fun</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;): u64 &#123;<br />   <a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(t)<br />&#125;<br /></code></pre>




<a id="0x1_big_vector_spec_table_contains"></a>


<pre><code><b>fun</b> <a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): bool &#123;<br />   <a href="table_with_length.md#0x1_table_with_length_spec_contains">table_with_length::spec_contains</a>(t, k)<br />&#125;<br /></code></pre>




<a id="0x1_big_vector_spec_at"></a>


<pre><code><b>fun</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>&lt;T&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): T &#123;<br />   <b>let</b> bucket &#61; i / v.bucket_size;<br />   <b>let</b> idx &#61; i % v.bucket_size;<br />   <b>let</b> v &#61; <a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(v.buckets, bucket);<br />   v[idx]<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
