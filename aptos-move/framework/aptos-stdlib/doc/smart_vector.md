
<a name="0x1_smart_vector"></a>

# Module `0x1::smart_vector`



-  [Struct `SmartVector`](#0x1_smart_vector_SmartVector)
-  [Constants](#@Constants_0)
-  [Function `empty`](#0x1_smart_vector_empty)
-  [Function `empty_with_config`](#0x1_smart_vector_empty_with_config)
-  [Function `singleton`](#0x1_smart_vector_singleton)
-  [Function `destroy_empty`](#0x1_smart_vector_destroy_empty)
-  [Function `borrow`](#0x1_smart_vector_borrow)
-  [Function `borrow_mut`](#0x1_smart_vector_borrow_mut)
-  [Function `append`](#0x1_smart_vector_append)
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


<pre><code><b>use</b> <a href="big_vector.md#0x1_big_vector">0x1::big_vector</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_smart_vector_SmartVector"></a>

## Struct `SmartVector`

A Scalable vector implementation based on tables, elements are grouped into buckets with <code>bucket_size</code>.
The option wrapping BigVector saves space in the metadata associated with BigVector when smart_vector is
so small that inline_vec vector can hold all the data.


<pre><code><b>struct</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; <b>has</b> store
</code></pre>



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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_smart_vector_EINDEX_OUT_OF_BOUNDS"></a>

Vector index is out of bounds


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 = 1;
</code></pre>



<a name="0x1_smart_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>: u64 = 3;
</code></pre>



<a name="0x1_smart_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non-empty vector


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>: u64 = 2;
</code></pre>



<a name="0x1_smart_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>: u64 = 4;
</code></pre>



<a name="0x1_smart_vector_empty"></a>

## Function `empty`

Regular Vector API
Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> {
        inline_vec: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        big_vec: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        inline_capacity: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        bucket_size: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_empty_with_config"></a>

## Function `empty_with_config`

Create an empty vector with customized config.
When inline_capacity = 0, SmartVector degrades to a wrapper of BigVector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <b>assert</b>!(bucket_size &gt; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>));
    <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> {
        inline_vec: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        big_vec: <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        inline_capacity: <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(inline_capacity),
        bucket_size: <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(bucket_size),
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in element.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_singleton">singleton</a>&lt;T: store&gt;(element: T): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_singleton">singleton</a>&lt;T: store&gt;(element: T): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <b>let</b> v = <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>();
    <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&<b>mut</b> v, element);
    v
}
</code></pre>



</details>

<a name="0x1_smart_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <b>assert</b>!(<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(&v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>));
    <b>let</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> { inline_vec, big_vec, inline_capacity: _, bucket_size: _} = v;
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(inline_vec);
    <a href="../../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(big_vec);
}
</code></pre>



</details>

<a name="0x1_smart_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): &T {
    <b>assert</b>!(i &lt; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>if</b> (i &lt; inline_len) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&v.inline_vec, i)
    } <b>else</b> {
        <a href="big_vector.md#0x1_big_vector_borrow">big_vector::borrow</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&v.big_vec), i - inline_len)
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T {
    <b>assert</b>!(i &lt; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>if</b> (i &lt; inline_len) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> v.inline_vec, i)
    } <b>else</b> {
        <a href="big_vector.md#0x1_big_vector_borrow_mut">big_vector::borrow_mut</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> v.big_vec), i - inline_len)
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the elements in the other vector onto the lhs vector in the
same order as they occurred in other.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(lhs: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(lhs: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <b>let</b> other_len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(&other);
    <b>let</b> half_other_len = other_len / 2;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; half_other_len) {
        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(lhs, <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>(&<b>mut</b> other, i));
        i = i + 1;
    };
    <b>while</b> (i &lt; other_len) {
        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(lhs, <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>(&<b>mut</b> other));
        i = i + 1;
    };
    <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>(other);
}
</code></pre>



</details>

<a name="0x1_smart_vector_push_back"></a>

## Function `push_back`

Add element <code>val</code> to the end of the vector <code>v</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: T) {
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>if</b> (len == inline_len) {
        <b>let</b> bucket_size = <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&v.inline_capacity)) {
            <b>if</b> (len &lt; *<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&v.inline_capacity)) {
                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v.inline_vec, val);
                <b>return</b>
            };
            *<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&v.bucket_size)
        } <b>else</b> {
            <b>let</b> val_size = size_of_val(&val);
            <b>if</b> (val_size * (inline_len + 1) &lt; 150 /* magic number */) {
                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v.inline_vec, val);
                <b>return</b>
            };
            <b>let</b> estimated_avg_size = max((size_of_val(&v.inline_vec) + val_size) / (inline_len + 1), 1);
            max(1024 /* free_write_quota */ / estimated_avg_size, 1)
        };
        <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(&<b>mut</b> v.big_vec, <a href="big_vector.md#0x1_big_vector_empty">big_vector::empty</a>(bucket_size));
    };
    <a href="big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> v.big_vec), val);
}
</code></pre>



