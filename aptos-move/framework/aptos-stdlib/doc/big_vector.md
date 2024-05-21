
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


<pre><code>use 0x1::error;
use 0x1::table_with_length;
use 0x1::vector;
</code></pre>



<a id="0x1_big_vector_BigVector"></a>

## Struct `BigVector`

A scalable vector implementation based on tables where elements are grouped into buckets.
Each bucket has a capacity of <code>bucket_size</code> elements.


<pre><code>struct BigVector&lt;T&gt; has store
</code></pre>



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


<pre><code>const EINDEX_OUT_OF_BOUNDS: u64 &#61; 1;
</code></pre>



<a id="0x1_big_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code>const EVECTOR_EMPTY: u64 &#61; 3;
</code></pre>



<a id="0x1_big_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non-empty vector


<pre><code>const EVECTOR_NOT_EMPTY: u64 &#61; 2;
</code></pre>



<a id="0x1_big_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code>const EZERO_BUCKET_SIZE: u64 &#61; 4;
</code></pre>



<a id="0x1_big_vector_empty"></a>

## Function `empty`

Regular Vector API
Create an empty vector.


<pre><code>public(friend) fun empty&lt;T: store&gt;(bucket_size: u64): big_vector::BigVector&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun empty&lt;T: store&gt;(bucket_size: u64): BigVector&lt;T&gt; &#123;
    assert!(bucket_size &gt; 0, error::invalid_argument(EZERO_BUCKET_SIZE));
    BigVector &#123;
        buckets: table_with_length::new(),
        end_index: 0,
        bucket_size,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in element.


<pre><code>public(friend) fun singleton&lt;T: store&gt;(element: T, bucket_size: u64): big_vector::BigVector&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun singleton&lt;T: store&gt;(element: T, bucket_size: u64): BigVector&lt;T&gt; &#123;
    let v &#61; empty(bucket_size);
    push_back(&amp;mut v, element);
    v
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code>public fun destroy_empty&lt;T&gt;(v: big_vector::BigVector&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;T&gt;(v: BigVector&lt;T&gt;) &#123;
    assert!(is_empty(&amp;v), error::invalid_argument(EVECTOR_NOT_EMPTY));
    let BigVector &#123; buckets, end_index: _, bucket_size: _ &#125; &#61; v;
    table_with_length::destroy_empty(buckets);
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_destroy"></a>

## Function `destroy`

Destroy the vector <code>v</code> if T has <code>drop</code>


<pre><code>public fun destroy&lt;T: drop&gt;(v: big_vector::BigVector&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy&lt;T: drop&gt;(v: BigVector&lt;T&gt;) &#123;
    let BigVector &#123; buckets, end_index, bucket_size: _ &#125; &#61; v;
    let i &#61; 0;
    while (end_index &gt; 0) &#123;
        let num_elements &#61; vector::length(&amp;table_with_length::remove(&amp;mut buckets, i));
        end_index &#61; end_index &#45; num_elements;
        i &#61; i &#43; 1;
    &#125;;
    table_with_length::destroy_empty(buckets);
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun borrow&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, i: u64): &amp;T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;T&gt;(v: &amp;BigVector&lt;T&gt;, i: u64): &amp;T &#123;
    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
    vector::borrow(table_with_length::borrow(&amp;v.buckets, i / v.bucket_size), i % v.bucket_size)
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): &amp;mut T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64): &amp;mut T &#123;
    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
    vector::borrow_mut(table_with_length::borrow_mut(&amp;mut v.buckets, i / v.bucket_size), i % v.bucket_size)
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the elements in the other vector onto the lhs vector in the
same order as they occurred in other.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut big_vector::BigVector&lt;T&gt;, other: big_vector::BigVector&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut BigVector&lt;T&gt;, other: BigVector&lt;T&gt;) &#123;
    let other_len &#61; length(&amp;other);
    let half_other_len &#61; other_len / 2;
    let i &#61; 0;
    while (i &lt; half_other_len) &#123;
        push_back(lhs, swap_remove(&amp;mut other, i));
        i &#61; i &#43; 1;
    &#125;;
    while (i &lt; other_len) &#123;
        push_back(lhs, pop_back(&amp;mut other));
        i &#61; i &#43; 1;
    &#125;;
    destroy_empty(other);
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_push_back"></a>

## Function `push_back`

