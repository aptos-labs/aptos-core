
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


<pre><code><b>use</b> <a href="big_vector.md#0x1_big_vector">0x1::big_vector</a>;
<b>use</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_smart_vector_SmartVector"></a>

## Struct `SmartVector`

A Scalable vector implementation based on tables, Ts are grouped into buckets with <code>bucket_size</code>.
The option wrapping BigVector saves space in the metadata associated with BigVector when smart_vector is
so small that inline_vec vector can hold all the data.


<pre><code><b>struct</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inline_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>big_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inline_capacity: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_smart_vector_EINDEX_OUT_OF_BOUNDS"></a>

Vector index is out of bounds


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 = 1;
</code></pre>



<a id="0x1_smart_vector_EVECTOR_EMPTY"></a>

Cannot pop back from an empty vector


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>: u64 = 3;
</code></pre>



<a id="0x1_smart_vector_EVECTOR_NOT_EMPTY"></a>

Cannot destroy a non-empty vector


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>: u64 = 2;
</code></pre>



<a id="0x1_smart_vector_EZERO_BUCKET_SIZE"></a>

bucket_size cannot be 0


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>: u64 = 4;
</code></pre>



<a id="0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH"></a>

The length of the smart vectors are not equal.


<pre><code><b>const</b> <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a>: u64 = 131077;
</code></pre>



<a id="0x1_smart_vector_new"></a>

## Function `new`

Regular Vector API
Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.
This is exactly the same as empty() but is more standardized as all other data structures have new().


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_new">new</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_new">new</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>()
}
</code></pre>



</details>

<a id="0x1_smart_vector_empty"></a>

## Function `empty`

Create an empty vector using default logic to estimate <code>inline_capacity</code> and <code>bucket_size</code>, which may be
inaccurate.


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> {
        inline_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[],
        big_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>(),
        inline_capacity: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>(),
        bucket_size: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_empty_with_config"></a>

## Function `empty_with_config`

Create an empty vector with customized config.
When inline_capacity = 0, SmartVector degrades to a wrapper of BigVector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <b>assert</b>!(bucket_size &gt; 0, <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EZERO_BUCKET_SIZE">EZERO_BUCKET_SIZE</a>));
    <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> {
        inline_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[],
        big_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_none">option::none</a>(),
        inline_capacity: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(inline_capacity),
        bucket_size: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_some">option::some</a>(bucket_size),
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_singleton"></a>

## Function `singleton`

Create a vector of length 1 containing the passed in T.


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

<a id="0x1_smart_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>self</code>.
Aborts if <code>self</code> is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <b>assert</b>!(<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(&self), <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EVECTOR_NOT_EMPTY">EVECTOR_NOT_EMPTY</a>));
    <b>let</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a> { inline_vec, big_vec, inline_capacity: _, bucket_size: _ } = self;
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(inline_vec);
    <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(big_vec);
}
</code></pre>



</details>

<a id="0x1_smart_vector_destroy"></a>

## Function `destroy`

Destroy a vector completely when T has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy">destroy</a>&lt;T: drop&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy">destroy</a>&lt;T: drop&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <a href="smart_vector.md#0x1_smart_vector_clear">clear</a>(&<b>mut</b> self);
    <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>(self);
}
</code></pre>



</details>

<a id="0x1_smart_vector_clear"></a>

## Function `clear`

Clear a vector completely when T has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_clear">clear</a>&lt;T: drop&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_clear">clear</a>&lt;T: drop&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    self.inline_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[];
    <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&self.big_vec)) {
        <a href="big_vector.md#0x1_big_vector_destroy">big_vector::destroy</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> self.big_vec));
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th T of the vector <code>self</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): &T {
    <b>assert</b>!(i &lt; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self), <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>if</b> (i &lt; inline_len) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&self.inline_vec, i)
    } <b>else</b> {
        <a href="big_vector.md#0x1_big_vector_borrow">big_vector::borrow</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&self.big_vec), i - inline_len)
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th T in the vector <code>self</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): &<b>mut</b> T {
    <b>assert</b>!(i &lt; <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self), <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>if</b> (i &lt; inline_len) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> self.inline_vec, i)
    } <b>else</b> {
        <a href="big_vector.md#0x1_big_vector_borrow_mut">big_vector::borrow_mut</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> self.big_vec), i - inline_len)
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_append"></a>

