
<a name="0x1_aggregator_v2"></a>

# Module `0x1::aggregator_v2`

This module provides an interface for aggregators (version 2).


-  [Struct `Aggregator`](#0x1_aggregator_v2_Aggregator)
-  [Struct `AggregatorSnapshot`](#0x1_aggregator_v2_AggregatorSnapshot)
-  [Function `limit`](#0x1_aggregator_v2_limit)
-  [Function `create_aggregator`](#0x1_aggregator_v2_create_aggregator)
-  [Function `try_add`](#0x1_aggregator_v2_try_add)
-  [Function `try_sub`](#0x1_aggregator_v2_try_sub)
-  [Function `read`](#0x1_aggregator_v2_read)
-  [Function `deferred_read`](#0x1_aggregator_v2_deferred_read)
-  [Function `deferred_read_convert_u64`](#0x1_aggregator_v2_deferred_read_convert_u64)
-  [Function `read_snapshot`](#0x1_aggregator_v2_read_snapshot)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a name="0x1_aggregator_v2_Aggregator"></a>

## Struct `Aggregator`

Represents an integer which supports parallel additions and subtractions
across multiple transactions. See the module description for more details.


<pre><code><b>struct</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u128</code>
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

<a name="0x1_aggregator_v2_AggregatorSnapshot"></a>

## Struct `AggregatorSnapshot`



<pre><code><b>struct</b> <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: Element</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_aggregator_v2_limit"></a>

## Function `limit`

Returns <code>limit</code> exceeding which aggregator overflows.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_limit">limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_limit">limit</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>): u128 {
    <a href="aggregator.md#0x1_aggregator">aggregator</a>.limit
}
</code></pre>



</details>

<a name="0x1_aggregator_v2_create_aggregator"></a>

## Function `create_aggregator`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>(limit: u128): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>(limit: u128): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>;
</code></pre>



</details>

<a name="0x1_aggregator_v2_try_add"></a>

## Function `try_add`

Adds <code>value</code> to aggregator.
Returns <code><b>true</b></code> if the addition succeeded and <code><b>false</b></code> if it exceeded the limit.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>, value: u128): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>, value: u128): bool;
</code></pre>



</details>

<a name="0x1_aggregator_v2_try_sub"></a>

## Function `try_sub`

Subtracts <code>value</code> from aggregator.
Returns <code><b>true</b></code> if the subtraction succeeded and <code><b>false</b></code> if it tried going below 0.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>, value: u128): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>, value: u128): bool;
</code></pre>



</details>

<a name="0x1_aggregator_v2_read"></a>

## Function `read`

Returns a value stored in this aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>): u128;
</code></pre>



</details>

<a name="0x1_aggregator_v2_deferred_read"></a>

## Function `deferred_read`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_deferred_read">deferred_read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_deferred_read">deferred_read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;u128&gt;;
</code></pre>



</details>

<a name="0x1_aggregator_v2_deferred_read_convert_u64"></a>

## Function `deferred_read_convert_u64`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_deferred_read_convert_u64">deferred_read_convert_u64</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;u64&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_deferred_read_convert_u64">deferred_read_convert_u64</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>): Option&lt;<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;u64&gt;&gt;;
</code></pre>



</details>

<a name="0x1_aggregator_v2_read_snapshot"></a>

## Function `read_snapshot`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read_snapshot">read_snapshot</a>&lt;Element&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read_snapshot">read_snapshot</a>&lt;Element&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;): Element;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
