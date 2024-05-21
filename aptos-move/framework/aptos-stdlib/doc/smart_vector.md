
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


<pre><code>use 0x1::big_vector;<br/>use 0x1::error;<br/>use 0x1::math64;<br/>use 0x1::option;<br/>use 0x1::type_info;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_smart_vector_SmartVector"></a>

## Struct `SmartVector`

A Scalable vector implementation based on tables, Ts are grouped into buckets with <code>bucket_size</code>.
The option wrapping BigVector saves space in the metadata associated with BigVector when smart_vector is
so small that inline_vec vector can hold all the data.


<pre><code>struct SmartVector&lt;T&gt; has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inline_vec: vector&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>big_vec: option::Option&lt;big_vector::BigVector&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inline_capacity: option::Option&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: option::Option&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_smart_vector_EINDEX_OUT_OF_BOUNDS"></a>

Vector index is out of bounds


<pre><code>const EINDEX_OUT_OF_BOUNDS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_smart_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code>const EVECTOR_EMPTY: u64 &#61; 3;<br/></code></pre>



<a id="0x1_smart_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non-empty vector


<pre><code>const EVECTOR_NOT_EMPTY: u64 &#61; 2;<br/></code></pre>



<a id="0x1_smart_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code>const EZERO_BUCKET_SIZE: u64 &#61; 4;<br/></code></pre>



<a id="0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH"></a>

The length of the smart vectors are not equal.


<pre><code>const ESMART_VECTORS_LENGTH_MISMATCH: u64 &#61; 131077;<br/></code></pre>



<a id="0x1_smart_vector_new"></a>

## Function `new`

Regular Vector API
Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.
This is exactly the same as empty() but is more standardized as all other data structures have new().


<pre><code>public fun new&lt;T: store&gt;(): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;T: store&gt;(): SmartVector&lt;T&gt; &#123;<br/>    empty()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_empty"></a>

## Function `empty`

Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.


<pre><code>&#35;[deprecated]<br/>public fun empty&lt;T: store&gt;(): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun empty&lt;T: store&gt;(): SmartVector&lt;T&gt; &#123;<br/>    SmartVector &#123;<br/>        inline_vec: vector[],<br/>        big_vec: option::none(),<br/>        inline_capacity: option::none(),<br/>        bucket_size: option::none(),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_empty_with_config"></a>

## Function `empty_with_config`

Create an empty vector with customized config.
When inline_capacity = 0, SmartVector degrades to a wrapper of BigVector.


<pre><code>public fun empty_with_config&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun empty_with_config&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): SmartVector&lt;T&gt; &#123;<br/>    assert!(bucket_size &gt; 0, error::invalid_argument(EZERO_BUCKET_SIZE));<br/>    SmartVector &#123;<br/>        inline_vec: vector[],<br/>        big_vec: option::none(),<br/>        inline_capacity: option::some(inline_capacity),<br/>        bucket_size: option::some(bucket_size),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in T.


<pre><code>public fun singleton&lt;T: store&gt;(element: T): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun singleton&lt;T: store&gt;(element: T): SmartVector&lt;T&gt; &#123;<br/>    let v &#61; empty();<br/>    push_back(&amp;mut v, element);<br/>    v<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code>public fun destroy_empty&lt;T&gt;(v: smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;T&gt;(v: SmartVector&lt;T&gt;) &#123;<br/>    assert!(is_empty(&amp;v), error::invalid_argument(EVECTOR_NOT_EMPTY));<br/>    let SmartVector &#123; inline_vec, big_vec, inline_capacity: _, bucket_size: _ &#125; &#61; v;<br/>    vector::destroy_empty(inline_vec);<br/>    option::destroy_none(big_vec);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_destroy"></a>

## Function `destroy`

Destroy a vector completely when T has <code>drop</code>.