## Function `append`

Empty and destroy the other vector, and push each of the Ts in the other vector onto the self vector in the
same order as they occurred in other.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <b>let</b> other_len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(&other);
    <b>let</b> half_other_len = other_len / 2;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; half_other_len) {
        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(self, <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>(&<b>mut</b> other, i));
        i = i + 1;
    };
    <b>while</b> (i &lt; other_len) {
        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(self, <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>(&<b>mut</b> other));
        i = i + 1;
    };
    <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>(other);
}
</code></pre>



</details>

<a id="0x1_smart_vector_add_all"></a>

## Function `add_all`

Add multiple values to the vector at once.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_add_all">add_all</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, vals: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_add_all">add_all</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, vals: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;T&gt;) {
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_for_each">vector::for_each</a>(vals, |val| { <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(self, val); })
}
</code></pre>



</details>

<a id="0x1_smart_vector_to_vector"></a>

## Function `to_vector`

Convert a smart vector to a native vector, which is supposed to be called mostly by view functions to get an
atomic view of the whole vector.
Disclaimer: This function may be costly as the smart vector may be huge in size. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_to_vector">to_vector</a>&lt;T: <b>copy</b>, store&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_to_vector">to_vector</a>&lt;T: store + <b>copy</b>&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> res = self.inline_vec;
    <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&self.big_vec)) {
        <b>let</b> big_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&self.big_vec);
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> res, <a href="big_vector.md#0x1_big_vector_to_vector">big_vector::to_vector</a>(big_vec));
    };
    res
}
</code></pre>



</details>

<a id="0x1_smart_vector_push_back"></a>

## Function `push_back`

Add T <code>val</code> to the end of the vector <code>self</code>. It grows the buckets when the current buckets are full.
This operation will cost more gas when it adds new bucket.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: T) {
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>if</b> (len == inline_len) {
        <b>let</b> bucket_size = <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&self.inline_capacity)) {
            <b>if</b> (len &lt; *<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&self.inline_capacity)) {
                <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> self.inline_vec, val);
                <b>return</b>
            };
            *<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&self.bucket_size)
        } <b>else</b> {
            <b>let</b> val_size = size_of_val(&val);
            <b>if</b> (val_size * (inline_len + 1) &lt; 150 /* magic number */) {
                <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> self.inline_vec, val);
                <b>return</b>
            };
            <b>let</b> estimated_avg_size = max((size_of_val(&self.inline_vec) + val_size) / (inline_len + 1), 1);
            max(1024 /* free_write_quota */ / estimated_avg_size, 1)
        };
        <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_fill">option::fill</a>(&<b>mut</b> self.big_vec, <a href="big_vector.md#0x1_big_vector_empty">big_vector::empty</a>(bucket_size));
    };
    <a href="big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> self.big_vec), val);
}
</code></pre>



</details>

<a id="0x1_smart_vector_pop_back"></a>

## Function `pop_back`

Pop an T from the end of vector <code>self</code>. It does shrink the buckets if they're empty.
Aborts if <code>self</code> is empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): T {
    <b>assert</b>!(!<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(self), <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="smart_vector.md#0x1_smart_vector_EVECTOR_EMPTY">EVECTOR_EMPTY</a>));
    <b>let</b> big_vec_wrapper = &<b>mut</b> self.big_vec;
    <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(big_vec_wrapper)) {
        <b>let</b> big_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
        <b>let</b> val = <a href="big_vector.md#0x1_big_vector_pop_back">big_vector::pop_back</a>(&<b>mut</b> big_vec);
        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
        } <b>else</b> {
            <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
        };
        val
    } <b>else</b> {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> self.inline_vec)
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_remove"></a>

## Function `remove`

