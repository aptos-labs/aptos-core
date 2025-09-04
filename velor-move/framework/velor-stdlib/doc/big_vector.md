
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_big_vector_BigVector"></a>

## Struct `BigVector`

A scalable vector implementation based on tables where elements are grouped into buckets.
Each bucket has a capacity of <code>bucket_size</code> elements.


<pre><code><b>struct</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; <b>has</b> store
</code></pre>



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


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 = 1;
</code></pre>



<a id="0x1_big_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>: u64 = 3;
</code></pre>



<a id="0x1_big_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non-empty vector


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>: u64 = 2;
</code></pre>



<a id="0x1_big_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code><b>const</b> <a href="big_vector.md#0x1_big_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>: u64 = 4;
</code></pre>



<a id="0x1_big_vector_empty"></a>

## Function `empty`

Regular Vector API
Create an empty vector.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_empty">empty</a>&lt;T: store&gt;(bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_empty">empty</a>&lt;T: store&gt;(bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; {
    <b>assert</b>!(bucket_size &gt; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>));
    <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a> {
        buckets: <a href="table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),
        end_index: 0,
        bucket_size,
    }
}
</code></pre>



</details>

<a id="0x1_big_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in element.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_singleton">singleton</a>&lt;T: store&gt;(element: T, bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_singleton">singleton</a>&lt;T: store&gt;(element: T, bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; {
    <b>let</b> v = <a href="big_vector.md#0x1_big_vector_empty">empty</a>(bucket_size);
    v.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(element);
    v
}
</code></pre>



</details>

<a id="0x1_big_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>self</code>.
Aborts if <code>self</code> is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(self: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(self: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) {
    <b>assert</b>!(self.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>));
    <b>let</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a> { buckets, end_index: _, bucket_size: _ } = self;
    buckets.<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_big_vector_destroy"></a>

## Function `destroy`

Destroy the vector <code>self</code> if T has <code>drop</code>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy">destroy</a>&lt;T: drop&gt;(self: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy">destroy</a>&lt;T: drop&gt;(self: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) {
    <b>let</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a> { buckets, end_index, bucket_size: _ } = self;
    <b>let</b> i = 0;
    <b>while</b> (end_index &gt; 0) {
        <b>let</b> num_elements = buckets.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(i).<a href="big_vector.md#0x1_big_vector_length">length</a>();
        end_index -= num_elements;
        i += 1;
    };
    buckets.<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_big_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>self</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow">borrow</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow">borrow</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): &T {
    <b>assert</b>!(i &lt; self.<a href="big_vector.md#0x1_big_vector_length">length</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    self.buckets.<a href="big_vector.md#0x1_big_vector_borrow">borrow</a>(i / self.bucket_size).<a href="big_vector.md#0x1_big_vector_borrow">borrow</a>(i % self.bucket_size)
}
</code></pre>



</details>

<a id="0x1_big_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>self</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T {
    <b>assert</b>!(i &lt; self.<a href="big_vector.md#0x1_big_vector_length">length</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(i / self.bucket_size).<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(i % self.bucket_size)
}
</code></pre>



</details>

<a id="0x1_big_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the elements in the other vector onto the self vector in the
same order as they occurred in other.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_append">append</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, other: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_append">append</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, other: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) {
    <b>let</b> other_len = other.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>let</b> half_other_len = other_len / 2;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; half_other_len) {
        self.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(other.<a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>(i));
        i += 1;
    };
    <b>while</b> (i &lt; other_len) {
        self.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(other.<a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>());
        i += 1;
    };
    other.<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_big_vector_push_back"></a>

## Function `push_back`

Add element <code>val</code> to the end of the vector <code>self</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, val: T) {
    <b>let</b> num_buckets = self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>if</b> (self.end_index == num_buckets * self.bucket_size) {
        self.buckets.add(num_buckets, <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());
        self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(num_buckets).<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(val);
    } <b>else</b> {
        self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(num_buckets - 1).<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(val);
    };
    self.end_index += 1;
}
</code></pre>



</details>