<pre><code>public fun destroy&lt;T: drop&gt;(v: smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy&lt;T: drop&gt;(v: SmartVector&lt;T&gt;) &#123;<br/>    clear(&amp;mut v);<br/>    destroy_empty(v);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_clear"></a>

## Function `clear`

Clear a vector completely when T has <code>drop</code>.


<pre><code>public fun clear&lt;T: drop&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun clear&lt;T: drop&gt;(v: &amp;mut SmartVector&lt;T&gt;) &#123;<br/>    v.inline_vec &#61; vector[];<br/>    if (option::is_some(&amp;v.big_vec)) &#123;<br/>        big_vector::destroy(option::extract(&amp;mut v.big_vec));<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th T of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun borrow&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;, i: u64): &amp;T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;, i: u64): &amp;T &#123;<br/>    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    if (i &lt; inline_len) &#123;<br/>        vector::borrow(&amp;v.inline_vec, i)<br/>    &#125; else &#123;<br/>        big_vector::borrow(option::borrow(&amp;v.big_vec), i &#45; inline_len)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th T in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64): &amp;mut T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;T&gt;(v: &amp;mut SmartVector&lt;T&gt;, i: u64): &amp;mut T &#123;<br/>    assert!(i &lt; length(v), error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    if (i &lt; inline_len) &#123;<br/>        vector::borrow_mut(&amp;mut v.inline_vec, i)<br/>    &#125; else &#123;<br/>        big_vector::borrow_mut(option::borrow_mut(&amp;mut v.big_vec), i &#45; inline_len)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the Ts in the other vector onto the lhs vector in the
same order as they occurred in other.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut smart_vector::SmartVector&lt;T&gt;, other: smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut SmartVector&lt;T&gt;, other: SmartVector&lt;T&gt;) &#123;<br/>    let other_len &#61; length(&amp;other);<br/>    let half_other_len &#61; other_len / 2;<br/>    let i &#61; 0;<br/>    while (i &lt; half_other_len) &#123;<br/>        push_back(lhs, swap_remove(&amp;mut other, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    while (i &lt; other_len) &#123;<br/>        push_back(lhs, pop_back(&amp;mut other));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    destroy_empty(other);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_add_all"></a>

## Function `add_all`

Add multiple values to the vector at once.


<pre><code>public fun add_all&lt;T: store&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, vals: vector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_all&lt;T: store&gt;(v: &amp;mut SmartVector&lt;T&gt;, vals: vector&lt;T&gt;) &#123;<br/>    vector::for_each(vals, &#124;val&#124; &#123; push_back(v, val); &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_to_vector"></a>

## Function `to_vector`

Convert a smart vector to a native vector, which is supposed to be called mostly by view functions to get an
atomic view of the whole vector.
Disclaimer: This function may be costly as the smart vector may be huge in size. Use it at your own discretion.


<pre><code>public fun to_vector&lt;T: copy, store&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;): vector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_vector&lt;T: store &#43; copy&gt;(v: &amp;SmartVector&lt;T&gt;): vector&lt;T&gt; &#123;<br/>    let res &#61; v.inline_vec;<br/>    if (option::is_some(&amp;v.big_vec)) &#123;<br/>        let big_vec &#61; option::borrow(&amp;v.big_vec);<br/>        vector::append(&amp;mut res, big_vector::to_vector(big_vec));<br/>    &#125;;<br/>    res<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_push_back"></a>

## Function `push_back`

Add T <code>val</code> to the end of the vector <code>v</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, val: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut SmartVector&lt;T&gt;, val: T) &#123;<br/>    let len &#61; length(v);<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    if (len &#61;&#61; inline_len) &#123;<br/>        let bucket_size &#61; if (option::is_some(&amp;v.inline_capacity)) &#123;<br/>            if (len &lt; &#42;option::borrow(&amp;v.inline_capacity)) &#123;<br/>                vector::push_back(&amp;mut v.inline_vec, val);<br/>                return<br/>            &#125;;<br/>            &#42;option::borrow(&amp;v.bucket_size)<br/>        &#125; else &#123;<br/>            let val_size &#61; size_of_val(&amp;val);<br/>            if (val_size &#42; (inline_len &#43; 1) &lt; 150 /&#42; magic number &#42;/) &#123;<br/>                vector::push_back(&amp;mut v.inline_vec, val);<br/>                return<br/>            &#125;;<br/>            let estimated_avg_size &#61; max((size_of_val(&amp;v.inline_vec) &#43; val_size) / (inline_len &#43; 1), 1);<br/>            max(1024 /&#42; free_write_quota &#42;/ / estimated_avg_size, 1)<br/>        &#125;;<br/>        option::fill(&amp;mut v.big_vec, big_vector::empty(bucket_size));<br/>    &#125;;<br/>    big_vector::push_back(option::borrow_mut(&amp;mut v.big_vec), val);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_pop_back"></a>