Remove the T at index i in the vector self and return the owned value that was previously stored at i in self.
All Ts occurring at indices greater than i will be shifted down by 1. Will abort if i is out of bounds.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): T {
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
    <b>assert</b>!(i &lt; len, <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>if</b> (i &lt; inline_len) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> self.inline_vec, i)
    } <b>else</b> {
        <b>let</b> big_vec_wrapper = &<b>mut</b> self.big_vec;
        <b>let</b> big_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
        <b>let</b> val = <a href="big_vector.md#0x1_big_vector_remove">big_vector::remove</a>(&<b>mut</b> big_vec, i - inline_len);
        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
        } <b>else</b> {
            <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
        };
        val
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th T of the vector <code>self</code> with the last T and then pop the vector.
This is O(1), but does not preserve ordering of Ts in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64): T {
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
    <b>assert</b>!(i &lt; len, <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>let</b> big_vec_wrapper = &<b>mut</b> self.big_vec;
    <b>let</b> inline_vec = &<b>mut</b> self.inline_vec;
    <b>if</b> (i &gt;= inline_len) {
        <b>let</b> big_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
        <b>let</b> val = <a href="big_vector.md#0x1_big_vector_swap_remove">big_vector::swap_remove</a>(&<b>mut</b> big_vec, i - inline_len);
        <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
            <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
        } <b>else</b> {
            <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
        };
        val
    } <b>else</b> {
        <b>if</b> (inline_len &lt; len) {
            <b>let</b> big_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_extract">option::extract</a>(big_vec_wrapper);
            <b>let</b> last_from_big_vec = <a href="big_vector.md#0x1_big_vector_pop_back">big_vector::pop_back</a>(&<b>mut</b> big_vec);
            <b>if</b> (<a href="big_vector.md#0x1_big_vector_is_empty">big_vector::is_empty</a>(&big_vec)) {
                <a href="big_vector.md#0x1_big_vector_destroy_empty">big_vector::destroy_empty</a>(big_vec)
            } <b>else</b> {
                <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_fill">option::fill</a>(big_vec_wrapper, big_vec);
            };
            <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(inline_vec, last_from_big_vec);
        };
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(inline_vec, i)
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_swap"></a>

## Function `swap`

Swap the Ts at the i'th and j'th indices in the vector v. Will abort if either of i or j are out of bounds
for self.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64, j: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, i: u64, j: u64) {
    <b>if</b> (i &gt; j) {
        <b>return</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>(self, j, i)
    };
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
    <b>assert</b>!(j &lt; len, <a href="../../../move-stdlib/tests/compiler-v2-doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_vector.md#0x1_smart_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>if</b> (i &gt;= inline_len) {
        <a href="big_vector.md#0x1_big_vector_swap">big_vector::swap</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> self.big_vec), i - inline_len, j - inline_len);
    } <b>else</b> <b>if</b> (j &lt; inline_len) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap">vector::swap</a>(&<b>mut</b> self.inline_vec, i, j);
    } <b>else</b> {
        <b>let</b> big_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> self.big_vec);
        <b>let</b> inline_vec = &<b>mut</b> self.inline_vec;
        <b>let</b> element_i = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(inline_vec, i);
        <b>let</b> element_j = <a href="big_vector.md#0x1_big_vector_swap_remove">big_vector::swap_remove</a>(big_vec, j - inline_len);
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(inline_vec, element_j);
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_swap">vector::swap</a>(inline_vec, i, inline_len - 1);
        <a href="big_vector.md#0x1_big_vector_push_back">big_vector::push_back</a>(big_vec, element_i);
        <a href="big_vector.md#0x1_big_vector_swap">big_vector::swap</a>(big_vec, j - inline_len, len - inline_len - 1);
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_reverse"></a>

## Function `reverse`

Reverse the order of the Ts in the vector self in-place.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_reverse">reverse</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_reverse">reverse</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;) {
    <b>let</b> inline_len = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec);
    <b>let</b> i = 0;
    <b>let</b> new_inline_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[];
    // Push the last `inline_len` Ts into a temp <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>.
    <b>while</b> (i &lt; inline_len) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_inline_vec, <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>(self));
        i = i + 1;
    };
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> new_inline_vec);
    // Reverse the <a href="big_vector.md#0x1_big_vector">big_vector</a> left <b>if</b> <b>exists</b>.
    <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&self.big_vec)) {
        <a href="big_vector.md#0x1_big_vector_reverse">big_vector::reverse</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> self.big_vec));
    };
    // Mem::swap the two vectors.
    <b>let</b> temp_vec = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[];
    <b>while</b> (!<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<b>mut</b> self.inline_vec)) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> temp_vec, <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> self.inline_vec));
    };
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> temp_vec);
    <b>while</b> (!<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<b>mut</b> new_inline_vec)) {
        <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> self.inline_vec, <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> new_inline_vec));
    };
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(new_inline_vec);
    // Push the rest Ts originally left in inline_vector back <b>to</b> the end of the smart <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>.
    <b>while</b> (!<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&<b>mut</b> temp_vec)) {
        <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(self, <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> temp_vec));
    };
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(temp_vec);
}
</code></pre>



