
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



<pre><code><b>struct</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bit_field: <a href="vector.md#0x1_vector">vector</a>&lt;bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_bit_vector_EINDEX"></a>

The provided index is out of bounds


<pre><code><b>const</b> <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>: u64 &#61; 131072;<br /></code></pre>



<a id="0x1_bit_vector_ELENGTH"></a>

An invalid length of bitvector was given


<pre><code><b>const</b> <a href="bit_vector.md#0x1_bit_vector_ELENGTH">ELENGTH</a>: u64 &#61; 131073;<br /></code></pre>



<a id="0x1_bit_vector_MAX_SIZE"></a>

The maximum allowed bitvector size


<pre><code><b>const</b> <a href="bit_vector.md#0x1_bit_vector_MAX_SIZE">MAX_SIZE</a>: u64 &#61; 1024;<br /></code></pre>



<a id="0x1_bit_vector_WORD_SIZE"></a>



<pre><code><b>const</b> <a href="bit_vector.md#0x1_bit_vector_WORD_SIZE">WORD_SIZE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_bit_vector_new"></a>

## Function `new`



<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_new">new</a>(length: u64): <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_new">new</a>(length: u64): <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a> &#123;<br />    <b>assert</b>!(length &gt; 0, <a href="bit_vector.md#0x1_bit_vector_ELENGTH">ELENGTH</a>);<br />    <b>assert</b>!(<a href="bit_vector.md#0x1_bit_vector_length">length</a> &lt; <a href="bit_vector.md#0x1_bit_vector_MAX_SIZE">MAX_SIZE</a>, <a href="bit_vector.md#0x1_bit_vector_ELENGTH">ELENGTH</a>);<br />    <b>let</b> counter &#61; 0;<br />    <b>let</b> bit_field &#61; <a href="vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>while</b> (&#123;<b>spec</b> &#123;<br />        <b>invariant</b> counter &lt;&#61; length;<br />        <b>invariant</b> len(bit_field) &#61;&#61; counter;<br />    &#125;;<br />        (counter &lt; length)&#125;) &#123;<br />        <a href="vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> bit_field, <b>false</b>);<br />        counter &#61; counter &#43; 1;<br />    &#125;;<br />    <b>spec</b> &#123;<br />        <b>assert</b> counter &#61;&#61; length;<br />        <b>assert</b> len(bit_field) &#61;&#61; length;<br />    &#125;;<br /><br />    <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a> &#123;<br />        length,<br />        bit_field,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_set"></a>

## Function `set`

Set the bit at <code>bit_index</code> in the <code>bitvector</code> regardless of its previous state.


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_set">set</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, bit_index: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_set">set</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, bit_index: u64) &#123;<br />    <b>assert</b>!(bit_index &lt; <a href="vector.md#0x1_vector_length">vector::length</a>(&amp;bitvector.bit_field), <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>);<br />    <b>let</b> x &#61; <a href="vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> bitvector.bit_field, bit_index);<br />    &#42;x &#61; <b>true</b>;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_unset"></a>

## Function `unset`

Unset the bit at <code>bit_index</code> in the <code>bitvector</code> regardless of its previous state.


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, bit_index: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, bit_index: u64) &#123;<br />    <b>assert</b>!(bit_index &lt; <a href="vector.md#0x1_vector_length">vector::length</a>(&amp;bitvector.bit_field), <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>);<br />    <b>let</b> x &#61; <a href="vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> bitvector.bit_field, bit_index);<br />    &#42;x &#61; <b>false</b>;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_shift_left"></a>

## Function `shift_left`

Shift the <code>bitvector</code> left by <code>amount</code>. If <code>amount</code> is greater than the
bitvector&apos;s length the bitvector will be zeroed out.


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_shift_left">shift_left</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_shift_left">shift_left</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, amount: u64) &#123;<br />    <b>if</b> (amount &gt;&#61; bitvector.length) &#123;<br />        <a href="vector.md#0x1_vector_for_each_mut">vector::for_each_mut</a>(&amp;<b>mut</b> bitvector.bit_field, &#124;elem&#124; &#123;<br />            &#42;elem &#61; <b>false</b>;<br />        &#125;);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> i &#61; amount;<br /><br />        <b>while</b> (i &lt; bitvector.length) &#123;<br />            <b>if</b> (<a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector, i)) <a href="bit_vector.md#0x1_bit_vector_set">set</a>(bitvector, i &#45; amount)<br />            <b>else</b> <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector, i &#45; amount);<br />            i &#61; i &#43; 1;<br />        &#125;;<br /><br />        i &#61; bitvector.length &#45; amount;<br /><br />        <b>while</b> (i &lt; bitvector.length) &#123;<br />            <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector, i);<br />            i &#61; i &#43; 1;<br />        &#125;;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_is_index_set"></a>