</details>

<a name="0x1_smart_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>v</code>. It does shrink the buckets if they're empty.
Aborts if <code>v</code> is empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): T {
    <b>assert</b>!(!<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="smart_vector.md#0x1_smart_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));
    <b>let</b> big_vec_wrapper = &<b>mut</b> v.big_vec;
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(big_vec_wrapper)) {
        <b>let</b> big_vec = <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
        <b>let</b> val = <a href="big_vector.md#0x1_big_vector_pop_back">big_vector::pop_back</a>(&<b>mut</b> big_vec);
        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
        } <b>else</b> {
            <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
        };
        val
    } <b>else</b> {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> v.inline_vec)
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_remove"></a>

## Function `remove`

Remove the element at index i in the vector v and return the owned value that was previously stored at i in v.
All elements occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): T {
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);
    <b>assert</b>!(i &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>if</b> (i &lt; inline_len) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> v.inline_vec, i)
    } <b>else</b> {
        <b>let</b> big_vec_wrapper = &<b>mut</b> v.big_vec;
        <b>let</b> big_vec = <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
        <b>let</b> val = <a href="big_vector.md#0x1_big_vector_remove">big_vector::remove</a>(&<b>mut</b> big_vec, i - inline_len);
        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
        } <b>else</b> {
            <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
        };
        val
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>v</code> with the last element and then pop the vector.
This is O(1), but does not preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): T {
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);
    <b>assert</b>!(i &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>let</b> big_vec_wrapper = &<b>mut</b> v.big_vec;
    <b>let</b> inline_vec = &<b>mut</b> v.inline_vec;
    <b>if</b> (i &gt;= inline_len) {
        <b>let</b> big_vec = <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
        <b>let</b> val = <a href="big_vector.md#0x1_big_vector_swap_remove">big_vector::swap_remove</a>(&<b>mut</b> big_vec, i - inline_len);
        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
        } <b>else</b> {
            <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
        };
        val
    } <b>else</b> {
        <b>if</b> (inline_len &lt; len) {
            <b>let</b> big_vec = <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
            <b>let</b> last_from_big_vec = <a href="big_vector.md#0x1_big_vector_pop_back">big_vector::pop_back</a>(&<b>mut</b> big_vec);
            <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
                <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
            } <b>else</b> {
                <a href="../../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
            };
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(inline_vec, last_from_big_vec);
        };
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(inline_vec, i)
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_swap"></a>

## Function `swap`

Swap the elements at the i'th and j'th indices in the vector v. Will abort if either of i or j are out of bounds
for v.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64, j: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64, j: u64) {
    <b>if</b> (i &gt; j) {
        <b>return</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>(v, j, i)
    };
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);
    <b>assert</b>!(j &lt; len, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>if</b> (i &gt;= inline_len) {
        <a href="big_vector.md#0x1_big_vector_swap">big_vector::swap</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> v.big_vec), i - inline_len, j - inline_len);
    } <b>else</b> <b>if</b> (j &lt; inline_len) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(&<b>mut</b> v.inline_vec, i, j);
    } <b>else</b> {
        <b>let</b> big_vec = <a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> v.big_vec);
        <b>let</b> inline_vec = &<b>mut</b> v.inline_vec;
        <b>let</b> element_i = <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(inline_vec, i);
        <b>let</b> element_j = <a href="big_vector.md#0x1_big_vector_swap_remove">big_vector::swap_remove</a>(big_vec, j - inline_len);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(inline_vec, element_j);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap">vector::swap</a>(inline_vec, i, inline_len - 1);
        <a href="big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(big_vec, element_i);
        <a href="big_vector.md#0x1_big_vector_swap">big_vector::swap</a>(big_vec, j - inline_len, len - inline_len - 1);
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_reverse"></a>

## Function `reverse`

