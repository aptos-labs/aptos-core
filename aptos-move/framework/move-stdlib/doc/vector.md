
<a id="0x1_vector"></a>

# Module `0x1::vector`

A variable-sized container that can hold any type. Indexing is 0-based, and
vectors are growable. This module has many native functions.
Verification of modules that use this one uses model functions that are implemented
directly in Boogie. The specification language has built-in functions operations such
as <code>singleton_vector</code>. There are some helper functions defined here for specifications in other
modules as well.

>Note: We did not verify most of the
Move functions here because many have loops, requiring loop invariants to prove, and
the return on investment didn't seem worth it for these simple functions.


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


<pre><code>const EINDEX_OUT_OF_BOUNDS: u64 &#61; 131072;<br/></code></pre>



<a id="0x1_vector_EINVALID_RANGE"></a>

The index into the vector is out of bounds


<pre><code>const EINVALID_RANGE: u64 &#61; 131073;<br/></code></pre>



<a id="0x1_vector_EINVALID_SLICE_RANGE"></a>

The range in <code>slice</code> is invalid.


<pre><code>const EINVALID_SLICE_RANGE: u64 &#61; 131076;<br/></code></pre>



<a id="0x1_vector_EINVALID_STEP"></a>

The step provided in <code>range</code> is invalid, must be greater than zero.


<pre><code>const EINVALID_STEP: u64 &#61; 131075;<br/></code></pre>



<a id="0x1_vector_EVECTORS_LENGTH_MISMATCH"></a>

The length of the vectors are not equal.


<pre><code>const EVECTORS_LENGTH_MISMATCH: u64 &#61; 131074;<br/></code></pre>



<a id="0x1_vector_empty"></a>

## Function `empty`

Create an empty vector.


<pre><code>&#35;[bytecode_instruction]<br/>public fun empty&lt;Element&gt;(): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun empty&lt;Element&gt;(): vector&lt;Element&gt;;<br/></code></pre>



</details>

<a id="0x1_vector_length"></a>

## Function `length`

Return the length of the vector.


<pre><code>&#35;[bytecode_instruction]<br/>public fun length&lt;Element&gt;(v: &amp;vector&lt;Element&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun length&lt;Element&gt;(v: &amp;vector&lt;Element&gt;): u64;<br/></code></pre>



</details>

<a id="0x1_vector_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the <code>i</code>th element of the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>&#35;[bytecode_instruction]<br/>public fun borrow&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, i: u64): &amp;Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun borrow&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, i: u64): &amp;Element;<br/></code></pre>



</details>

<a id="0x1_vector_push_back"></a>

## Function `push_back`

Add element <code>e</code> to the end of the vector <code>v</code>.


<pre><code>&#35;[bytecode_instruction]<br/>public fun push_back&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, e: Element)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun push_back&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, e: Element);<br/></code></pre>



</details>

<a id="0x1_vector_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the <code>i</code>th element in the vector <code>v</code>.
Aborts if <code>i</code> is out of bounds.


<pre><code>&#35;[bytecode_instruction]<br/>public fun borrow_mut&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): &amp;mut Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun borrow_mut&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): &amp;mut Element;<br/></code></pre>



</details>

<a id="0x1_vector_pop_back"></a>

## Function `pop_back`

Pop an element from the end of vector <code>v</code>.
Aborts if <code>v</code> is empty.


<pre><code>&#35;[bytecode_instruction]<br/>public fun pop_back&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun pop_back&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;): Element;<br/></code></pre>



</details>

<a id="0x1_vector_destroy_empty"></a>

## Function `destroy_empty`

Destroy the vector <code>v</code>.
Aborts if <code>v</code> is not empty.


<pre><code>&#35;[bytecode_instruction]<br/>public fun destroy_empty&lt;Element&gt;(v: vector&lt;Element&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun destroy_empty&lt;Element&gt;(v: vector&lt;Element&gt;);<br/></code></pre>



</details>

<a id="0x1_vector_swap"></a>

## Function `swap`

Swaps the elements at the <code>i</code>th and <code>j</code>th indices in the vector <code>v</code>.
Aborts if <code>i</code> or <code>j</code> is out of bounds.


<pre><code>&#35;[bytecode_instruction]<br/>public fun swap&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64, j: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun swap&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64, j: u64);<br/></code></pre>



</details>

<a id="0x1_vector_singleton"></a>