## Function `pop_back`

Pop an T from the end of vector <code>v</code>. It does shrink the buckets if they're empty.
Aborts if <code>v</code> is empty.


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut SmartVector&lt;T&gt;): T &#123;<br/>    assert!(!is_empty(v), error::invalid_state(EVECTOR_EMPTY));<br/>    let big_vec_wrapper &#61; &amp;mut v.big_vec;<br/>    if (option::is_some(big_vec_wrapper)) &#123;<br/>        let big_vec &#61; option::extract(big_vec_wrapper);<br/>        let val &#61; big_vector::pop_back(&amp;mut big_vec);<br/>        if (big_vector::is_empty(&amp;big_vec)) &#123;<br/>            big_vector::destroy_empty(big_vec)<br/>        &#125; else &#123;<br/>            option::fill(big_vec_wrapper, big_vec);<br/>        &#125;;<br/>        val<br/>    &#125; else &#123;<br/>        vector::pop_back(&amp;mut v.inline_vec)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_remove"></a>

## Function `remove`

Remove the T at index i in the vector v and return the owned value that was previously stored at i in v.
All Ts occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut SmartVector&lt;T&gt;, i: u64): T &#123;<br/>    let len &#61; length(v);<br/>    assert!(i &lt; len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    if (i &lt; inline_len) &#123;<br/>        vector::remove(&amp;mut v.inline_vec, i)<br/>    &#125; else &#123;<br/>        let big_vec_wrapper &#61; &amp;mut v.big_vec;<br/>        let big_vec &#61; option::extract(big_vec_wrapper);<br/>        let val &#61; big_vector::remove(&amp;mut big_vec, i &#45; inline_len);<br/>        if (big_vector::is_empty(&amp;big_vec)) &#123;<br/>            big_vector::destroy_empty(big_vec)<br/>        &#125; else &#123;<br/>            option::fill(big_vec_wrapper, big_vec);<br/>        &#125;;<br/>        val<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th T of the vector <code>v</code> with the last T and then pop the vector.
This is O(1), but does not preserve ordering of Ts in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut SmartVector&lt;T&gt;, i: u64): T &#123;<br/>    let len &#61; length(v);<br/>    assert!(i &lt; len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    let big_vec_wrapper &#61; &amp;mut v.big_vec;<br/>    let inline_vec &#61; &amp;mut v.inline_vec;<br/>    if (i &gt;&#61; inline_len) &#123;<br/>        let big_vec &#61; option::extract(big_vec_wrapper);<br/>        let val &#61; big_vector::swap_remove(&amp;mut big_vec, i &#45; inline_len);<br/>        if (big_vector::is_empty(&amp;big_vec)) &#123;<br/>            big_vector::destroy_empty(big_vec)<br/>        &#125; else &#123;<br/>            option::fill(big_vec_wrapper, big_vec);<br/>        &#125;;<br/>        val<br/>    &#125; else &#123;<br/>        if (inline_len &lt; len) &#123;<br/>            let big_vec &#61; option::extract(big_vec_wrapper);<br/>            let last_from_big_vec &#61; big_vector::pop_back(&amp;mut big_vec);<br/>            if (big_vector::is_empty(&amp;big_vec)) &#123;<br/>                big_vector::destroy_empty(big_vec)<br/>            &#125; else &#123;<br/>                option::fill(big_vec_wrapper, big_vec);<br/>            &#125;;<br/>            vector::push_back(inline_vec, last_from_big_vec);<br/>        &#125;;<br/>        vector::swap_remove(inline_vec, i)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_swap"></a>

## Function `swap`

Swap the Ts at the i'th and j'th indices in the vector v. Will abort if either of i or j are out of bounds
for v.