</details>

<a id="0x1_smart_vector_index_of"></a>

## Function `index_of`

Return <code>(<b>true</b>, i)</code> if <code>val</code> is in the vector <code>self</code> at index <code>i</code>.
Otherwise, returns <code>(<b>false</b>, 0)</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: &T): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: &T): (bool, u64) {
    <b>let</b> (found, i) = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&self.inline_vec, val);
    <b>if</b> (found) {
        (<b>true</b>, i)
    } <b>else</b> <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&self.big_vec)) {
        <b>let</b> (found, i) = <a href="big_vector.md#0x1_big_vector_index_of">big_vector::index_of</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&self.big_vec), val);
        (found, i + <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec))
    } <b>else</b> {
        (<b>false</b>, 0)
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_contains"></a>

## Function `contains`

Return true if <code>val</code> is in the vector <code>self</code>.
Disclaimer: This function may be costly. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_contains">contains</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: &T): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_contains">contains</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, val: &T): bool {
    <b>if</b> (<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(self)) <b>return</b> <b>false</b>;
    <b>let</b> (exist, _) = <a href="smart_vector.md#0x1_smart_vector_index_of">index_of</a>(self, val);
    exist
}
</code></pre>



</details>

<a id="0x1_smart_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): u64 {
    <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&self.inline_vec) + <b>if</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_none">option::is_none</a>(&self.big_vec)) {
        0
    } <b>else</b> {
        <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&self.big_vec))
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_is_empty"></a>

## Function `is_empty`

Return <code><b>true</b></code> if the vector <code>self</code> has no Ts and <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;): bool {
    <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self) == 0
}
</code></pre>



</details>

<a id="0x1_smart_vector_for_each"></a>

## Function `for_each`

Apply the function to each T in the vector, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each">for_each</a>&lt;T: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: |T|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each">for_each</a>&lt;T: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: |T|) {
    aptos_std::smart_vector::reverse(&<b>mut</b> self); // We need <b>to</b> reverse the <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a> <b>to</b> consume it efficiently
    aptos_std::smart_vector::for_each_reverse(self, |e| f(e));
}
</code></pre>



</details>

<a id="0x1_smart_vector_for_each_reverse"></a>

## Function `for_each_reverse`

Apply the function to each T in the vector, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_reverse">for_each_reverse</a>&lt;T&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: |T|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_reverse">for_each_reverse</a>&lt;T&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: |T|) {
    <b>let</b> len = aptos_std::smart_vector::length(&self);
    <b>while</b> (len &gt; 0) {
        f(aptos_std::smart_vector::pop_back(&<b>mut</b> self));
        len = len - 1;
    };
    aptos_std::smart_vector::destroy_empty(self)
}
</code></pre>



</details>

