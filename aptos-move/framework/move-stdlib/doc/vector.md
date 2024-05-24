
<a id="0x1_vector"></a>

# Module `0x1::vector`

A variable&#45;sized container that can hold any type. Indexing is 0&#45;based, and
vectors are growable. This module has many native functions.
Verification of modules that use this one uses model functions that are implemented
directly in Boogie. The specification language has built&#45;in functions operations such
as <code>singleton_vector</code>. There are some helper functions defined here for specifications in other
modules as well.

&gt;Note: We did not verify most of the
Move functions here because many have loops, requiring loop invariants to prove, and
the return on investment didn&apos;t seem worth it for these simple functions.


-  [Constants](#@Constants_0)
-  [Function `empty`](#0x1_vector_empty)
-  [Function `length`](#0x1_vector_length)
-  [Function `borrow`](#0x1_vector_borrow)
-  [Function `push_back`](#0x1_vector_push_back)
-  [Function `borrow_mut`](#0x1_vector_borrow_mut)
-  [Function `pop_back`](#0x1_vector_pop_back)
-  [Function `destroy_empty`](#0x1_vector_destroy_empty)
-  [Function `swap`](#0x1_vector_swap)
-  [Function `singleton`](#0x1_vector_singleton)
-  [Function `reverse`](#0x1_vector_reverse)
-  [Function `reverse_slice`](#0x1_vector_reverse_slice)
-  [Function `append`](#0x1_vector_append)
-  [Function `reverse_append`](#0x1_vector_reverse_append)
-  [Function `trim`](#0x1_vector_trim)
-  [Function `trim_reverse`](#0x1_vector_trim_reverse)
-  [Function `is_empty`](#0x1_vector_is_empty)
-  [Function `contains`](#0x1_vector_contains)
-  [Function `index_of`](#0x1_vector_index_of)
-  [Function `find`](#0x1_vector_find)
-  [Function `insert`](#0x1_vector_insert)
-  [Function `remove`](#0x1_vector_remove)
-  [Function `remove_value`](#0x1_vector_remove_value)
-  [Function `swap_remove`](#0x1_vector_swap_remove)
-  [Function `for_each`](#0x1_vector_for_each)
-  [Function `for_each_reverse`](#0x1_vector_for_each_reverse)
-  [Function `for_each_ref`](#0x1_vector_for_each_ref)
-  [Function `zip`](#0x1_vector_zip)
-  [Function `zip_reverse`](#0x1_vector_zip_reverse)
-  [Function `zip_ref`](#0x1_vector_zip_ref)
-  [Function `enumerate_ref`](#0x1_vector_enumerate_ref)
-  [Function `for_each_mut`](#0x1_vector_for_each_mut)
-  [Function `zip_mut`](#0x1_vector_zip_mut)
-  [Function `enumerate_mut`](#0x1_vector_enumerate_mut)
-  [Function `fold`](#0x1_vector_fold)
-  [Function `foldr`](#0x1_vector_foldr)
-  [Function `map_ref`](#0x1_vector_map_ref)
-  [Function `zip_map_ref`](#0x1_vector_zip_map_ref)
-  [Function `map`](#0x1_vector_map)
-  [Function `zip_map`](#0x1_vector_zip_map)
-  [Function `filter`](#0x1_vector_filter)
-  [Function `partition`](#0x1_vector_partition)
-  [Function `rotate`](#0x1_vector_rotate)
-  [Function `rotate_slice`](#0x1_vector_rotate_slice)
-  [Function `stable_partition`](#0x1_vector_stable_partition)
-  [Function `any`](#0x1_vector_any)
-  [Function `all`](#0x1_vector_all)
-  [Function `destroy`](#0x1_vector_destroy)
-  [Function `range`](#0x1_vector_range)
-  [Function `range_with_step`](#0x1_vector_range_with_step)
-  [Function `slice`](#0x1_vector_slice)
-  [Specification](#@Specification_1)
    -  [Helper Functions](#@Helper_Functions_2)
    -  [Function `singleton`](#@Specification_1_singleton)
    -  [Function `reverse`](#@Specification_1_reverse)
    -  [Function `reverse_slice`](#@Specification_1_reverse_slice)
    -  [Function `append`](#@Specification_1_append)
    -  [Function `reverse_append`](#@Specification_1_reverse_append)
    -  [Function `trim`](#@Specification_1_trim)
    -  [Function `trim_reverse`](#@Specification_1_trim_reverse)
    -  [Function `is_empty`](#@Specification_1_is_empty)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `index_of`](#@Specification_1_index_of)
    -  [Function `insert`](#@Specification_1_insert)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `remove_value`](#@Specification_1_remove_value)
    -  [Function `swap_remove`](#@Specification_1_swap_remove)
    -  [Function `rotate`](#@Specification_1_rotate)
    -  [Function `rotate_slice`](#@Specification_1_rotate_slice)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_vector_EINDEX_OUT_OF_BOUNDS"></a>

The index into the vector is out of bounds


<pre><code><b>const</b> <a href="vector.md#0x1_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 &#61; 131072;<br /></code></pre>



<a id="0x1_vector_EINVALID_RANGE"></a>

The index into the vector is out of bounds


<pre><code><b>const</b> <a href="vector.md#0x1_vector_EINVALID_RANGE">EINVALID_RANGE</a>: u64 &#61; 131073;<br /></code></pre>



<a id="0x1_vector_EINVALID_SLICE_RANGE"></a>

The range in <code>slice</code> is invalid.


<pre><code><b>const</b> <a href="vector.md#0x1_vector_EINVALID_SLICE_RANGE">EINVALID_SLICE_RANGE</a>: u64 &#61; 131076;<br /></code></pre>



<a id="0x1_vector_EINVALID_STEP"></a>

The step provided in <code>range</code> is invalid, must be greater than zero.


<pre><code><b>const</b> <a href="vector.md#0x1_vector_EINVALID_STEP">EINVALID_STEP</a>: u64 &#61; 131075;<br /></code></pre>



<a id="0x1_vector_EVECTORS_LENGTH_MISMATCH"></a>

The length of the vectors are not equal.


<pre><code><b>const</b> <a href="vector.md#0x1_vector_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a>: u64 &#61; 131074;<br /></code></pre>



<a id="0x1_vector_empty"></a>

## Function `empty`

Create an empty vector.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_empty">empty</a>&lt;Element&gt;(): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_empty">empty</a>&lt;Element&gt;(): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;;<br /></code></pre>



</details>

<a id="0x1_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_length">length</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_length">length</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): u64;<br /></code></pre>



</details>

<a id="0x1_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_borrow">borrow</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): &amp;Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_borrow">borrow</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): &amp;Element;<br /></code></pre>



</details>

<a id="0x1_vector_push_back"></a>

## Function `push_back`

Add element <code>e</code> to the end of the vector <code>v</code>.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_push_back">push_back</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: Element)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_push_back">push_back</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: Element);<br /></code></pre>



</details>

<a id="0x1_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_borrow_mut">borrow_mut</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): &amp;<b>mut</b> Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_borrow_mut">borrow_mut</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): &amp;<b>mut</b> Element;<br /></code></pre>



</details>

<a id="0x1_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>v</code>.
Aborts if <code>v</code> is empty.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_pop_back">pop_back</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_pop_back">pop_back</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): Element;<br /></code></pre>



</details>

<a id="0x1_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_destroy_empty">destroy_empty</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_destroy_empty">destroy_empty</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;);<br /></code></pre>



</details>

<a id="0x1_vector_swap"></a>

## Function `swap`

Swaps the elements at the <code>i</code>th and <code>j</code>th indices in the vector <code>v</code>.
Aborts if <code>i</code> or <code>j</code> is out of bounds.


<pre><code>&#35;[bytecode_instruction]<br /><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_swap">swap</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, j: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_swap">swap</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, j: u64);<br /></code></pre>



</details>

<a id="0x1_vector_singleton"></a>

## Function `singleton`

Return an vector of size one containing element <code>e</code>.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_singleton">singleton</a>&lt;Element&gt;(e: Element): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_singleton">singleton</a>&lt;Element&gt;(e: Element): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    <b>let</b> v &#61; <a href="vector.md#0x1_vector_empty">empty</a>();<br />    <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> v, e);<br />    v<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_reverse"></a>

## Function `reverse`

Reverses the order of the elements in the vector <code>v</code> in place.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse">reverse</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse">reverse</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;) &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>(v, 0, len);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_reverse_slice"></a>

## Function `reverse_slice`

Reverses the order of the elements [left, right) in the vector <code>v</code> in place.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, left: u64, right: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, left: u64, right: u64) &#123;<br />    <b>assert</b>!(left &lt;&#61; right, <a href="vector.md#0x1_vector_EINVALID_RANGE">EINVALID_RANGE</a>);<br />    <b>if</b> (left &#61;&#61; right) <b>return</b>;<br />    right &#61; right &#45; 1;<br />    <b>while</b> (left &lt; right) &#123;<br />        <a href="vector.md#0x1_vector_swap">swap</a>(v, left, right);<br />        left &#61; left &#43; 1;<br />        right &#61; right &#45; 1;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_append"></a>

## Function `append`

Pushes all of the elements of the <code>other</code> vector into the <code>lhs</code> vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_append">append</a>&lt;Element&gt;(lhs: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_append">append</a>&lt;Element&gt;(lhs: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;) &#123;<br />    <a href="vector.md#0x1_vector_reverse">reverse</a>(&amp;<b>mut</b> other);<br />    <a href="vector.md#0x1_vector_reverse_append">reverse_append</a>(lhs, other);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_reverse_append"></a>

## Function `reverse_append`

Pushes all of the elements of the <code>other</code> vector into the <code>lhs</code> vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse_append">reverse_append</a>&lt;Element&gt;(lhs: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse_append">reverse_append</a>&lt;Element&gt;(lhs: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;) &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(&amp;other);<br />    <b>while</b> (len &gt; 0) &#123;<br />        <a href="vector.md#0x1_vector_push_back">push_back</a>(lhs, <a href="vector.md#0x1_vector_pop_back">pop_back</a>(&amp;<b>mut</b> other));<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    <a href="vector.md#0x1_vector_destroy_empty">destroy_empty</a>(other);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_trim"></a>

## Function `trim`

Trim a vector to a smaller size, returning the evicted elements in order


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_trim">trim</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_len: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_trim">trim</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_len: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    <b>let</b> res &#61; <a href="vector.md#0x1_vector_trim_reverse">trim_reverse</a>(v, new_len);<br />    <a href="vector.md#0x1_vector_reverse">reverse</a>(&amp;<b>mut</b> res);<br />    res<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_trim_reverse"></a>

## Function `trim_reverse`

Trim a vector to a smaller size, returning the evicted elements in reverse order


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_trim_reverse">trim_reverse</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_len: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_trim_reverse">trim_reverse</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_len: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>assert</b>!(new_len &lt;&#61; len, <a href="vector.md#0x1_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>);<br />    <b>let</b> result &#61; <a href="vector.md#0x1_vector_empty">empty</a>();<br />    <b>while</b> (new_len &lt; len) &#123;<br />        <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> result, <a href="vector.md#0x1_vector_pop_back">pop_back</a>(v));<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_is_empty"></a>

## Function `is_empty`

Return <code><b>true</b></code> if the vector <code>v</code> has no elements and <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_is_empty">is_empty</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_is_empty">is_empty</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): bool &#123;<br />    <a href="vector.md#0x1_vector_length">length</a>(v) &#61;&#61; 0<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_contains"></a>

## Function `contains`

Return true if <code>e</code> is in the vector <code>v</code>.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_contains">contains</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: &amp;Element): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_contains">contains</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: &amp;Element): bool &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        <b>if</b> (<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i) &#61;&#61; e) <b>return</b> <b>true</b>;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <b>false</b><br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_index_of"></a>

## Function `index_of`

Return <code>(<b>true</b>, i)</code> if <code>e</code> is in the vector <code>v</code> at index <code>i</code>.
Otherwise, returns <code>(<b>false</b>, 0)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_index_of">index_of</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: &amp;Element): (bool, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_index_of">index_of</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: &amp;Element): (bool, u64) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        <b>if</b> (<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i) &#61;&#61; e) <b>return</b> (<b>true</b>, i);<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    (<b>false</b>, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_find"></a>

## Function `find`

Return <code>(<b>true</b>, i)</code> if there&apos;s an element that matches the predicate. If there are multiple elements that match
the predicate, only the index of the first one is returned.
Otherwise, returns <code>(<b>false</b>, 0)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_find">find</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): (bool, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_find">find</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): (bool, u64) &#123;<br />    <b>let</b> find &#61; <b>false</b>;<br />    <b>let</b> found_index &#61; 0;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        // Cannot call <b>return</b> in an inline function so we need <b>to</b> resort <b>to</b> <b>break</b> here.<br />        <b>if</b> (f(<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i))) &#123;<br />            find &#61; <b>true</b>;<br />            found_index &#61; i;<br />            <b>break</b><br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    (find, found_index)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_insert"></a>

## Function `insert`

Insert a new element at position 0 &lt;&#61; i &lt;&#61; length, using O(length &#45; i) time.
Aborts if out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_insert">insert</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, e: Element)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_insert">insert</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, e: Element) &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>assert</b>!(i &lt;&#61; len, <a href="vector.md#0x1_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>);<br />    <a href="vector.md#0x1_vector_push_back">push_back</a>(v, e);<br />    <b>while</b> (i &lt; len) &#123;<br />        <a href="vector.md#0x1_vector_swap">swap</a>(v, i, len);<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_remove"></a>

## Function `remove`

Remove the <code>i</code>th element of the vector <code>v</code>, shifting all subsequent elements.
This is O(n) and preserves ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_remove">remove</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_remove">remove</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    // i out of bounds; <b>abort</b><br />    <b>if</b> (i &gt;&#61; len) <b>abort</b> <a href="vector.md#0x1_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>;<br /><br />    len &#61; len &#45; 1;<br />    <b>while</b> (i &lt; len) <a href="vector.md#0x1_vector_swap">swap</a>(v, i, &#123; i &#61; i &#43; 1; i &#125;);<br />    <a href="vector.md#0x1_vector_pop_back">pop_back</a>(v)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_remove_value"></a>

## Function `remove_value`

Remove the first occurrence of a given value in the vector <code>v</code> and return it in a vector, shifting all
subsequent elements.
This is O(n) and preserves ordering of elements in the vector.
This returns an empty vector if the value isn&apos;t present in the vector.
Note that this cannot return an option as option uses vector and there&apos;d be a circular dependency between option
and vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_remove_value">remove_value</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, val: &amp;Element): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_remove_value">remove_value</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, val: &amp;Element): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    // This doesn&apos;t cost a O(2N) run time <b>as</b> index_of scans from left <b>to</b> right and stops when the element is found,<br />    // <b>while</b> remove would <b>continue</b> from the identified index <b>to</b> the end of the <a href="vector.md#0x1_vector">vector</a>.<br />    <b>let</b> (found, index) &#61; <a href="vector.md#0x1_vector_index_of">index_of</a>(v, val);<br />    <b>if</b> (found) &#123;<br />        <a href="vector.md#0x1_vector">vector</a>[<a href="vector.md#0x1_vector_remove">remove</a>(v, index)]<br />    &#125; <b>else</b> &#123;<br />       <a href="vector.md#0x1_vector">vector</a>[]<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>v</code> with the last element and then pop the vector.
This is O(1), but does not preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_swap_remove">swap_remove</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_swap_remove">swap_remove</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element &#123;<br />    <b>assert</b>!(!<a href="vector.md#0x1_vector_is_empty">is_empty</a>(v), <a href="vector.md#0x1_vector_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>);<br />    <b>let</b> last_idx &#61; <a href="vector.md#0x1_vector_length">length</a>(v) &#45; 1;<br />    <a href="vector.md#0x1_vector_swap">swap</a>(v, i, last_idx);<br />    <a href="vector.md#0x1_vector_pop_back">pop_back</a>(v)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_for_each"></a>

## Function `for_each`

Apply the function to each element in the vector, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_for_each">for_each</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_for_each">for_each</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;Element&#124;) &#123;<br />    <a href="vector.md#0x1_vector_reverse">reverse</a>(&amp;<b>mut</b> v); // We need <b>to</b> reverse the <a href="vector.md#0x1_vector">vector</a> <b>to</b> consume it efficiently<br />    <a href="vector.md#0x1_vector_for_each_reverse">for_each_reverse</a>(v, &#124;e&#124; f(e));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_for_each_reverse"></a>

## Function `for_each_reverse`

Apply the function to each element in the vector, consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_for_each_reverse">for_each_reverse</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_for_each_reverse">for_each_reverse</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;Element&#124;) &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(&amp;v);<br />    <b>while</b> (len &gt; 0) &#123;<br />        f(<a href="vector.md#0x1_vector_pop_back">pop_back</a>(&amp;<b>mut</b> v));<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    <a href="vector.md#0x1_vector_destroy_empty">destroy_empty</a>(v)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each element in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_for_each_ref">for_each_ref</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_for_each_ref">for_each_ref</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_zip"></a>

## Function `zip`

Apply the function to each pair of elements in the two given vectors, consuming them.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_zip">zip</a>&lt;Element1, Element2&gt;(v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;(Element1, Element2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_zip">zip</a>&lt;Element1, Element2&gt;(v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;Element1, Element2&#124;) &#123;<br />    // We need <b>to</b> reverse the vectors <b>to</b> consume it efficiently<br />    <a href="vector.md#0x1_vector_reverse">reverse</a>(&amp;<b>mut</b> v1);<br />    <a href="vector.md#0x1_vector_reverse">reverse</a>(&amp;<b>mut</b> v2);<br />    <a href="vector.md#0x1_vector_zip_reverse">zip_reverse</a>(v1, v2, &#124;e1, e2&#124; f(e1, e2));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_zip_reverse"></a>

## Function `zip_reverse`

Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_zip_reverse">zip_reverse</a>&lt;Element1, Element2&gt;(v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;(Element1, Element2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_zip_reverse">zip_reverse</a>&lt;Element1, Element2&gt;(<br />    v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;,<br />    v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;,<br />    f: &#124;Element1, Element2&#124;,<br />) &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(&amp;v1);<br />    // We can&apos;t <b>use</b> the constant <a href="vector.md#0x1_vector_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(len &#61;&#61; <a href="vector.md#0x1_vector_length">length</a>(&amp;v2), 0x20002);<br />    <b>while</b> (len &gt; 0) &#123;<br />        f(<a href="vector.md#0x1_vector_pop_back">pop_back</a>(&amp;<b>mut</b> v1), <a href="vector.md#0x1_vector_pop_back">pop_back</a>(&amp;<b>mut</b> v2));<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    <a href="vector.md#0x1_vector_destroy_empty">destroy_empty</a>(v1);<br />    <a href="vector.md#0x1_vector_destroy_empty">destroy_empty</a>(v2);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_zip_ref"></a>

## Function `zip_ref`

Apply the function to the references of each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_zip_ref">zip_ref</a>&lt;Element1, Element2&gt;(v1: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;(&amp;Element1, &amp;Element2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_zip_ref">zip_ref</a>&lt;Element1, Element2&gt;(<br />    v1: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;,<br />    v2: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;,<br />    f: &#124;&amp;Element1, &amp;Element2&#124;,<br />) &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v1);<br />    // We can&apos;t <b>use</b> the constant <a href="vector.md#0x1_vector_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(len &#61;&#61; <a href="vector.md#0x1_vector_length">length</a>(v2), 0x20002);<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; len) &#123;<br />        f(<a href="vector.md#0x1_vector_borrow">borrow</a>(v1, i), <a href="vector.md#0x1_vector_borrow">borrow</a>(v2, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_enumerate_ref"></a>

## Function `enumerate_ref`

Apply the function to a reference of each element in the vector with its index.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_enumerate_ref">enumerate_ref</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;(u64, &amp;Element)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_enumerate_ref">enumerate_ref</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;u64, &amp;Element&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(i, <a href="vector.md#0x1_vector_borrow">borrow</a>(v, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference to each element in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_for_each_mut">for_each_mut</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;<b>mut</b> Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_for_each_mut">for_each_mut</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;<b>mut</b> Element&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(<a href="vector.md#0x1_vector_borrow_mut">borrow_mut</a>(v, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_zip_mut"></a>

## Function `zip_mut`

Apply the function to mutable references to each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_zip_mut">zip_mut</a>&lt;Element1, Element2&gt;(v1: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;(&amp;<b>mut</b> Element1, &amp;<b>mut</b> Element2)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_zip_mut">zip_mut</a>&lt;Element1, Element2&gt;(<br />    v1: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;,<br />    v2: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;,<br />    f: &#124;&amp;<b>mut</b> Element1, &amp;<b>mut</b> Element2&#124;,<br />) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v1);<br />    // We can&apos;t <b>use</b> the constant <a href="vector.md#0x1_vector_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(len &#61;&#61; <a href="vector.md#0x1_vector_length">length</a>(v2), 0x20002);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(<a href="vector.md#0x1_vector_borrow_mut">borrow_mut</a>(v1, i), <a href="vector.md#0x1_vector_borrow_mut">borrow_mut</a>(v2, i));<br />        i &#61; i &#43; 1<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_enumerate_mut"></a>

## Function `enumerate_mut`

Apply the function to a mutable reference of each element in the vector with its index.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_enumerate_mut">enumerate_mut</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;(u64, &amp;<b>mut</b> Element)&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_enumerate_mut">enumerate_mut</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;u64, &amp;<b>mut</b> Element&#124;) &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        f(i, <a href="vector.md#0x1_vector_borrow_mut">borrow_mut</a>(v, i));<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_fold"></a>

## Function `fold`

Fold the function over the elements. For example, <code><a href="vector.md#0x1_vector_fold">fold</a>(<a href="vector.md#0x1_vector">vector</a>[1,2,3], 0, f)</code> will execute
<code>f(f(f(0, 1), 2), 3)</code>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_fold">fold</a>&lt;Accumulator, Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, init: Accumulator, f: &#124;(Accumulator, Element)&#124;Accumulator): Accumulator<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_fold">fold</a>&lt;Accumulator, Element&gt;(<br />    v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    init: Accumulator,<br />    f: &#124;Accumulator,Element&#124;Accumulator<br />): Accumulator &#123;<br />    <b>let</b> accu &#61; init;<br />    <a href="vector.md#0x1_vector_for_each">for_each</a>(v, &#124;elem&#124; accu &#61; f(accu, elem));<br />    accu<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_foldr"></a>

## Function `foldr`

Fold right like fold above but working right to left. For example, <code><a href="vector.md#0x1_vector_fold">fold</a>(<a href="vector.md#0x1_vector">vector</a>[1,2,3], 0, f)</code> will execute
<code>f(1, f(2, f(3, 0)))</code>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_foldr">foldr</a>&lt;Accumulator, Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, init: Accumulator, f: &#124;(Element, Accumulator)&#124;Accumulator): Accumulator<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_foldr">foldr</a>&lt;Accumulator, Element&gt;(<br />    v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    init: Accumulator,<br />    f: &#124;Element, Accumulator&#124;Accumulator<br />): Accumulator &#123;<br />    <b>let</b> accu &#61; init;<br />    <a href="vector.md#0x1_vector_for_each_reverse">for_each_reverse</a>(v, &#124;elem&#124; accu &#61; f(elem, accu));<br />    accu<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_map_ref"></a>

## Function `map_ref`

Map the function over the references of the elements of the vector, producing a new vector without modifying the
original vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_map_ref">map_ref</a>&lt;Element, NewElement&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;NewElement): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_map_ref">map_ref</a>&lt;Element, NewElement&gt;(<br />    v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    f: &#124;&amp;Element&#124;NewElement<br />): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt; &#123;<br />    <b>let</b> result &#61; <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;[];<br />    <a href="vector.md#0x1_vector_for_each_ref">for_each_ref</a>(v, &#124;elem&#124; <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(elem)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_zip_map_ref"></a>

## Function `zip_map_ref`

Map the function over the references of the element pairs of two vectors, producing a new vector from the return
values without modifying the original vectors.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_zip_map_ref">zip_map_ref</a>&lt;Element1, Element2, NewElement&gt;(v1: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;(&amp;Element1, &amp;Element2)&#124;NewElement): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_zip_map_ref">zip_map_ref</a>&lt;Element1, Element2, NewElement&gt;(<br />    v1: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;,<br />    v2: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;,<br />    f: &#124;&amp;Element1, &amp;Element2&#124;NewElement<br />): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt; &#123;<br />    // We can&apos;t <b>use</b> the constant <a href="vector.md#0x1_vector_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(<a href="vector.md#0x1_vector_length">length</a>(v1) &#61;&#61; <a href="vector.md#0x1_vector_length">length</a>(v2), 0x20002);<br /><br />    <b>let</b> result &#61; <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;[];<br />    <a href="vector.md#0x1_vector_zip_ref">zip_ref</a>(v1, v2, &#124;e1, e2&#124; <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(e1, e2)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_map"></a>

## Function `map`

Map the function over the elements of the vector, producing a new vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_map">map</a>&lt;Element, NewElement&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, f: &#124;Element&#124;NewElement): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_map">map</a>&lt;Element, NewElement&gt;(<br />    v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    f: &#124;Element&#124;NewElement<br />): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt; &#123;<br />    <b>let</b> result &#61; <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;[];<br />    <a href="vector.md#0x1_vector_for_each">for_each</a>(v, &#124;elem&#124; <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(elem)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_zip_map"></a>

## Function `zip_map`

Map the function over the element pairs of the two vectors, producing a new vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_zip_map">zip_map</a>&lt;Element1, Element2, NewElement&gt;(v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;, f: &#124;(Element1, Element2)&#124;NewElement): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_zip_map">zip_map</a>&lt;Element1, Element2, NewElement&gt;(<br />    v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element1&gt;,<br />    v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element2&gt;,<br />    f: &#124;Element1, Element2&#124;NewElement<br />): <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt; &#123;<br />    // We can&apos;t <b>use</b> the constant <a href="vector.md#0x1_vector_EVECTORS_LENGTH_MISMATCH">EVECTORS_LENGTH_MISMATCH</a> here <b>as</b> all calling code would then need <b>to</b> define it<br />    // due <b>to</b> how inline functions work.<br />    <b>assert</b>!(<a href="vector.md#0x1_vector_length">length</a>(&amp;v1) &#61;&#61; <a href="vector.md#0x1_vector_length">length</a>(&amp;v2), 0x20002);<br /><br />    <b>let</b> result &#61; <a href="vector.md#0x1_vector">vector</a>&lt;NewElement&gt;[];<br />    <a href="vector.md#0x1_vector_zip">zip</a>(v1, v2, &#124;e1, e2&#124; <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> result, f(e1, e2)));<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_filter"></a>

## Function `filter`

Filter the vector using the boolean function, removing all elements for which <code>p(e)</code> is not true.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_filter">filter</a>&lt;Element: drop&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_filter">filter</a>&lt;Element:drop&gt;(<br />    v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    p: &#124;&amp;Element&#124;bool<br />): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    <b>let</b> result &#61; <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;[];<br />    <a href="vector.md#0x1_vector_for_each">for_each</a>(v, &#124;elem&#124; &#123;<br />        <b>if</b> (p(&amp;elem)) <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> result, elem);<br />    &#125;);<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_partition"></a>

## Function `partition`

Partition, sorts all elements for which pred is true to the front.
Preserves the relative order of the elements for which pred is true,
BUT NOT for the elements for which pred is false.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_partition">partition</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, pred: &#124;&amp;Element&#124;bool): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_partition">partition</a>&lt;Element&gt;(<br />    v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    pred: &#124;&amp;Element&#124;bool<br />): u64 &#123;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>while</b> (i &lt; len) &#123;<br />        <b>if</b> (!pred(<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i))) <b>break</b>;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <b>let</b> p &#61; i;<br />    i &#61; i &#43; 1;<br />    <b>while</b> (i &lt; len) &#123;<br />        <b>if</b> (pred(<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i))) &#123;<br />            <a href="vector.md#0x1_vector_swap">swap</a>(v, p, i);<br />            p &#61; p &#43; 1;<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    p<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_rotate"></a>

## Function `rotate`

rotate(&amp;mut [1, 2, 3, 4, 5], 2) &#45;&gt; [3, 4, 5, 1, 2] in place, returns the split point
ie. 3 in the example above


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_rotate">rotate</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, rot: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_rotate">rotate</a>&lt;Element&gt;(<br />    v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    rot: u64<br />): u64 &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <a href="vector.md#0x1_vector_rotate_slice">rotate_slice</a>(v, 0, rot, len)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_rotate_slice"></a>

## Function `rotate_slice`

Same as above but on a sub&#45;slice of an array [left, right) with left &lt;&#61; rot &lt;&#61; right
returns the


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_rotate_slice">rotate_slice</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, left: u64, rot: u64, right: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_rotate_slice">rotate_slice</a>&lt;Element&gt;(<br />    v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    left: u64,<br />    rot: u64,<br />    right: u64<br />): u64 &#123;<br />    <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>(v, left, rot);<br />    <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>(v, rot, right);<br />    <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>(v, left, right);<br />    left &#43; (right &#45; rot)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_stable_partition"></a>

## Function `stable_partition`

Partition the array based on a predicate p, this routine is stable and thus
preserves the relative order of the elements in the two partitions.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_stable_partition">stable_partition</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_stable_partition">stable_partition</a>&lt;Element&gt;(<br />    v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    p: &#124;&amp;Element&#124;bool<br />): u64 &#123;<br />    <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">length</a>(v);<br />    <b>let</b> t &#61; <a href="vector.md#0x1_vector_empty">empty</a>();<br />    <b>let</b> f &#61; <a href="vector.md#0x1_vector_empty">empty</a>();<br />    <b>while</b> (len &gt; 0) &#123;<br />        <b>let</b> e &#61; <a href="vector.md#0x1_vector_pop_back">pop_back</a>(v);<br />        <b>if</b> (p(&amp;e)) &#123;<br />            <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> t, e);<br />        &#125; <b>else</b> &#123;<br />            <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> f, e);<br />        &#125;;<br />        len &#61; len &#45; 1;<br />    &#125;;<br />    <b>let</b> pos &#61; <a href="vector.md#0x1_vector_length">length</a>(&amp;t);<br />    <a href="vector.md#0x1_vector_reverse_append">reverse_append</a>(v, t);<br />    <a href="vector.md#0x1_vector_reverse_append">reverse_append</a>(v, f);<br />    pos<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_any"></a>

## Function `any`

Return true if any element in the vector satisfies the predicate.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_any">any</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_any">any</a>&lt;Element&gt;(<br />    v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    p: &#124;&amp;Element&#124;bool<br />): bool &#123;<br />    <b>let</b> result &#61; <b>false</b>;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; <a href="vector.md#0x1_vector_length">length</a>(v)) &#123;<br />        result &#61; p(<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i));<br />        <b>if</b> (result) &#123;<br />            <b>break</b><br />        &#125;;<br />        i &#61; i &#43; 1<br />    &#125;;<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_all"></a>

## Function `all`

Return true if all elements in the vector satisfy the predicate.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_all">all</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_all">all</a>&lt;Element&gt;(<br />    v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    p: &#124;&amp;Element&#124;bool<br />): bool &#123;<br />    <b>let</b> result &#61; <b>true</b>;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (i &lt; <a href="vector.md#0x1_vector_length">length</a>(v)) &#123;<br />        result &#61; p(<a href="vector.md#0x1_vector_borrow">borrow</a>(v, i));<br />        <b>if</b> (!result) &#123;<br />            <b>break</b><br />        &#125;;<br />        i &#61; i &#43; 1<br />    &#125;;<br />    result<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_destroy"></a>

## Function `destroy`

Destroy a vector, just a wrapper around for_each_reverse with a descriptive name
when used in the context of destroying a vector.


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_destroy">destroy</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, d: &#124;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="vector.md#0x1_vector_destroy">destroy</a>&lt;Element&gt;(<br />    v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    d: &#124;Element&#124;<br />) &#123;<br />    <a href="vector.md#0x1_vector_for_each_reverse">for_each_reverse</a>(v, &#124;e&#124; d(e))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_range"></a>

## Function `range`



<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_range">range</a>(start: u64, end: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_range">range</a>(start: u64, end: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt; &#123;<br />    <a href="vector.md#0x1_vector_range_with_step">range_with_step</a>(start, end, 1)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_range_with_step"></a>

## Function `range_with_step`



<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_range_with_step">range_with_step</a>(start: u64, end: u64, step: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_range_with_step">range_with_step</a>(start: u64, end: u64, step: u64): <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt; &#123;<br />    <b>assert</b>!(step &gt; 0, <a href="vector.md#0x1_vector_EINVALID_STEP">EINVALID_STEP</a>);<br /><br />    <b>let</b> vec &#61; <a href="vector.md#0x1_vector">vector</a>[];<br />    <b>while</b> (start &lt; end) &#123;<br />        <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> vec, start);<br />        start &#61; start &#43; step;<br />    &#125;;<br />    vec<br />&#125;<br /></code></pre>



</details>

<a id="0x1_vector_slice"></a>

## Function `slice`



<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_slice">slice</a>&lt;Element: <b>copy</b>&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, start: u64, end: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_slice">slice</a>&lt;Element: <b>copy</b>&gt;(<br />    v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;,<br />    start: u64,<br />    end: u64<br />): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    <b>assert</b>!(start &lt;&#61; end &amp;&amp; end &lt;&#61; <a href="vector.md#0x1_vector_length">length</a>(v), <a href="vector.md#0x1_vector_EINVALID_SLICE_RANGE">EINVALID_SLICE_RANGE</a>);<br /><br />    <b>let</b> vec &#61; <a href="vector.md#0x1_vector">vector</a>[];<br />    <b>while</b> (start &lt; end) &#123;<br />        <a href="vector.md#0x1_vector_push_back">push_back</a>(&amp;<b>mut</b> vec, &#42;<a href="vector.md#0x1_vector_borrow">borrow</a>(v, start));<br />        start &#61; start &#43; 1;<br />    &#125;;<br />    vec<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<a id="@Helper_Functions_2"></a>

### Helper Functions


Check if <code>v1</code> is equal to the result of adding <code>e</code> at the end of <code>v2</code>


<a id="0x1_vector_eq_push_back"></a>


<pre><code><b>fun</b> <a href="vector.md#0x1_vector_eq_push_back">eq_push_back</a>&lt;Element&gt;(v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: Element): bool &#123;<br />    len(v1) &#61;&#61; len(v2) &#43; 1 &amp;&amp;<br />    v1[len(v1)&#45;1] &#61;&#61; e &amp;&amp;<br />    v1[0..len(v1)&#45;1] &#61;&#61; v2[0..len(v2)]<br />&#125;<br /></code></pre>


Check if <code>v</code> is equal to the result of concatenating <code>v1</code> and <code>v2</code>


<a id="0x1_vector_eq_append"></a>


<pre><code><b>fun</b> <a href="vector.md#0x1_vector_eq_append">eq_append</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): bool &#123;<br />    len(v) &#61;&#61; len(v1) &#43; len(v2) &amp;&amp;<br />    v[0..len(v1)] &#61;&#61; v1 &amp;&amp;<br />    v[len(v1)..len(v)] &#61;&#61; v2<br />&#125;<br /></code></pre>


Check <code>v1</code> is equal to the result of removing the first element of <code>v2</code>


<a id="0x1_vector_eq_pop_front"></a>


<pre><code><b>fun</b> <a href="vector.md#0x1_vector_eq_pop_front">eq_pop_front</a>&lt;Element&gt;(v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): bool &#123;<br />    len(v1) &#43; 1 &#61;&#61; len(v2) &amp;&amp;<br />    v1 &#61;&#61; v2[1..len(v2)]<br />&#125;<br /></code></pre>


Check that <code>v1</code> is equal to the result of removing the element at index <code>i</code> from <code>v2</code>.


<a id="0x1_vector_eq_remove_elem_at_index"></a>


<pre><code><b>fun</b> <a href="vector.md#0x1_vector_eq_remove_elem_at_index">eq_remove_elem_at_index</a>&lt;Element&gt;(i: u64, v1: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, v2: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): bool &#123;<br />    len(v1) &#43; 1 &#61;&#61; len(v2) &amp;&amp;<br />    v1[0..i] &#61;&#61; v2[0..i] &amp;&amp;<br />    v1[i..len(v1)] &#61;&#61; v2[i &#43; 1..len(v2)]<br />&#125;<br /></code></pre>


Check if <code>v</code> contains <code>e</code>.


<a id="0x1_vector_spec_contains"></a>


<pre><code><b>fun</b> <a href="vector.md#0x1_vector_spec_contains">spec_contains</a>&lt;Element&gt;(v: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: Element): bool &#123;<br />    <b>exists</b> x in v: x &#61;&#61; e<br />&#125;<br /></code></pre>



<a id="@Specification_1_singleton"></a>

### Function `singleton`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_singleton">singleton</a>&lt;Element&gt;(e: Element): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; vec(e);<br /></code></pre>



<a id="@Specification_1_reverse"></a>

### Function `reverse`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse">reverse</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_reverse_slice"></a>

### Function `reverse_slice`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse_slice">reverse_slice</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, left: u64, right: u64)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_append">append</a>&lt;Element&gt;(lhs: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_reverse_append"></a>

### Function `reverse_append`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_reverse_append">reverse_append</a>&lt;Element&gt;(lhs: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, other: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_trim"></a>

### Function `trim`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_trim">trim</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_len: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_trim_reverse"></a>

### Function `trim_reverse`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_trim_reverse">trim_reverse</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, new_len: u64): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_is_empty"></a>

### Function `is_empty`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_is_empty">is_empty</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_contains">contains</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: &amp;Element): bool<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_index_of"></a>

### Function `index_of`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_index_of">index_of</a>&lt;Element&gt;(v: &amp;<a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, e: &amp;Element): (bool, u64)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_insert"></a>

### Function `insert`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_insert">insert</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64, e: Element)<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_remove">remove</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_remove_value"></a>

### Function `remove_value`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_remove_value">remove_value</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, val: &amp;Element): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_swap_remove">swap_remove</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, i: u64): Element<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_rotate"></a>

### Function `rotate`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_rotate">rotate</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, rot: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_rotate_slice"></a>

### Function `rotate_slice`


<pre><code><b>public</b> <b>fun</b> <a href="vector.md#0x1_vector_rotate_slice">rotate_slice</a>&lt;Element&gt;(v: &amp;<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;, left: u64, rot: u64, right: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> intrinsic &#61; <b>true</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