## Function `singleton`

Return an vector of size one containing element <code>e</code>.


<pre><code>public fun singleton&lt;Element&gt;(e: Element): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun singleton&lt;Element&gt;(e: Element): vector&lt;Element&gt; &#123;<br/>    let v &#61; empty();<br/>    push_back(&amp;mut v, e);<br/>    v<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_reverse"></a>

## Function `reverse`

Reverses the order of the elements in the vector <code>v</code> in place.


<pre><code>public fun reverse&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reverse&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;) &#123;<br/>    let len &#61; length(v);<br/>    reverse_slice(v, 0, len);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_reverse_slice"></a>

## Function `reverse_slice`

Reverses the order of the elements [left, right) in the vector <code>v</code> in place.


<pre><code>public fun reverse_slice&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, left: u64, right: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reverse_slice&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, left: u64, right: u64) &#123;<br/>    assert!(left &lt;&#61; right, EINVALID_RANGE);<br/>    if (left &#61;&#61; right) return;<br/>    right &#61; right &#45; 1;<br/>    while (left &lt; right) &#123;<br/>        swap(v, left, right);<br/>        left &#61; left &#43; 1;<br/>        right &#61; right &#45; 1;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_append"></a>

## Function `append`

Pushes all of the elements of the <code>other</code> vector into the <code>lhs</code> vector.


<pre><code>public fun append&lt;Element&gt;(lhs: &amp;mut vector&lt;Element&gt;, other: vector&lt;Element&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun append&lt;Element&gt;(lhs: &amp;mut vector&lt;Element&gt;, other: vector&lt;Element&gt;) &#123;<br/>    reverse(&amp;mut other);<br/>    reverse_append(lhs, other);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_reverse_append"></a>

## Function `reverse_append`

Pushes all of the elements of the <code>other</code> vector into the <code>lhs</code> vector.


<pre><code>public fun reverse_append&lt;Element&gt;(lhs: &amp;mut vector&lt;Element&gt;, other: vector&lt;Element&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reverse_append&lt;Element&gt;(lhs: &amp;mut vector&lt;Element&gt;, other: vector&lt;Element&gt;) &#123;<br/>    let len &#61; length(&amp;other);<br/>    while (len &gt; 0) &#123;<br/>        push_back(lhs, pop_back(&amp;mut other));<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    destroy_empty(other);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_trim"></a>

## Function `trim`

Trim a vector to a smaller size, returning the evicted elements in order


<pre><code>public fun trim&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, new_len: u64): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun trim&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, new_len: u64): vector&lt;Element&gt; &#123;<br/>    let res &#61; trim_reverse(v, new_len);<br/>    reverse(&amp;mut res);<br/>    res<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_trim_reverse"></a>

## Function `trim_reverse`

Trim a vector to a smaller size, returning the evicted elements in reverse order


<pre><code>public fun trim_reverse&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, new_len: u64): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun trim_reverse&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, new_len: u64): vector&lt;Element&gt; &#123;<br/>    let len &#61; length(v);<br/>    assert!(new_len &lt;&#61; len, EINDEX_OUT_OF_BOUNDS);<br/>    let result &#61; empty();<br/>    while (new_len &lt; len) &#123;<br/>        push_back(&amp;mut result, pop_back(v));<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_is_empty"></a>

## Function `is_empty`

Return <code>true</code> if the vector <code>v</code> has no elements and <code>false</code> otherwise.


<pre><code>public fun is_empty&lt;Element&gt;(v: &amp;vector&lt;Element&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_empty&lt;Element&gt;(v: &amp;vector&lt;Element&gt;): bool &#123;<br/>    length(v) &#61;&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_contains"></a>

## Function `contains`

Return true if <code>e</code> is in the vector <code>v</code>.


<pre><code>public fun contains&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, e: &amp;Element): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, e: &amp;Element): bool &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        if (borrow(v, i) &#61;&#61; e) return true;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    false<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_index_of"></a>

## Function `index_of`

Return <code>(true, i)</code> if <code>e</code> is in the vector <code>v</code> at index <code>i</code>.
Otherwise, returns <code>(false, 0)</code>.


<pre><code>public fun index_of&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, e: &amp;Element): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun index_of&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, e: &amp;Element): (bool, u64) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        if (borrow(v, i) &#61;&#61; e) return (true, i);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    (false, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_find"></a>

## Function `find`

Return <code>(true, i)</code> if there's an element that matches the predicate. If there are multiple elements that match
the predicate, only the index of the first one is returned.
Otherwise, returns <code>(false, 0)</code>.


<pre><code>public fun find&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): (bool, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun find&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): (bool, u64) &#123;<br/>    let find &#61; false;<br/>    let found_index &#61; 0;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        // Cannot call return in an inline function so we need to resort to break here.<br/>        if (f(borrow(v, i))) &#123;<br/>            find &#61; true;<br/>            found_index &#61; i;<br/>            break<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    (find, found_index)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_insert"></a>