## Function `is_index_set`

Return the value of the bit at <code>bit_index</code> in the <code>bitvector</code>. <code><b>true</b></code>
represents &quot;1&quot; and <code><b>false</b></code> represents a 0


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, bit_index: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, bit_index: u64): bool &#123;<br />    <b>assert</b>!(bit_index &lt; <a href="vector.md#0x1_vector_length">vector::length</a>(&amp;bitvector.bit_field), <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>);<br />    &#42;<a href="vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;bitvector.bit_field, bit_index)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_length"></a>

## Function `length`

Return the length (number of usable bits) of this bitvector


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_length">length</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_length">length</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>): u64 &#123;<br />    <a href="vector.md#0x1_vector_length">vector::length</a>(&amp;bitvector.bit_field)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_longest_set_sequence_starting_at"></a>

## Function `longest_set_sequence_starting_at`

Returns the length of the longest sequence of set bits starting at (and
including) <code>start_index</code> in the <code>bitvector</code>. If there is no such
sequence, then <code>0</code> is returned.


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_longest_set_sequence_starting_at">longest_set_sequence_starting_at</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, start_index: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_longest_set_sequence_starting_at">longest_set_sequence_starting_at</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, start_index: u64): u64 &#123;<br />    <b>assert</b>!(start_index &lt; bitvector.length, <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>);<br />    <b>let</b> index &#61; start_index;<br /><br />    // Find the greatest index in the <a href="vector.md#0x1_vector">vector</a> such that all indices less than it are set.<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> index &gt;&#61; start_index;<br />            <b>invariant</b> index &#61;&#61; start_index &#124;&#124; <a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector, index &#45; 1);<br />            <b>invariant</b> index &#61;&#61; start_index &#124;&#124; index &#45; 1 &lt; <a href="vector.md#0x1_vector_length">vector::length</a>(bitvector.bit_field);<br />            <b>invariant</b> <b>forall</b> j in start_index..index: <a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector, j);<br />            <b>invariant</b> <b>forall</b> j in start_index..index: j &lt; <a href="vector.md#0x1_vector_length">vector::length</a>(bitvector.bit_field);<br />        &#125;;<br />        index &lt; bitvector.length<br />    &#125;) &#123;<br />        <b>if</b> (!<a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector, index)) <b>break</b>;<br />        index &#61; index &#43; 1;<br />    &#125;;<br /><br />    index &#45; start_index<br />&#125;<br /></code></pre>



</details>

<a id="0x1_bit_vector_shift_left_for_verification_only"></a>

## Function `shift_left_for_verification_only`



