
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


<pre><code>use 0x1::error;<br/>use 0x1::table_with_length;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_big_vector_BigVector"></a>

## Struct `BigVector`

A scalable vector implementation based on tables where elements are grouped into buckets.<br/> Each bucket has a capacity of <code>bucket_size</code> elements.


<pre><code>struct BigVector&lt;T&gt; has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>buckets: table_with_length::TableWithLength&lt;u64, vector&lt;T&gt;&gt;</code>
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


<pre><code>const EINDEX_OUT_OF_BOUNDS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_big_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code>const EVECTOR_EMPTY: u64 &#61; 3;<br/></code></pre>



<a id="0x1_big_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non&#45;empty vector


<pre><code>const EVECTOR_NOT_EMPTY: u64 &#61; 2;<br/></code></pre>



<a id="0x1_big_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code>const EZERO_BUCKET_SIZE: u64 &#61; 4;<br/></code></pre>



<a id="0x1_big_vector_empty"></a>

## Function `empty`

Regular Vector API<br/> Create an empty vector.


<pre><code>public(friend) fun empty&lt;T: store&gt;(bucket_size: u64): big_vector::BigVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun empty&lt;T: store&gt;(bucket_size: u64): BigVector&lt;T&gt; &#123;<br/>    assert!(bucket_size &gt; 0, error::invalid_argument(EZERO_BUCKET_SIZE));<br/>    BigVector &#123;<br/>        buckets: table_with_length::new(),<br/>        end_index: 0,<br/>        bucket_size,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in element.


<pre><code>public(friend) fun singleton&lt;T: store&gt;(element: T, bucket_size: u64): big_vector::BigVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun singleton&lt;T: store&gt;(element: T, bucket_size: u64): BigVector&lt;T&gt; &#123;<br/>    let v &#61; empty(bucket_size);<br/>    push_back(&amp;mut v, element);<br/>    v<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.<br/> Aborts if <code>v</code> is not empty.


<pre><code>public fun destroy_empty&lt;T&gt;(v: big_vector::BigVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;T&gt;(v: BigVector&lt;T&gt;) &#123;<br/>    assert!(is_empty(&amp;v), error::invalid_argument(EVECTOR_NOT_EMPTY));<br/>    let BigVector &#123; buckets, end_index: _, bucket_size: _ &#125; &#61; v;<br/>    table_with_length::destroy_empty(buckets);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_destroy"></a>

## Function `destroy`

Destroy the vector <code>v</code> if T has <code>drop</code>


<pre><code>public fun destroy&lt;T: drop&gt;(v: big_vector::BigVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy&lt;T: drop&gt;(v: BigVector&lt;T&gt;) &#123;<br/>    let BigVector &#123; buckets, end_index, bucket_size: _ &#125; &#61; v;<br/>    let i &#61; 0;<br/>    while (end_index &gt; 0) &#123;<br/>        let num_elements &#61; vector::length(&amp;table_with_length::remove(&amp;mut buckets, i));<br/>        end_index &#61; end_index &#45; num_elements;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    table_with_length::destroy_empty(buckets);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>v</code>.<br/> Aborts if <code>i</code> is out of bounds.


<pre><code>public fun borrow&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, i: u64): &amp;T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;T&gt;(v: &amp;BigVector&lt;T&gt;, i: u64): &amp;T &#123;<br/>    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    vector::borrow(table_with_length::borrow(&amp;v.buckets, i / v.bucket_size), i % v.bucket_size)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>v</code>.<br/> Aborts if <code>i</code> is out of bounds.


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): &amp;mut T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64): &amp;mut T &#123;<br/>    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    vector::borrow_mut(table_with_length::borrow_mut(&amp;mut v.buckets, i / v.bucket_size), i % v.bucket_size)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the elements in the other vector onto the lhs vector in the<br/> same order as they occurred in other.<br/> Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut big_vector::BigVector&lt;T&gt;, other: big_vector::BigVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut BigVector&lt;T&gt;, other: BigVector&lt;T&gt;) &#123;<br/>    let other_len &#61; length(&amp;other);<br/>    let half_other_len &#61; other_len / 2;<br/>    let i &#61; 0;<br/>    while (i &lt; half_other_len) &#123;<br/>        push_back(lhs, swap_remove(&amp;mut other, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    while (i &lt; other_len) &#123;<br/>        push_back(lhs, pop_back(&amp;mut other));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    destroy_empty(other);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_push_back"></a>

## Function `push_back`