<a id="0x1_big_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>self</code>. It doesn't shrink the buckets even if they're empty.
Call <code>shrink_to_fit</code> explicity to deallocate empty buckets.
Aborts if <code>self</code> is empty.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): T {
    <b>assert</b>!(!self.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="big_vector.md#0x1_big_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));
    <b>let</b> num_buckets = self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>let</b> last_bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(num_buckets - 1);
    <b>let</b> val = last_bucket.<a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>();
    // Shrink the <a href="table.md#0x1_table">table</a> <b>if</b> the last <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> is empty.
    <b>if</b> (last_bucket.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>()) {
        <b>move</b> last_bucket;
        self.buckets.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(num_buckets - 1).<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
    };
    self.end_index -= 1;
    val
}
</code></pre>



</details>

<a id="0x1_big_vector_remove"></a>

## Function `remove`

Remove the element at index i in the vector v and return the owned value that was previously stored at i in self.
All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): T {
    <b>let</b> len = self.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>assert</b>!(i &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> num_buckets = self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>let</b> cur_bucket_index = i / self.bucket_size + 1;
    <b>let</b> cur_bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(cur_bucket_index - 1);
    <b>let</b> res = cur_bucket.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(i % self.bucket_size);
    self.end_index -= 1;
    <b>move</b> cur_bucket;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> cur_bucket_index &lt;= num_buckets;
            <b>invariant</b> <a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(self.buckets) == num_buckets;
        };
        (cur_bucket_index &lt; num_buckets)
    }) {
        // remove one element from the start of current <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>
        <b>let</b> cur_bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(cur_bucket_index);
        <b>let</b> t = cur_bucket.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(0);
        <b>move</b> cur_bucket;
        // and put it at the end of the last one
        <b>let</b> prev_bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(cur_bucket_index - 1);
        prev_bucket.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(t);
        cur_bucket_index += 1;
    };
    <b>spec</b> {
        <b>assert</b> cur_bucket_index == num_buckets;
    };

    // Shrink the <a href="table.md#0x1_table">table</a> <b>if</b> the last <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> is empty.
    <b>let</b> last_bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(num_buckets - 1);
    <b>if</b> (last_bucket.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>()) {
        <b>move</b> last_bucket;
        self.buckets.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(num_buckets - 1).<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
    };

    res
}
</code></pre>



</details>

