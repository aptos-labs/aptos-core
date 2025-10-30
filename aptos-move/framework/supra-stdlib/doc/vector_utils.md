
<a id="0x1_vector_utils"></a>

# Module `0x1::vector_utils`



-  [Constants](#@Constants_0)
-  [Function `replace`](#0x1_vector_utils_replace)
-  [Function `sort_vector_u64`](#0x1_vector_utils_sort_vector_u64)
-  [Function `sort_vector_u64_by_keys`](#0x1_vector_utils_sort_vector_u64_by_keys)
-  [Function `native_sort_vector_u64`](#0x1_vector_utils_native_sort_vector_u64)
-  [Function `native_sort_vector_u64_by_key`](#0x1_vector_utils_native_sort_vector_u64_by_key)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_vector_utils_EINDEX_OUT_OF_BOUNDS"></a>

The index into the vector is out of bounds


<pre><code><b>const</b> <a href="vector_utils.md#0x1_vector_utils_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 = 0;
</code></pre>



<a id="0x1_vector_utils_EVECTORS_LENGTH_MISMATCH"></a>

Input vectors length does not match.


<pre><code><b>const</b> <a href="vector_utils.md#0x1_vector_utils_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a>: u64 = 2;
</code></pre>



<a id="0x1_vector_utils_replace"></a>

## Function `replace`

Replace the <code>i</code>th element of the vector <code>v</code> with the input element.
This is O(1), but does preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_replace">replace</a>&lt;Element&gt;(v: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, element: Element): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_replace">replace</a>&lt;Element&gt;(v: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, element: Element): Element {
    <b>assert</b>!(i &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(v), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="vector_utils.md#0x1_vector_utils_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(v, element);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(v, i)
}
</code></pre>



</details>

<a id="0x1_vector_utils_sort_vector_u64"></a>

## Function `sort_vector_u64`

Sorts values in ascending order.


<pre><code><b>public</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_sort_vector_u64">sort_vector_u64</a>(values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_sort_vector_u64">sort_vector_u64</a>(values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="vector_utils.md#0x1_vector_utils_native_sort_vector_u64">native_sort_vector_u64</a>(values)
}
</code></pre>



</details>

<a id="0x1_vector_utils_sort_vector_u64_by_keys"></a>

## Function `sort_vector_u64_by_keys`

Sorts values based on the input keys in ascending order.
The keys and values should match in length, otherwise function will abort.


<pre><code><b>public</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_sort_vector_u64_by_keys">sort_vector_u64_by_keys</a>(keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_sort_vector_u64_by_keys">sort_vector_u64_by_keys</a>(keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&keys) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&values), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="vector_utils.md#0x1_vector_utils_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a>));
    <a href="vector_utils.md#0x1_vector_utils_native_sort_vector_u64_by_key">native_sort_vector_u64_by_key</a>(keys, values)
}
</code></pre>



</details>

<a id="0x1_vector_utils_native_sort_vector_u64"></a>

## Function `native_sort_vector_u64`

Sorts values in ascending order.


<pre><code><b>fun</b> <a href="vector_utils.md#0x1_vector_utils_native_sort_vector_u64">native_sort_vector_u64</a>(values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_native_sort_vector_u64">native_sort_vector_u64</a>(values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;;
</code></pre>



</details>

<a id="0x1_vector_utils_native_sort_vector_u64_by_key"></a>

## Function `native_sort_vector_u64_by_key`

Sorts values based on the input keys in ascending order.


<pre><code><b>fun</b> <a href="vector_utils.md#0x1_vector_utils_native_sort_vector_u64_by_key">native_sort_vector_u64_by_key</a>(keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="vector_utils.md#0x1_vector_utils_native_sort_vector_u64_by_key">native_sort_vector_u64_by_key</a>(keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