Add element <code>val</code> to the end of the vector <code>v</code>. It grows the buckets when the current buckets are full.<br/> This operation will cost more gas when it adds new bucket.


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, val: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut BigVector&lt;T&gt;, val: T) &#123;<br/>    let num_buckets &#61; table_with_length::length(&amp;v.buckets);<br/>    if (v.end_index &#61;&#61; num_buckets &#42; v.bucket_size) &#123;<br/>        table_with_length::add(&amp;mut v.buckets, num_buckets, vector::empty());<br/>        vector::push_back(table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets), val);<br/>    &#125; else &#123;<br/>        vector::push_back(table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets &#45; 1), val);<br/>    &#125;;<br/>    v.end_index &#61; v.end_index &#43; 1;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>v</code>. It doesn&apos;t shrink the buckets even if they&apos;re empty.<br/> Call <code>shrink_to_fit</code> explicity to deallocate empty buckets.<br/> Aborts if <code>v</code> is empty.


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;): T &#123;<br/>    assert!(!is_empty(v), error::invalid_state(EVECTOR_EMPTY));<br/>    let num_buckets &#61; table_with_length::length(&amp;v.buckets);<br/>    let last_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets &#45; 1);<br/>    let val &#61; vector::pop_back(last_bucket);<br/>    // Shrink the table if the last vector is empty.<br/>    if (vector::is_empty(last_bucket)) &#123;<br/>        move last_bucket;<br/>        vector::destroy_empty(table_with_length::remove(&amp;mut v.buckets, num_buckets &#45; 1));<br/>    &#125;;<br/>    v.end_index &#61; v.end_index &#45; 1;<br/>    val<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_remove"></a>

## Function `remove`

Remove the element at index i in the vector v and return the owned value that was previously stored at i in v.<br/> All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.<br/> Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64): T &#123;<br/>    let len &#61; length(v);<br/>    assert!(i &lt; len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let num_buckets &#61; table_with_length::length(&amp;v.buckets);<br/>    let cur_bucket_index &#61; i / v.bucket_size &#43; 1;<br/>    let cur_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, cur_bucket_index &#45; 1);<br/>    let res &#61; vector::remove(cur_bucket, i % v.bucket_size);<br/>    v.end_index &#61; v.end_index &#45; 1;<br/>    move cur_bucket;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant cur_bucket_index &lt;&#61; num_buckets;<br/>            invariant table_with_length::spec_len(v.buckets) &#61;&#61; num_buckets;<br/>        &#125;;<br/>        (cur_bucket_index &lt; num_buckets)<br/>    &#125;) &#123;<br/>        // remove one element from the start of current vector<br/>        let cur_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, cur_bucket_index);<br/>        let t &#61; vector::remove(cur_bucket, 0);<br/>        move cur_bucket;<br/>        // and put it at the end of the last one<br/>        let prev_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, cur_bucket_index &#45; 1);<br/>        vector::push_back(prev_bucket, t);<br/>        cur_bucket_index &#61; cur_bucket_index &#43; 1;<br/>    &#125;;<br/>    spec &#123;<br/>        assert cur_bucket_index &#61;&#61; num_buckets;<br/>    &#125;;<br/><br/>    // Shrink the table if the last vector is empty.<br/>    let last_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets &#45; 1);<br/>    if (vector::is_empty(last_bucket)) &#123;<br/>        move last_bucket;<br/>        vector::destroy_empty(table_with_length::remove(&amp;mut v.buckets, num_buckets &#45; 1));<br/>    &#125;;<br/><br/>    res<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>v</code> with the last element and then pop the vector.<br/> This is O(1), but does not preserve ordering of elements in the vector.<br/> Aborts if <code>i</code> is out of bounds.


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64): T &#123;<br/>    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let last_val &#61; pop_back(v);<br/>    // if the requested value is the last one, return it<br/>    if (v.end_index &#61;&#61; i) &#123;<br/>        return last_val<br/>    &#125;;<br/>    // because the lack of mem::swap, here we swap remove the requested value from the bucket<br/>    // and append the last_val to the bucket then swap the last bucket val back<br/>    let bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, i / v.bucket_size);<br/>    let bucket_len &#61; vector::length(bucket);<br/>    let val &#61; vector::swap_remove(bucket, i % v.bucket_size);<br/>    vector::push_back(bucket, last_val);<br/>    vector::swap(bucket, i % v.bucket_size, bucket_len &#45; 1);<br/>    val<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_swap"></a>

## Function `swap`

Swap the elements at the i&apos;th and j&apos;th indices in the vector v. Will abort if either of i or j are out of bounds<br/> for v.