<a id="0x1_big_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>self</code> with the last element and then pop the vector.
This is O(1), but does not preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): T {
    <b>assert</b>!(i &lt; self.<a href="big_vector.md#0x1_big_vector_length">length</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> last_val = self.<a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>();
    // <b>if</b> the requested value is the last one, <b>return</b> it
    <b>if</b> (self.end_index == i) {
        <b>return</b> last_val
    };
    // because the lack of <a href="../../move-stdlib/doc/mem.md#0x1_mem_swap">mem::swap</a>, here we swap remove the requested value from the bucket
    // and append the last_val <b>to</b> the bucket then swap the last bucket val back
    <b>let</b> bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(i / self.bucket_size);
    <b>let</b> bucket_len = bucket.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>let</b> val = bucket.<a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>(i % self.bucket_size);
    bucket.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(last_val);
    bucket.<a href="big_vector.md#0x1_big_vector_swap">swap</a>(i % self.bucket_size, bucket_len - 1);
    val
}
</code></pre>



</details>

<a id="0x1_big_vector_swap"></a>

## Function `swap`

Swap the elements at the i'th and j'th indices in the vector self. Will abort if either of i or j are out of bounds
for self.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap">swap</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64, j: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap">swap</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64, j: u64) {
    <b>assert</b>!(i &lt; self.<a href="big_vector.md#0x1_big_vector_length">length</a>() && j &lt; self.<a href="big_vector.md#0x1_big_vector_length">length</a>(), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="big_vector.md#0x1_big_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> i_bucket_index = i / self.bucket_size;
    <b>let</b> j_bucket_index = j / self.bucket_size;
    <b>let</b> i_vector_index = i % self.bucket_size;
    <b>let</b> j_vector_index = j % self.bucket_size;
    <b>if</b> (i_bucket_index == j_bucket_index) {
        self.buckets.<a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>(i_bucket_index).<a href="big_vector.md#0x1_big_vector_swap">swap</a>(i_vector_index, j_vector_index);
        <b>return</b>
    };
    // If i and j are in different buckets, take the buckets out first for easy mutation.
    <b>let</b> bucket_i = self.buckets.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(i_bucket_index);
    <b>let</b> bucket_j = self.buckets.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(j_bucket_index);
    // Get the elements from buckets by calling `swap_remove`.
    <b>let</b> element_i = bucket_i.<a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>(i_vector_index);
    <b>let</b> element_j = bucket_j.<a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>(j_vector_index);
    // Swap the elements and push back <b>to</b> the other bucket.
    bucket_i.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(element_j);
    bucket_j.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(element_i);
    <b>let</b> last_index_in_bucket_i = bucket_i.<a href="big_vector.md#0x1_big_vector_length">length</a>() - 1;
    <b>let</b> last_index_in_bucket_j = bucket_j.<a href="big_vector.md#0x1_big_vector_length">length</a>() - 1;
    // Re-position the swapped elements <b>to</b> the right index.
    bucket_i.<a href="big_vector.md#0x1_big_vector_swap">swap</a>(i_vector_index, last_index_in_bucket_i);
    bucket_j.<a href="big_vector.md#0x1_big_vector_swap">swap</a>(j_vector_index, last_index_in_bucket_j);
    // Add back the buckets.
    self.buckets.add(i_bucket_index, bucket_i);
    self.buckets.add(j_bucket_index, bucket_j);
}
</code></pre>



</details>

<a id="0x1_big_vector_reverse"></a>

## Function `reverse`

Reverse the order of the elements in the vector self in-place.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_reverse">reverse</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_reverse">reverse</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;) {
    <b>let</b> new_buckets = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> push_bucket = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> num_buckets = self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>let</b> num_buckets_left = num_buckets;

    <b>while</b> (num_buckets_left &gt; 0) {
        <b>let</b> pop_bucket = self.buckets.<a href="big_vector.md#0x1_big_vector_remove">remove</a>(num_buckets_left - 1);
        pop_bucket.for_each_reverse(|val| {
            push_bucket.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(val);
            <b>if</b> (push_bucket.<a href="big_vector.md#0x1_big_vector_length">length</a>() == self.bucket_size) {
                new_buckets.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(push_bucket);
                push_bucket = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
            };
        });
        num_buckets_left -= 1;
    };

    <b>if</b> (push_bucket.<a href="big_vector.md#0x1_big_vector_length">length</a>() &gt; 0) {
        new_buckets.<a href="big_vector.md#0x1_big_vector_push_back">push_back</a>(push_bucket);
    } <b>else</b> {
        push_bucket.<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
    };

    new_buckets.<a href="big_vector.md#0x1_big_vector_reverse">reverse</a>();
    <b>assert</b>!(self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>() == 0, 0);
    for (i in 0..num_buckets) {
        self.buckets.add(i, new_buckets.<a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>());
    };
    new_buckets.<a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>();
}
</code></pre>



</details>

<a id="0x1_big_vector_index_of"></a>

## Function `index_of`

Return the index of the first occurrence of an element in self that is equal to e. Returns (true, index) if such an
element was found, and (false, 0) otherwise.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: &T): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, val: &T): (bool, u64) {
    <b>let</b> num_buckets = self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    <b>let</b> bucket_index = 0;
    <b>while</b> (bucket_index &lt; num_buckets) {
        <b>let</b> cur = self.buckets.<a href="big_vector.md#0x1_big_vector_borrow">borrow</a>(bucket_index);
        <b>let</b> (found, i) = cur.<a href="big_vector.md#0x1_big_vector_index_of">index_of</a>(val);
        <b>if</b> (found) {
            <b>return</b> (<b>true</b>, bucket_index * self.bucket_size + i)
        };
        bucket_index += 1;
    };
    (<b>false</b>, 0)
}
</code></pre>



</details>

<a id="0x1_big_vector_contains"></a>

## Function `contains`

Return if an element equal to e exists in the vector self.
Disclaimer: This function is costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_contains">contains</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: &T): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_contains">contains</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, val: &T): bool {
    <b>if</b> (self.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>()) <b>return</b> <b>false</b>;
    <b>let</b> (exist, _) = self.<a href="big_vector.md#0x1_big_vector_index_of">index_of</a>(val);
    exist
}
</code></pre>



</details>

<a id="0x1_big_vector_to_vector"></a>

## Function `to_vector`

