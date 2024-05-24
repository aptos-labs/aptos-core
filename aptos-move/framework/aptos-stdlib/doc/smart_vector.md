
<a id="0x1_smart_vector"></a>

# Module `0x1::smart_vector`



-  [Struct `SmartVector`](#0x1_smart_vector_SmartVector)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_smart_vector_new)
-  [Function `empty`](#0x1_smart_vector_empty)
-  [Function `empty_with_config`](#0x1_smart_vector_empty_with_config)
-  [Function `singleton`](#0x1_smart_vector_singleton)
-  [Function `destroy_empty`](#0x1_smart_vector_destroy_empty)
-  [Function `destroy`](#0x1_smart_vector_destroy)
-  [Function `clear`](#0x1_smart_vector_clear)
-  [Function `borrow`](#0x1_smart_vector_borrow)
-  [Function `borrow_mut`](#0x1_smart_vector_borrow_mut)
-  [Function `append`](#0x1_smart_vector_append)
-  [Function `add_all`](#0x1_smart_vector_add_all)
-  [Function `to_vector`](#0x1_smart_vector_to_vector)
-  [Function `push_back`](#0x1_smart_vector_push_back)
-  [Function `pop_back`](#0x1_smart_vector_pop_back)
-  [Function `remove`](#0x1_smart_vector_remove)
-  [Function `swap_remove`](#0x1_smart_vector_swap_remove)
-  [Function `swap`](#0x1_smart_vector_swap)
-  [Function `reverse`](#0x1_smart_vector_reverse)
-  [Function `index_of`](#0x1_smart_vector_index_of)
-  [Function `contains`](#0x1_smart_vector_contains)
-  [Function `length`](#0x1_smart_vector_length)
-  [Function `is_empty`](#0x1_smart_vector_is_empty)
-  [Function `for_each`](#0x1_smart_vector_for_each)
-  [Function `for_each_reverse`](#0x1_smart_vector_for_each_reverse)
-  [Function `for_each_ref`](#0x1_smart_vector_for_each_ref)
-  [Function `for_each_mut`](#0x1_smart_vector_for_each_mut)
-  [Function `enumerate_ref`](#0x1_smart_vector_enumerate_ref)
-  [Function `enumerate_mut`](#0x1_smart_vector_enumerate_mut)
-  [Function `fold`](#0x1_smart_vector_fold)
-  [Function `foldr`](#0x1_smart_vector_foldr)
-  [Function `map_ref`](#0x1_smart_vector_map_ref)
-  [Function `map`](#0x1_smart_vector_map)
-  [Function `filter`](#0x1_smart_vector_filter)
-  [Function `zip`](#0x1_smart_vector_zip)
-  [Function `zip_reverse`](#0x1_smart_vector_zip_reverse)
-  [Function `zip_ref`](#0x1_smart_vector_zip_ref)
-  [Function `zip_mut`](#0x1_smart_vector_zip_mut)
-  [Function `zip_map`](#0x1_smart_vector_zip_map)
-  [Function `zip_map_ref`](#0x1_smart_vector_zip_map_ref)
-  [Specification](#@Specification_1)
    -  [Struct `SmartVector`](#@Specification_1_SmartVector)
    -  [Function `empty`](#@Specification_1_empty)
    -  [Function `empty_with_config`](#@Specification_1_empty_with_config)
    -  [Function `destroy_empty`](#@Specification_1_destroy_empty)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `append`](#@Specification_1_append)
    -  [Function `push_back`](#@Specification_1_push_back)
    -  [Function `pop_back`](#@Specification_1_pop_back)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `swap_remove`](#@Specification_1_swap_remove)
    -  [Function `swap`](#@Specification_1_swap)
    -  [Function `length`](#@Specification_1_length)


<pre><code><b>use</b> <a href="big_vector.md#0x1_big_vector">0x1::big_vector</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="math64.md#0x1_math64">0x1::math64</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_smart_vector_SmartVector"></a>

## Struct `SmartVector`

A Scalable vector implementation based on tables, Ts are grouped into buckets with <code>bucket_size</code>.
The option wrapping BigVector saves space in the metadata associated with BigVector when smart_vector is
so small that inline_vec vector can hold all the data.


<pre><code><b>struct</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inline_vec: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>big_vec: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inline_capacity: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_smart_vector_EINDEX_OUT_OF_BOUNDS"></a>

Vector index is out of bounds


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_smart_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_smart_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non&#45;empty vector


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_smart_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH"></a>

The length of the smart vectors are not equal.


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a>: u64 &#61; 131077;<br /></code></pre>



<a id="0x1_smart_vector_new"></a>

## Function `new`

Regular Vector API
Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.
This is exactly the same as empty() but is more standardized as all other data structures have new().


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_new">new</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_new">new</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; &#123;<br />    <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>()<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_empty"></a>

## Function `empty`

Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; &#123;<br />    <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> &#123;<br />        inline_vec: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />        big_vec: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        inline_capacity: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        bucket_size: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_empty_with_config"></a>

## Function `empty_with_config`

Create an empty vector with customized config.
When inline_capacity &#61; 0, SmartVector degrades to a wrapper of BigVector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; &#123;<br />    <b>assert</b>!(bucket_size &gt; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>));<br />    <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> &#123;<br />        inline_vec: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />        big_vec: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        inline_capacity: <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(inline_capacity),<br />        bucket_size: <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(bucket_size),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in T.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_singleton">singleton</a>&lt;T: store&gt;(element: T): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_singleton">singleton</a>&lt;T: store&gt;(element: T): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; &#123;<br />    <b>let</b> v &#61; <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>();<br />    <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&amp;<b>mut</b> v, element);<br />    v<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) &#123;<br />    <b>assert</b>!(<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(&amp;v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>));<br />    <b>let</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> &#123; inline_vec, big_vec, inline_capacity: _, bucket_size: _ &#125; &#61; v;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(inline_vec);<br />    <a href="../../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(big_vec);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_destroy"></a>

## Function `destroy`

Destroy a vector completely when T has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy">destroy</a>&lt;T: drop&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy">destroy</a>&lt;T: drop&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) &#123;<br />    <a href="smart_vector.md#0x1_smart_vector_clear">clear</a>(&amp;<b>mut</b> v);<br />    <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>(v);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_clear"></a>

## Function `clear`

Clear a vector completely when T has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_clear">clear</a>&lt;T: drop&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_clear">clear</a>&lt;T: drop&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) &#123;<br />    v.inline_vec &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;v.big_vec)) &#123;<br />        <a href="big_vector.md#0x1_big_vector_destroy">big_vector::destroy</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> v.big_vec));<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th T of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &amp;T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): &amp;T &#123;<br />    <b>assert</b>!(i &lt; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>if</b> (i &lt; inline_len) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;v.inline_vec, i)<br />    &#125; <b>else</b> &#123;<br />        <a href="big_vector.md#0x1_big_vector_borrow">big_vector::borrow</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;v.big_vec), i &#45; inline_len)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th T in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &amp;<b>mut</b> T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): &amp;<b>mut</b> T &#123;<br />    <b>assert</b>!(i &lt; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>if</b> (i &lt; inline_len) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> v.inline_vec, i)<br />    &#125; <b>else</b> &#123;<br />        <a href="big_vector.md#0x1_big_vector_borrow_mut">big_vector::borrow_mut</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> v.big_vec), i &#45; inline_len)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the Ts in the other vector onto the lhs vector in the
same order as they occurred in other.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(lhs: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(lhs: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) &#123;<br />    <b>let</b> other_len &#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(&amp;other);<br />    <b>let</b> half_other_len &#61; other_len / 2;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; half_other_len) &#123;<br />        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(lhs, <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>(&amp;<b>mut</b> other, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <b>while</b> (i &lt; other_len) &#123;<br />        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(lhs, <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>(&amp;<b>mut</b> other));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>(other);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_add_all"></a>

## Function `add_all`

Add multiple values to the vector at once.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_add_all">add_all</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, vals: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_add_all">add_all</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, vals: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;) &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(vals, &#124;val&#124; &#123; <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(v, val); &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_to_vector"></a>

## Function `to_vector`

Convert a smart vector to a native vector, which is supposed to be called mostly by view functions to get an
atomic view of the whole vector.
Disclaimer: This function may be costly as the smart vector may be huge in size. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_to_vector">to_vector</a>&lt;T: <b>copy</b>, store&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_to_vector">to_vector</a>&lt;T: store &#43; <b>copy</b>&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; &#123;<br />    <b>let</b> res &#61; v.inline_vec;<br />    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;v.big_vec)) &#123;<br />        <b>let</b> big_vec &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;v.big_vec);<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> res, <a href="big_vector.md#0x1_big_vector_to_vector">big_vector::to_vector</a>(big_vec));<br />    &#125;;<br />    res<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_push_back"></a>

## Function `push_back`

Add T <code>val</code> to the end of the vector <code>v</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: T) &#123;<br />    <b>let</b> len &#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>if</b> (len &#61;&#61; inline_len) &#123;<br />        <b>let</b> bucket_size &#61; <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;v.inline_capacity)) &#123;<br />            <b>if</b> (len &lt; &#42;<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;v.inline_capacity)) &#123;<br />                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> v.inline_vec, val);<br />                <b>return</b><br />            &#125;;<br />            &#42;<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;v.bucket_size)<br />        &#125; <b>else</b> &#123;<br />            <b>let</b> val_size &#61; size_of_val(&amp;val);<br />            <b>if</b> (val_size &#42; (inline_len &#43; 1) &lt; 150 /&#42; magic number &#42;/) &#123;<br />                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> v.inline_vec, val);<br />                <b>return</b><br />            &#125;;<br />            <b>let</b> estimated_avg_size &#61; max((size_of_val(&amp;v.inline_vec) &#43; val_size) / (inline_len &#43; 1), 1);<br />            max(1024 /&#42; free_write_quota &#42;/ / estimated_avg_size, 1)<br />        &#125;;<br />        <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(&amp;<b>mut</b> v.big_vec, <a href="big_vector.md#0x1_big_vector_empty">big_vector::empty</a>(bucket_size));<br />    &#125;;<br />    <a href="big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> v.big_vec), val);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_pop_back"></a>

## Function `pop_back`

Pop an T from the end of vector <code>v</code>. It does shrink the buckets if they&apos;re empty.
Aborts if <code>v</code> is empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): T &#123;<br />    <b>assert</b>!(!<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="smart_vector.md#0x1_smart_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));<br />    <b>let</b> big_vec_wrapper &#61; &amp;<b>mut</b> v.big_vec;<br />    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(big_vec_wrapper)) &#123;<br />        <b>let</b> big_vec &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);<br />        <b>let</b> val &#61; <a href="big_vector.md#0x1_big_vector_pop_back">big_vector::pop_back</a>(&amp;<b>mut</b> big_vec);<br />        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&amp;big_vec)) &#123;<br />            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)<br />        &#125; <b>else</b> &#123;<br />            <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);<br />        &#125;;<br />        val<br />    &#125; <b>else</b> &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> v.inline_vec)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_remove"></a>

## Function `remove`

Remove the T at index i in the vector v and return the owned value that was previously stored at i in v.
All Ts occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): T &#123;<br />    <b>let</b> len &#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br />    <b>assert</b>!(i &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>if</b> (i &lt; inline_len) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&amp;<b>mut</b> v.inline_vec, i)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> big_vec_wrapper &#61; &amp;<b>mut</b> v.big_vec;<br />        <b>let</b> big_vec &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);<br />        <b>let</b> val &#61; <a href="big_vector.md#0x1_big_vector_remove">big_vector::remove</a>(&amp;<b>mut</b> big_vec, i &#45; inline_len);<br />        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&amp;big_vec)) &#123;<br />            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)<br />        &#125; <b>else</b> &#123;<br />            <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);<br />        &#125;;<br />        val<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th T of the vector <code>v</code> with the last T and then pop the vector.
This is O(1), but does not preserve ordering of Ts in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): T &#123;<br />    <b>let</b> len &#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br />    <b>assert</b>!(i &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>let</b> big_vec_wrapper &#61; &amp;<b>mut</b> v.big_vec;<br />    <b>let</b> inline_vec &#61; &amp;<b>mut</b> v.inline_vec;<br />    <b>if</b> (i &gt;&#61; inline_len) &#123;<br />        <b>let</b> big_vec &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);<br />        <b>let</b> val &#61; <a href="big_vector.md#0x1_big_vector_swap_remove">big_vector::swap_remove</a>(&amp;<b>mut</b> big_vec, i &#45; inline_len);<br />        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&amp;big_vec)) &#123;<br />            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)<br />        &#125; <b>else</b> &#123;<br />            <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);<br />        &#125;;<br />        val<br />    &#125; <b>else</b> &#123;<br />        <b>if</b> (inline_len &lt; len) &#123;<br />            <b>let</b> big_vec &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);<br />            <b>let</b> last_from_big_vec &#61; <a href="big_vector.md#0x1_big_vector_pop_back">big_vector::pop_back</a>(&amp;<b>mut</b> big_vec);<br />            <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&amp;big_vec)) &#123;<br />                <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)<br />            &#125; <b>else</b> &#123;<br />                <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);<br />            &#125;;<br />            <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(inline_vec, last_from_big_vec);<br />        &#125;;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(inline_vec, i)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_swap"></a>

## Function `swap`

Swap the Ts at the i&apos;th and j&apos;th indices in the vector v. Will abort if either of i or j are out of bounds
for v.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64, j: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64, j: u64) &#123;<br />    <b>if</b> (i &gt; j) &#123;<br />        <b>return</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>(v, j, i)<br />    &#125;;<br />    <b>let</b> len &#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br />    <b>assert</b>!(j &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>if</b> (i &gt;&#61; inline_len) &#123;<br />        <a href="big_vector.md#0x1_big_vector_swap">big_vector::swap</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> v.big_vec), i &#45; inline_len, j &#45; inline_len);<br />    &#125; <b>else</b> <b>if</b> (j &lt; inline_len) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(&amp;<b>mut</b> v.inline_vec, i, j);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> big_vec &#61; <a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> v.big_vec);<br />        <b>let</b> inline_vec &#61; &amp;<b>mut</b> v.inline_vec;<br />        <b>let</b> element_i &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(inline_vec, i);<br />        <b>let</b> element_j &#61; <a href="big_vector.md#0x1_big_vector_swap_remove">big_vector::swap_remove</a>(big_vec, j &#45; inline_len);<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(inline_vec, element_j);<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(inline_vec, i, inline_len &#45; 1);<br />        <a href="big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(big_vec, element_i);<br />        <a href="big_vector.md#0x1_big_vector_swap">big_vector::swap</a>(big_vec, j &#45; inline_len, len &#45; inline_len &#45; 1);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_reverse"></a>

## Function `reverse`

Reverse the order of the Ts in the vector v in&#45;place.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_reverse">reverse</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_reverse">reverse</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) &#123;<br />    <b>let</b> inline_len &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec);<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> new_inline_vec &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    // Push the last `inline_len` Ts into a temp <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>.<br />    <b>while</b> (i &lt; inline_len) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> new_inline_vec, <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>(v));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&amp;<b>mut</b> new_inline_vec);<br />    // Reverse the <a href="big_vector.md#0x1_big_vector">big_vector</a> left <b>if</b> <b>exists</b>.<br />    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;v.big_vec)) &#123;<br />        <a href="big_vector.md#0x1_big_vector_reverse">big_vector::reverse</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> v.big_vec));<br />    &#125;;<br />    // Mem::swap the two vectors.<br />    <b>let</b> temp_vec &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />    <b>while</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;<b>mut</b> v.inline_vec)) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> temp_vec, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> v.inline_vec));<br />    &#125;;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&amp;<b>mut</b> temp_vec);<br />    <b>while</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;<b>mut</b> new_inline_vec)) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> v.inline_vec, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> new_inline_vec));<br />    &#125;;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(new_inline_vec);<br />    // Push the rest Ts originally left in inline_vector back <b>to</b> the end of the smart <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>.<br />    <b>while</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;<b>mut</b> temp_vec)) &#123;<br />        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(v, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> temp_vec));<br />    &#125;;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(temp_vec);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_index_of"></a>

## Function `index_of`

Return <code>(<b>true</b>, i)</code> if <code>val</code> is in the vector <code>v</code> at index <code>i</code>.
Otherwise, returns <code>(<b>false</b>, 0)</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: &amp;T): (bool, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: &amp;T): (bool, u64) &#123;<br />    <b>let</b> (found, i) &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&amp;v.inline_vec, val);<br />    <b>if</b> (found) &#123;<br />        (<b>true</b>, i)<br />    &#125; <b>else</b> <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;v.big_vec)) &#123;<br />        <b>let</b> (found, i) &#61; <a href="big_vector.md#0x1_big_vector_index_of">big_vector::index_of</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;v.big_vec), val);<br />        (found, i &#43; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec))<br />    &#125; <b>else</b> &#123;<br />        (<b>false</b>, 0)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_contains"></a>

## Function `contains`

Return true if <code>val</code> is in the vector <code>v</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_contains">contains</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: &amp;T): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_contains">contains</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: &amp;T): bool &#123;<br />    <b>if</b> (<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v)) <b>return</b> <b>false</b>;<br />    <b>let</b> (exist, _) &#61; <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>(v, val);<br />    exist<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): u64 &#123;<br />    <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;v.inline_vec) &#43; <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&amp;v.big_vec)) &#123;<br />        0<br />    &#125; <b>else</b> &#123;<br />        <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;v.big_vec))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_is_empty"></a>

## Function `is_empty`

Return <code><b>true</b></code> if the vector <code>v</code> has no Ts and <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): bool &#123;<br />    <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v) &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_for_each"></a>

## Function `for_each`

Apply the function to each T in the vector, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each">for_each</a>&lt;T: store&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: &#124;T&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each">for_each</a>&lt;T: store&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: &#124;T&#124;) &#123;<br />    aptos_std::smart_vector::reverse(&amp;<b>mut</b> v); // We need <b>to</b> reverse the <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> <b>to</b> consume it efficiently<br />    aptos_std::smart_vector::for_each_reverse(v, &#124;e&#124; f(e));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_for_each_reverse"></a>

## Function `for_each_reverse`

Apply the function to each T in the vector, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_reverse">for_each_reverse</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: &#124;T&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_reverse">for_each_reverse</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: &#124;T&#124;) &#123;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(&amp;v);<br />    <b>while</b> (len &gt; 0) &#123;<br />        f(aptos_std::smart_vector::pop_back(&amp;<b>mut</b> v));<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    aptos_std::smart_vector::destroy_empty(v)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each T in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_ref">for_each_ref</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: &#124;&amp;T&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_ref">for_each_ref</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: &#124;&amp;T&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(aptos_std::smart_vector::borrow(v, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference to each T in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_mut">for_each_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: &#124;&amp;<b>mut</b> T&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_mut">for_each_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: &#124;&amp;<b>mut</b> T&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(aptos_std::smart_vector::borrow_mut(v, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_enumerate_ref"></a>

## Function `enumerate_ref`

Apply the function to a reference of each T in the vector with its index.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_ref">enumerate_ref</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: &#124;(u64, &amp;T)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_ref">enumerate_ref</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: &#124;u64, &amp;T&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(i, aptos_std::smart_vector::borrow(v, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_enumerate_mut"></a>

## Function `enumerate_mut`

Apply the function to a mutable reference of each T in the vector with its index.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_mut">enumerate_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: &#124;(u64, &amp;<b>mut</b> T)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_mut">enumerate_mut</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: &#124;u64, &amp;<b>mut</b> T&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(i, <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>(v, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_fold"></a>

## Function `fold`

Fold the function over the Ts. For example, <code><a href="smart_vector.md#0x1_smart_vector_fold">fold</a>(<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1,2,3], 0, f)</code> will execute
<code>f(f(f(0, 1), 2), 3)</code>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_fold">fold</a>&lt;Accumulator, T: store&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, init: Accumulator, f: &#124;(Accumulator, T)&#124;Accumulator): Accumulator<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_fold">fold</a>&lt;Accumulator, T: store&gt;(<br />    v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;,<br />    init: Accumulator,<br />    f: &#124;Accumulator, T&#124;Accumulator<br />): Accumulator &#123;<br />    <b>let</b> accu &#61; init;<br />    aptos_std::smart_vector::for_each(v, &#124;elem&#124; accu &#61; f(accu, elem));<br />    accu<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_foldr"></a>

## Function `foldr`

Fold right like fold above but working right to left. For example, <code><a href="smart_vector.md#0x1_smart_vector_fold">fold</a>(<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1,2,3], 0, f)</code> will execute
<code>f(1, f(2, f(3, 0)))</code>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_foldr">foldr</a>&lt;Accumulator, T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, init: Accumulator, f: &#124;(T, Accumulator)&#124;Accumulator): Accumulator<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_foldr">foldr</a>&lt;Accumulator, T&gt;(<br />    v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;,<br />    init: Accumulator,<br />    f: &#124;T, Accumulator&#124;Accumulator<br />): Accumulator &#123;<br />    <b>let</b> accu &#61; init;<br />    aptos_std::smart_vector::for_each_reverse(v, &#124;elem&#124; accu &#61; f(elem, accu));<br />    accu<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_map_ref"></a>

## Function `map_ref`

Map the function over the references of the Ts of the vector, producing a new vector without modifying the
original vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map_ref">map_ref</a>&lt;T1, T2: store&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, f: &#124;&amp;T1&#124;T2): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map_ref">map_ref</a>&lt;T1, T2: store&gt;(<br />    v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    f: &#124;&amp;T1&#124;T2<br />): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt; &#123;<br />    <b>let</b> result &#61; aptos_std::smart_vector::new&lt;T2&gt;();<br />    aptos_std::smart_vector::for_each_ref(v, &#124;elem&#124; aptos_std::smart_vector::push_back(&amp;<b>mut</b> result, f(elem)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_map"></a>

## Function `map`

Map the function over the Ts of the vector, producing a new vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map">map</a>&lt;T1: store, T2: store&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, f: &#124;T1&#124;T2): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map">map</a>&lt;T1: store, T2: store&gt;(<br />    v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    f: &#124;T1&#124;T2<br />): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt; &#123;<br />    <b>let</b> result &#61; aptos_std::smart_vector::new&lt;T2&gt;();<br />    aptos_std::smart_vector::for_each(v, &#124;elem&#124; <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(elem)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_filter"></a>

## Function `filter`

Filter the vector using the boolean function, removing all Ts for which <code>p(e)</code> is not true.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_filter">filter</a>&lt;T: drop, store&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, p: &#124;&amp;T&#124;bool): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_filter">filter</a>&lt;T: store &#43; drop&gt;(<br />    v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;,<br />    p: &#124;&amp;T&#124;bool<br />): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; &#123;<br />    <b>let</b> result &#61; aptos_std::smart_vector::new&lt;T&gt;();<br />    aptos_std::smart_vector::for_each(v, &#124;elem&#124; &#123;<br />        <b>if</b> (p(&amp;elem)) aptos_std::smart_vector::push_back(&amp;<b>mut</b> result, elem);<br />    &#125;);<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_zip"></a>

## Function `zip`



<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip">zip</a>&lt;T1: store, T2: store&gt;(v1: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: &#124;(T1, T2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip">zip</a>&lt;T1: store, T2: store&gt;(v1: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;, f: &#124;T1, T2&#124;) &#123;<br />    // We need <b>to</b> reverse the vectors <b>to</b> consume it efficiently<br />    aptos_std::smart_vector::reverse(&amp;<b>mut</b> v1);<br />    aptos_std::smart_vector::reverse(&amp;<b>mut</b> v2);<br />    aptos_std::smart_vector::zip_reverse(v1, v2, &#124;e1, e2&#124; f(e1, e2));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_zip_reverse"></a>

## Function `zip_reverse`

Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_reverse">zip_reverse</a>&lt;T1, T2&gt;(v1: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: &#124;(T1, T2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_reverse">zip_reverse</a>&lt;T1, T2&gt;(<br />    v1: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,<br />    f: &#124;T1, T2&#124;,<br />) &#123;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(&amp;v1);<br />    // We can&apos;t <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(len &#61;&#61; aptos_std::smart_vector::length(&amp;v2), 0x20005);<br />    <b>while</b> (len &gt; 0) &#123;<br />        f(aptos_std::smart_vector::pop_back(&amp;<b>mut</b> v1), aptos_std::smart_vector::pop_back(&amp;<b>mut</b> v2));<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    aptos_std::smart_vector::destroy_empty(v1);<br />    aptos_std::smart_vector::destroy_empty(v2);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_zip_ref"></a>

## Function `zip_ref`

Apply the function to the references of each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_ref">zip_ref</a>&lt;T1, T2&gt;(v1: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: &#124;(&amp;T1, &amp;T2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_ref">zip_ref</a>&lt;T1, T2&gt;(<br />    v1: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    v2: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,<br />    f: &#124;&amp;T1, &amp;T2&#124;,<br />) &#123;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(v1);<br />    // We can&apos;t <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(len &#61;&#61; aptos_std::smart_vector::length(v2), 0x20005);<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; len) &#123;<br />        f(aptos_std::smart_vector::borrow(v1, i), aptos_std::smart_vector::borrow(v2, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_zip_mut"></a>

## Function `zip_mut`

Apply the function to mutable references to each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_mut">zip_mut</a>&lt;T1, T2&gt;(v1: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: &#124;(&amp;<b>mut</b> T1, &amp;<b>mut</b> T2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_mut">zip_mut</a>&lt;T1, T2&gt;(<br />    v1: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    v2: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,<br />    f: &#124;&amp;<b>mut</b> T1, &amp;<b>mut</b> T2&#124;,<br />) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; aptos_std::smart_vector::length(v1);<br />    // We can&apos;t <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(len &#61;&#61; aptos_std::smart_vector::length(v2), 0x20005);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(aptos_std::smart_vector::borrow_mut(v1, i), aptos_std::smart_vector::borrow_mut(v2, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_zip_map"></a>

## Function `zip_map`

Map the function over the element pairs of the two vectors, producing a new vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map">zip_map</a>&lt;T1: store, T2: store, NewT: store&gt;(v1: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: &#124;(T1, T2)&#124;NewT): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;NewT&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map">zip_map</a>&lt;T1: store, T2: store, NewT: store&gt;(<br />    v1: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,<br />    f: &#124;T1, T2&#124;NewT<br />): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;NewT&gt; &#123;<br />    // We can&apos;t <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(aptos_std::smart_vector::length(&amp;v1) &#61;&#61; aptos_std::smart_vector::length(&amp;v2), 0x20005);<br /><br />    <b>let</b> result &#61; aptos_std::smart_vector::new&lt;NewT&gt;();<br />    aptos_std::smart_vector::zip(v1, v2, &#124;e1, e2&#124; <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(e1, e2)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_smart_vector_zip_map_ref"></a>

## Function `zip_map_ref`

Map the function over the references of the element pairs of two vectors, producing a new vector from the return
values without modifying the original vectors.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map_ref">zip_map_ref</a>&lt;T1, T2, NewT: store&gt;(v1: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: &#124;(&amp;T1, &amp;T2)&#124;NewT): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;NewT&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map_ref">zip_map_ref</a>&lt;T1, T2, NewT: store&gt;(<br />    v1: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,<br />    v2: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,<br />    f: &#124;&amp;T1, &amp;T2&#124;NewT<br />): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;NewT&gt; &#123;<br />    // We can&apos;t <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(aptos_std::smart_vector::length(v1) &#61;&#61; aptos_std::smart_vector::length(v2), 0x20005);<br /><br />    <b>let</b> result &#61; aptos_std::smart_vector::new&lt;NewT&gt;();<br />    aptos_std::smart_vector::zip_ref(v1, v2, &#124;e1, e2&#124; <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(e1, e2)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SmartVector"></a>

### Struct `SmartVector`


<pre><code><b>struct</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; <b>has</b> store<br /></code></pre>



<dl>
<dt>
<code>inline_vec: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>big_vec: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inline_capacity: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(bucket_size)<br />    &#124;&#124; (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(bucket_size) &amp;&amp; <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(bucket_size) !&#61; 0);<br /><b>invariant</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(inline_capacity)<br />    &#124;&#124; (len(inline_vec) &lt;&#61; <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(inline_capacity));<br /><b>invariant</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(inline_capacity) &amp;&amp; <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(bucket_size))<br />    &#124;&#124; (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(inline_capacity) &amp;&amp; <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(bucket_size));<br /></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_empty_with_config"></a>

### Function `empty_with_config`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> bucket_size &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>




<pre><code><b>aborts_if</b> !(<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v));<br /><b>aborts_if</b> len(v.inline_vec) !&#61; 0<br />    &#124;&#124; <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec);<br /></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &amp;T<br /></code></pre>




<pre><code><b>aborts_if</b> i &gt;&#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br /><b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) &amp;&amp; (<br />    (len(v.inline_vec) &#43; <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec))) &gt; MAX_U64<br />);<br /></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(lhs: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: T)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): T<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>aborts_if</b>  <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec)<br />    &amp;&amp;<br />    (<a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec).buckets) &#61;&#61; 0);<br /><b>aborts_if</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v);<br /><b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) &amp;&amp; (<br />    (len(v.inline_vec) &#43; <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec))) &gt; MAX_U64<br />);<br /><b>ensures</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v) &#61;&#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(<b>old</b>(v)) &#45; 1;<br /></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> i &gt;&#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);<br /><b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) &amp;&amp; (<br />    (len(v.inline_vec) &#43; <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec))) &gt; MAX_U64<br />);<br /><b>ensures</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v) &#61;&#61; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(<b>old</b>(v)) &#45; 1;<br /></code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(v: &amp;<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64, j: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(v: &amp;<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): u64<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) &amp;&amp; len(v.inline_vec) &#43; <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(v.big_vec)) &gt; MAX_U64;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