<pre><code>public fun swap&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64, j: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64, j: u64) &#123;<br/>    assert!(i &lt; length(v) &amp;&amp; j &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let i_bucket_index &#61; i / v.bucket_size;<br/>    let j_bucket_index &#61; j / v.bucket_size;<br/>    let i_vector_index &#61; i % v.bucket_size;<br/>    let j_vector_index &#61; j % v.bucket_size;<br/>    if (i_bucket_index &#61;&#61; j_bucket_index) &#123;<br/>        vector::swap(table_with_length::borrow_mut(&amp;mut v.buckets, i_bucket_index), i_vector_index, j_vector_index);<br/>        return<br/>    &#125;;<br/>    // If i and j are in different buckets, take the buckets out first for easy mutation.<br/>    let bucket_i &#61; table_with_length::remove(&amp;mut v.buckets, i_bucket_index);<br/>    let bucket_j &#61; table_with_length::remove(&amp;mut v.buckets, j_bucket_index);<br/>    // Get the elements from buckets by calling `swap_remove`.<br/>    let element_i &#61; vector::swap_remove(&amp;mut bucket_i, i_vector_index);<br/>    let element_j &#61; vector::swap_remove(&amp;mut bucket_j, j_vector_index);<br/>    // Swap the elements and push back to the other bucket.<br/>    vector::push_back(&amp;mut bucket_i, element_j);<br/>    vector::push_back(&amp;mut bucket_j, element_i);<br/>    let last_index_in_bucket_i &#61; vector::length(&amp;bucket_i) &#45; 1;<br/>    let last_index_in_bucket_j &#61; vector::length(&amp;bucket_j) &#45; 1;<br/>    // Re&#45;position the swapped elements to the right index.<br/>    vector::swap(&amp;mut bucket_i, i_vector_index, last_index_in_bucket_i);<br/>    vector::swap(&amp;mut bucket_j, j_vector_index, last_index_in_bucket_j);<br/>    // Add back the buckets.<br/>    table_with_length::add(&amp;mut v.buckets, i_bucket_index, bucket_i);<br/>    table_with_length::add(&amp;mut v.buckets, j_bucket_index, bucket_j);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_reverse"></a>

## Function `reverse`

Reverse the order of the elements in the vector v in&#45;place.<br/> Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun reverse&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reverse&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;) &#123;<br/>    let new_buckets &#61; vector[];<br/>    let push_bucket &#61; vector[];<br/>    let num_buckets &#61; table_with_length::length(&amp;v.buckets);<br/>    let num_buckets_left &#61; num_buckets;<br/><br/>    while (num_buckets_left &gt; 0) &#123;<br/>        let pop_bucket &#61; table_with_length::remove(&amp;mut v.buckets, num_buckets_left &#45; 1);<br/>        vector::for_each_reverse(pop_bucket, &#124;val&#124; &#123;<br/>            vector::push_back(&amp;mut push_bucket, val);<br/>            if (vector::length(&amp;push_bucket) &#61;&#61; v.bucket_size) &#123;<br/>                vector::push_back(&amp;mut new_buckets, push_bucket);<br/>                push_bucket &#61; vector[];<br/>            &#125;;<br/>        &#125;);<br/>        num_buckets_left &#61; num_buckets_left &#45; 1;<br/>    &#125;;<br/><br/>    if (vector::length(&amp;push_bucket) &gt; 0) &#123;<br/>        vector::push_back(&amp;mut new_buckets, push_bucket);<br/>    &#125; else &#123;<br/>        vector::destroy_empty(push_bucket);<br/>    &#125;;<br/><br/>    vector::reverse(&amp;mut new_buckets);<br/>    let i &#61; 0;<br/>    assert!(table_with_length::length(&amp;v.buckets) &#61;&#61; 0, 0);<br/>    while (i &lt; num_buckets) &#123;<br/>        table_with_length::add(&amp;mut v.buckets, i, vector::pop_back(&amp;mut new_buckets));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    vector::destroy_empty(new_buckets);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_index_of"></a>

## Function `index_of`

Return the index of the first occurrence of an element in v that is equal to e. Returns (true, index) if such an<br/> element was found, and (false, 0) otherwise.<br/> Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun index_of&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, val: &amp;T): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun index_of&lt;T&gt;(v: &amp;BigVector&lt;T&gt;, val: &amp;T): (bool, u64) &#123;<br/>    let num_buckets &#61; table_with_length::length(&amp;v.buckets);<br/>    let bucket_index &#61; 0;<br/>    while (bucket_index &lt; num_buckets) &#123;<br/>        let cur &#61; table_with_length::borrow(&amp;v.buckets, bucket_index);<br/>        let (found, i) &#61; vector::index_of(cur, val);<br/>        if (found) &#123;<br/>            return (true, bucket_index &#42; v.bucket_size &#43; i)<br/>        &#125;;<br/>        bucket_index &#61; bucket_index &#43; 1;<br/>    &#125;;<br/>    (false, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_contains"></a>

