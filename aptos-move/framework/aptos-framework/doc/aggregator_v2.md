
<a id="0x1_aggregator_v2"></a>

# Module `0x1::aggregator_v2`

This module provides an interface for aggregators (version 2). Aggregators are
similar to unsigned integers and support addition and subtraction (aborting on
underflow or on overflowing a custom upper limit). The difference from integers
is that aggregators allow to perform both additions and subtractions in parallel
across multiple transactions, enabling parallel execution. For example, if the
first transaction is doing <code>try_add(X, 1)</code> for aggregator <code>X</code>, and the second is
doing <code>try_sub(X,3)</code>, they can be executed in parallel avoiding a read-modify-write
dependency.
However, reading the aggregator value (i.e. calling <code>read(X)</code>) is a resource-intensive
operation that also reduced parallelism, and should be avoided as much as possible.


-  [Struct `Aggregator`](#0x1_aggregator_v2_Aggregator)
-  [Struct `AggregatorSnapshot`](#0x1_aggregator_v2_AggregatorSnapshot)
-  [Struct `DerivedStringSnapshot`](#0x1_aggregator_v2_DerivedStringSnapshot)
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
-  [Function `read_snapshot`](#0x1_aggregator_v2_read_snapshot)
-  [Function `read_derived_string`](#0x1_aggregator_v2_read_derived_string)
-  [Function `create_derived_string`](#0x1_aggregator_v2_create_derived_string)
-  [Function `derive_string_concat`](#0x1_aggregator_v2_derive_string_concat)
-  [Function `copy_snapshot`](#0x1_aggregator_v2_copy_snapshot)
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


<pre><code>use 0x1::error;<br/>use 0x1::string;<br/></code></pre>



<a id="0x1_aggregator_v2_Aggregator"></a>

## Struct `Aggregator`

Represents an integer which supports parallel additions and subtractions
across multiple transactions. See the module description for more details.

Currently supported types for IntElement are u64 and u128.


<pre><code>struct Aggregator&lt;IntElement&gt; has drop, store<br/></code></pre>



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


<pre><code>struct AggregatorSnapshot&lt;IntElement&gt; has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: IntElement</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aggregator_v2_DerivedStringSnapshot"></a>

## Struct `DerivedStringSnapshot`



<pre><code>struct DerivedStringSnapshot has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>padding: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aggregator_v2_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator overflows. Raised by uncoditional add() call


<pre><code>const EAGGREGATOR_OVERFLOW: u64 &#61; 1;<br/></code></pre>



<a id="0x1_aggregator_v2_EAGGREGATOR_UNDERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call


<pre><code>const EAGGREGATOR_UNDERFLOW: u64 &#61; 2;<br/></code></pre>



<a id="0x1_aggregator_v2_EAGGREGATOR_API_V2_NOT_ENABLED"></a>

The aggregator api v2 feature flag is not enabled.


<pre><code>const EAGGREGATOR_API_V2_NOT_ENABLED: u64 &#61; 6;<br/></code></pre>



<a id="0x1_aggregator_v2_EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED"></a>

The native aggregator function, that is in the move file, is not yet supported.
and any calls will raise this error.


<pre><code>const EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED: u64 &#61; 9;<br/></code></pre>



<a id="0x1_aggregator_v2_ECONCAT_STRING_LENGTH_TOO_LARGE"></a>

Arguments passed to concat exceed max limit of 256 bytes (for prefix and suffix together).


<pre><code>const ECONCAT_STRING_LENGTH_TOO_LARGE: u64 &#61; 8;<br/></code></pre>



<a id="0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE"></a>

The generic type supplied to the aggregator snapshot is not supported.


<pre><code>const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 &#61; 5;<br/></code></pre>



<a id="0x1_aggregator_v2_EUNSUPPORTED_AGGREGATOR_TYPE"></a>

The generic type supplied to the aggregator is not supported.


<pre><code>const EUNSUPPORTED_AGGREGATOR_TYPE: u64 &#61; 7;<br/></code></pre>



<a id="0x1_aggregator_v2_max_value"></a>

## Function `max_value`

Returns <code>max_value</code> exceeding which aggregator overflows.


<pre><code>public fun max_value&lt;IntElement: copy, drop&gt;(aggregator: &amp;aggregator_v2::Aggregator&lt;IntElement&gt;): IntElement<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max_value&lt;IntElement: copy &#43; drop&gt;(aggregator: &amp;Aggregator&lt;IntElement&gt;): IntElement &#123;<br/>    aggregator.max_value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_create_aggregator"></a>

## Function `create_aggregator`

Creates new aggregator, with given 'max_value'.

Currently supported types for IntElement are u64 and u128.
EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.


<pre><code>public fun create_aggregator&lt;IntElement: copy, drop&gt;(max_value: IntElement): aggregator_v2::Aggregator&lt;IntElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun create_aggregator&lt;IntElement: copy &#43; drop&gt;(max_value: IntElement): Aggregator&lt;IntElement&gt;;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_create_unbounded_aggregator"></a>

## Function `create_unbounded_aggregator`

Creates new aggregator, without any 'max_value' on top of the implicit bound restriction
due to the width of the type (i.e. MAX_U64 for u64, MAX_U128 for u128).

Currently supported types for IntElement are u64 and u128.
EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.


<pre><code>public fun create_unbounded_aggregator&lt;IntElement: copy, drop&gt;(): aggregator_v2::Aggregator&lt;IntElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun create_unbounded_aggregator&lt;IntElement: copy &#43; drop&gt;(): Aggregator&lt;IntElement&gt;;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_try_add"></a>

## Function `try_add`

Adds <code>value</code> to aggregator.
If addition would exceed the max_value, <code>false</code> is returned, and aggregator value is left unchanged.


<pre><code>public fun try_add&lt;IntElement&gt;(aggregator: &amp;mut aggregator_v2::Aggregator&lt;IntElement&gt;, value: IntElement): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun try_add&lt;IntElement&gt;(aggregator: &amp;mut Aggregator&lt;IntElement&gt;, value: IntElement): bool;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_add"></a>

## Function `add`



<pre><code>public fun add&lt;IntElement&gt;(aggregator: &amp;mut aggregator_v2::Aggregator&lt;IntElement&gt;, value: IntElement)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;IntElement&gt;(aggregator: &amp;mut Aggregator&lt;IntElement&gt;, value: IntElement) &#123;<br/>    assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_try_sub"></a>

## Function `try_sub`

Subtracts <code>value</code> from aggregator.
If subtraction would result in a negative value, <code>false</code> is returned, and aggregator value is left unchanged.


<pre><code>public fun try_sub&lt;IntElement&gt;(aggregator: &amp;mut aggregator_v2::Aggregator&lt;IntElement&gt;, value: IntElement): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun try_sub&lt;IntElement&gt;(aggregator: &amp;mut Aggregator&lt;IntElement&gt;, value: IntElement): bool;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_sub"></a>

## Function `sub`



<pre><code>public fun sub&lt;IntElement&gt;(aggregator: &amp;mut aggregator_v2::Aggregator&lt;IntElement&gt;, value: IntElement)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sub&lt;IntElement&gt;(aggregator: &amp;mut Aggregator&lt;IntElement&gt;, value: IntElement) &#123;<br/>    assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_read"></a>

## Function `read`

Returns a value stored in this aggregator.
Note: This operation is resource-intensive, and reduces parallelism.
(Especially if called in a transaction that also modifies the aggregator,
or has other read/write conflicts)


<pre><code>public fun read&lt;IntElement&gt;(aggregator: &amp;aggregator_v2::Aggregator&lt;IntElement&gt;): IntElement<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun read&lt;IntElement&gt;(aggregator: &amp;Aggregator&lt;IntElement&gt;): IntElement;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_snapshot"></a>

## Function `snapshot`

Returns a wrapper of a current value of an aggregator
Unlike read(), it is fast and avoids sequential dependencies.


<pre><code>public fun snapshot&lt;IntElement&gt;(aggregator: &amp;aggregator_v2::Aggregator&lt;IntElement&gt;): aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun snapshot&lt;IntElement&gt;(aggregator: &amp;Aggregator&lt;IntElement&gt;): AggregatorSnapshot&lt;IntElement&gt;;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_create_snapshot"></a>

## Function `create_snapshot`

Creates a snapshot of a given value.
Useful for when object is sometimes created via snapshot() or string_concat(), and sometimes directly.


<pre><code>public fun create_snapshot&lt;IntElement: copy, drop&gt;(value: IntElement): aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun create_snapshot&lt;IntElement: copy &#43; drop&gt;(value: IntElement): AggregatorSnapshot&lt;IntElement&gt;;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_read_snapshot"></a>

## Function `read_snapshot`

Returns a value stored in this snapshot.
Note: This operation is resource-intensive, and reduces parallelism.
(Especially if called in a transaction that also modifies the aggregator,
or has other read/write conflicts)


<pre><code>public fun read_snapshot&lt;IntElement&gt;(snapshot: &amp;aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;): IntElement<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun read_snapshot&lt;IntElement&gt;(snapshot: &amp;AggregatorSnapshot&lt;IntElement&gt;): IntElement;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_read_derived_string"></a>

## Function `read_derived_string`

Returns a value stored in this DerivedStringSnapshot.
Note: This operation is resource-intensive, and reduces parallelism.
(Especially if called in a transaction that also modifies the aggregator,
or has other read/write conflicts)


<pre><code>public fun read_derived_string(snapshot: &amp;aggregator_v2::DerivedStringSnapshot): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun read_derived_string(snapshot: &amp;DerivedStringSnapshot): String;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_create_derived_string"></a>

## Function `create_derived_string`

Creates a DerivedStringSnapshot of a given value.
Useful for when object is sometimes created via string_concat(), and sometimes directly.


<pre><code>public fun create_derived_string(value: string::String): aggregator_v2::DerivedStringSnapshot<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun create_derived_string(value: String): DerivedStringSnapshot;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_derive_string_concat"></a>

## Function `derive_string_concat`

Concatenates <code>before</code>, <code>snapshot</code> and <code>after</code> into a single string.
snapshot passed needs to have integer type - currently supported types are u64 and u128.
Raises EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE if called with another type.
If length of prefix and suffix together exceed 256 bytes, ECONCAT_STRING_LENGTH_TOO_LARGE is raised.


<pre><code>public fun derive_string_concat&lt;IntElement&gt;(before: string::String, snapshot: &amp;aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;, after: string::String): aggregator_v2::DerivedStringSnapshot<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun derive_string_concat&lt;IntElement&gt;(before: String, snapshot: &amp;AggregatorSnapshot&lt;IntElement&gt;, after: String): DerivedStringSnapshot;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_copy_snapshot"></a>

## Function `copy_snapshot`

NOT YET IMPLEMENTED, always raises EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED.


<pre><code>&#35;[deprecated]<br/>public fun copy_snapshot&lt;IntElement: copy, drop&gt;(snapshot: &amp;aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;): aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun copy_snapshot&lt;IntElement: copy &#43; drop&gt;(snapshot: &amp;AggregatorSnapshot&lt;IntElement&gt;): AggregatorSnapshot&lt;IntElement&gt;;<br/></code></pre>



</details>

<a id="0x1_aggregator_v2_string_concat"></a>

## Function `string_concat`

DEPRECATED, use derive_string_concat() instead. always raises EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED.


<pre><code>&#35;[deprecated]<br/>public fun string_concat&lt;IntElement&gt;(before: string::String, snapshot: &amp;aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;, after: string::String): aggregator_v2::AggregatorSnapshot&lt;string::String&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun string_concat&lt;IntElement&gt;(before: String, snapshot: &amp;AggregatorSnapshot&lt;IntElement&gt;, after: String): AggregatorSnapshot&lt;String&gt;;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_create_aggregator"></a>

### Function `create_aggregator`


<pre><code>public fun create_aggregator&lt;IntElement: copy, drop&gt;(max_value: IntElement): aggregator_v2::Aggregator&lt;IntElement&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_create_unbounded_aggregator"></a>

### Function `create_unbounded_aggregator`


<pre><code>public fun create_unbounded_aggregator&lt;IntElement: copy, drop&gt;(): aggregator_v2::Aggregator&lt;IntElement&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_try_add"></a>

### Function `try_add`


<pre><code>public fun try_add&lt;IntElement&gt;(aggregator: &amp;mut aggregator_v2::Aggregator&lt;IntElement&gt;, value: IntElement): bool<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_try_sub"></a>

### Function `try_sub`


<pre><code>public fun try_sub&lt;IntElement&gt;(aggregator: &amp;mut aggregator_v2::Aggregator&lt;IntElement&gt;, value: IntElement): bool<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code>public fun read&lt;IntElement&gt;(aggregator: &amp;aggregator_v2::Aggregator&lt;IntElement&gt;): IntElement<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_snapshot"></a>

### Function `snapshot`


<pre><code>public fun snapshot&lt;IntElement&gt;(aggregator: &amp;aggregator_v2::Aggregator&lt;IntElement&gt;): aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_create_snapshot"></a>

### Function `create_snapshot`


<pre><code>public fun create_snapshot&lt;IntElement: copy, drop&gt;(value: IntElement): aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_copy_snapshot"></a>

### Function `copy_snapshot`


<pre><code>&#35;[deprecated]<br/>public fun copy_snapshot&lt;IntElement: copy, drop&gt;(snapshot: &amp;aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;): aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_string_concat"></a>

### Function `string_concat`


<pre><code>&#35;[deprecated]<br/>public fun string_concat&lt;IntElement&gt;(before: string::String, snapshot: &amp;aggregator_v2::AggregatorSnapshot&lt;IntElement&gt;, after: string::String): aggregator_v2::AggregatorSnapshot&lt;string::String&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