Add element <code>val</code> to the end of the vector <code>v</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, val: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut BigVector&lt;T&gt;, val: T) &#123;
    let num_buckets &#61; table_with_length::length(&amp;v.buckets);
    if (v.end_index &#61;&#61; num_buckets &#42; v.bucket_size) &#123;
        table_with_length::add(&amp;mut v.buckets, num_buckets, vector::empty());
        vector::push_back(table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets), val);
    &#125; else &#123;
        vector::push_back(table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets &#45; 1), val);
    &#125;;
    v.end_index &#61; v.end_index &#43; 1;
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>v</code>. It doesn't shrink the buckets even if they're empty.
Call <code>shrink_to_fit</code> explicity to deallocate empty buckets.
Aborts if <code>v</code> is empty.


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;): T &#123;
    assert!(!is_empty(v), error::invalid_state(EVECTOR_EMPTY));
    let num_buckets &#61; table_with_length::length(&amp;v.buckets);
    let last_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets &#45; 1);
    let val &#61; vector::pop_back(last_bucket);
    // Shrink the table if the last vector is empty.
    if (vector::is_empty(last_bucket)) &#123;
        move last_bucket;
        vector::destroy_empty(table_with_length::remove(&amp;mut v.buckets, num_buckets &#45; 1));
    &#125;;
    v.end_index &#61; v.end_index &#45; 1;
    val
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_remove"></a>

## Function `remove`

Remove the element at index i in the vector v and return the owned value that was previously stored at i in v.
All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64): T &#123;
    let len &#61; length(v);
    assert!(i &lt; len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
    let num_buckets &#61; table_with_length::length(&amp;v.buckets);
    let cur_bucket_index &#61; i / v.bucket_size &#43; 1;
    let cur_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, cur_bucket_index &#45; 1);
    let res &#61; vector::remove(cur_bucket, i % v.bucket_size);
    v.end_index &#61; v.end_index &#45; 1;
    move cur_bucket;
    while (&#123;
        spec &#123;
            invariant cur_bucket_index &lt;&#61; num_buckets;
            invariant table_with_length::spec_len(v.buckets) &#61;&#61; num_buckets;
        &#125;;
        (cur_bucket_index &lt; num_buckets)
    &#125;) &#123;
        // remove one element from the start of current vector
        let cur_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, cur_bucket_index);
        let t &#61; vector::remove(cur_bucket, 0);
        move cur_bucket;
        // and put it at the end of the last one
        let prev_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, cur_bucket_index &#45; 1);
        vector::push_back(prev_bucket, t);
        cur_bucket_index &#61; cur_bucket_index &#43; 1;
    &#125;;
    spec &#123;
        assert cur_bucket_index &#61;&#61; num_buckets;
    &#125;;

    // Shrink the table if the last vector is empty.
    let last_bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, num_buckets &#45; 1);
    if (vector::is_empty(last_bucket)) &#123;
        move last_bucket;
        vector::destroy_empty(table_with_length::remove(&amp;mut v.buckets, num_buckets &#45; 1));
    &#125;;

    res
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>v</code> with the last element and then pop the vector.
This is O(1), but does not preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64): T &#123;
    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
    let last_val &#61; pop_back(v);
    // if the requested value is the last one, return it
    if (v.end_index &#61;&#61; i) &#123;
        return last_val
    &#125;;
    // because the lack of mem::swap, here we swap remove the requested value from the bucket
    // and append the last_val to the bucket then swap the last bucket val back
    let bucket &#61; table_with_length::borrow_mut(&amp;mut v.buckets, i / v.bucket_size);
    let bucket_len &#61; vector::length(bucket);
    let val &#61; vector::swap_remove(bucket, i % v.bucket_size);
    vector::push_back(bucket, last_val);
    vector::swap(bucket, i % v.bucket_size, bucket_len &#45; 1);
    val
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_swap"></a>

## Function `swap`

Swap the elements at the i'th and j'th indices in the vector v. Will abort if either of i or j are out of bounds
for v.


<pre><code>public fun swap&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64, j: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;, i: u64, j: u64) &#123;
    assert!(i &lt; length(v) &amp;&amp; j &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));
    let i_bucket_index &#61; i / v.bucket_size;
    let j_bucket_index &#61; j / v.bucket_size;
    let i_vector_index &#61; i % v.bucket_size;
    let j_vector_index &#61; j % v.bucket_size;
    if (i_bucket_index &#61;&#61; j_bucket_index) &#123;
        vector::swap(table_with_length::borrow_mut(&amp;mut v.buckets, i_bucket_index), i_vector_index, j_vector_index);
        return
    &#125;;
    // If i and j are in different buckets, take the buckets out first for easy mutation.
    let bucket_i &#61; table_with_length::remove(&amp;mut v.buckets, i_bucket_index);
    let bucket_j &#61; table_with_length::remove(&amp;mut v.buckets, j_bucket_index);
    // Get the elements from buckets by calling `swap_remove`.
    let element_i &#61; vector::swap_remove(&amp;mut bucket_i, i_vector_index);
    let element_j &#61; vector::swap_remove(&amp;mut bucket_j, j_vector_index);
    // Swap the elements and push back to the other bucket.
    vector::push_back(&amp;mut bucket_i, element_j);
    vector::push_back(&amp;mut bucket_j, element_i);
    let last_index_in_bucket_i &#61; vector::length(&amp;bucket_i) &#45; 1;
    let last_index_in_bucket_j &#61; vector::length(&amp;bucket_j) &#45; 1;
    // Re&#45;position the swapped elements to the right index.
    vector::swap(&amp;mut bucket_i, i_vector_index, last_index_in_bucket_i);
    vector::swap(&amp;mut bucket_j, j_vector_index, last_index_in_bucket_j);
    // Add back the buckets.
    table_with_length::add(&amp;mut v.buckets, i_bucket_index, bucket_i);
    table_with_length::add(&amp;mut v.buckets, j_bucket_index, bucket_j);
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_reverse"></a>

## Function `reverse`

Reverse the order of the elements in the vector v in-place.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun reverse&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reverse&lt;T&gt;(v: &amp;mut BigVector&lt;T&gt;) &#123;
    let new_buckets &#61; vector[];
    let push_bucket &#61; vector[];
    let num_buckets &#61; table_with_length::length(&amp;v.buckets);
    let num_buckets_left &#61; num_buckets;

    while (num_buckets_left &gt; 0) &#123;
        let pop_bucket &#61; table_with_length::remove(&amp;mut v.buckets, num_buckets_left &#45; 1);
        vector::for_each_reverse(pop_bucket, &#124;val&#124; &#123;
            vector::push_back(&amp;mut push_bucket, val);
            if (vector::length(&amp;push_bucket) &#61;&#61; v.bucket_size) &#123;
                vector::push_back(&amp;mut new_buckets, push_bucket);
                push_bucket &#61; vector[];
            &#125;;
        &#125;);
        num_buckets_left &#61; num_buckets_left &#45; 1;
    &#125;;

    if (vector::length(&amp;push_bucket) &gt; 0) &#123;
        vector::push_back(&amp;mut new_buckets, push_bucket);
    &#125; else &#123;
        vector::destroy_empty(push_bucket);
    &#125;;

    vector::reverse(&amp;mut new_buckets);
    let i &#61; 0;
    assert!(table_with_length::length(&amp;v.buckets) &#61;&#61; 0, 0);
    while (i &lt; num_buckets) &#123;
        table_with_length::add(&amp;mut v.buckets, i, vector::pop_back(&amp;mut new_buckets));
        i &#61; i &#43; 1;
    &#125;;
    vector::destroy_empty(new_buckets);
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_index_of"></a>

## Function `index_of`

Return the index of the first occurrence of an element in v that is equal to e. Returns (true, index) if such an
element was found, and (false, 0) otherwise.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun index_of&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, val: &amp;T): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun index_of&lt;T&gt;(v: &amp;BigVector&lt;T&gt;, val: &amp;T): (bool, u64) &#123;
    let num_buckets &#61; table_with_length::length(&amp;v.buckets);
    let bucket_index &#61; 0;
    while (bucket_index &lt; num_buckets) &#123;
        let cur &#61; table_with_length::borrow(&amp;v.buckets, bucket_index);
        let (found, i) &#61; vector::index_of(cur, val);
        if (found) &#123;
            return (true, bucket_index &#42; v.bucket_size &#43; i)
        &#125;;
        bucket_index &#61; bucket_index &#43; 1;
    &#125;;
    (false, 0)
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_contains"></a>

## Function `contains`

Return if an element equal to e exists in the vector v.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code>public fun contains&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, val: &amp;T): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;T&gt;(v: &amp;BigVector&lt;T&gt;, val: &amp;T): bool &#123;
    if (is_empty(v)) return false;
    let (exist, _) &#61; index_of(v, val);
    exist
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_to_vector"></a>

## Function `to_vector`

Convert a big vector to a native vector, which is supposed to be called mostly by view functions to get an
atomic view of the whole vector.
Disclaimer: This function may be costly as the big vector may be huge in size. Use it at your own discretion.


<pre><code>public fun to_vector&lt;T: copy&gt;(v: &amp;big_vector::BigVector&lt;T&gt;): vector&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_vector&lt;T: copy&gt;(v: &amp;BigVector&lt;T&gt;): vector&lt;T&gt; &#123;
    let res &#61; vector[];
    let num_buckets &#61; table_with_length::length(&amp;v.buckets);
    let i &#61; 0;
    while (i &lt; num_buckets) &#123;
        vector::append(&amp;mut res, &#42;table_with_length::borrow(&amp;v.buckets, i));
        i &#61; i &#43; 1;
    &#125;;
    res
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code>public fun length&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;T&gt;(v: &amp;BigVector&lt;T&gt;): u64 &#123;
    v.end_index
&#125;
</code></pre>



</details>

<a id="0x1_big_vector_is_empty"></a>

## Function `is_empty`

Return <code>true</code> if the vector <code>v</code> has no elements and <code>false</code> otherwise.


<pre><code>public fun is_empty&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_empty&lt;T&gt;(v: &amp;BigVector&lt;T&gt;): bool &#123;
    length(v) &#61;&#61; 0
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_BigVector"></a>

### Struct `BigVector`


<pre><code>struct BigVector&lt;T&gt; has store
</code></pre>



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



<pre><code>invariant bucket_size !&#61; 0;
invariant spec_table_len(buckets) &#61;&#61; 0 &#61;&#61;&gt; end_index &#61;&#61; 0;
invariant end_index &#61;&#61; 0 &#61;&#61;&gt; spec_table_len(buckets) &#61;&#61; 0;
invariant end_index &lt;&#61; spec_table_len(buckets) &#42; bucket_size;
invariant spec_table_len(buckets) &#61;&#61; 0
    &#124;&#124; (forall i in 0..spec_table_len(buckets)&#45;1: len(table_with_length::spec_get(buckets, i)) &#61;&#61; bucket_size);
invariant spec_table_len(buckets) &#61;&#61; 0
    &#124;&#124; len(table_with_length::spec_get(buckets, spec_table_len(buckets) &#45;1 )) &lt;&#61; bucket_size;
invariant forall i in 0..spec_table_len(buckets): spec_table_contains(buckets, i);
invariant spec_table_len(buckets) &#61;&#61; (end_index &#43; bucket_size &#45; 1) / bucket_size;
invariant (spec_table_len(buckets) &#61;&#61; 0 &amp;&amp; end_index &#61;&#61; 0)
    &#124;&#124; (spec_table_len(buckets) !&#61; 0 &amp;&amp; ((spec_table_len(buckets) &#45; 1) &#42; bucket_size) &#43; (len(table_with_length::spec_get(buckets, spec_table_len(buckets) &#45; 1))) &#61;&#61; end_index);
invariant forall i: u64 where i &gt;&#61; spec_table_len(buckets):  &#123;
    !spec_table_contains(buckets, i)
&#125;;
invariant forall i: u64 where i &lt; spec_table_len(buckets):  &#123;
    spec_table_contains(buckets, i)
&#125;;
invariant spec_table_len(buckets) &#61;&#61; 0
    &#124;&#124; (len(table_with_length::spec_get(buckets, spec_table_len(buckets) &#45; 1)) &gt; 0);
</code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>public(friend) fun empty&lt;T: store&gt;(bucket_size: u64): big_vector::BigVector&lt;T&gt;
</code></pre>




<pre><code>aborts_if bucket_size &#61;&#61; 0;
ensures length(result) &#61;&#61; 0;
ensures result.bucket_size &#61;&#61; bucket_size;
</code></pre>



<a id="@Specification_1_singleton"></a>

### Function `singleton`


<pre><code>public(friend) fun singleton&lt;T: store&gt;(element: T, bucket_size: u64): big_vector::BigVector&lt;T&gt;
</code></pre>




<pre><code>aborts_if bucket_size &#61;&#61; 0;
ensures length(result) &#61;&#61; 1;
ensures result.bucket_size &#61;&#61; bucket_size;
</code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code>public fun destroy_empty&lt;T&gt;(v: big_vector::BigVector&lt;T&gt;)
</code></pre>




<pre><code>aborts_if !is_empty(v);
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, i: u64): &amp;T
</code></pre>




<pre><code>aborts_if i &gt;&#61; length(v);
ensures result &#61;&#61; spec_at(v, i);
</code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): &amp;mut T
</code></pre>




<pre><code>aborts_if i &gt;&#61; length(v);
ensures result &#61;&#61; spec_at(v, i);
</code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut big_vector::BigVector&lt;T&gt;, other: big_vector::BigVector&lt;T&gt;)
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, val: T)
</code></pre>




<pre><code>let num_buckets &#61; spec_table_len(v.buckets);
include PushbackAbortsIf&lt;T&gt;;
ensures length(v) &#61;&#61; length(old(v)) &#43; 1;
ensures v.end_index &#61;&#61; old(v.end_index) &#43; 1;
ensures spec_at(v, v.end_index&#45;1) &#61;&#61; val;
ensures forall i in 0..v.end_index&#45;1: spec_at(v, i) &#61;&#61; spec_at(old(v), i);
ensures v.bucket_size &#61;&#61; old(v).bucket_size;
</code></pre>




<a id="0x1_big_vector_PushbackAbortsIf"></a>


<pre><code>schema PushbackAbortsIf&lt;T&gt; &#123;
    v: BigVector&lt;T&gt;;
    let num_buckets &#61; spec_table_len(v.buckets);
    aborts_if num_buckets &#42; v.bucket_size &gt; MAX_U64;
    aborts_if v.end_index &#43; 1 &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;): T
</code></pre>




<pre><code>aborts_if is_empty(v);
ensures length(v) &#61;&#61; length(old(v)) &#45; 1;
ensures result &#61;&#61; old(spec_at(v, v.end_index&#45;1));
ensures forall i in 0..v.end_index: spec_at(v, i) &#61;&#61; spec_at(old(v), i);
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64): T
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
aborts_if i &gt;&#61; length(v);
ensures length(v) &#61;&#61; length(old(v)) &#45; 1;
ensures result &#61;&#61; spec_at(old(v), i);
</code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code>public fun swap&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;, i: u64, j: u64)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1000;
aborts_if i &gt;&#61; length(v) &#124;&#124; j &gt;&#61; length(v);
ensures length(v) &#61;&#61; length(old(v));
ensures spec_at(v, i) &#61;&#61; spec_at(old(v), j);
ensures spec_at(v, j) &#61;&#61; spec_at(old(v), i);
ensures forall idx in 0..length(v)
    where idx !&#61; i &amp;&amp; idx !&#61; j:
    spec_at(v, idx) &#61;&#61; spec_at(old(v), idx);
</code></pre>



<a id="@Specification_1_reverse"></a>

### Function `reverse`


<pre><code>public fun reverse&lt;T&gt;(v: &amp;mut big_vector::BigVector&lt;T&gt;)
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>



<a id="@Specification_1_index_of"></a>

### Function `index_of`


<pre><code>public fun index_of&lt;T&gt;(v: &amp;big_vector::BigVector&lt;T&gt;, val: &amp;T): (bool, u64)
</code></pre>




<pre><code>pragma verify&#61;false;
</code></pre>




<a id="0x1_big_vector_spec_table_len"></a>


<pre><code>fun spec_table_len&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;): u64 &#123;
   table_with_length::spec_len(t)
&#125;
</code></pre>




<a id="0x1_big_vector_spec_table_contains"></a>


<pre><code>fun spec_table_contains&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): bool &#123;
   table_with_length::spec_contains(t, k)
&#125;
</code></pre>




<a id="0x1_big_vector_spec_at"></a>


<pre><code>fun spec_at&lt;T&gt;(v: BigVector&lt;T&gt;, i: u64): T &#123;
   let bucket &#61; i / v.bucket_size;
   let idx &#61; i % v.bucket_size;
   let v &#61; table_with_length::spec_get(v.buckets, bucket);
   v[idx]
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
