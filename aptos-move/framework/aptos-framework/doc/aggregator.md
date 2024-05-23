
<a id="0x1_aggregator"></a>

# Module `0x1::aggregator`

This module provides an interface for aggregators. Aggregators are similar to<br/> unsigned integers and support addition and subtraction (aborting on underflow<br/> or on overflowing a custom upper limit). The difference from integers is that<br/> aggregators allow to perform both additions and subtractions in parallel across<br/> multiple transactions, enabling parallel execution. For example, if the first<br/> transaction is doing <code>add(X, 1)</code> for aggregator resource <code>X</code>, and the second<br/> is doing <code>sub(X,3)</code>, they can be executed in parallel avoiding a read&#45;modify&#45;write<br/> dependency.<br/> However, reading the aggregator value (i.e. calling <code>read(X)</code>) is an expensive<br/> operation and should be avoided as much as possible because it reduces the<br/> parallelism. Moreover, &#42;&#42;aggregators can only be created by Aptos Framework (0x1)<br/> at the moment.&#42;&#42;


-  [Struct `Aggregator`](#0x1_aggregator_Aggregator)
-  [Constants](#@Constants_0)
-  [Function `limit`](#0x1_aggregator_limit)
-  [Function `add`](#0x1_aggregator_add)
-  [Function `sub`](#0x1_aggregator_sub)
-  [Function `read`](#0x1_aggregator_read)
-  [Function `destroy`](#0x1_aggregator_destroy)
-  [Specification](#@Specification_1)
    -  [Struct `Aggregator`](#@Specification_1_Aggregator)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `limit`](#@Specification_1_limit)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `sub`](#@Specification_1_sub)
    -  [Function `read`](#@Specification_1_read)
    -  [Function `destroy`](#@Specification_1_destroy)


<pre><code></code></pre>



<a id="0x1_aggregator_Aggregator"></a>

## Struct `Aggregator`

Represents an integer which supports parallel additions and subtractions<br/> across multiple transactions. See the module description for more details.


<pre><code>struct Aggregator has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: address</code>
</dt>
<dd>

</dd>
<dt>
<code>key: address</code>
</dt>
<dd>

</dd>
<dt>
<code>limit: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aggregator_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator overflows. Raised by native code.


<pre><code>const EAGGREGATOR_OVERFLOW: u64 &#61; 1;<br/></code></pre>



<a id="0x1_aggregator_EAGGREGATOR_UNDERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by native code.


<pre><code>const EAGGREGATOR_UNDERFLOW: u64 &#61; 2;<br/></code></pre>



<a id="0x1_aggregator_ENOT_SUPPORTED"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code>const ENOT_SUPPORTED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_aggregator_limit"></a>

## Function `limit`

Returns <code>limit</code> exceeding which aggregator overflows.


<pre><code>public fun limit(aggregator: &amp;aggregator::Aggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun limit(aggregator: &amp;Aggregator): u128 &#123;<br/>    aggregator.limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to aggregator. Aborts on overflowing the limit.


<pre><code>public fun add(aggregator: &amp;mut aggregator::Aggregator, value: u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun add(aggregator: &amp;mut Aggregator, value: u128);<br/></code></pre>



</details>

<a id="0x1_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from aggregator. Aborts on going below zero.


<pre><code>public fun sub(aggregator: &amp;mut aggregator::Aggregator, value: u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun sub(aggregator: &amp;mut Aggregator, value: u128);<br/></code></pre>



</details>

<a id="0x1_aggregator_read"></a>

## Function `read`

Returns a value stored in this aggregator.


<pre><code>public fun read(aggregator: &amp;aggregator::Aggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun read(aggregator: &amp;Aggregator): u128;<br/></code></pre>



</details>

<a id="0x1_aggregator_destroy"></a>

## Function `destroy`

Destroys an aggregator and removes it from its <code>AggregatorFactory</code>.


<pre><code>public fun destroy(aggregator: aggregator::Aggregator)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun destroy(aggregator: Aggregator);<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Aggregator"></a>

### Struct `Aggregator`


<pre><code>struct Aggregator has store<br/></code></pre>



<dl>
<dt>
<code>handle: address</code>
</dt>
<dd>

</dd>
<dt>
<code>key: address</code>
</dt>
<dd>

</dd>
<dt>
<code>limit: u128</code>
</dt>
<dd>

</dd>
</dl>




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;For a given aggregator, it should always be possible to: Return the limit value of the aggregator. Return the current value stored in the aggregator. Destroy an aggregator, removing it from its AggregatorFactory.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The following functions should not abort if EventHandle exists: limit(), read(), destroy().&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.1&quot;&gt;read&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.2&quot;&gt;destroy&lt;/a&gt;, and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.3&quot;&gt;limit&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;If the value during addition exceeds the limit, an overflow occurs.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The native add() function checks the value of the addition to ensure it does not pass the defined limit and results in aggregator overflow.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;add&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Operations over aggregators should be correct.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The implementation of the add, sub, read and destroy functions is correct.&lt;/td&gt;<br/>&lt;td&gt;The native implementation of the add, sub, read and destroy functions have been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma intrinsic;<br/></code></pre>



<a id="@Specification_1_limit"></a>

### Function `limit`


<pre><code>public fun limit(aggregator: &amp;aggregator::Aggregator): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if false;<br/>ensures [abstract] result &#61;&#61; spec_get_limit(aggregator);<br/></code></pre>




<a id="0x1_aggregator_spec_read"></a>


<pre><code>native fun spec_read(aggregator: Aggregator): u128;<br/></code></pre>




<a id="0x1_aggregator_spec_get_limit"></a>


<pre><code>native fun spec_get_limit(a: Aggregator): u128;<br/></code></pre>




<a id="0x1_aggregator_spec_get_handle"></a>


<pre><code>native fun spec_get_handle(a: Aggregator): u128;<br/></code></pre>




<a id="0x1_aggregator_spec_get_key"></a>


<pre><code>native fun spec_get_key(a: Aggregator): u128;<br/></code></pre>




<a id="0x1_aggregator_spec_aggregator_set_val"></a>


<pre><code>native fun spec_aggregator_set_val(a: Aggregator, v: u128): Aggregator;<br/></code></pre>




<a id="0x1_aggregator_spec_aggregator_get_val"></a>


<pre><code>native fun spec_aggregator_get_val(a: Aggregator): u128;<br/></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add(aggregator: &amp;mut aggregator::Aggregator, value: u128)<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if spec_aggregator_get_val(aggregator) &#43; value &gt; spec_get_limit(aggregator);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if spec_aggregator_get_val(aggregator) &#43; value &gt; MAX_U128;<br/>ensures spec_get_limit(aggregator) &#61;&#61; spec_get_limit(old(aggregator));<br/>ensures aggregator &#61;&#61; spec_aggregator_set_val(old(aggregator),<br/>    spec_aggregator_get_val(old(aggregator)) &#43; value);<br/></code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code>public fun sub(aggregator: &amp;mut aggregator::Aggregator, value: u128)<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if spec_aggregator_get_val(aggregator) &lt; value;<br/>ensures spec_get_limit(aggregator) &#61;&#61; spec_get_limit(old(aggregator));<br/>ensures aggregator &#61;&#61; spec_aggregator_set_val(old(aggregator),<br/>    spec_aggregator_get_val(old(aggregator)) &#45; value);<br/></code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code>public fun read(aggregator: &amp;aggregator::Aggregator): u128<br/></code></pre>




<pre><code>pragma opaque;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if false;<br/>ensures result &#61;&#61; spec_read(aggregator);<br/>ensures result &lt;&#61; spec_get_limit(aggregator);<br/></code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code>public fun destroy(aggregator: aggregator::Aggregator)<br/></code></pre>




<pre><code>pragma opaque;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