## Function `contains`

Return if an element equal to e exists in the vector v.<br/> Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun contains&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, val: &amp;T): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;T&gt;(v: &amp;BigVector&lt;T&gt;, val: &amp;T): bool &#123;<br/>    if (is_empty(v)) return false;<br/>    let (exist, _) &#61; index_of(v, val);<br/>    exist<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_to_vector"></a>

## Function `to_vector`

Convert a big vector to a native vector, which is supposed to be called mostly by view functions to get an<br/> atomic view of the whole vector.<br/> Disclaimer: This function may be costly as the big vector may be huge in size. Use it at your own discretion.


<pre><code>public fun to_vector&lt;T: copy&gt;(v: &amp;big_vector::BigVector&lt;T&gt;): vector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_vector&lt;T: copy&gt;(v: &amp;BigVector&lt;T&gt;): vector&lt;T&gt; &#123;<br/>    let res &#61; vector[];<br/>    let num_buckets &#61; table_with_length::length(&amp;v.buckets);<br/>    let i &#61; 0;<br/>    while (i &lt; num_buckets) &#123;<br/>        vector::append(&amp;mut res, &#42;table_with_length::borrow(&amp;v.buckets, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    res<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code>public fun length&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;T&gt;(v: &amp;BigVector&lt;T&gt;): u64 &#123;<br/>    v.end_index<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_big_vector_is_empty"></a>

## Function `is_empty`

Return <code>true</code> if the vector <code>v</code> has no elements and <code>false</code> otherwise.


<pre><code>public fun is_empty&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_empty&lt;T&gt;(v: &amp;BigVector&lt;T&gt;): bool &#123;<br/>    length(v) &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_BigVector"></a>

### Struct `BigVector`


<pre><code>struct BigVector&lt;T&gt; has store<br/></code></pre>



<dl>
<dt>
<code>buckets: table_with_length::TableWithLength&lt;u64, vector&lt;T&gt;&gt;</code>
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



<pre><code>invariant bucket_size !&#61; 0;<br/>invariant spec_table_len(buckets) &#61;&#61; 0 &#61;&#61;&gt; end_index &#61;&#61; 0;<br/>invariant end_index &#61;&#61; 0 &#61;&#61;&gt; spec_table_len(buckets) &#61;&#61; 0;<br/>invariant end_index &lt;&#61; spec_table_len(buckets) &#42; bucket_size;<br/>invariant spec_table_len(buckets) &#61;&#61; 0<br/>    &#124;&#124; (forall i in 0..spec_table_len(buckets)&#45;1: len(table_with_length::spec_get(buckets, i)) &#61;&#61; bucket_size);<br/>invariant spec_table_len(buckets) &#61;&#61; 0<br/>    &#124;&#124; len(table_with_length::spec_get(buckets, spec_table_len(buckets) &#45;1 )) &lt;&#61; bucket_size;<br/>invariant forall i in 0..spec_table_len(buckets): spec_table_contains(buckets, i);<br/>invariant spec_table_len(buckets) &#61;&#61; (end_index &#43; bucket_size &#45; 1) / bucket_size;<br/>invariant (spec_table_len(buckets) &#61;&#61; 0 &amp;&amp; end_index &#61;&#61; 0)<br/>    &#124;&#124; (spec_table_len(buckets) !&#61; 0 &amp;&amp; ((spec_table_len(buckets) &#45; 1) &#42; bucket_size) &#43; (len(table_with_length::spec_get(buckets, spec_table_len(buckets) &#45; 1))) &#61;&#61; end_index);<br/>invariant forall i: u64 where i &gt;&#61; spec_table_len(buckets):  &#123;<br/>    !spec_table_contains(buckets, i)<br/>&#125;;<br/>invariant forall i: u64 where i &lt; spec_table_len(buckets):  &#123;<br/>    spec_table_contains(buckets, i)<br/>&#125;;<br/>invariant spec_table_len(buckets) &#61;&#61; 0<br/>    &#124;&#124; (len(table_with_length::spec_get(buckets, spec_table_len(buckets) &#45; 1)) &gt; 0);<br/></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>public(friend) fun empty&lt;T: store&gt;(bucket_size: u64): big_vector::BigVector&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if bucket_size &#61;&#61; 0;<br/>ensures length(result) &#61;&#61; 0;<br/>ensures result.bucket_size &#61;&#61; bucket_size;<br/></code></pre>



