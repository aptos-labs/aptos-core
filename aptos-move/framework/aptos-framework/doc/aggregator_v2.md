
<a id="0x1_aggregator_v2"></a>

# Module `0x1::aggregator_v2`

This module provides an interface for aggregators (version 2). Aggregators are
similar to unsigned integers and support addition and subtraction (aborting on
underflow or on overflowing a custom upper limit). The difference from integers
is that aggregators allow to perform both additions and subtractions in parallel
across multiple transactions, enabling parallel execution. For example, if the
first transaction is doing <code><a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>(X, 1)</code> for aggregator <code>X</code>, and the second is
doing <code><a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>(X,3)</code>, they can be executed in parallel avoiding a read-modify-write
dependency.
However, reading the aggregator value (i.e. calling <code><a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>(X)</code>) is a resource-intensive
operation that also reduced parallelism, and should be avoided as much as possible.


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
-  [Specification](#@Specification_1)
    -  [Function `create_aggregator`](#@Specification_1_create_aggregator)
    -  [Function `create_unbounded_aggregator`](#@Specification_1_create_unbounded_aggregator)
    -  [Function `try_add`](#@Specification_1_try_add)
    -  [Function `try_sub`](#@Specification_1_try_sub)
    -  [Function `read`](#@Specification_1_read)
    -  [Function `snapshot`](#@Specification_1_snapshot)
    -  [Function `create_snapshot`](#@Specification_1_create_snapshot)
    -  [Function `copy_snapshot`](#@Specification_1_copy_snapshot)
    -  [Function `string_concat`](#@Specification_1_string_concat)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_aggregator_v2_Aggregator"></a>

## Struct `Aggregator`

Represents an integer which supports parallel additions and subtractions
across multiple transactions. See the module description for more details.

Currently supported types for IntElement are u64 and u128.


<pre><code><b>struct</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt; <b>has</b> drop, store
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

<a id="0x1_aggregator_v2_AggregatorSnapshot"></a>

## Struct `AggregatorSnapshot`

Represents a constant value, that was derived from an aggregator at given instant in time.
Unlike read() and storing the value directly, this enables parallel execution of transactions,
while storing snapshot of aggregator state elsewhere.


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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aggregator_v2_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator overflows. Raised by uncoditional add() call


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>: u64 = 1;
</code></pre>



<a id="0x1_aggregator_v2_EAGGREGATOR_UNDERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>: u64 = 2;
</code></pre>



<a id="0x1_aggregator_v2_EAGGREGATOR_API_V2_NOT_ENABLED"></a>

The aggregator api v2 feature flag is not enabled.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_API_V2_NOT_ENABLED">EAGGREGATOR_API_V2_NOT_ENABLED</a>: u64 = 6;
</code></pre>



<a id="0x1_aggregator_v2_EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED"></a>

The native aggregator function, that is in the move file, is not yet supported.
and any calls will raise this error.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED">EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED</a>: u64 = 9;
</code></pre>



<a id="0x1_aggregator_v2_ECONCAT_STRING_LENGTH_TOO_LARGE"></a>

Arguments passed to concat exceed max limit of 256 bytes (for prefix and suffix together).


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_ECONCAT_STRING_LENGTH_TOO_LARGE">ECONCAT_STRING_LENGTH_TOO_LARGE</a>: u64 = 8;
</code></pre>



<a id="0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE"></a>

The generic type supplied to the aggregator snapshot is not supported.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE">EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE</a>: u64 = 5;
</code></pre>



<a id="0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_TYPE"></a>

The generic type supplied to the aggregator is not supported.


<pre><code><b>const</b> <a href="aggregator_v2.md#0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_TYPE">EUNSUPPORTED_AGGREGATOR_TYPE</a>: u64 = 7;
</code></pre>



<a id="0x1_aggregator_v2_max_value"></a>

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

<a id="0x1_aggregator_v2_create_aggregator"></a>

## Function `create_aggregator`

Creates new aggregator, with given 'max_value'.

Currently supported types for IntElement are u64 and u128.
EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>&lt;IntElement: <b>copy</b>, drop&gt;(max_value: IntElement): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>&lt;IntElement: <b>copy</b> + drop&gt;(max_value: IntElement): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;;
</code></pre>



</details>

<a id="0x1_aggregator_v2_create_unbounded_aggregator"></a>

## Function `create_unbounded_aggregator`

Creates new aggregator, without any 'max_value' on top of the implicit bound restriction
due to the width of the type (i.e. MAX_U64 for u64, MAX_U128 for u128).

Currently supported types for IntElement are u64 and u128.
EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">create_unbounded_aggregator</a>&lt;IntElement: <b>copy</b>, drop&gt;(): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">create_unbounded_aggregator</a>&lt;IntElement: <b>copy</b> + drop&gt;(): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;;
</code></pre>



</details>

<a id="0x1_aggregator_v2_try_add"></a>

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

<a id="0x1_aggregator_v2_add"></a>

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

<a id="0x1_aggregator_v2_try_sub"></a>

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

<a id="0x1_aggregator_v2_sub"></a>

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

<a id="0x1_aggregator_v2_read"></a>

## Function `read`

Returns a value stored in this aggregator.
Note: This operation is resource-intensive, and reduces parallelism.
(Especially if called in a transaction that also modifies the aggregator,
or has other read/write conflicts)


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): IntElement
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;): IntElement;
</code></pre>



</details>

<a id="0x1_aggregator_v2_snapshot"></a>

## Function `snapshot`

Returns a wrapper of a current value of an aggregator
Unlike read(), it is fast and avoids sequential dependencies.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_snapshot">snapshot</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;IntElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_snapshot">snapshot</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">Aggregator</a>&lt;IntElement&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;IntElement&gt;;
</code></pre>



</details>

<a id="0x1_aggregator_v2_create_snapshot"></a>

## Function `create_snapshot`

Creates a snapshot of a given value.
Useful for when object is sometimes created via snapshot() or string_concat(), and sometimes directly.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_snapshot">create_snapshot</a>&lt;Element: <b>copy</b>, drop&gt;(value: Element): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_snapshot">create_snapshot</a>&lt;Element: <b>copy</b> + drop&gt;(value: Element): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;;
</code></pre>



</details>

<a id="0x1_aggregator_v2_copy_snapshot"></a>

## Function `copy_snapshot`

NOT YET IMPLEMENTED, always raises EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_copy_snapshot">copy_snapshot</a>&lt;Element: <b>copy</b>, drop&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_copy_snapshot">copy_snapshot</a>&lt;Element: <b>copy</b> + drop&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;;
</code></pre>



</details>

<a id="0x1_aggregator_v2_read_snapshot"></a>

## Function `read_snapshot`

Returns a value stored in this snapshot.
Note: This operation is resource-intensive, and reduces parallelism.
(Especially if called in a transaction that also modifies the aggregator,
or has other read/write conflicts)


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read_snapshot">read_snapshot</a>&lt;Element&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read_snapshot">read_snapshot</a>&lt;Element&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;Element&gt;): Element;
</code></pre>



</details>

<a id="0x1_aggregator_v2_string_concat"></a>

## Function `string_concat`

Concatenates <code>before</code>, <code>snapshot</code> and <code>after</code> into a single string.
snapshot passed needs to have integer type - currently supported types are u64 and u128.
Raises EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE if called with another type.
If length of prefix and suffix together exceed 256 bytes, ECONCAT_STRING_LENGTH_TOO_LARGE is raised.


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_string_concat">string_concat</a>&lt;IntElement&gt;(before: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;IntElement&gt;, after: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_string_concat">string_concat</a>&lt;IntElement&gt;(before: String, snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;IntElement&gt;, after: String): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">AggregatorSnapshot</a>&lt;String&gt;;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_create_aggregator"></a>

### Function `create_aggregator`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator">create_aggregator</a>&lt;IntElement: <b>copy</b>, drop&gt;(max_value: IntElement): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_create_unbounded_aggregator"></a>

### Function `create_unbounded_aggregator`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_unbounded_aggregator">create_unbounded_aggregator</a>&lt;IntElement: <b>copy</b>, drop&gt;(): <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_try_add"></a>

### Function `try_add`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_add">try_add</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;, value: IntElement): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_try_sub"></a>

### Function `try_sub`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_try_sub">try_sub</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<b>mut</b> <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;, value: IntElement): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_read">read</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): IntElement
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_snapshot"></a>

### Function `snapshot`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_snapshot">snapshot</a>&lt;IntElement&gt;(<a href="aggregator.md#0x1_aggregator">aggregator</a>: &<a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;IntElement&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;IntElement&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_create_snapshot"></a>

### Function `create_snapshot`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_create_snapshot">create_snapshot</a>&lt;Element: <b>copy</b>, drop&gt;(value: Element): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_copy_snapshot"></a>

### Function `copy_snapshot`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_copy_snapshot">copy_snapshot</a>&lt;Element: <b>copy</b>, drop&gt;(snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;Element&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>



<a id="@Specification_1_string_concat"></a>

### Function `string_concat`


<pre><code><b>public</b> <b>fun</b> <a href="aggregator_v2.md#0x1_aggregator_v2_string_concat">string_concat</a>&lt;IntElement&gt;(before: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, snapshot: &<a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;IntElement&gt;, after: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="aggregator_v2.md#0x1_aggregator_v2_AggregatorSnapshot">aggregator_v2::AggregatorSnapshot</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