<a id="0x1_smart_vector_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each T in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_ref">for_each_ref</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: |&T|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_ref">for_each_ref</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: |&T|) {
    <b>let</b> i = 0;
    <b>let</b> len = aptos_std::smart_vector::length(self);
    <b>while</b> (i &lt; len) {
        f(aptos_std::smart_vector::borrow(self, i));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference to each T in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_mut">for_each_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: |&<b>mut</b> T|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_for_each_mut">for_each_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: |&<b>mut</b> T|) {
    <b>let</b> i = 0;
    <b>let</b> len = aptos_std::smart_vector::length(self);
    <b>while</b> (i &lt; len) {
        f(aptos_std::smart_vector::borrow_mut(self, i));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_enumerate_ref"></a>

## Function `enumerate_ref`

Apply the function to a reference of each T in the vector with its index.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_ref">enumerate_ref</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: |(u64, &T)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_ref">enumerate_ref</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: |u64, &T|) {
    <b>let</b> i = 0;
    <b>let</b> len = aptos_std::smart_vector::length(self);
    <b>while</b> (i &lt; len) {
        f(i, aptos_std::smart_vector::borrow(self, i));
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x1_smart_vector_enumerate_mut"></a>

## Function `enumerate_mut`

Apply the function to a mutable reference of each T in the vector with its index.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_mut">enumerate_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, f: |(u64, &<b>mut</b> T)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_enumerate_mut">enumerate_mut</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;, f: |u64, &<b>mut</b> T|) {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
    <b>while</b> (i &lt; len) {
        f(i, <a href="smart_vector.md#0x1_smart_vector_borrow_mut">borrow_mut</a>(self, i));
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x1_smart_vector_fold"></a>

## Function `fold`

Fold the function over the Ts. For example, <code><a href="smart_vector.md#0x1_smart_vector_fold">fold</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[1,2,3], 0, f)</code> will execute
<code>f(f(f(0, 1), 2), 3)</code>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_fold">fold</a>&lt;Accumulator, T: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, init: Accumulator, f: |(Accumulator, T)|Accumulator): Accumulator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_fold">fold</a>&lt;Accumulator, T: store&gt;(
    self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;,
    init: Accumulator,
    f: |Accumulator, T|Accumulator
): Accumulator {
    <b>let</b> accu = init;
    aptos_std::smart_vector::for_each(self, |elem| accu = f(accu, elem));
    accu
}
</code></pre>



</details>

<a id="0x1_smart_vector_foldr"></a>

## Function `foldr`

Fold right like fold above but working right to left. For example, <code><a href="smart_vector.md#0x1_smart_vector_fold">fold</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[1,2,3], 0, f)</code> will execute
<code>f(1, f(2, f(3, 0)))</code>


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_foldr">foldr</a>&lt;Accumulator, T&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, init: Accumulator, f: |(T, Accumulator)|Accumulator): Accumulator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_foldr">foldr</a>&lt;Accumulator, T&gt;(
    self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;,
    init: Accumulator,
    f: |T, Accumulator|Accumulator
): Accumulator {
    <b>let</b> accu = init;
    aptos_std::smart_vector::for_each_reverse(self, |elem| accu = f(elem, accu));
    accu
}
</code></pre>



</details>

<a id="0x1_smart_vector_map_ref"></a>

## Function `map_ref`

Map the function over the references of the Ts of the vector, producing a new vector without modifying the
original vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map_ref">map_ref</a>&lt;T1, T2: store&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, f: |&T1|T2): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map_ref">map_ref</a>&lt;T1, T2: store&gt;(
    self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    f: |&T1|T2
): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt; {
    <b>let</b> result = aptos_std::smart_vector::new&lt;T2&gt;();
    aptos_std::smart_vector::for_each_ref(self, |elem| aptos_std::smart_vector::push_back(&<b>mut</b> result, f(elem)));
    result
}
</code></pre>



</details>

<a id="0x1_smart_vector_map"></a>

## Function `map`

Map the function over the Ts of the vector, producing a new vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map">map</a>&lt;T1: store, T2: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, f: |T1|T2): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_map">map</a>&lt;T1: store, T2: store&gt;(
    self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    f: |T1|T2
): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt; {
    <b>let</b> result = aptos_std::smart_vector::new&lt;T2&gt;();
    aptos_std::smart_vector::for_each(self, |elem| <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&<b>mut</b> result, f(elem)));
    result
}
</code></pre>



</details>

<a id="0x1_smart_vector_filter"></a>

## Function `filter`

Filter the vector using the boolean function, removing all Ts for which <code>p(e)</code> is not true.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_filter">filter</a>&lt;T: drop, store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, p: |&T|bool): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_filter">filter</a>&lt;T: store + drop&gt;(
    self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt;,
    p: |&T|bool
): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; {
    <b>let</b> result = aptos_std::smart_vector::new&lt;T&gt;();
    aptos_std::smart_vector::for_each(self, |elem| {
        <b>if</b> (p(&elem)) aptos_std::smart_vector::push_back(&<b>mut</b> result, elem);
    });
    result
}
</code></pre>



</details>

<a id="0x1_smart_vector_zip"></a>

## Function `zip`



<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip">zip</a>&lt;T1: store, T2: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: |(T1, T2)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip">zip</a>&lt;T1: store, T2: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;, f: |T1, T2|) {
    // We need <b>to</b> reverse the vectors <b>to</b> consume it efficiently
    aptos_std::smart_vector::reverse(&<b>mut</b> self);
    aptos_std::smart_vector::reverse(&<b>mut</b> v2);
    aptos_std::smart_vector::zip_reverse(self, v2, |e1, e2| f(e1, e2));
}
</code></pre>