<pre><code>public fun swap&lt;T: store&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64, j: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap&lt;T: store&gt;(v: &amp;mut SmartVector&lt;T&gt;, i: u64, j: u64) &#123;<br/>    if (i &gt; j) &#123;<br/>        return swap(v, j, i)<br/>    &#125;;<br/>    let len &#61; length(v);<br/>    assert!(j &lt; len, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    if (i &gt;&#61; inline_len) &#123;<br/>        big_vector::swap(option::borrow_mut(&amp;mut v.big_vec), i &#45; inline_len, j &#45; inline_len);<br/>    &#125; else if (j &lt; inline_len) &#123;<br/>        vector::swap(&amp;mut v.inline_vec, i, j);<br/>    &#125; else &#123;<br/>        let big_vec &#61; option::borrow_mut(&amp;mut v.big_vec);<br/>        let inline_vec &#61; &amp;mut v.inline_vec;<br/>        let element_i &#61; vector::swap_remove(inline_vec, i);<br/>        let element_j &#61; big_vector::swap_remove(big_vec, j &#45; inline_len);<br/>        vector::push_back(inline_vec, element_j);<br/>        vector::swap(inline_vec, i, inline_len &#45; 1);<br/>        big_vector::push_back(big_vec, element_i);<br/>        big_vector::swap(big_vec, j &#45; inline_len, len &#45; inline_len &#45; 1);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_reverse"></a>

## Function `reverse`

Reverse the order of the Ts in the vector v in-place.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code>public fun reverse&lt;T: store&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reverse&lt;T: store&gt;(v: &amp;mut SmartVector&lt;T&gt;) &#123;<br/>    let inline_len &#61; vector::length(&amp;v.inline_vec);<br/>    let i &#61; 0;<br/>    let new_inline_vec &#61; vector[];<br/>    // Push the last `inline_len` Ts into a temp vector.<br/>    while (i &lt; inline_len) &#123;<br/>        vector::push_back(&amp;mut new_inline_vec, pop_back(v));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    vector::reverse(&amp;mut new_inline_vec);<br/>    // Reverse the big_vector left if exists.<br/>    if (option::is_some(&amp;v.big_vec)) &#123;<br/>        big_vector::reverse(option::borrow_mut(&amp;mut v.big_vec));<br/>    &#125;;<br/>    // Mem::swap the two vectors.<br/>    let temp_vec &#61; vector[];<br/>    while (!vector::is_empty(&amp;mut v.inline_vec)) &#123;<br/>        vector::push_back(&amp;mut temp_vec, vector::pop_back(&amp;mut v.inline_vec));<br/>    &#125;;<br/>    vector::reverse(&amp;mut temp_vec);<br/>    while (!vector::is_empty(&amp;mut new_inline_vec)) &#123;<br/>        vector::push_back(&amp;mut v.inline_vec, vector::pop_back(&amp;mut new_inline_vec));<br/>    &#125;;<br/>    vector::destroy_empty(new_inline_vec);<br/>    // Push the rest Ts originally left in inline_vector back to the end of the smart vector.<br/>    while (!vector::is_empty(&amp;mut temp_vec)) &#123;<br/>        push_back(v, vector::pop_back(&amp;mut temp_vec));<br/>    &#125;;<br/>    vector::destroy_empty(temp_vec);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_index_of"></a>

## Function `index_of`

Return <code>(true, i)</code> if <code>val</code> is in the vector <code>v</code> at index <code>i</code>.
Otherwise, returns <code>(false, 0)</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code>public fun index_of&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;, val: &amp;T): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun index_of&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;, val: &amp;T): (bool, u64) &#123;<br/>    let (found, i) &#61; vector::index_of(&amp;v.inline_vec, val);<br/>    if (found) &#123;<br/>        (true, i)<br/>    &#125; else if (option::is_some(&amp;v.big_vec)) &#123;<br/>        let (found, i) &#61; big_vector::index_of(option::borrow(&amp;v.big_vec), val);<br/>        (found, i &#43; vector::length(&amp;v.inline_vec))<br/>    &#125; else &#123;<br/>        (false, 0)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_contains"></a>

## Function `contains`

Return true if <code>val</code> is in the vector <code>v</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code>public fun contains&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;, val: &amp;T): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;, val: &amp;T): bool &#123;<br/>    if (is_empty(v)) return false;<br/>    let (exist, _) &#61; index_of(v, val);<br/>    exist<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code>public fun length&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;): u64 &#123;<br/>    vector::length(&amp;v.inline_vec) &#43; if (option::is_none(&amp;v.big_vec)) &#123;<br/>        0<br/>    &#125; else &#123;<br/>        big_vector::length(option::borrow(&amp;v.big_vec))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_is_empty"></a>

## Function `is_empty`

Return <code>true</code> if the vector <code>v</code> has no Ts and <code>false</code> otherwise.


