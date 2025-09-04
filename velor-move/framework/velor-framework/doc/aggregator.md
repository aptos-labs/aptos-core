
<a id="0x1_aggregator"></a>

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
parallelism. Moreover, **aggregators can only be created by Velor Framework (0x1)
at the moment.**


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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aggregator_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator overflows. Raised by native code.


<pre><code><b>const</b> <a href="aggregator.md#0x1_aggregator_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>: u64 = 1;
</code></pre>



<a id="0x1_aggregator_EAGGREGATOR_UNDERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by native code.


<pre><code><b>const</b> <a href="aggregator.md#0x1_aggregator_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>: u64 = 2;
</code></pre>



<a id="0x1_aggregator_ENOT_SUPPORTED"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code><b>const</b> <a href="aggregator.md#0x1_aggregator_ENOT_SUPPORTED">ENOT_SUPPORTED</a>: u64 = 3;
</code></pre>



<a id="0x1_aggregator_limit"></a>

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

<a id="0x1_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to aggregator. Aborts on overflowing the limit.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_add">add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_add">add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>, value: u128);
</code></pre>



</details>

<a id="0x1_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from aggregator. Aborts on going below zero.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_sub">sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_sub">sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>, value: u128);
</code></pre>



</details>

<a id="0x1_aggregator_read"></a>

## Function `read`

Returns a value stored in this aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>



</details>

<a id="0x1_aggregator_destroy"></a>

## Function `destroy`

Destroys an aggregator and removes it from its <code>AggregatorFactory</code>.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_destroy">destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_destroy">destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>);
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Aggregator"></a>

### Struct `Aggregator`


<pre><code><b>struct</b> <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a> <b>has</b> store
</code></pre>



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




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>For a given aggregator, it should always be possible to: Return the limit value of the aggregator. Return the current value stored in the aggregator. Destroy an aggregator, removing it from its AggregatorFactory.</td>
<td>Low</td>
<td>The following functions should not abort if EventHandle exists: limit(), read(), destroy().</td>
<td>Formally verified via <a href="#high-level-req-1.1">read</a>, <a href="#high-level-req-1.2">destroy</a>, and <a href="#high-level-req-1.3">limit</a>.</td>
</tr>

<tr>
<td>2</td>
<td>If the value during addition exceeds the limit, an overflow occurs.</td>
<td>High</td>
<td>The native add() function checks the value of the addition to ensure it does not pass the defined limit and results in aggregator overflow.</td>
<td>Formally verified via <a href="#high-level-req-2">add</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Operations over aggregators should be correct.</td>
<td>High</td>
<td>The implementation of the add, sub, read and destroy functions is correct.</td>
<td>The native implementation of the add, sub, read and destroy functions have been manually audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_1_limit"></a>

### Function `limit`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_limit">limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>): u128
</code></pre>




<pre><code><b>pragma</b> intrinsic;
// This enforces <a id="high-level-req-1.2" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> [abstract] <b>false</b>;
</code></pre>




<a id="0x1_aggregator_spec_read"></a>


<pre><code><b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_spec_read">spec_read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>




<a id="0x1_aggregator_spec_get_limit"></a>


<pre><code><b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(a: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>




<a id="0x1_aggregator_spec_get_handle"></a>


<pre><code><b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_spec_get_handle">spec_get_handle</a>(a: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>




<a id="0x1_aggregator_spec_get_key"></a>


<pre><code><b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_spec_get_key">spec_get_key</a>(a: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>




<a id="0x1_aggregator_spec_aggregator_set_val"></a>


<pre><code><b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_set_val">spec_aggregator_set_val</a>(a: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>, v: u128): <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>;
</code></pre>




<a id="0x1_aggregator_spec_aggregator_get_val"></a>


<pre><code><b>native</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">spec_aggregator_get_val</a>(a: <a href="aggregator.md#0x1_aggregator_Aggregator">Aggregator</a>): u128;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_add">add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">spec_aggregator_get_val</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) + value &gt; <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">spec_aggregator_get_val</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) + value &gt; MAX_U128;
<b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) == <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(<b>old</b>(<a href="aggregator.md#0x1_aggregator">aggregator</a>));
<b>ensures</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> == <a href="aggregator.md#0x1_aggregator_spec_aggregator_set_val">spec_aggregator_set_val</a>(<b>old</b>(<a href="aggregator.md#0x1_aggregator">aggregator</a>),
    <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">spec_aggregator_get_val</a>(<b>old</b>(<a href="aggregator.md#0x1_aggregator">aggregator</a>)) + value);
</code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_sub">sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>, value: u128)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">spec_aggregator_get_val</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) &lt; value;
<b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) == <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(<b>old</b>(<a href="aggregator.md#0x1_aggregator">aggregator</a>));
<b>ensures</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> == <a href="aggregator.md#0x1_aggregator_spec_aggregator_set_val">spec_aggregator_set_val</a>(<b>old</b>(<a href="aggregator.md#0x1_aggregator">aggregator</a>),
    <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">spec_aggregator_get_val</a>(<b>old</b>(<a href="aggregator.md#0x1_aggregator">aggregator</a>)) - value);
</code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>): u128
</code></pre>




<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-1.1" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="aggregator.md#0x1_aggregator_spec_read">spec_read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);
<b>ensures</b> result &lt;= <a href="aggregator.md#0x1_aggregator_spec_get_limit">spec_get_limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);
</code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator.md#0x1_aggregator_destroy">destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>)
</code></pre>




<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-1.2" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