</details>

<a id="0x1_smart_vector_zip_reverse"></a>

## Function `zip_reverse`

Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_reverse">zip_reverse</a>&lt;T1, T2&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: |(T1, T2)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_reverse">zip_reverse</a>&lt;T1, T2&gt;(
    self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,
    f: |T1, T2|,
) {
    <b>let</b> len = aptos_std::smart_vector::length(&self);
    // We can't <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it
    // due <b>to</b> how inline functions work.
    <b>assert</b>!(len == aptos_std::smart_vector::length(&v2), 0x20005);
    <b>while</b> (len &gt; 0) {
        f(aptos_std::smart_vector::pop_back(&<b>mut</b> self), aptos_std::smart_vector::pop_back(&<b>mut</b> v2));
        len = len - 1;
    };
    aptos_std::smart_vector::destroy_empty(self);
    aptos_std::smart_vector::destroy_empty(v2);
}
</code></pre>



</details>

<a id="0x1_smart_vector_zip_ref"></a>

## Function `zip_ref`

Apply the function to the references of each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_ref">zip_ref</a>&lt;T1, T2&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: |(&T1, &T2)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_ref">zip_ref</a>&lt;T1, T2&gt;(
    self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    v2: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,
    f: |&T1, &T2|,
) {
    <b>let</b> len = aptos_std::smart_vector::length(self);
    // We can't <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it
    // due <b>to</b> how inline functions work.
    <b>assert</b>!(len == aptos_std::smart_vector::length(v2), 0x20005);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        f(aptos_std::smart_vector::borrow(self, i), aptos_std::smart_vector::borrow(v2, i));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_zip_mut"></a>

## Function `zip_mut`

Apply the function to mutable references to each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_mut">zip_mut</a>&lt;T1, T2&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: |(&<b>mut</b> T1, &<b>mut</b> T2)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_mut">zip_mut</a>&lt;T1, T2&gt;(
    self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    v2: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,
    f: |&<b>mut</b> T1, &<b>mut</b> T2|,
) {
    <b>let</b> i = 0;
    <b>let</b> len = aptos_std::smart_vector::length(self);
    // We can't <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it
    // due <b>to</b> how inline functions work.
    <b>assert</b>!(len == aptos_std::smart_vector::length(v2), 0x20005);
    <b>while</b> (i &lt; len) {
        f(aptos_std::smart_vector::borrow_mut(self, i), aptos_std::smart_vector::borrow_mut(v2, i));
        i = i + 1
    }
}
</code></pre>



</details>

<a id="0x1_smart_vector_zip_map"></a>

## Function `zip_map`

Map the function over the element pairs of the two vectors, producing a new vector.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map">zip_map</a>&lt;T1: store, T2: store, NewT: store&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: |(T1, T2)|NewT): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;NewT&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map">zip_map</a>&lt;T1: store, T2: store, NewT: store&gt;(
    self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    v2: <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,
    f: |T1, T2|NewT
): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;NewT&gt; {
    // We can't <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it
    // due <b>to</b> how inline functions work.
    <b>assert</b>!(aptos_std::smart_vector::length(&self) == aptos_std::smart_vector::length(&v2), 0x20005);

    <b>let</b> result = aptos_std::smart_vector::new&lt;NewT&gt;();
    aptos_std::smart_vector::zip(self, v2, |e1, e2| <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&<b>mut</b> result, f(e1, e2)));
    result
}
</code></pre>



</details>

<a id="0x1_smart_vector_zip_map_ref"></a>

## Function `zip_map_ref`

