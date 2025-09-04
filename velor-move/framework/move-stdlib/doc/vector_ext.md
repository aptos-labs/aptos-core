
<a id="0x1_vector_ext"></a>

# Module `0x1::vector_ext`



-  [Constants](#@Constants_0)
-  [Function `range_move`](#0x1_vector_ext_range_move)
-  [Function `split_off`](#0x1_vector_ext_split_off)
-  [Function `append`](#0x1_vector_ext_append)
-  [Function `insert`](#0x1_vector_ext_insert)
-  [Function `remove`](#0x1_vector_ext_remove)


<pre><code><b>use</b> <a href="vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_vector_ext_EINDEX_OUT_OF_BOUNDS"></a>

The index into the vector is out of bounds


<pre><code><b>const</b> <a href="vector_ext.md#0x1_vector_ext_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 = 131072;
</code></pre>



<a id="0x1_vector_ext_range_move"></a>

## Function `range_move`



<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_range_move">range_move</a>&lt;T&gt;(from: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;T&gt;, removal_position: u64, length: u64, <b>to</b>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;T&gt;, insert_position: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_range_move">range_move</a>&lt;T&gt;(from: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;T&gt;, removal_position: u64, length: u64, <b>to</b>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;T&gt;, insert_position: u64);
</code></pre>



</details>

<a id="0x1_vector_ext_split_off"></a>

## Function `split_off`

Splits the collection into two at the given index.
Returns a newly allocated vector containing the elements in the range [at, len).
After the call, the original vector will be left containing the elements [0, at)
with its previous capacity unchanged.


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_split_off">split_off</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, at: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_split_off">split_off</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, at: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; {
    <b>let</b> len = <a href="vector.md#0x1_vector_length">vector::length</a>(self);
    <b>assert</b>!(at &lt;= len, <a href="vector_ext.md#0x1_vector_ext_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>);

    <b>let</b> other = <a href="vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="vector_ext.md#0x1_vector_ext_range_move">range_move</a>(self, at, len - at, &<b>mut</b> other, 0);

    // <b>let</b> other = empty();
    // <b>while</b> (len &gt; at) {
    //     push_back(&<b>mut</b> other, pop_back(self));
    //     len = len - 1;
    // };
    // reverse(&<b>mut</b> other);
    other
}
</code></pre>



</details>

<a id="0x1_vector_ext_append"></a>

## Function `append`

Pushes all of the elements of the <code>other</code> vector into the <code>self</code> vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_append">append</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_append">append</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;) {
    <b>let</b> self_length = self.length();
    <b>let</b> other_length = other.length();
    <a href="vector_ext.md#0x1_vector_ext_range_move">range_move</a>(&<b>mut</b> other, 0, other_length, self, self_length);
    other.destroy_empty();
    // reverse(&<b>mut</b> other);
    // reverse_append(self, other);
}
</code></pre>



</details>

<a id="0x1_vector_ext_insert"></a>

## Function `insert`



<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_insert">insert</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, e: Element)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_insert">insert</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, e: Element) {
    <b>let</b> len = self.length();
    <b>assert</b>!(i &lt;= len, <a href="vector_ext.md#0x1_vector_ext_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>);

    <b>if</b> (i == len) {
        self.push_back(e);
    } <b>else</b> {
        <b>let</b> other = <a href="vector.md#0x1_vector_singleton">vector::singleton</a>(e);
        <a href="vector_ext.md#0x1_vector_ext_range_move">range_move</a>(&<b>mut</b> other, 0, 1, self, i);
        other.destroy_empty();
    }
}
</code></pre>



</details>

<a id="0x1_vector_ext_remove"></a>

## Function `remove`

Remove the <code>i</code>th element of the vector <code>self</code>, shifting all subsequent elements.
This is O(n) and preserves ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_remove">remove</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector_ext.md#0x1_vector_ext_remove">remove</a>&lt;Element&gt;(self: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element {
    <b>let</b> len = self.length();
    // i out of bounds; <b>abort</b>
    <b>if</b> (i &gt;= len) <b>abort</b> <a href="vector_ext.md#0x1_vector_ext_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>;

    <b>if</b> (i + 1 == len) {
        self.pop_back()
    } <b>else</b> {
        <b>let</b> other = <a href="vector.md#0x1_vector_empty">vector::empty</a>();
        <a href="vector_ext.md#0x1_vector_ext_range_move">range_move</a>(self, i, 1, &<b>mut</b> other, 0);
        <b>let</b> result = other.pop_back();
        other.destroy_empty();
        result
    }
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
