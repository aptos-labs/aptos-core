
<a name="0x1_aggregator"></a>

# Module `0x1::aggregator`

This module provides an interface for aggregators. Aggregators are similar to
unsigned integers and support addition and subtraction (aborting on underflow
or on overflowing a custom upper limit). The difference from integers is that
aggregators allow to perform both additions and subtractions in parallel across
multiple transactions, enabling parallel execution. For example, if the first
transaction is doing <code><a href="aggregator.md#0x1_aggregator_add">add</a>(X, 1)</code> for aggregator resource <code>X</code>, and the second
is doing <code><a href="aggregator.md#0x1_aggregator_sub">sub</a>(X,3)</code>, they can be executed in parallel avoiding a read-modify-write
dependency.
However, reading the aggregator value (i.e. calling <code><a href="aggregator.md#0x1_aggregator_read">read</a>(X)</code>) is an expensive
operation and should be avoided as much as possible because it reduces the
parallelism. Moreover, **aggregators can only be created by Aptos Framework (0x1)
at the moment.**


-  [Struct `Aggregator`](#0x1_aggregator_Aggregator)
-  [Constants](#@Constants_0)
-  [Function `limit`](#0x1_aggregator_limit)
-  [Function `add`](#0x1_aggregator_add)
-  [Function `sub`](#0x1_aggregator_sub)
-  [Function `read`](#0x1_aggregator_read)
-  [Function `destroy`](#0x1_aggregator_destroy)
-  [Specification](#@Specification_1)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `sub`](#@Specification_1_sub)
    -  [Function `read`](#@Specification_1_read)
    -  [Function `destroy`](#@Specification_1_destroy)


<pre><code></code></pre>



<a name="0x1_aggregator_Aggregator"></a>

## Struct `Aggregator`

Represents an integer which supports parallel additions and subtractions
across multiple transactions. See the module description for more details.


<pre><code><b>struct</b> <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>handle: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>key: <b>address</b></code>
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_aggregator_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator overflows. Raised by native code.


<pre><code><b>const</b> <a href="aggregator.md#0x1_aggregator_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>: u64 = 1;
</code></pre>



<a name="0x1_aggregator_EAGGREGATOR_UNDERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by native code.


<pre><code><b>const</b> <a href="aggregator.md#0x1_aggregator_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>: u64 = 2;
</code></pre>



<a name="0x1_aggregator_ENOT_SUPPORTED"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code><b>const</b> <a href="aggregator.md#0x1_aggregator_ENOT_SUPPORTED">ENOT_SUPPORTED</a>: u64 = 3;
</code></pre>



<a name="0x1_aggregator_limit"></a>

## Function `limit`

Returns <code>limit</code> exceeding which aggregator overflows.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_limit">limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_limit">limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128 {
    <a href="aggregator.md#0x1_aggregator">aggregator</a>.limit
}
</code></pre>



</details>

<a name="0x1_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to aggregator. Aborts on overflowing the limit.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_add">add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_add">add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>, value: u128);
</code></pre>



</details>

<a name="0x1_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from aggregator. Aborts on going below zero.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_sub">sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_sub">sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>, value: u128);
</code></pre>



</details>

<a name="0x1_aggregator_read"></a>

## Function `read`

Returns a value stored in this aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>



</details>

<a name="0x1_aggregator_destroy"></a>

## Function `destroy`

Destroys an aggregator and removes it from its <code>AggregatorFactory</code>.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_destroy">destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_destroy">destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>);
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a name="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_add">add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_1_sub"></a>

### Function `sub`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_sub">sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_1_read"></a>

### Function `read`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a name="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_destroy">destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>)
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