Map the function over the references of the element pairs of two vectors, producing a new vector from the return
values without modifying the original vectors.


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map_ref">zip_map_ref</a>&lt;T1, T2, NewT: store&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T1&gt;, v2: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T2&gt;, f: |(&T1, &T2)|NewT): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;NewT&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_zip_map_ref">zip_map_ref</a>&lt;T1, T2, NewT: store&gt;(
    self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T1&gt;,
    v2: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T2&gt;,
    f: |&T1, &T2|NewT
): <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;NewT&gt; {
    // We can't <b>use</b> the constant <a href="smart_vector.md#0x1_smart_vector_ESMART_VECTORS_LENGTH_MISMATCH">ESMART_VECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it
    // due <b>to</b> how inline functions work.
    <b>assert</b>!(aptos_std::smart_vector::length(self) == aptos_std::smart_vector::length(v2), 0x20005);

    <b>let</b> result = aptos_std::smart_vector::new&lt;NewT&gt;();
    aptos_std::smart_vector::zip_ref(self, v2, |e1, e2| <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>(&<b>mut</b> result, f(e1, e2)));
    result
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SmartVector"></a>

### Struct `SmartVector`


<pre><code><b>struct</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">SmartVector</a>&lt;T&gt; <b>has</b> store
</code></pre>



<dl>
<dt>
<code>inline_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>big_vec: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="big_vector.md#0x1_big_vector_BigVector">big_vector::BigVector</a>&lt;T&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>inline_capacity: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>bucket_size: <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_none">option::is_none</a>(bucket_size)
    || (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(bucket_size) && <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(bucket_size) != 0);
<b>invariant</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_none">option::is_none</a>(inline_capacity)
    || (len(inline_vec) &lt;= <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(inline_capacity));
<b>invariant</b> (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_none">option::is_none</a>(inline_capacity) && <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_none">option::is_none</a>(bucket_size))
    || (<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(inline_capacity) && <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(bucket_size));
</code></pre>



<a id="@Specification_1_empty"></a>

### Function `empty`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty">empty</a>&lt;T: store&gt;(): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_empty_with_config"></a>

### Function `empty_with_config`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_empty_with_config">empty_with_config</a>&lt;T: store&gt;(inline_capacity: u64, bucket_size: u64): <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;
</code></pre>




<pre><code><b>aborts_if</b> bucket_size == 0;
</code></pre>



<a id="@Specification_1_destroy_empty"></a>

### Function `destroy_empty`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_destroy_empty">destroy_empty</a>&lt;T&gt;(self: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>aborts_if</b> !(<a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(self));
<b>aborts_if</b> len(self.inline_vec) != 0
    || <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(self.big_vec);
</code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_borrow">borrow</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): &T
</code></pre>




<pre><code><b>aborts_if</b> i &gt;= <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
<b>aborts_if</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(self.big_vec) && (
    (len(self.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(self.big_vec))) &gt; MAX_U64
);
</code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_append">append</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, other: <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_push_back"></a>

### Function `push_back`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_push_back">push_back</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, val: T)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_pop_back"></a>

### Function `pop_back`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_pop_back">pop_back</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): T
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b>  <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(self.big_vec)
    &&
    (<a href="table_with_length.md#0x1_table_with_length_spec_len">table_with_length::spec_len</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(self.big_vec).buckets) == 0);
<b>aborts_if</b> <a href="smart_vector.md#0x1_smart_vector_is_empty">is_empty</a>(self);
<b>aborts_if</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(self.big_vec) && (
    (len(self.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(self.big_vec))) &gt; MAX_U64
);
<b>ensures</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self) == <a href="smart_vector.md#0x1_smart_vector_length">length</a>(<b>old</b>(self)) - 1;
</code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_remove">remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap_remove">swap_remove</a>&lt;T&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64): T
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> i &gt;= <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self);
<b>aborts_if</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(self.big_vec) && (
    (len(self.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>&lt;T&gt;(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(self.big_vec))) &gt; MAX_U64
);
<b>ensures</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>(self) == <a href="smart_vector.md#0x1_smart_vector_length">length</a>(<b>old</b>(self)) - 1;
</code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_swap">swap</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;, i: u64, j: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_length"></a>

### Function `length`


<pre><code><b>public</b> <b>fun</b> <a href="smart_vector.md#0x1_smart_vector_length">length</a>&lt;T&gt;(self: &<a href="smart_vector.md#0x1_smart_vector_SmartVector">smart_vector::SmartVector</a>&lt;T&gt;): u64
</code></pre>




<pre><code><b>aborts_if</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(self.big_vec) && len(self.inline_vec) + <a href="big_vector.md#0x1_big_vector_length">big_vector::length</a>(<a href="../../../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(
    self.big_vec)) &gt; MAX_U64;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
