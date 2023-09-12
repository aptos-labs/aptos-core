
<a name="0x1_aggregator_v2"></a>

# Module `0x1::aggregator_v2`

This module provides an interface for aggregators (version 2).


-  [Struct `Aggregator`](#0x1_aggregator_v2_Aggregator)
-  [Struct `AggregatorSnapshot`](#0x1_aggregator_v2_AggregatorSnapshot)
-  [Constants](#@Constants_0)
-  [Function `max_value`](#0x1_aggregator_v2_max_value)
-  [Function `create_aggregator`](#0x1_aggregator_v2_create_aggregator)
-  [Function `create_unbounded_aggregator`](#0x1_aggregator_v2_create_unbounded_aggregator)
-  [Function `try_add`](#0x1_aggregator_v2_try_add)
-  [Function `add`](#0x1_aggregator_v2_add)
-  [Function `try_sub`](#0x1_aggregator_v2_try_sub)
-  [Function `sub`](#0x1_aggregator_v2_sub)
-  [Function `read`](#0x1_aggregator_v2_read)
-  [Function `snapshot`](#0x1_aggregator_v2_snapshot)
-  [Function `create_snapshot`](#0x1_aggregator_v2_create_snapshot)
-  [Function `copy_snapshot`](#0x1_aggregator_v2_copy_snapshot)
-  [Function `read_snapshot`](#0x1_aggregator_v2_read_snapshot)
-  [Function `string_concat`](#0x1_aggregator_v2_string_concat)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_aggregator_v2_Aggregator"></a>

## Struct `Aggregator`

Represents an integer which supports parallel additions and subtractions
across multiple transactions. See the module description for more details.

Currently supported types for Element are u64 and u128.


<pre><code><b>struct</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: IntElement</code>
</dt>
<dd>

</dd>
<dt>
<code>max_value: IntElement</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_aggregator_v2_AggregatorSnapshot"></a>

## Struct `AggregatorSnapshot`



<pre><code><b>struct</b> <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt; <b>has</b> drop, store
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_aggregator_v2_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator overflows. Raised by uncoditional add() call


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>: u64 = 1;
</code></pre>



<a name="0x1_aggregator_v2_EAGGREGATOR_UNDERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>: u64 = 2;
</code></pre>



<a name="0x1_aggregator_v2_EAGGREGATOR_SNAPSHOTS_NOT_ENABLED"></a>

The aggregator snapshots feature flag is not enabled.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_SNAPSHOTS_NOT_ENABLED">EAGGREGATOR_SNAPSHOTS_NOT_ENABLED</a>: u64 = 6;
</code></pre>



<a name="0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE"></a>

The generic type supplied to the aggregator snapshot is not supported.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE">EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE</a>: u64 = 5;
</code></pre>



<a name="0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_TYPE"></a>

The generic type supplied to the aggregator is not supported.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_TYPE">EUNSUPPORTED_AGGREGATOR_TYPE</a>: u64 = 7;
</code></pre>



<a name="0x1_aggregator_v2_max_value"></a>

## Function `max_value`

Returns <code>max_value</code> exceeding which aggregator overflows.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_max_value">max_value</a>&lt;IntElement: <b>copy</b>, drop&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): IntElement
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_max_value">max_value</a>&lt;IntElement: <b>copy</b> + drop&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;): IntElement {
    <a href="aggregator.md#0x1_aggregator">aggregator</a>.max_value
}
</code></pre>



</details>

<a name="0x1_aggregator_v2_create_aggregator"></a>

## Function `create_aggregator`

Creates new aggregator, with given 'max_value'.

Currently supported types for Element are u64 and u128.
EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>&lt;IntElement: <b>copy</b>, drop&gt;(max_value: IntElement): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>&lt;IntElement: <b>copy</b> + drop&gt;(max_value: IntElement): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;;
</code></pre>



</details>

<a name="0x1_aggregator_v2_create_unbounded_aggregator"></a>

## Function `create_unbounded_aggregator`

Creates new aggregator, without any 'max_value' on top of the implicit bound restriction
due to the width of the type (i.e. MAX_U64 for u64, MAX_U128 for u128).

Currently supported types for Element are u64 and u128.
EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">create_unbounded_aggregator</a>&lt;Element: <b>copy</b>, drop&gt;(): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">create_unbounded_aggregator</a>&lt;Element: <b>copy</b> + drop&gt;(): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;Element&gt;;
</code></pre>



</details>

<a name="0x1_aggregator_v2_try_add"></a>

## Function `try_add`

Adds <code>value</code> to aggregator.
If addition would exceed the max_value, <code><b>false</b></code> is returned, and aggregator value is left unchanged.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;, value: IntElement): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;, value: IntElement): bool;
</code></pre>



</details>

<a name="0x1_aggregator_v2_add"></a>

## Function `add`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_add">add</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;, value: IntElement)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_add">add</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;, value: IntElement) {
    <b>assert</b>!(<a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>, value), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>));
}
</code></pre>



</details>

<a name="0x1_aggregator_v2_try_sub"></a>

## Function `try_sub`

Subtracts <code>value</code> from aggregator.
If subtraction would result in a negative value, <code><b>false</b></code> is returned, and aggregator value is left unchanged.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;, value: IntElement): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;, value: IntElement): bool;
</code></pre>



</details>

<a name="0x1_aggregator_v2_sub"></a>

## Function `sub`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_sub">sub</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;, value: IntElement)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_sub">sub</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;, value: IntElement) {
    <b>assert</b>!(<a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>, value), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>));
}
</code></pre>



</details>

<a name="0x1_aggregator_v2_read"></a>

## Function `read`

Returns a value stored in this aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): IntElement
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;): IntElement;
</code></pre>



</details>

<a name="0x1_aggregator_v2_snapshot"></a>

## Function `snapshot`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_snapshot">snapshot</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;IntElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_snapshot">snapshot</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;IntElement&gt;;
</code></pre>



</details>

<a name="0x1_aggregator_v2_create_snapshot"></a>

## Function `create_snapshot`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_snapshot">create_snapshot</a>&lt;Element: <b>copy</b>, drop&gt;(value: Element): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_snapshot">create_snapshot</a>&lt;Element: <b>copy</b> + drop&gt;(value: Element): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;;
</code></pre>



</details>

<a name="0x1_aggregator_v2_copy_snapshot"></a>

## Function `copy_snapshot`



<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_copy_snapshot">copy_snapshot</a>&lt;Element: <b>copy</b>, drop&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_copy_snapshot">copy_snapshot</a>&lt;Element: <b>copy</b> + drop&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;;
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

<a name="0x1_aggregator_v2_string_concat"></a>

## Function `string_concat`

Concatenates <code>before</code>, <code>snapshot</code> and <code>after</code> into a single string.
snapshot passed needs to have integer type - currently supported types are u64 and u128.
raises EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE if called with another type.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_string_concat">string_concat</a>&lt;IntElement&gt;(before: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;IntElement&gt;, after: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_string_concat">string_concat</a>&lt;IntElement&gt;(before: String, snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;IntElement&gt;, after: String): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;String&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