<pre><code>&#35;[verify_only]<br /><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_shift_left_for_verification_only">shift_left_for_verification_only</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_shift_left_for_verification_only">shift_left_for_verification_only</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, amount: u64) &#123;<br />    <b>if</b> (amount &gt;&#61; bitvector.length) &#123;<br />        <b>let</b> len &#61; <a href="vector.md#0x1_vector_length">vector::length</a>(&amp;bitvector.bit_field);<br />        <b>let</b> i &#61; 0;<br />        <b>while</b> (&#123;<br />            <b>spec</b> &#123;<br />                <b>invariant</b> len &#61;&#61; bitvector.length;<br />                <b>invariant</b> <b>forall</b> k in 0..i: !bitvector.bit_field[k];<br />                <b>invariant</b> <b>forall</b> k in i..bitvector.length: bitvector.bit_field[k] &#61;&#61; <b>old</b>(bitvector).bit_field[k];<br />            &#125;;<br />            i &lt; len<br />        &#125;) &#123;<br />            <b>let</b> elem &#61; <a href="vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> bitvector.bit_field, i);<br />            &#42;elem &#61; <b>false</b>;<br />            i &#61; i &#43; 1;<br />        &#125;;<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> i &#61; amount;<br /><br />        <b>while</b> (&#123;<br />            <b>spec</b> &#123;<br />                <b>invariant</b> i &gt;&#61; amount;<br />                <b>invariant</b> bitvector.length &#61;&#61; <b>old</b>(bitvector).length;<br />                <b>invariant</b> <b>forall</b> j in amount..i: <b>old</b>(bitvector).bit_field[j] &#61;&#61; bitvector.bit_field[j &#45; amount];<br />                <b>invariant</b> <b>forall</b> j in (i&#45;amount)..bitvector.length : <b>old</b>(bitvector).bit_field[j] &#61;&#61; bitvector.bit_field[j];<br />                <b>invariant</b> <b>forall</b> k in 0..i&#45;amount: bitvector.bit_field[k] &#61;&#61; <b>old</b>(bitvector).bit_field[k &#43; amount];<br />            &#125;;<br />            i &lt; bitvector.length<br />        &#125;) &#123;<br />            <b>if</b> (<a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector, i)) <a href="bit_vector.md#0x1_bit_vector_set">set</a>(bitvector, i &#45; amount)<br />            <b>else</b> <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector, i &#45; amount);<br />            i &#61; i &#43; 1;<br />        &#125;;<br /><br /><br />        i &#61; bitvector.length &#45; amount;<br /><br />        <b>while</b> (&#123;<br />            <b>spec</b> &#123;<br />                <b>invariant</b> <b>forall</b> j in bitvector.length &#45; amount..i: !bitvector.bit_field[j];<br />                <b>invariant</b> <b>forall</b> k in 0..bitvector.length &#45; amount: bitvector.bit_field[k] &#61;&#61; <b>old</b>(bitvector).bit_field[k &#43; amount];<br />                <b>invariant</b> i &gt;&#61; bitvector.length &#45; amount;<br />            &#125;;<br />            i &lt; bitvector.length<br />        &#125;) &#123;<br />            <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector, i);<br />            i &#61; i &#43; 1;<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_BitVector"></a>

### Struct `BitVector`


<pre><code><b>struct</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<dl>
<dt>
<code>length: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bit_field: <a href="vector.md#0x1_vector">vector</a>&lt;bool&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> length &#61;&#61; len(bit_field);<br /></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_new">new</a>(length: u64): <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a><br /></code></pre>




<pre><code><b>include</b> <a href="bit_vector.md#0x1_bit_vector_NewAbortsIf">NewAbortsIf</a>;<br /><b>ensures</b> result.length &#61;&#61; length;<br /><b>ensures</b> len(result.bit_field) &#61;&#61; length;<br /></code></pre>




<a id="0x1_bit_vector_NewAbortsIf"></a>


<pre><code><b>schema</b> <a href="bit_vector.md#0x1_bit_vector_NewAbortsIf">NewAbortsIf</a> &#123;<br />length: u64;<br /><b>aborts_if</b> <a href="bit_vector.md#0x1_bit_vector_length">length</a> &lt;&#61; 0 <b>with</b> <a href="bit_vector.md#0x1_bit_vector_ELENGTH">ELENGTH</a>;<br /><b>aborts_if</b> length &gt;&#61; <a href="bit_vector.md#0x1_bit_vector_MAX_SIZE">MAX_SIZE</a> <b>with</b> <a href="bit_vector.md#0x1_bit_vector_ELENGTH">ELENGTH</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_set">set</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, bit_index: u64)<br /></code></pre>




<pre><code><b>include</b> <a href="bit_vector.md#0x1_bit_vector_SetAbortsIf">SetAbortsIf</a>;<br /><b>ensures</b> bitvector.bit_field[bit_index];<br /></code></pre>




<a id="0x1_bit_vector_SetAbortsIf"></a>


<pre><code><b>schema</b> <a href="bit_vector.md#0x1_bit_vector_SetAbortsIf">SetAbortsIf</a> &#123;<br />bitvector: <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>;<br />bit_index: u64;<br /><b>aborts_if</b> bit_index &gt;&#61; <a href="bit_vector.md#0x1_bit_vector_length">length</a>(bitvector) <b>with</b> <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_unset"></a>