## Function `insert`

Insert a new element at position 0 <= i <= length, using O(length - i) time.
Aborts if out of bounds.


<pre><code>public fun insert&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64, e: Element)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun insert&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64, e: Element) &#123;<br/>    let len &#61; length(v);<br/>    assert!(i &lt;&#61; len, EINDEX_OUT_OF_BOUNDS);<br/>    push_back(v, e);<br/>    while (i &lt; len) &#123;<br/>        swap(v, i, len);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_remove"></a>

## Function `remove`

Remove the <code>i</code>th element of the vector <code>v</code>, shifting all subsequent elements.
This is O(n) and preserves ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun remove&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): Element &#123;<br/>    let len &#61; length(v);<br/>    // i out of bounds; abort<br/>    if (i &gt;&#61; len) abort EINDEX_OUT_OF_BOUNDS;<br/><br/>    len &#61; len &#45; 1;<br/>    while (i &lt; len) swap(v, i, &#123; i &#61; i &#43; 1; i &#125;);<br/>    pop_back(v)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_remove_value"></a>

## Function `remove_value`

Remove the first occurrence of a given value in the vector <code>v</code> and return it in a vector, shifting all
subsequent elements.
This is O(n) and preserves ordering of elements in the vector.
This returns an empty vector if the value isn't present in the vector.
Note that this cannot return an option as option uses vector and there'd be a circular dependency between option
and vector.


<pre><code>public fun remove_value&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, val: &amp;Element): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_value&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, val: &amp;Element): vector&lt;Element&gt; &#123;<br/>    // This doesn&apos;t cost a O(2N) run time as index_of scans from left to right and stops when the element is found,<br/>    // while remove would continue from the identified index to the end of the vector.<br/>    let (found, index) &#61; index_of(v, val);<br/>    if (found) &#123;<br/>        vector[remove(v, index)]<br/>    &#125; else &#123;<br/>       vector[]<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_swap_remove"></a>

## Function `swap_remove`

Swap the <code>i</code>th element of the vector <code>v</code> with the last element and then pop the vector.
This is O(1), but does not preserve ordering of elements in the vector.
Aborts if <code>i</code> is out of bounds.


<pre><code>public fun swap_remove&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap_remove&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): Element &#123;<br/>    assert!(!is_empty(v), EINDEX_OUT_OF_BOUNDS);<br/>    let last_idx &#61; length(v) &#45; 1;<br/>    swap(v, i, last_idx);<br/>    pop_back(v)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_for_each"></a>

## Function `for_each`

Apply the function to each element in the vector, consuming it.