Convert a big vector to a native vector, which is supposed to be called mostly by view functions to get an
atomic view of the whole vector.
Disclaimer: This function may be costly as the big vector may be huge in size. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_to_vector">to_vector</a>&lt;T: <b>copy</b>&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_to_vector">to_vector</a>&lt;T: <b>copy</b>&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> res = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> num_buckets = self.buckets.<a href="big_vector.md#0x1_big_vector_length">length</a>();
    for (i in 0..num_buckets) {
        res.<a href="big_vector.md#0x1_big_vector_append">append</a>(*self.buckets.<a href="big_vector.md#0x1_big_vector_borrow">borrow</a>(i));
    };
    res
}
</code></pre>



</details>

<a id="0x1_big_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_length">length</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_length">length</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): u64 {
    self.end_index
}
</code></pre>



</details>

<a id="0x1_big_vector_is_empty"></a>

## Function `is_empty`

Return <code><b>true</b></code> if the vector <code>v</code> has no elements and <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;): bool {
    self.<a href="big_vector.md#0x1_big_vector_length">length</a>() == 0
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_BigVector"></a>

### Struct `BigVector`


<pre><code><b>struct</b> <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt; <b>has</b> store
</code></pre>



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



<pre><code><b>invariant</b> bucket_size != 0;
<b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == 0 ==&gt; end_index == 0;
<b>invariant</b> end_index == 0 ==&gt; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == 0;
<b>invariant</b> end_index &lt;= <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) * bucket_size;
<b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == 0
    || (<b>forall</b> i in 0..<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets)-1: len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, i)) == bucket_size);
<b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == 0
    || len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) -1 )) &lt;= bucket_size;
<b>invariant</b> <b>forall</b> i in 0..<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets): <a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>(buckets, i);
<b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == (end_index + bucket_size - 1) / bucket_size;
<b>invariant</b> (<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == 0 && end_index == 0)
    || (<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) != 0 && ((<a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) - 1) * bucket_size) + (len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) - 1))) == end_index);
<b>invariant</b> <b>forall</b> i: u64 <b>where</b> i &gt;= <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets):  {
    !<a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>(buckets, i)
};
<b>invariant</b> <b>forall</b> i: u64 <b>where</b> i &lt; <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets):  {
    <a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>(buckets, i)
};
<b>invariant</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) == 0
    || (len(<a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(buckets, <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(buckets) - 1)) &gt; 0);
</code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_empty">empty</a>&lt;T: store&gt;(bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> bucket_size == 0;
<b>ensures</b> result.<a href="big_vector.md#0x1_big_vector_length">length</a>() == 0;
<b>ensures</b> result.bucket_size == bucket_size;
</code></pre>



<a id="@Specification_1_singleton"></a>

### Function `singleton`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="big_vector.md#0x1_big_vector_singleton">singleton</a>&lt;T: store&gt;(element: T, bucket_size: u64): <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> bucket_size == 0;
<b>ensures</b> result.<a href="big_vector.md#0x1_big_vector_length">length</a>() == 1;
<b>ensures</b> result.bucket_size == bucket_size;
</code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(self: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>aborts_if</b> !self.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>();
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow">borrow</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &T
</code></pre>




<pre><code><b>aborts_if</b> i &gt;= self.<a href="big_vector.md#0x1_big_vector_length">length</a>();
<b>ensures</b> result == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, i);
</code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T
</code></pre>




<pre><code><b>aborts_if</b> i &gt;= self.<a href="big_vector.md#0x1_big_vector_length">length</a>();
<b>ensures</b> result == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, i);
</code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_append">append</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, other: <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_push_back">push_back</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: T)
</code></pre>




<pre><code><b>let</b> num_buckets = <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(self.buckets);
<b>include</b> <a href="big_vector.md#0x1_big_vector_PushbackAbortsIf">PushbackAbortsIf</a>&lt;T&gt;;
<b>ensures</b> self.<a href="big_vector.md#0x1_big_vector_length">length</a>() == <b>old</b>(self).<a href="big_vector.md#0x1_big_vector_length">length</a>() + 1;
<b>ensures</b> self.end_index == <b>old</b>(self.end_index) + 1;
<b>ensures</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, self.end_index-1) == val;
<b>ensures</b> <b>forall</b> i in 0..self.end_index-1: <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, i) == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(self), i);
<b>ensures</b> self.bucket_size == <b>old</b>(self).bucket_size;
</code></pre>