### Function `unset`


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_unset">unset</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, bit_index: u64)<br /></code></pre>




<pre><code><b>include</b> <a href="bit_vector.md#0x1_bit_vector_UnsetAbortsIf">UnsetAbortsIf</a>;<br /><b>ensures</b> !bitvector.bit_field[bit_index];<br /></code></pre>




<a id="0x1_bit_vector_UnsetAbortsIf"></a>


<pre><code><b>schema</b> <a href="bit_vector.md#0x1_bit_vector_UnsetAbortsIf">UnsetAbortsIf</a> &#123;<br />bitvector: <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>;<br />bit_index: u64;<br /><b>aborts_if</b> bit_index &gt;&#61; <a href="bit_vector.md#0x1_bit_vector_length">length</a>(bitvector) <b>with</b> <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_shift_left"></a>

### Function `shift_left`


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_shift_left">shift_left</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_is_index_set"></a>

### Function `is_index_set`


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, bit_index: u64): bool<br /></code></pre>




<pre><code><b>include</b> <a href="bit_vector.md#0x1_bit_vector_IsIndexSetAbortsIf">IsIndexSetAbortsIf</a>;<br /><b>ensures</b> result &#61;&#61; bitvector.bit_field[bit_index];<br /></code></pre>




<a id="0x1_bit_vector_IsIndexSetAbortsIf"></a>


<pre><code><b>schema</b> <a href="bit_vector.md#0x1_bit_vector_IsIndexSetAbortsIf">IsIndexSetAbortsIf</a> &#123;<br />bitvector: <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>;<br />bit_index: u64;<br /><b>aborts_if</b> bit_index &gt;&#61; <a href="bit_vector.md#0x1_bit_vector_length">length</a>(bitvector) <b>with</b> <a href="bit_vector.md#0x1_bit_vector_EINDEX">EINDEX</a>;<br />&#125;<br /></code></pre>




<a id="0x1_bit_vector_spec_is_index_set"></a>


<pre><code><b>fun</b> <a href="bit_vector.md#0x1_bit_vector_spec_is_index_set">spec_is_index_set</a>(bitvector: <a href="bit_vector.md#0x1_bit_vector_BitVector">BitVector</a>, bit_index: u64): bool &#123;<br />   <b>if</b> (bit_index &gt;&#61; <a href="bit_vector.md#0x1_bit_vector_length">length</a>(bitvector)) &#123;<br />       <b>false</b><br />   &#125; <b>else</b> &#123;<br />       bitvector.bit_field[bit_index]<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_longest_set_sequence_starting_at"></a>

### Function `longest_set_sequence_starting_at`


<pre><code><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_longest_set_sequence_starting_at">longest_set_sequence_starting_at</a>(bitvector: &amp;<a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, start_index: u64): u64<br /></code></pre>




<pre><code><b>aborts_if</b> start_index &gt;&#61; bitvector.length;<br /><b>ensures</b> <b>forall</b> i in start_index..result: <a href="bit_vector.md#0x1_bit_vector_is_index_set">is_index_set</a>(bitvector, i);<br /></code></pre>



<a id="@Specification_1_shift_left_for_verification_only"></a>

### Function `shift_left_for_verification_only`


<pre><code>&#35;[verify_only]<br /><b>public</b> <b>fun</b> <a href="bit_vector.md#0x1_bit_vector_shift_left_for_verification_only">shift_left_for_verification_only</a>(bitvector: &amp;<b>mut</b> <a href="bit_vector.md#0x1_bit_vector_BitVector">bit_vector::BitVector</a>, amount: u64)<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> amount &gt;&#61; bitvector.length &#61;&#61;&gt; (<b>forall</b> k in 0..bitvector.length: !bitvector.bit_field[k]);<br /><b>ensures</b> amount &lt; bitvector.length &#61;&#61;&gt;<br />    (<b>forall</b> i in bitvector.length &#45; amount..bitvector.length: !bitvector.bit_field[i]);<br /><b>ensures</b> amount &lt; bitvector.length &#61;&#61;&gt;<br />    (<b>forall</b> i in 0..bitvector.length &#45; amount: bitvector.bit_field[i] &#61;&#61; <b>old</b>(bitvector).bit_field[i &#43; amount]);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
