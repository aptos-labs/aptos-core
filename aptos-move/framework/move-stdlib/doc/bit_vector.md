
<a id="0x1_bit_vector"></a>

# Module `0x1::bit_vector`



-  [Struct `BitVector`](#0x1_bit_vector_BitVector)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_bit_vector_new)
-  [Function `set`](#0x1_bit_vector_set)
-  [Function `unset`](#0x1_bit_vector_unset)
-  [Function `shift_left`](#0x1_bit_vector_shift_left)
-  [Function `is_index_set`](#0x1_bit_vector_is_index_set)
-  [Function `length`](#0x1_bit_vector_length)
-  [Function `longest_set_sequence_starting_at`](#0x1_bit_vector_longest_set_sequence_starting_at)
-  [Function `shift_left_for_verification_only`](#0x1_bit_vector_shift_left_for_verification_only)
-  [Specification](#@Specification_1)
    -  [Struct `BitVector`](#@Specification_1_BitVector)
    -  [Function `new`](#@Specification_1_new)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `unset`](#@Specification_1_unset)
    -  [Function `shift_left`](#@Specification_1_shift_left)
    -  [Function `is_index_set`](#@Specification_1_is_index_set)
    -  [Function `longest_set_sequence_starting_at`](#@Specification_1_longest_set_sequence_starting_at)
    -  [Function `shift_left_for_verification_only`](#@Specification_1_shift_left_for_verification_only)


<pre><code></code></pre>



<a id="0x1_bit_vector_BitVector"></a>

## Struct `BitVector`



<pre><code>struct BitVector has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bit_field: vector&lt;bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_bit_vector_EINDEX"></a>

The provided index is out of bounds


<pre><code>const EINDEX: u64 &#61; 131072;<br/></code></pre>



<a id="0x1_bit_vector_ELENGTH"></a>

An invalid length of bitvector was given


<pre><code>const ELENGTH: u64 &#61; 131073;<br/></code></pre>



<a id="0x1_bit_vector_MAX_SIZE"></a>

The maximum allowed bitvector size


<pre><code>const MAX_SIZE: u64 &#61; 1024;<br/></code></pre>



<a id="0x1_bit_vector_WORD_SIZE"></a>



<pre><code>const WORD_SIZE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_bit_vector_new"></a>

## Function `new`



<pre><code>public fun new(length: u64): bit_vector::BitVector<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new(length: u64): BitVector &#123;<br/>    assert!(length &gt; 0, ELENGTH);<br/>    assert!(length &lt; MAX_SIZE, ELENGTH);<br/>    let counter &#61; 0;<br/>    let bit_field &#61; vector::empty();<br/>    while (&#123;spec &#123;<br/>        invariant counter &lt;&#61; length;<br/>        invariant len(bit_field) &#61;&#61; counter;<br/>    &#125;;<br/>        (counter &lt; length)&#125;) &#123;<br/>        vector::push_back(&amp;mut bit_field, false);<br/>        counter &#61; counter &#43; 1;<br/>    &#125;;<br/>    spec &#123;<br/>        assert counter &#61;&#61; length;<br/>        assert len(bit_field) &#61;&#61; length;<br/>    &#125;;<br/><br/>    BitVector &#123;<br/>        length,<br/>        bit_field,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_set"></a>

## Function `set`

Set the bit at <code>bit_index</code> in the <code>bitvector</code> regardless of its previous state.


<pre><code>public fun set(bitvector: &amp;mut bit_vector::BitVector, bit_index: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set(bitvector: &amp;mut BitVector, bit_index: u64) &#123;<br/>    assert!(bit_index &lt; vector::length(&amp;bitvector.bit_field), EINDEX);<br/>    let x &#61; vector::borrow_mut(&amp;mut bitvector.bit_field, bit_index);<br/>    &#42;x &#61; true;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_unset"></a>

## Function `unset`

Unset the bit at <code>bit_index</code> in the <code>bitvector</code> regardless of its previous state.


<pre><code>public fun unset(bitvector: &amp;mut bit_vector::BitVector, bit_index: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unset(bitvector: &amp;mut BitVector, bit_index: u64) &#123;<br/>    assert!(bit_index &lt; vector::length(&amp;bitvector.bit_field), EINDEX);<br/>    let x &#61; vector::borrow_mut(&amp;mut bitvector.bit_field, bit_index);<br/>    &#42;x &#61; false;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_shift_left"></a>

## Function `shift_left`

Shift the <code>bitvector</code> left by <code>amount</code>. If <code>amount</code> is greater than the
bitvector's length the bitvector will be zeroed out.


<pre><code>public fun shift_left(bitvector: &amp;mut bit_vector::BitVector, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shift_left(bitvector: &amp;mut BitVector, amount: u64) &#123;<br/>    if (amount &gt;&#61; bitvector.length) &#123;<br/>        vector::for_each_mut(&amp;mut bitvector.bit_field, &#124;elem&#124; &#123;<br/>            &#42;elem &#61; false;<br/>        &#125;);<br/>    &#125; else &#123;<br/>        let i &#61; amount;<br/><br/>        while (i &lt; bitvector.length) &#123;<br/>            if (is_index_set(bitvector, i)) set(bitvector, i &#45; amount)<br/>            else unset(bitvector, i &#45; amount);<br/>            i &#61; i &#43; 1;<br/>        &#125;;<br/><br/>        i &#61; bitvector.length &#45; amount;<br/><br/>        while (i &lt; bitvector.length) &#123;<br/>            unset(bitvector, i);<br/>            i &#61; i &#43; 1;<br/>        &#125;;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_is_index_set"></a>

## Function `is_index_set`

Return the value of the bit at <code>bit_index</code> in the <code>bitvector</code>. <code>true</code>
represents "1" and <code>false</code> represents a 0


<pre><code>public fun is_index_set(bitvector: &amp;bit_vector::BitVector, bit_index: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_index_set(bitvector: &amp;BitVector, bit_index: u64): bool &#123;<br/>    assert!(bit_index &lt; vector::length(&amp;bitvector.bit_field), EINDEX);<br/>    &#42;vector::borrow(&amp;bitvector.bit_field, bit_index)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_length"></a>

## Function `length`

Return the length (number of usable bits) of this bitvector


<pre><code>public fun length(bitvector: &amp;bit_vector::BitVector): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length(bitvector: &amp;BitVector): u64 &#123;<br/>    vector::length(&amp;bitvector.bit_field)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_longest_set_sequence_starting_at"></a>

## Function `longest_set_sequence_starting_at`

Returns the length of the longest sequence of set bits starting at (and
including) <code>start_index</code> in the <code>bitvector</code>. If there is no such
sequence, then <code>0</code> is returned.


<pre><code>public fun longest_set_sequence_starting_at(bitvector: &amp;bit_vector::BitVector, start_index: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun longest_set_sequence_starting_at(bitvector: &amp;BitVector, start_index: u64): u64 &#123;<br/>    assert!(start_index &lt; bitvector.length, EINDEX);<br/>    let index &#61; start_index;<br/><br/>    // Find the greatest index in the vector such that all indices less than it are set.<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant index &gt;&#61; start_index;<br/>            invariant index &#61;&#61; start_index &#124;&#124; is_index_set(bitvector, index &#45; 1);<br/>            invariant index &#61;&#61; start_index &#124;&#124; index &#45; 1 &lt; vector::length(bitvector.bit_field);<br/>            invariant forall j in start_index..index: is_index_set(bitvector, j);<br/>            invariant forall j in start_index..index: j &lt; vector::length(bitvector.bit_field);<br/>        &#125;;<br/>        index &lt; bitvector.length<br/>    &#125;) &#123;<br/>        if (!is_index_set(bitvector, index)) break;<br/>        index &#61; index &#43; 1;<br/>    &#125;;<br/><br/>    index &#45; start_index<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_bit_vector_shift_left_for_verification_only"></a>

## Function `shift_left_for_verification_only`



<pre><code>&#35;[verify_only]<br/>public fun shift_left_for_verification_only(bitvector: &amp;mut bit_vector::BitVector, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shift_left_for_verification_only(bitvector: &amp;mut BitVector, amount: u64) &#123;<br/>    if (amount &gt;&#61; bitvector.length) &#123;<br/>        let len &#61; vector::length(&amp;bitvector.bit_field);<br/>        let i &#61; 0;<br/>        while (&#123;<br/>            spec &#123;<br/>                invariant len &#61;&#61; bitvector.length;<br/>                invariant forall k in 0..i: !bitvector.bit_field[k];<br/>                invariant forall k in i..bitvector.length: bitvector.bit_field[k] &#61;&#61; old(bitvector).bit_field[k];<br/>            &#125;;<br/>            i &lt; len<br/>        &#125;) &#123;<br/>            let elem &#61; vector::borrow_mut(&amp;mut bitvector.bit_field, i);<br/>            &#42;elem &#61; false;<br/>            i &#61; i &#43; 1;<br/>        &#125;;<br/>    &#125; else &#123;<br/>        let i &#61; amount;<br/><br/>        while (&#123;<br/>            spec &#123;<br/>                invariant i &gt;&#61; amount;<br/>                invariant bitvector.length &#61;&#61; old(bitvector).length;<br/>                invariant forall j in amount..i: old(bitvector).bit_field[j] &#61;&#61; bitvector.bit_field[j &#45; amount];<br/>                invariant forall j in (i&#45;amount)..bitvector.length : old(bitvector).bit_field[j] &#61;&#61; bitvector.bit_field[j];<br/>                invariant forall k in 0..i&#45;amount: bitvector.bit_field[k] &#61;&#61; old(bitvector).bit_field[k &#43; amount];<br/>            &#125;;<br/>            i &lt; bitvector.length<br/>        &#125;) &#123;<br/>            if (is_index_set(bitvector, i)) set(bitvector, i &#45; amount)<br/>            else unset(bitvector, i &#45; amount);<br/>            i &#61; i &#43; 1;<br/>        &#125;;<br/><br/><br/>        i &#61; bitvector.length &#45; amount;<br/><br/>        while (&#123;<br/>            spec &#123;<br/>                invariant forall j in bitvector.length &#45; amount..i: !bitvector.bit_field[j];<br/>                invariant forall k in 0..bitvector.length &#45; amount: bitvector.bit_field[k] &#61;&#61; old(bitvector).bit_field[k &#43; amount];<br/>                invariant i &gt;&#61; bitvector.length &#45; amount;<br/>            &#125;;<br/>            i &lt; bitvector.length<br/>        &#125;) &#123;<br/>            unset(bitvector, i);<br/>            i &#61; i &#43; 1;<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_BitVector"></a>

### Struct `BitVector`


<pre><code>struct BitVector has copy, drop, store<br/></code></pre>



<dl>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bit_field: vector&lt;bool&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant length &#61;&#61; len(bit_field);<br/></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public fun new(length: u64): bit_vector::BitVector<br/></code></pre>




<pre><code>include NewAbortsIf;<br/>ensures result.length &#61;&#61; length;<br/>ensures len(result.bit_field) &#61;&#61; length;<br/></code></pre>




<a id="0x1_bit_vector_NewAbortsIf"></a>


<pre><code>schema NewAbortsIf &#123;<br/>length: u64;<br/>aborts_if length &lt;&#61; 0 with ELENGTH;<br/>aborts_if length &gt;&#61; MAX_SIZE with ELENGTH;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code>public fun set(bitvector: &amp;mut bit_vector::BitVector, bit_index: u64)<br/></code></pre>




<pre><code>include SetAbortsIf;<br/>ensures bitvector.bit_field[bit_index];<br/></code></pre>




<a id="0x1_bit_vector_SetAbortsIf"></a>


<pre><code>schema SetAbortsIf &#123;<br/>bitvector: BitVector;<br/>bit_index: u64;<br/>aborts_if bit_index &gt;&#61; length(bitvector) with EINDEX;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_unset"></a>

### Function `unset`


<pre><code>public fun unset(bitvector: &amp;mut bit_vector::BitVector, bit_index: u64)<br/></code></pre>




<pre><code>include UnsetAbortsIf;<br/>ensures !bitvector.bit_field[bit_index];<br/></code></pre>




<a id="0x1_bit_vector_UnsetAbortsIf"></a>


<pre><code>schema UnsetAbortsIf &#123;<br/>bitvector: BitVector;<br/>bit_index: u64;<br/>aborts_if bit_index &gt;&#61; length(bitvector) with EINDEX;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_shift_left"></a>

### Function `shift_left`


<pre><code>public fun shift_left(bitvector: &amp;mut bit_vector::BitVector, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_is_index_set"></a>

### Function `is_index_set`


<pre><code>public fun is_index_set(bitvector: &amp;bit_vector::BitVector, bit_index: u64): bool<br/></code></pre>




<pre><code>include IsIndexSetAbortsIf;<br/>ensures result &#61;&#61; bitvector.bit_field[bit_index];<br/></code></pre>




<a id="0x1_bit_vector_IsIndexSetAbortsIf"></a>


<pre><code>schema IsIndexSetAbortsIf &#123;<br/>bitvector: BitVector;<br/>bit_index: u64;<br/>aborts_if bit_index &gt;&#61; length(bitvector) with EINDEX;<br/>&#125;<br/></code></pre>




<a id="0x1_bit_vector_spec_is_index_set"></a>


<pre><code>fun spec_is_index_set(bitvector: BitVector, bit_index: u64): bool &#123;<br/>   if (bit_index &gt;&#61; length(bitvector)) &#123;<br/>       false<br/>   &#125; else &#123;<br/>       bitvector.bit_field[bit_index]<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_longest_set_sequence_starting_at"></a>

### Function `longest_set_sequence_starting_at`


<pre><code>public fun longest_set_sequence_starting_at(bitvector: &amp;bit_vector::BitVector, start_index: u64): u64<br/></code></pre>




<pre><code>aborts_if start_index &gt;&#61; bitvector.length;<br/>ensures forall i in start_index..result: is_index_set(bitvector, i);<br/></code></pre>



<a id="@Specification_1_shift_left_for_verification_only"></a>

### Function `shift_left_for_verification_only`


<pre><code>&#35;[verify_only]<br/>public fun shift_left_for_verification_only(bitvector: &amp;mut bit_vector::BitVector, amount: u64)<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures amount &gt;&#61; bitvector.length &#61;&#61;&gt; (forall k in 0..bitvector.length: !bitvector.bit_field[k]);<br/>ensures amount &lt; bitvector.length &#61;&#61;&gt;<br/>    (forall i in bitvector.length &#45; amount..bitvector.length: !bitvector.bit_field[i]);<br/>ensures amount &lt; bitvector.length &#61;&#61;&gt;<br/>    (forall i in 0..bitvector.length &#45; amount: bitvector.bit_field[i] &#61;&#61; old(bitvector).bit_field[i &#43; amount]);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