Reverse the order of the elements in the vector v in-place.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_reverse">reverse</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_reverse">reverse</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <b>let</b> inline_len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec);
    <b>let</b> i = 0;
    <b>let</b> new_inline_vec = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    // Push the last `inline_len` elements into a temp <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>.
    <b>while</b> (i &lt; inline_len) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_inline_vec, <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>(v));
        i = i + 1;
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> new_inline_vec);
    // Reverse the <a href="big_vector.md#0x1_big_vector">big_vector</a> left <b>if</b> <b>exists</b>.
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&v.big_vec)) {
        <a href="big_vector.md#0x1_big_vector_reverse">big_vector::reverse</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> v.big_vec));
    };
    // Mem::swap the two vectors.
    <b>let</b> temp_vec = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>while</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<b>mut</b> v.inline_vec)) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> temp_vec, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> v.inline_vec));
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> temp_vec);
    <b>while</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<b>mut</b> new_inline_vec)) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v.inline_vec, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> new_inline_vec));
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(new_inline_vec);
    // Push the rest elements originally left in inline_vector back <b>to</b> the end of the smart <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>.
    <b>while</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<b>mut</b> temp_vec)) {
        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(v, <a href="../../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> temp_vec));
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(temp_vec);
}
</code></pre>



</details>

<a name="0x1_smart_vector_index_of"></a>

## Function `index_of`

Return <code>(<b>true</b>, i)</code> if <code>val</code> is in the vector <code>v</code> at index <code>i</code>.
Otherwise, returns <code>(<b>false</b>, 0)</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: &T): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: &T): (bool, u64) {
    <b>let</b> (found, i) = <a href="../../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&v.inline_vec, val);
    <b>if</b> (found) {
        (<b>true</b>, i)
    } <b>else</b> <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&v.big_vec)) {
        <b>let</b> (found, i) = <a href="big_vector.md#0x1_big_vector_index_of">big_vector::index_of</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&v.big_vec), val);
        (found, i + <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec))
    } <b>else</b> {
        (<b>false</b>, 0)
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_contains"></a>

## Function `contains`

Return true if <code>val</code> is in the vector <code>v</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_contains">contains</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: &T): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_contains">contains</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: &T): bool {
    <b>if</b> (<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v)) <b>return</b> <b>false</b>;
    <b>let</b> (exist, _) = <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>(v, val);
    exist
}
</code></pre>



</details>

<a name="0x1_smart_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): u64 {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&v.inline_vec) + <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&v.big_vec)) {
        0
    } <b>else</b> {
        <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&v.big_vec))
    }
}
</code></pre>



</details>

<a name="0x1_smart_vector_is_empty"></a>

## Function `is_empty`

Return <code><b>true</b></code> if the vector <code>v</code> has no elements and <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): bool {
    <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v) == 0
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification


<a name="@Specification_1_SmartVector"></a>

### Struct `SmartVector`


<pre><code><b>struct</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; <b>has</b> store
</code></pre>



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



<pre><code><b>invariant</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(bucket_size)
    || (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(bucket_size) && <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(bucket_size) != 0);
<b>invariant</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(inline_capacity)
    || (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(inline_vec) &lt;= <a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(inline_capacity));
<b>invariant</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(inline_capacity) && <a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(bucket_size))
    || (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(inline_capacity) && <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(bucket_size));
</code></pre>



<a name="@Specification_1_empty"></a>

### Function `empty`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a name="@Specification_1_empty_with_config"></a>

### Function `empty_with_config`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> bucket_size == 0;
</code></pre>



<a name="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(v: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>aborts_if</b> !(<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v));
<b>aborts_if</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v.inline_vec) != 0
    || <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec);
</code></pre>



<a name="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(v: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &T
</code></pre>




<pre><code><b>aborts_if</b> i &gt;= <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);
<b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) && (
    (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec))) &gt; MAX_U64
);
</code></pre>



<a name="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(lhs: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a name="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: T)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a name="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): T
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b>  <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec)
    &&
    (<a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec).buckets) == 0);
<b>aborts_if</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(v);
<b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) && (
    (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec))) &gt; MAX_U64
);
<b>ensures</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v) == <a href="smart_vector.md#0x1_smart_vector_length">length</a>(<b>old</b>(v)) - 1;
</code></pre>



<a name="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a name="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b> i &gt;= <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v);
<b>aborts_if</b> <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(v.big_vec) && (
    (<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(v.big_vec))) &gt; MAX_U64
);
<b>ensures</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>(v) == <a href="smart_vector.md#0x1_smart_vector_length">length</a>(<b>old</b>(v)) - 1;
</code></pre>



<a name="@Specification_1_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(v: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64, j: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
