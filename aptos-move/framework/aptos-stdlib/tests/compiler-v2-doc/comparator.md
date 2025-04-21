
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


<pre><code><b>use</b> <a href="../../../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs">0x1::bcs</a>;
</code></pre>



<a id="0x1_comparator_Result"></a>

## Struct `Result`



<pre><code><b>struct</b> <a href="comparator.md#0x1_comparator_Result">Result</a> <b>has</b> drop
</code></pre>



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



<pre><code><b>const</b> <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>: u8 = 0;
</code></pre>



<a id="0x1_comparator_GREATER"></a>



<pre><code><b>const</b> <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>: u8 = 2;
</code></pre>



<a id="0x1_comparator_SMALLER"></a>



<pre><code><b>const</b> <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>: u8 = 1;
</code></pre>



<a id="0x1_comparator_is_equal"></a>

## Function `is_equal`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_equal">is_equal</a>(self: &<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_equal">is_equal</a>(self: &<a href="comparator.md#0x1_comparator_Result">Result</a>): bool {
    self.inner == <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>
}
</code></pre>



</details>

<a id="0x1_comparator_is_smaller_than"></a>

## Function `is_smaller_than`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_smaller_than">is_smaller_than</a>(self: &<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_smaller_than">is_smaller_than</a>(self: &<a href="comparator.md#0x1_comparator_Result">Result</a>): bool {
    self.inner == <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>
}
</code></pre>



</details>

<a id="0x1_comparator_is_greater_than"></a>

## Function `is_greater_than`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_greater_than">is_greater_than</a>(self: &<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_greater_than">is_greater_than</a>(self: &<a href="comparator.md#0x1_comparator_Result">Result</a>): bool {
    self.inner == <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>
}
</code></pre>



</details>

<a id="0x1_comparator_compare"></a>

## Function `compare`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare">compare</a>&lt;T&gt;(left: &T, right: &T): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare">compare</a>&lt;T&gt;(left: &T, right: &T): <a href="comparator.md#0x1_comparator_Result">Result</a> {
    <b>let</b> left_bytes = <a href="../../../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(left);
    <b>let</b> right_bytes = <a href="../../../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(right);

    <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left_bytes, right_bytes)
}
</code></pre>



</details>

<a id="0x1_comparator_compare_u8_vector"></a>

## Function `compare_u8_vector`



<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">Result</a> {
    <b>let</b> left_length = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&left);
    <b>let</b> right_length = <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&right);

    <b>let</b> idx = 0;

    <b>while</b> (idx &lt; left_length && idx &lt; right_length) {
        <b>let</b> left_byte = *<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&left, idx);
        <b>let</b> right_byte = *<a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&right, idx);

        <b>if</b> (left_byte &lt; right_byte) {
            <b>return</b> <a href="comparator.md#0x1_comparator_Result">Result</a> { inner: <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a> }
        } <b>else</b> <b>if</b> (left_byte &gt; right_byte) {
            <b>return</b> <a href="comparator.md#0x1_comparator_Result">Result</a> { inner: <a href="comparator.md#0x1_comparator_GREATER">GREATER</a> }
        };
        idx = idx + 1;
    };

    <b>if</b> (left_length &lt; right_length) {
        <a href="comparator.md#0x1_comparator_Result">Result</a> { inner: <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a> }
    } <b>else</b> <b>if</b> (left_length &gt; right_length) {
        <a href="comparator.md#0x1_comparator_Result">Result</a> { inner: <a href="comparator.md#0x1_comparator_GREATER">GREATER</a> }
    } <b>else</b> {
        <a href="comparator.md#0x1_comparator_Result">Result</a> { inner: <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a> }
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Result"></a>

### Struct `Result`


<pre><code><b>struct</b> <a href="comparator.md#0x1_comparator_Result">Result</a> <b>has</b> drop
</code></pre>



<dl>
<dt>
<code>inner: u8</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> inner == <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a> || inner == <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a> || inner == <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>;
</code></pre>



<a id="@Specification_1_is_equal"></a>

### Function `is_equal`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_equal">is_equal</a>(self: &<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>let</b> res = self;
<b>ensures</b> result == (res.inner == <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>);
</code></pre>



<a id="@Specification_1_is_smaller_than"></a>

### Function `is_smaller_than`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_smaller_than">is_smaller_than</a>(self: &<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>let</b> res = self;
<b>ensures</b> result == (res.inner == <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>);
</code></pre>



<a id="@Specification_1_is_greater_than"></a>

### Function `is_greater_than`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_is_greater_than">is_greater_than</a>(self: &<a href="comparator.md#0x1_comparator_Result">comparator::Result</a>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>let</b> res = self;
<b>ensures</b> result == (res.inner == <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>);
</code></pre>



<a id="@Specification_1_compare"></a>

### Function `compare`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare">compare</a>&lt;T&gt;(left: &T, right: &T): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a>
</code></pre>




<pre><code><b>let</b> left_bytes = <a href="../../../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(left);
<b>let</b> right_bytes = <a href="../../../move-stdlib/tests/compiler-v2-doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(right);
<b>ensures</b> result == <a href="comparator.md#0x1_comparator_spec_compare_u8_vector">spec_compare_u8_vector</a>(left_bytes, right_bytes);
</code></pre>




<a id="0x1_comparator_spec_compare_u8_vector"></a>


<pre><code><b>fun</b> <a href="comparator.md#0x1_comparator_spec_compare_u8_vector">spec_compare_u8_vector</a>(left: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">Result</a>;
</code></pre>



<a id="@Specification_1_compare_u8_vector"></a>

### Function `compare_u8_vector`


<pre><code><b>public</b> <b>fun</b> <a href="comparator.md#0x1_comparator_compare_u8_vector">compare_u8_vector</a>(left: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, right: <a href="../../../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="comparator.md#0x1_comparator_Result">comparator::Result</a>
</code></pre>




<pre><code><b>pragma</b> unroll = 5;
<b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>let</b> left_length = len(left);
<b>let</b> right_length = len(right);
<b>ensures</b> (result.inner == <a href="comparator.md#0x1_comparator_EQUAL">EQUAL</a>) ==&gt; (
    (left_length == right_length) &&
        (<b>forall</b> i: u64 <b>where</b> i &lt; left_length: left[i] == right[i])
);
<b>ensures</b> (result.inner == <a href="comparator.md#0x1_comparator_SMALLER">SMALLER</a>) ==&gt; (
    (<b>exists</b> i: u64 <b>where</b> i &lt; left_length:
        (i &lt; right_length) &&
            (left[i] &lt; right[i]) &&
            (<b>forall</b> j: u64 <b>where</b> j &lt; i: left[j] == right[j])
    ) ||
        (left_length &lt; right_length)
);
<b>ensures</b> (result.inner == <a href="comparator.md#0x1_comparator_GREATER">GREATER</a>) ==&gt; (
    (<b>exists</b> i: u64 <b>where</b> i &lt; left_length:
        (i &lt; right_length) &&
            (left[i] &gt; right[i]) &&
            (<b>forall</b> j: u64 <b>where</b> j &lt; i: left[j] == right[j])
    ) ||
        (left_length &gt; right_length)
);
<b>ensures</b> [abstract] result == <a href="comparator.md#0x1_comparator_spec_compare_u8_vector">spec_compare_u8_vector</a>(left, right);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