<pre><code>public fun is_empty&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_empty&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;): bool &#123;<br/>    length(v) &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_for_each"></a>

## Function `for_each`

Apply the function to each T in the vector, consuming it.


<pre><code>public fun for_each&lt;T: store&gt;(v: smart_vector::SmartVector&lt;T&gt;, f: &#124;T&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each&lt;T: store&gt;(v: SmartVector&lt;T&gt;, f: &#124;T&#124;) &#123;<br/>    aptos_std::smart_vector::reverse(&amp;mut v); // We need to reverse the vector to consume it efficiently<br/>    aptos_std::smart_vector::for_each_reverse(v, &#124;e&#124; f(e));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_for_each_reverse"></a>

## Function `for_each_reverse`

Apply the function to each T in the vector, consuming it.


<pre><code>public fun for_each_reverse&lt;T&gt;(v: smart_vector::SmartVector&lt;T&gt;, f: &#124;T&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_reverse&lt;T&gt;(v: SmartVector&lt;T&gt;, f: &#124;T&#124;) &#123;<br/>    let len &#61; aptos_std::smart_vector::length(&amp;v);<br/>    while (len &gt; 0) &#123;<br/>        f(aptos_std::smart_vector::pop_back(&amp;mut v));<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    aptos_std::smart_vector::destroy_empty(v)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each T in the vector.


<pre><code>public fun for_each_ref&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;, f: &#124;&amp;T&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_ref&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;, f: &#124;&amp;T&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; aptos_std::smart_vector::length(v);<br/>    while (i &lt; len) &#123;<br/>        f(aptos_std::smart_vector::borrow(v, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference to each T in the vector.


<pre><code>public fun for_each_mut&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, f: &#124;&amp;mut T&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_mut&lt;T&gt;(v: &amp;mut SmartVector&lt;T&gt;, f: &#124;&amp;mut T&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; aptos_std::smart_vector::length(v);<br/>    while (i &lt; len) &#123;<br/>        f(aptos_std::smart_vector::borrow_mut(v, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_enumerate_ref"></a>

## Function `enumerate_ref`

Apply the function to a reference of each T in the vector with its index.


<pre><code>public fun enumerate_ref&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;, f: &#124;(u64, &amp;T)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun enumerate_ref&lt;T&gt;(v: &amp;SmartVector&lt;T&gt;, f: &#124;u64, &amp;T&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; aptos_std::smart_vector::length(v);<br/>    while (i &lt; len) &#123;<br/>        f(i, aptos_std::smart_vector::borrow(v, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_enumerate_mut"></a>

## Function `enumerate_mut`

Apply the function to a mutable reference of each T in the vector with its index.


<pre><code>public fun enumerate_mut&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, f: &#124;(u64, &amp;mut T)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun enumerate_mut&lt;T&gt;(v: &amp;mut SmartVector&lt;T&gt;, f: &#124;u64, &amp;mut T&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        f(i, borrow_mut(v, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_fold"></a>

## Function `fold`

Fold the function over the Ts. For example, <code>fold(vector[1,2,3], 0, f)</code> will execute
<code>f(f(f(0, 1), 2), 3)</code>


<pre><code>public fun fold&lt;Accumulator, T: store&gt;(v: smart_vector::SmartVector&lt;T&gt;, init: Accumulator, f: &#124;(Accumulator, T)&#124;Accumulator): Accumulator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun fold&lt;Accumulator, T: store&gt;(<br/>    v: SmartVector&lt;T&gt;,<br/>    init: Accumulator,<br/>    f: &#124;Accumulator, T&#124;Accumulator<br/>): Accumulator &#123;<br/>    let accu &#61; init;<br/>    aptos_std::smart_vector::for_each(v, &#124;elem&#124; accu &#61; f(accu, elem));<br/>    accu<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_foldr"></a>

## Function `foldr`

Fold right like fold above but working right to left. For example, <code>fold(vector[1,2,3], 0, f)</code> will execute
<code>f(1, f(2, f(3, 0)))</code>


<pre><code>public fun foldr&lt;Accumulator, T&gt;(v: smart_vector::SmartVector&lt;T&gt;, init: Accumulator, f: &#124;(T, Accumulator)&#124;Accumulator): Accumulator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun foldr&lt;Accumulator, T&gt;(<br/>    v: SmartVector&lt;T&gt;,<br/>    init: Accumulator,<br/>    f: &#124;T, Accumulator&#124;Accumulator<br/>): Accumulator &#123;<br/>    let accu &#61; init;<br/>    aptos_std::smart_vector::for_each_reverse(v, &#124;elem&#124; accu &#61; f(elem, accu));<br/>    accu<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_map_ref"></a>

## Function `map_ref`

Map the function over the references of the Ts of the vector, producing a new vector without modifying the
original vector.


<pre><code>public fun map_ref&lt;T1, T2: store&gt;(v: &amp;smart_vector::SmartVector&lt;T1&gt;, f: &#124;&amp;T1&#124;T2): smart_vector::SmartVector&lt;T2&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map_ref&lt;T1, T2: store&gt;(<br/>    v: &amp;SmartVector&lt;T1&gt;,<br/>    f: &#124;&amp;T1&#124;T2<br/>): SmartVector&lt;T2&gt; &#123;<br/>    let result &#61; aptos_std::smart_vector::new&lt;T2&gt;();<br/>    aptos_std::smart_vector::for_each_ref(v, &#124;elem&#124; aptos_std::smart_vector::push_back(&amp;mut result, f(elem)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_map"></a>

## Function `map`

Map the function over the Ts of the vector, producing a new vector.


<pre><code>public fun map&lt;T1: store, T2: store&gt;(v: smart_vector::SmartVector&lt;T1&gt;, f: &#124;T1&#124;T2): smart_vector::SmartVector&lt;T2&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map&lt;T1: store, T2: store&gt;(<br/>    v: SmartVector&lt;T1&gt;,<br/>    f: &#124;T1&#124;T2<br/>): SmartVector&lt;T2&gt; &#123;<br/>    let result &#61; aptos_std::smart_vector::new&lt;T2&gt;();<br/>    aptos_std::smart_vector::for_each(v, &#124;elem&#124; push_back(&amp;mut result, f(elem)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_filter"></a>

## Function `filter`

Filter the vector using the boolean function, removing all Ts for which <code>p(e)</code> is not true.


<pre><code>public fun filter&lt;T: drop, store&gt;(v: smart_vector::SmartVector&lt;T&gt;, p: &#124;&amp;T&#124;bool): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun filter&lt;T: store &#43; drop&gt;(<br/>    v: SmartVector&lt;T&gt;,<br/>    p: &#124;&amp;T&#124;bool<br/>): SmartVector&lt;T&gt; &#123;<br/>    let result &#61; aptos_std::smart_vector::new&lt;T&gt;();<br/>    aptos_std::smart_vector::for_each(v, &#124;elem&#124; &#123;<br/>        if (p(&amp;elem)) aptos_std::smart_vector::push_back(&amp;mut result, elem);<br/>    &#125;);<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_zip"></a>

## Function `zip`



<pre><code>public fun zip&lt;T1: store, T2: store&gt;(v1: smart_vector::SmartVector&lt;T1&gt;, v2: smart_vector::SmartVector&lt;T2&gt;, f: &#124;(T1, T2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip&lt;T1: store, T2: store&gt;(v1: SmartVector&lt;T1&gt;, v2: SmartVector&lt;T2&gt;, f: &#124;T1, T2&#124;) &#123;<br/>    // We need to reverse the vectors to consume it efficiently<br/>    aptos_std::smart_vector::reverse(&amp;mut v1);<br/>    aptos_std::smart_vector::reverse(&amp;mut v2);<br/>    aptos_std::smart_vector::zip_reverse(v1, v2, &#124;e1, e2&#124; f(e1, e2));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_zip_reverse"></a>

## Function `zip_reverse`

Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
This errors out if the vectors are not of the same length.


<pre><code>public fun zip_reverse&lt;T1, T2&gt;(v1: smart_vector::SmartVector&lt;T1&gt;, v2: smart_vector::SmartVector&lt;T2&gt;, f: &#124;(T1, T2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_reverse&lt;T1, T2&gt;(<br/>    v1: SmartVector&lt;T1&gt;,<br/>    v2: SmartVector&lt;T2&gt;,<br/>    f: &#124;T1, T2&#124;,<br/>) &#123;<br/>    let len &#61; aptos_std::smart_vector::length(&amp;v1);<br/>    // We can&apos;t use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(len &#61;&#61; aptos_std::smart_vector::length(&amp;v2), 0x20005);<br/>    while (len &gt; 0) &#123;<br/>        f(aptos_std::smart_vector::pop_back(&amp;mut v1), aptos_std::smart_vector::pop_back(&amp;mut v2));<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    aptos_std::smart_vector::destroy_empty(v1);<br/>    aptos_std::smart_vector::destroy_empty(v2);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_zip_ref"></a>

## Function `zip_ref`

Apply the function to the references of each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code>public fun zip_ref&lt;T1, T2&gt;(v1: &amp;smart_vector::SmartVector&lt;T1&gt;, v2: &amp;smart_vector::SmartVector&lt;T2&gt;, f: &#124;(&amp;T1, &amp;T2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_ref&lt;T1, T2&gt;(<br/>    v1: &amp;SmartVector&lt;T1&gt;,<br/>    v2: &amp;SmartVector&lt;T2&gt;,<br/>    f: &#124;&amp;T1, &amp;T2&#124;,<br/>) &#123;<br/>    let len &#61; aptos_std::smart_vector::length(v1);<br/>    // We can&apos;t use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(len &#61;&#61; aptos_std::smart_vector::length(v2), 0x20005);<br/>    let i &#61; 0;<br/>    while (i &lt; len) &#123;<br/>        f(aptos_std::smart_vector::borrow(v1, i), aptos_std::smart_vector::borrow(v2, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_zip_mut"></a>

## Function `zip_mut`

Apply the function to mutable references to each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code>public fun zip_mut&lt;T1, T2&gt;(v1: &amp;mut smart_vector::SmartVector&lt;T1&gt;, v2: &amp;mut smart_vector::SmartVector&lt;T2&gt;, f: &#124;(&amp;mut T1, &amp;mut T2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_mut&lt;T1, T2&gt;(<br/>    v1: &amp;mut SmartVector&lt;T1&gt;,<br/>    v2: &amp;mut SmartVector&lt;T2&gt;,<br/>    f: &#124;&amp;mut T1, &amp;mut T2&#124;,<br/>) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; aptos_std::smart_vector::length(v1);<br/>    // We can&apos;t use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(len &#61;&#61; aptos_std::smart_vector::length(v2), 0x20005);<br/>    while (i &lt; len) &#123;<br/>        f(aptos_std::smart_vector::borrow_mut(v1, i), aptos_std::smart_vector::borrow_mut(v2, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_zip_map"></a>

## Function `zip_map`

Map the function over the element pairs of the two vectors, producing a new vector.


<pre><code>public fun zip_map&lt;T1: store, T2: store, NewT: store&gt;(v1: smart_vector::SmartVector&lt;T1&gt;, v2: smart_vector::SmartVector&lt;T2&gt;, f: &#124;(T1, T2)&#124;NewT): smart_vector::SmartVector&lt;NewT&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_map&lt;T1: store, T2: store, NewT: store&gt;(<br/>    v1: SmartVector&lt;T1&gt;,<br/>    v2: SmartVector&lt;T2&gt;,<br/>    f: &#124;T1, T2&#124;NewT<br/>): SmartVector&lt;NewT&gt; &#123;<br/>    // We can&apos;t use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(aptos_std::smart_vector::length(&amp;v1) &#61;&#61; aptos_std::smart_vector::length(&amp;v2), 0x20005);<br/><br/>    let result &#61; aptos_std::smart_vector::new&lt;NewT&gt;();<br/>    aptos_std::smart_vector::zip(v1, v2, &#124;e1, e2&#124; push_back(&amp;mut result, f(e1, e2)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_vector_zip_map_ref"></a>

## Function `zip_map_ref`

Map the function over the references of the element pairs of two vectors, producing a new vector from the return
values without modifying the original vectors.


<pre><code>public fun zip_map_ref&lt;T1, T2, NewT: store&gt;(v1: &amp;smart_vector::SmartVector&lt;T1&gt;, v2: &amp;smart_vector::SmartVector&lt;T2&gt;, f: &#124;(&amp;T1, &amp;T2)&#124;NewT): smart_vector::SmartVector&lt;NewT&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_map_ref&lt;T1, T2, NewT: store&gt;(<br/>    v1: &amp;SmartVector&lt;T1&gt;,<br/>    v2: &amp;SmartVector&lt;T2&gt;,<br/>    f: &#124;&amp;T1, &amp;T2&#124;NewT<br/>): SmartVector&lt;NewT&gt; &#123;<br/>    // We can&apos;t use the constant ESMART_VECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(aptos_std::smart_vector::length(v1) &#61;&#61; aptos_std::smart_vector::length(v2), 0x20005);<br/><br/>    let result &#61; aptos_std::smart_vector::new&lt;NewT&gt;();<br/>    aptos_std::smart_vector::zip_ref(v1, v2, &#124;e1, e2&#124; push_back(&amp;mut result, f(e1, e2)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SmartVector"></a>

### Struct `SmartVector`


<pre><code>struct SmartVector&lt;T&gt; has store<br/></code></pre>



<dl>
<dt>
<code>inline_vec: vector&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>big_vec: option::Option&lt;big_vector::BigVector&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inline_capacity: option::Option&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: option::Option&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant option::is_none(bucket_size)<br/>    &#124;&#124; (option::is_some(bucket_size) &amp;&amp; option::borrow(bucket_size) !&#61; 0);<br/>invariant option::is_none(inline_capacity)<br/>    &#124;&#124; (len(inline_vec) &lt;&#61; option::borrow(inline_capacity));<br/>invariant (option::is_none(inline_capacity) &amp;&amp; option::is_none(bucket_size))<br/>    &#124;&#124; (option::is_some(inline_capacity) &amp;&amp; option::is_some(bucket_size));<br/></code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>&#35;[deprecated]<br/>public fun empty&lt;T: store&gt;(): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>



<a id="@Specification_1_empty_with_config"></a>

### Function `empty_with_config`


<pre><code>public fun empty_with_config&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): smart_vector::SmartVector&lt;T&gt;<br/></code></pre>




<pre><code>aborts_if bucket_size &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code>public fun destroy_empty&lt;T&gt;(v: smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>




<pre><code>aborts_if !(is_empty(v));<br/>aborts_if len(v.inline_vec) !&#61; 0<br/>    &#124;&#124; option::is_some(v.big_vec);<br/></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;, i: u64): &amp;T<br/></code></pre>




<pre><code>aborts_if i &gt;&#61; length(v);<br/>aborts_if option::is_some(v.big_vec) &amp;&amp; (<br/>    (len(v.inline_vec) &#43; big_vector::length&lt;T&gt;(option::borrow(v.big_vec))) &gt; MAX_U64<br/>);<br/></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code>public fun append&lt;T: store&gt;(lhs: &amp;mut smart_vector::SmartVector&lt;T&gt;, other: smart_vector::SmartVector&lt;T&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code>public fun push_back&lt;T: store&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, val: T)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code>public fun pop_back&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;): T<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>aborts_if  option::is_some(v.big_vec)<br/>    &amp;&amp;<br/>    (table_with_length::spec_len(option::borrow(v.big_vec).buckets) &#61;&#61; 0);<br/>aborts_if is_empty(v);<br/>aborts_if option::is_some(v.big_vec) &amp;&amp; (<br/>    (len(v.inline_vec) &#43; big_vector::length&lt;T&gt;(option::borrow(v.big_vec))) &gt; MAX_U64<br/>);<br/>ensures length(v) &#61;&#61; length(old(v)) &#45; 1;<br/></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64): T<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code>public fun swap_remove&lt;T&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64): T<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>aborts_if i &gt;&#61; length(v);<br/>aborts_if option::is_some(v.big_vec) &amp;&amp; (<br/>    (len(v.inline_vec) &#43; big_vector::length&lt;T&gt;(option::borrow(v.big_vec))) &gt; MAX_U64<br/>);<br/>ensures length(v) &#61;&#61; length(old(v)) &#45; 1;<br/></code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code>public fun swap&lt;T: store&gt;(v: &amp;mut smart_vector::SmartVector&lt;T&gt;, i: u64, j: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code>public fun length&lt;T&gt;(v: &amp;smart_vector::SmartVector&lt;T&gt;): u64<br/></code></pre>




<pre><code>aborts_if option::is_some(v.big_vec) &amp;&amp; len(v.inline_vec) &#43; big_vector::length(option::spec_borrow(v.big_vec)) &gt; MAX_U64;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