<a id="0x1_big_vector_PushbackAbortsIf"></a>


<pre><code><b>schema</b> <a href="big_vector.md#0x1_big_vector_PushbackAbortsIf">PushbackAbortsIf</a>&lt;T&gt; {
    self: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;;
    <b>let</b> num_buckets = <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>(self.buckets);
    <b>aborts_if</b> num_buckets * self.bucket_size &gt; MAX_U64;
    <b>aborts_if</b> self.end_index + 1 &gt; MAX_U64;
}
</code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_pop_back">pop_back</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;): T
</code></pre>




<pre><code><b>aborts_if</b> self.<a href="big_vector.md#0x1_big_vector_is_empty">is_empty</a>();
<b>ensures</b> self.<a href="big_vector.md#0x1_big_vector_length">length</a>() == <b>old</b>(self).<a href="big_vector.md#0x1_big_vector_length">length</a>() - 1;
<b>ensures</b> result == <b>old</b>(<a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, self.end_index-1));
<b>ensures</b> <b>forall</b> i in 0..self.end_index: <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, i) == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(self), i);
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap_remove">swap_remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64): T
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b> i &gt;= self.<a href="big_vector.md#0x1_big_vector_length">length</a>();
<b>ensures</b> self.<a href="big_vector.md#0x1_big_vector_length">length</a>() == <b>old</b>(self).<a href="big_vector.md#0x1_big_vector_length">length</a>() - 1;
<b>ensures</b> result == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(self), i);
</code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_swap">swap</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, i: u64, j: u64)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1000;
<b>aborts_if</b> i &gt;= self.<a href="big_vector.md#0x1_big_vector_length">length</a>() || j &gt;= self.<a href="big_vector.md#0x1_big_vector_length">length</a>();
<b>ensures</b> self.<a href="big_vector.md#0x1_big_vector_length">length</a>() == <b>old</b>(self).<a href="big_vector.md#0x1_big_vector_length">length</a>();
<b>ensures</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, i) == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(self), j);
<b>ensures</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, j) == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(self), i);
<b>ensures</b> <b>forall</b> idx in 0..self.<a href="big_vector.md#0x1_big_vector_length">length</a>()
    <b>where</b> idx != i && idx != j:
    <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(self, idx) == <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>(<b>old</b>(self), idx);
</code></pre>



<a id="@Specification_1_reverse"></a>

### Function `reverse`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_reverse">reverse</a>&lt;T&gt;(self: &<b>mut</b> <a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>



<a id="@Specification_1_index_of"></a>

### Function `index_of`


<pre><code><b>public</b> <b>fun</b> <a href="big_vector.md#0x1_big_vector_index_of">index_of</a>&lt;T&gt;(self: &<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;, val: &T): (bool, u64)
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>




<a id="0x1_big_vector_spec_table_len"></a>


<pre><code><b>fun</b> <a href="big_vector.md#0x1_big_vector_spec_table_len">spec_table_len</a>&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;): u64 {
   <a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(t)
}
</code></pre>




<a id="0x1_big_vector_spec_table_contains"></a>


<pre><code><b>fun</b> <a href="big_vector.md#0x1_big_vector_spec_table_contains">spec_table_contains</a>&lt;K, V&gt;(t: TableWithLength&lt;K, V&gt;, k: K): bool {
   <a href="table_with_length.md#0x1_table_with_length_spec_contains">table_with_length::spec_contains</a>(t, k)
}
</code></pre>




<a id="0x1_big_vector_spec_at"></a>


<pre><code><b>fun</b> <a href="big_vector.md#0x1_big_vector_spec_at">spec_at</a>&lt;T&gt;(v: <a href="big_vector.md#0x1_big_vector_BigVector">BigVector</a>&lt;T&gt;, i: u64): T {
   <b>let</b> bucket = i / v.bucket_size;
   <b>let</b> idx = i % v.bucket_size;
   <b>let</b> v = <a href="table_with_length.md#0x1_table_with_length_spec_get">table_with_length::spec_get</a>(v.buckets, bucket);
   v[idx]
}
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