<a id="@Specification_1_singleton"></a>

### Function `singleton`


<pre><code>public(friend) fun singleton&lt;T: store&gt;(element: T, bucket_size: u64): big_vector::BigVector&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if bucket_size &#61;&#61; 0;<br/>ensures length(result) &#61;&#61; 1;<br/>ensures result.bucket_size &#61;&#61; bucket_size;<br/></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code>public fun destroy_empty&lt;T&gt;(v: big_vector::BigVector&lt;T&gt;)<br/></code></pre>




<pre><code>aborts_if !is_empty(v);<br/></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, i: u64): &amp;T<br/></code></pre>




<pre><code>aborts_if i &gt;&#61; length(v);<br/>ensures result &#61;&#61; spec_at(v, i);<br/></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): &amp;mut T<br/></code></pre>




<pre><code>aborts_if i &gt;&#61; length(v);<br/>ensures result &#61;&#61; spec_at(v, i);<br/></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut big_vector::BigVector&lt;T&gt;, other: big_vector::BigVector&lt;T&gt;)<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, val: T)<br/></code></pre>




<pre><code>let num_buckets &#61; spec_table_len(v.buckets);<br/>include PushbackAbortsIf&lt;T&gt;;<br/>ensures length(v) &#61;&#61; length(old(v)) &#43; 1;<br/>ensures v.end_index &#61;&#61; old(v.end_index) &#43; 1;<br/>ensures spec_at(v, v.end_index&#45;1) &#61;&#61; val;<br/>ensures forall i in 0..v.end_index&#45;1: spec_at(v, i) &#61;&#61; spec_at(old(v), i);<br/>ensures v.bucket_size &#61;&#61; old(v).bucket_size;<br/></code></pre>




<a id="0x1_big_vector_PushbackAbortsIf"></a>


<pre><code>schema PushbackAbortsIf&lt;T&gt; &#123;<br/>v: BigVector&lt;T&gt;;<br/>let num_buckets &#61; spec_table_len(v.buckets);<br/>aborts_if num_buckets &#42; v.bucket_size &gt; MAX_U64;<br/>aborts_if v.end_index &#43; 1 &gt; MAX_U64;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;): T<br/></code></pre>




<pre><code>aborts_if is_empty(v);<br/>ensures length(v) &#61;&#61; length(old(v)) &#45; 1;<br/>ensures result &#61;&#61; old(spec_at(v, v.end_index&#45;1));<br/>ensures forall i in 0..v.end_index: spec_at(v, i) &#61;&#61; spec_at(old(v), i);<br/></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>aborts_if i &gt;&#61; length(v);<br/>ensures length(v) &#61;&#61; length(old(v)) &#45; 1;<br/>ensures result &#61;&#61; spec_at(old(v), i);<br/></code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code>public fun swap&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64, j: u64)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;<br/>aborts_if i &gt;&#61; length(v) &#124;&#124; j &gt;&#61; length(v);<br/>ensures length(v) &#61;&#61; length(old(v));<br/>ensures spec_at(v, i) &#61;&#61; spec_at(old(v), j);<br/>ensures spec_at(v, j) &#61;&#61; spec_at(old(v), i);<br/>ensures forall idx in 0..length(v)<br/>    where idx !&#61; i &amp;&amp; idx !&#61; j:<br/>    spec_at(v, idx) &#61;&#61; spec_at(old(v), idx);<br/></code></pre>



<a id="@Specification_1_reverse"></a>

### Function `reverse`


<pre><code>public fun reverse&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;)<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>



<a id="@Specification_1_index_of"></a>

### Function `index_of`


<pre><code>public fun index_of&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, val: &amp;T): (bool, u64)<br/></code></pre>




<pre><code>pragma verify&#61;false;<br/></code></pre>




<a id="0x1_big_vector_spec_table_len"></a>


<pre><code>fun spec_table_len&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;): u64 &#123;<br/>   table_with_length::spec_len(t)<br/>&#125;<br/></code></pre>




<a id="0x1_big_vector_spec_table_contains"></a>


<pre><code>fun spec_table_contains&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): bool &#123;<br/>   table_with_length::spec_contains(t, k)<br/>&#125;<br/></code></pre>




<a id="0x1_big_vector_spec_at"></a>


<pre><code>fun spec_at&lt;T&gt;(v: BigVector&lt;T&gt;, i: u64): T &#123;<br/>   let bucket &#61; i / v.bucket_size;<br/>   let idx &#61; i % v.bucket_size;<br/>   let v &#61; table_with_length::spec_get(v.buckets, bucket);<br/>   v[idx]<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