<pre><code>public fun for_each&lt;Element&gt;(v: vector&lt;Element&gt;, f: &#124;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each&lt;Element&gt;(v: vector&lt;Element&gt;, f: &#124;Element&#124;) &#123;<br/>    reverse(&amp;mut v); // We need to reverse the vector to consume it efficiently<br/>    for_each_reverse(v, &#124;e&#124; f(e));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_for_each_reverse"></a>

## Function `for_each_reverse`

Apply the function to each element in the vector, consuming it.


<pre><code>public fun for_each_reverse&lt;Element&gt;(v: vector&lt;Element&gt;, f: &#124;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_reverse&lt;Element&gt;(v: vector&lt;Element&gt;, f: &#124;Element&#124;) &#123;<br/>    let len &#61; length(&amp;v);<br/>    while (len &gt; 0) &#123;<br/>        f(pop_back(&amp;mut v));<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    destroy_empty(v)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each element in the vector.


<pre><code>public fun for_each_ref&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;&amp;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_ref&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;&amp;Element&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        f(borrow(v, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_zip"></a>

## Function `zip`

Apply the function to each pair of elements in the two given vectors, consuming them.


<pre><code>public fun zip&lt;Element1, Element2&gt;(v1: vector&lt;Element1&gt;, v2: vector&lt;Element2&gt;, f: &#124;(Element1, Element2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip&lt;Element1, Element2&gt;(v1: vector&lt;Element1&gt;, v2: vector&lt;Element2&gt;, f: &#124;Element1, Element2&#124;) &#123;<br/>    // We need to reverse the vectors to consume it efficiently<br/>    reverse(&amp;mut v1);<br/>    reverse(&amp;mut v2);<br/>    zip_reverse(v1, v2, &#124;e1, e2&#124; f(e1, e2));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_zip_reverse"></a>

## Function `zip_reverse`

Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
This errors out if the vectors are not of the same length.


<pre><code>public fun zip_reverse&lt;Element1, Element2&gt;(v1: vector&lt;Element1&gt;, v2: vector&lt;Element2&gt;, f: &#124;(Element1, Element2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_reverse&lt;Element1, Element2&gt;(<br/>    v1: vector&lt;Element1&gt;,<br/>    v2: vector&lt;Element2&gt;,<br/>    f: &#124;Element1, Element2&#124;,<br/>) &#123;<br/>    let len &#61; length(&amp;v1);<br/>    // We can&apos;t use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(len &#61;&#61; length(&amp;v2), 0x20002);<br/>    while (len &gt; 0) &#123;<br/>        f(pop_back(&amp;mut v1), pop_back(&amp;mut v2));<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    destroy_empty(v1);<br/>    destroy_empty(v2);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_zip_ref"></a>

## Function `zip_ref`

Apply the function to the references of each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code>public fun zip_ref&lt;Element1, Element2&gt;(v1: &amp;vector&lt;Element1&gt;, v2: &amp;vector&lt;Element2&gt;, f: &#124;(&amp;Element1, &amp;Element2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_ref&lt;Element1, Element2&gt;(<br/>    v1: &amp;vector&lt;Element1&gt;,<br/>    v2: &amp;vector&lt;Element2&gt;,<br/>    f: &#124;&amp;Element1, &amp;Element2&#124;,<br/>) &#123;<br/>    let len &#61; length(v1);<br/>    // We can&apos;t use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(len &#61;&#61; length(v2), 0x20002);<br/>    let i &#61; 0;<br/>    while (i &lt; len) &#123;<br/>        f(borrow(v1, i), borrow(v2, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_enumerate_ref"></a>

## Function `enumerate_ref`

Apply the function to a reference of each element in the vector with its index.


<pre><code>public fun enumerate_ref&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;(u64, &amp;Element)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun enumerate_ref&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;u64, &amp;Element&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        f(i, borrow(v, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference to each element in the vector.


<pre><code>public fun for_each_mut&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, f: &#124;&amp;mut Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_mut&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, f: &#124;&amp;mut Element&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        f(borrow_mut(v, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_zip_mut"></a>

## Function `zip_mut`

Apply the function to mutable references to each pair of elements in the two given vectors.
This errors out if the vectors are not of the same length.


<pre><code>public fun zip_mut&lt;Element1, Element2&gt;(v1: &amp;mut vector&lt;Element1&gt;, v2: &amp;mut vector&lt;Element2&gt;, f: &#124;(&amp;mut Element1, &amp;mut Element2)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_mut&lt;Element1, Element2&gt;(<br/>    v1: &amp;mut vector&lt;Element1&gt;,<br/>    v2: &amp;mut vector&lt;Element2&gt;,<br/>    f: &#124;&amp;mut Element1, &amp;mut Element2&#124;,<br/>) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v1);<br/>    // We can&apos;t use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(len &#61;&#61; length(v2), 0x20002);<br/>    while (i &lt; len) &#123;<br/>        f(borrow_mut(v1, i), borrow_mut(v2, i));<br/>        i &#61; i &#43; 1<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_enumerate_mut"></a>

## Function `enumerate_mut`

Apply the function to a mutable reference of each element in the vector with its index.


<pre><code>public fun enumerate_mut&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, f: &#124;(u64, &amp;mut Element)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun enumerate_mut&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, f: &#124;u64, &amp;mut Element&#124;) &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        f(i, borrow_mut(v, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_fold"></a>

## Function `fold`

Fold the function over the elements. For example, <code>fold(vector[1,2,3], 0, f)</code> will execute
<code>f(f(f(0, 1), 2), 3)</code>


<pre><code>public fun fold&lt;Accumulator, Element&gt;(v: vector&lt;Element&gt;, init: Accumulator, f: &#124;(Accumulator, Element)&#124;Accumulator): Accumulator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun fold&lt;Accumulator, Element&gt;(<br/>    v: vector&lt;Element&gt;,<br/>    init: Accumulator,<br/>    f: &#124;Accumulator,Element&#124;Accumulator<br/>): Accumulator &#123;<br/>    let accu &#61; init;<br/>    for_each(v, &#124;elem&#124; accu &#61; f(accu, elem));<br/>    accu<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_foldr"></a>

## Function `foldr`

Fold right like fold above but working right to left. For example, <code>fold(vector[1,2,3], 0, f)</code> will execute
<code>f(1, f(2, f(3, 0)))</code>


<pre><code>public fun foldr&lt;Accumulator, Element&gt;(v: vector&lt;Element&gt;, init: Accumulator, f: &#124;(Element, Accumulator)&#124;Accumulator): Accumulator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun foldr&lt;Accumulator, Element&gt;(<br/>    v: vector&lt;Element&gt;,<br/>    init: Accumulator,<br/>    f: &#124;Element, Accumulator&#124;Accumulator<br/>): Accumulator &#123;<br/>    let accu &#61; init;<br/>    for_each_reverse(v, &#124;elem&#124; accu &#61; f(elem, accu));<br/>    accu<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_map_ref"></a>

## Function `map_ref`

Map the function over the references of the elements of the vector, producing a new vector without modifying the
original vector.


<pre><code>public fun map_ref&lt;Element, NewElement&gt;(v: &amp;vector&lt;Element&gt;, f: &#124;&amp;Element&#124;NewElement): vector&lt;NewElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map_ref&lt;Element, NewElement&gt;(<br/>    v: &amp;vector&lt;Element&gt;,<br/>    f: &#124;&amp;Element&#124;NewElement<br/>): vector&lt;NewElement&gt; &#123;<br/>    let result &#61; vector&lt;NewElement&gt;[];<br/>    for_each_ref(v, &#124;elem&#124; push_back(&amp;mut result, f(elem)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_zip_map_ref"></a>

## Function `zip_map_ref`

Map the function over the references of the element pairs of two vectors, producing a new vector from the return
values without modifying the original vectors.


<pre><code>public fun zip_map_ref&lt;Element1, Element2, NewElement&gt;(v1: &amp;vector&lt;Element1&gt;, v2: &amp;vector&lt;Element2&gt;, f: &#124;(&amp;Element1, &amp;Element2)&#124;NewElement): vector&lt;NewElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_map_ref&lt;Element1, Element2, NewElement&gt;(<br/>    v1: &amp;vector&lt;Element1&gt;,<br/>    v2: &amp;vector&lt;Element2&gt;,<br/>    f: &#124;&amp;Element1, &amp;Element2&#124;NewElement<br/>): vector&lt;NewElement&gt; &#123;<br/>    // We can&apos;t use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(length(v1) &#61;&#61; length(v2), 0x20002);<br/><br/>    let result &#61; vector&lt;NewElement&gt;[];<br/>    zip_ref(v1, v2, &#124;e1, e2&#124; push_back(&amp;mut result, f(e1, e2)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_map"></a>

## Function `map`

Map the function over the elements of the vector, producing a new vector.


<pre><code>public fun map&lt;Element, NewElement&gt;(v: vector&lt;Element&gt;, f: &#124;Element&#124;NewElement): vector&lt;NewElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map&lt;Element, NewElement&gt;(<br/>    v: vector&lt;Element&gt;,<br/>    f: &#124;Element&#124;NewElement<br/>): vector&lt;NewElement&gt; &#123;<br/>    let result &#61; vector&lt;NewElement&gt;[];<br/>    for_each(v, &#124;elem&#124; push_back(&amp;mut result, f(elem)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_zip_map"></a>

## Function `zip_map`

Map the function over the element pairs of the two vectors, producing a new vector.


<pre><code>public fun zip_map&lt;Element1, Element2, NewElement&gt;(v1: vector&lt;Element1&gt;, v2: vector&lt;Element2&gt;, f: &#124;(Element1, Element2)&#124;NewElement): vector&lt;NewElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun zip_map&lt;Element1, Element2, NewElement&gt;(<br/>    v1: vector&lt;Element1&gt;,<br/>    v2: vector&lt;Element2&gt;,<br/>    f: &#124;Element1, Element2&#124;NewElement<br/>): vector&lt;NewElement&gt; &#123;<br/>    // We can&apos;t use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it<br/>    // due to how inline functions work.<br/>    assert!(length(&amp;v1) &#61;&#61; length(&amp;v2), 0x20002);<br/><br/>    let result &#61; vector&lt;NewElement&gt;[];<br/>    zip(v1, v2, &#124;e1, e2&#124; push_back(&amp;mut result, f(e1, e2)));<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_filter"></a>

## Function `filter`

Filter the vector using the boolean function, removing all elements for which <code>p(e)</code> is not true.


<pre><code>public fun filter&lt;Element: drop&gt;(v: vector&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun filter&lt;Element:drop&gt;(<br/>    v: vector&lt;Element&gt;,<br/>    p: &#124;&amp;Element&#124;bool<br/>): vector&lt;Element&gt; &#123;<br/>    let result &#61; vector&lt;Element&gt;[];<br/>    for_each(v, &#124;elem&#124; &#123;<br/>        if (p(&amp;elem)) push_back(&amp;mut result, elem);<br/>    &#125;);<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_partition"></a>

## Function `partition`

Partition, sorts all elements for which pred is true to the front.
Preserves the relative order of the elements for which pred is true,
BUT NOT for the elements for which pred is false.


<pre><code>public fun partition&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, pred: &#124;&amp;Element&#124;bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun partition&lt;Element&gt;(<br/>    v: &amp;mut vector&lt;Element&gt;,<br/>    pred: &#124;&amp;Element&#124;bool<br/>): u64 &#123;<br/>    let i &#61; 0;<br/>    let len &#61; length(v);<br/>    while (i &lt; len) &#123;<br/>        if (!pred(borrow(v, i))) break;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    let p &#61; i;<br/>    i &#61; i &#43; 1;<br/>    while (i &lt; len) &#123;<br/>        if (pred(borrow(v, i))) &#123;<br/>            swap(v, p, i);<br/>            p &#61; p &#43; 1;<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    p<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_rotate"></a>

## Function `rotate`

rotate(&mut [1, 2, 3, 4, 5], 2) -> [3, 4, 5, 1, 2] in place, returns the split point
ie. 3 in the example above


<pre><code>public fun rotate&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, rot: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun rotate&lt;Element&gt;(<br/>    v: &amp;mut vector&lt;Element&gt;,<br/>    rot: u64<br/>): u64 &#123;<br/>    let len &#61; length(v);<br/>    rotate_slice(v, 0, rot, len)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_rotate_slice"></a>

## Function `rotate_slice`

Same as above but on a sub-slice of an array [left, right) with left <= rot <= right
returns the


<pre><code>public fun rotate_slice&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, left: u64, rot: u64, right: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun rotate_slice&lt;Element&gt;(<br/>    v: &amp;mut vector&lt;Element&gt;,<br/>    left: u64,<br/>    rot: u64,<br/>    right: u64<br/>): u64 &#123;<br/>    reverse_slice(v, left, rot);<br/>    reverse_slice(v, rot, right);<br/>    reverse_slice(v, left, right);<br/>    left &#43; (right &#45; rot)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_stable_partition"></a>

## Function `stable_partition`

Partition the array based on a predicate p, this routine is stable and thus
preserves the relative order of the elements in the two partitions.


<pre><code>public fun stable_partition&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun stable_partition&lt;Element&gt;(<br/>    v: &amp;mut vector&lt;Element&gt;,<br/>    p: &#124;&amp;Element&#124;bool<br/>): u64 &#123;<br/>    let len &#61; length(v);<br/>    let t &#61; empty();<br/>    let f &#61; empty();<br/>    while (len &gt; 0) &#123;<br/>        let e &#61; pop_back(v);<br/>        if (p(&amp;e)) &#123;<br/>            push_back(&amp;mut t, e);<br/>        &#125; else &#123;<br/>            push_back(&amp;mut f, e);<br/>        &#125;;<br/>        len &#61; len &#45; 1;<br/>    &#125;;<br/>    let pos &#61; length(&amp;t);<br/>    reverse_append(v, t);<br/>    reverse_append(v, f);<br/>    pos<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_any"></a>

## Function `any`

Return true if any element in the vector satisfies the predicate.


<pre><code>public fun any&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun any&lt;Element&gt;(<br/>    v: &amp;vector&lt;Element&gt;,<br/>    p: &#124;&amp;Element&#124;bool<br/>): bool &#123;<br/>    let result &#61; false;<br/>    let i &#61; 0;<br/>    while (i &lt; length(v)) &#123;<br/>        result &#61; p(borrow(v, i));<br/>        if (result) &#123;<br/>            break<br/>        &#125;;<br/>        i &#61; i &#43; 1<br/>    &#125;;<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_all"></a>

## Function `all`

Return true if all elements in the vector satisfy the predicate.


<pre><code>public fun all&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun all&lt;Element&gt;(<br/>    v: &amp;vector&lt;Element&gt;,<br/>    p: &#124;&amp;Element&#124;bool<br/>): bool &#123;<br/>    let result &#61; true;<br/>    let i &#61; 0;<br/>    while (i &lt; length(v)) &#123;<br/>        result &#61; p(borrow(v, i));<br/>        if (!result) &#123;<br/>            break<br/>        &#125;;<br/>        i &#61; i &#43; 1<br/>    &#125;;<br/>    result<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_destroy"></a>

## Function `destroy`

Destroy a vector, just a wrapper around for_each_reverse with a descriptive name
when used in the context of destroying a vector.


<pre><code>public fun destroy&lt;Element&gt;(v: vector&lt;Element&gt;, d: &#124;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun destroy&lt;Element&gt;(<br/>    v: vector&lt;Element&gt;,<br/>    d: &#124;Element&#124;<br/>) &#123;<br/>    for_each_reverse(v, &#124;e&#124; d(e))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_range"></a>

## Function `range`



<pre><code>public fun range(start: u64, end: u64): vector&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun range(start: u64, end: u64): vector&lt;u64&gt; &#123;<br/>    range_with_step(start, end, 1)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_range_with_step"></a>

## Function `range_with_step`



<pre><code>public fun range_with_step(start: u64, end: u64, step: u64): vector&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun range_with_step(start: u64, end: u64, step: u64): vector&lt;u64&gt; &#123;<br/>    assert!(step &gt; 0, EINVALID_STEP);<br/><br/>    let vec &#61; vector[];<br/>    while (start &lt; end) &#123;<br/>        push_back(&amp;mut vec, start);<br/>        start &#61; start &#43; step;<br/>    &#125;;<br/>    vec<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_vector_slice"></a>

## Function `slice`



<pre><code>public fun slice&lt;Element: copy&gt;(v: &amp;vector&lt;Element&gt;, start: u64, end: u64): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun slice&lt;Element: copy&gt;(<br/>    v: &amp;vector&lt;Element&gt;,<br/>    start: u64,<br/>    end: u64<br/>): vector&lt;Element&gt; &#123;<br/>    assert!(start &lt;&#61; end &amp;&amp; end &lt;&#61; length(v), EINVALID_SLICE_RANGE);<br/><br/>    let vec &#61; vector[];<br/>    while (start &lt; end) &#123;<br/>        push_back(&amp;mut vec, &#42;borrow(v, start));<br/>        start &#61; start &#43; 1;<br/>    &#125;;<br/>    vec<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<a id="@Helper_Functions_2"></a>

### Helper Functions


Check if <code>v1</code> is equal to the result of adding <code>e</code> at the end of <code>v2</code>


<a id="0x1_vector_eq_push_back"></a>


<pre><code>fun eq_push_back&lt;Element&gt;(v1: vector&lt;Element&gt;, v2: vector&lt;Element&gt;, e: Element): bool &#123;<br/>    len(v1) &#61;&#61; len(v2) &#43; 1 &amp;&amp;<br/>    v1[len(v1)&#45;1] &#61;&#61; e &amp;&amp;<br/>    v1[0..len(v1)&#45;1] &#61;&#61; v2[0..len(v2)]<br/>&#125;<br/></code></pre>


Check if <code>v</code> is equal to the result of concatenating <code>v1</code> and <code>v2</code>


<a id="0x1_vector_eq_append"></a>


<pre><code>fun eq_append&lt;Element&gt;(v: vector&lt;Element&gt;, v1: vector&lt;Element&gt;, v2: vector&lt;Element&gt;): bool &#123;<br/>    len(v) &#61;&#61; len(v1) &#43; len(v2) &amp;&amp;<br/>    v[0..len(v1)] &#61;&#61; v1 &amp;&amp;<br/>    v[len(v1)..len(v)] &#61;&#61; v2<br/>&#125;<br/></code></pre>


Check <code>v1</code> is equal to the result of removing the first element of <code>v2</code>


<a id="0x1_vector_eq_pop_front"></a>


<pre><code>fun eq_pop_front&lt;Element&gt;(v1: vector&lt;Element&gt;, v2: vector&lt;Element&gt;): bool &#123;<br/>    len(v1) &#43; 1 &#61;&#61; len(v2) &amp;&amp;<br/>    v1 &#61;&#61; v2[1..len(v2)]<br/>&#125;<br/></code></pre>


Check that <code>v1</code> is equal to the result of removing the element at index <code>i</code> from <code>v2</code>.


<a id="0x1_vector_eq_remove_elem_at_index"></a>


<pre><code>fun eq_remove_elem_at_index&lt;Element&gt;(i: u64, v1: vector&lt;Element&gt;, v2: vector&lt;Element&gt;): bool &#123;<br/>    len(v1) &#43; 1 &#61;&#61; len(v2) &amp;&amp;<br/>    v1[0..i] &#61;&#61; v2[0..i] &amp;&amp;<br/>    v1[i..len(v1)] &#61;&#61; v2[i &#43; 1..len(v2)]<br/>&#125;<br/></code></pre>


Check if <code>v</code> contains <code>e</code>.


<a id="0x1_vector_spec_contains"></a>


<pre><code>fun spec_contains&lt;Element&gt;(v: vector&lt;Element&gt;, e: Element): bool &#123;<br/>    exists x in v: x &#61;&#61; e<br/>&#125;<br/></code></pre>



<a id="@Specification_1_singleton"></a>

### Function `singleton`


<pre><code>public fun singleton&lt;Element&gt;(e: Element): vector&lt;Element&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; vec(e);<br/></code></pre>



<a id="@Specification_1_reverse"></a>

### Function `reverse`


<pre><code>public fun reverse&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;)<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_reverse_slice"></a>

### Function `reverse_slice`


<pre><code>public fun reverse_slice&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, left: u64, right: u64)<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_append"></a>

### Function `append`


<pre><code>public fun append&lt;Element&gt;(lhs: &amp;mut vector&lt;Element&gt;, other: vector&lt;Element&gt;)<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_reverse_append"></a>

### Function `reverse_append`


<pre><code>public fun reverse_append&lt;Element&gt;(lhs: &amp;mut vector&lt;Element&gt;, other: vector&lt;Element&gt;)<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_trim"></a>

### Function `trim`


<pre><code>public fun trim&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, new_len: u64): vector&lt;Element&gt;<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_trim_reverse"></a>

### Function `trim_reverse`


<pre><code>public fun trim_reverse&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, new_len: u64): vector&lt;Element&gt;<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_is_empty"></a>

### Function `is_empty`


<pre><code>public fun is_empty&lt;Element&gt;(v: &amp;vector&lt;Element&gt;): bool<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code>public fun contains&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, e: &amp;Element): bool<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_index_of"></a>

### Function `index_of`


<pre><code>public fun index_of&lt;Element&gt;(v: &amp;vector&lt;Element&gt;, e: &amp;Element): (bool, u64)<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_insert"></a>

### Function `insert`


<pre><code>public fun insert&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64, e: Element)<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): Element<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_remove_value"></a>

### Function `remove_value`


<pre><code>public fun remove_value&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, val: &amp;Element): vector&lt;Element&gt;<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_swap_remove"></a>

### Function `swap_remove`


<pre><code>public fun swap_remove&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, i: u64): Element<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_rotate"></a>

### Function `rotate`


<pre><code>public fun rotate&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, rot: u64): u64<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>



<a id="@Specification_1_rotate_slice"></a>

### Function `rotate_slice`


<pre><code>public fun rotate_slice&lt;Element&gt;(v: &amp;mut vector&lt;Element&gt;, left: u64, rot: u64, right: u64): u64<br/></code></pre>




<pre><code>pragma intrinsic &#61; true;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
