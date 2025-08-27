
<a id="0x1_optional_aggregator"></a>

# Module `0x1::optional_aggregator`

This module provides an interface to aggregate integers either via
aggregator (parallelizable) or via normal integers.


-  [Struct `Integer`](#0x1_optional_aggregator_Integer)
-  [Struct `OptionalAggregator`](#0x1_optional_aggregator_OptionalAggregator)
-  [Constants](#@Constants_0)
-  [Function `new_integer`](#0x1_optional_aggregator_new_integer)
-  [Function `add_integer`](#0x1_optional_aggregator_add_integer)
-  [Function `sub_integer`](#0x1_optional_aggregator_sub_integer)
-  [Function `limit`](#0x1_optional_aggregator_limit)
-  [Function `read_integer`](#0x1_optional_aggregator_read_integer)
-  [Function `destroy_integer`](#0x1_optional_aggregator_destroy_integer)
-  [Function `new`](#0x1_optional_aggregator_new)
-  [Function `switch`](#0x1_optional_aggregator_switch)
-  [Function `destroy`](#0x1_optional_aggregator_destroy)
-  [Function `destroy_optional_aggregator`](#0x1_optional_aggregator_destroy_optional_aggregator)
-  [Function `destroy_optional_integer`](#0x1_optional_aggregator_destroy_optional_integer)
-  [Function `add`](#0x1_optional_aggregator_add)
-  [Function `sub`](#0x1_optional_aggregator_sub)
-  [Function `read`](#0x1_optional_aggregator_read)
-  [Function `is_parallelizable`](#0x1_optional_aggregator_is_parallelizable)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Struct `OptionalAggregator`](#@Specification_1_OptionalAggregator)
    -  [Function `new_integer`](#@Specification_1_new_integer)
    -  [Function `add_integer`](#@Specification_1_add_integer)
    -  [Function `sub_integer`](#@Specification_1_sub_integer)
    -  [Function `limit`](#@Specification_1_limit)
    -  [Function `read_integer`](#@Specification_1_read_integer)
    -  [Function `destroy_integer`](#@Specification_1_destroy_integer)
    -  [Function `new`](#@Specification_1_new)
    -  [Function `switch`](#@Specification_1_switch)
    -  [Function `destroy`](#@Specification_1_destroy)
    -  [Function `destroy_optional_aggregator`](#@Specification_1_destroy_optional_aggregator)
    -  [Function `destroy_optional_integer`](#@Specification_1_destroy_optional_integer)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `sub`](#@Specification_1_sub)
    -  [Function `read`](#@Specification_1_read)


<pre><code><b>use</b> <a href="aggregator.md#0x1_aggregator">0x1::aggregator</a>;
<b>use</b> <a href="aggregator_factory.md#0x1_aggregator_factory">0x1::aggregator_factory</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x1_optional_aggregator_Integer"></a>

## Struct `Integer`

Wrapper around integer with a custom overflow limit. Supports add, subtract and read just like <code>Aggregator</code>.


<pre><code><b>struct</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> <b>has</b> store
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

<a id="0x1_optional_aggregator_OptionalAggregator"></a>

## Struct `OptionalAggregator`

Contains either an aggregator or a normal integer, both overflowing on limit.


<pre><code><b>struct</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>integer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_optional_aggregator_MAX_U128"></a>



<pre><code><b>const</b> <a href="optional_aggregator.md#0x1_optional_aggregator_MAX_U128">MAX_U128</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_optional_aggregator_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by native code.


<pre><code><b>const</b> <a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>: u64 = 1;
</code></pre>



<a id="0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code><b>const</b> <a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>: u64 = 2;
</code></pre>



<a id="0x1_optional_aggregator_ESWITCH_DEPRECATED"></a>

OptionalAggregator (Agg V1) switch not supported any more.


<pre><code><b>const</b> <a href="optional_aggregator.md#0x1_optional_aggregator_ESWITCH_DEPRECATED">ESWITCH_DEPRECATED</a>: u64 = 3;
</code></pre>



<a id="0x1_optional_aggregator_new_integer"></a>

## Function `new_integer`

Creates a new integer which overflows on exceeding a <code>limit</code>.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit: u128): <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit: u128): <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> {
    <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> {
        value: 0,
        limit,
    }
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_add_integer"></a>

## Function `add_integer`

Adds <code>value</code> to integer. Aborts on overflowing the limit.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>, value: u128) {
    <b>assert</b>!(
        value &lt;= (integer.limit - integer.value),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>)
    );
    integer.value = integer.value + value;
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_sub_integer"></a>

## Function `sub_integer`

Subtracts <code>value</code> from integer. Aborts on going below zero.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>, value: u128) {
    <b>assert</b>!(value &lt;= integer.value, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>));
    integer.value = integer.value - value;
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_limit"></a>

## Function `limit`

Returns an overflow limit of integer.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(integer: &<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(integer: &<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>): u128 {
    integer.limit
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_read_integer"></a>

## Function `read_integer`

Returns a value stored in this integer.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer: &<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer: &<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>): u128 {
    integer.value
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_integer"></a>

## Function `destroy_integer`

Destroys an integer.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer: <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer: <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>) {
    <b>let</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> { value: _, limit: _ } = integer;
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_new"></a>

## Function `new`

Creates a new optional aggregator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new">new</a>(parallelizable: bool): <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new">new</a>(parallelizable: bool): <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> {
    <b>if</b> (parallelizable) {
        <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> {
            <a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="aggregator_factory.md#0x1_aggregator_factory_create_aggregator_internal">aggregator_factory::create_aggregator_internal</a>()),
            integer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        }
    } <b>else</b> {
        <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> {
            <a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            integer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_MAX_U128">MAX_U128</a>)),
        }
    }
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_switch"></a>

## Function `switch`

Switches between parallelizable and non-parallelizable implementations.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch">switch</a>(_optional_aggregator: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch">switch</a>(_optional_aggregator: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>) {
    <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_ESWITCH_DEPRECATED">ESWITCH_DEPRECATED</a>)
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy"></a>

## Function `destroy`

Destroys optional aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy">destroy</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy">destroy</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>) {
    <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) {
        <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);
    } <b>else</b> {
        <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);
    }
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_aggregator"></a>

## Function `destroy_optional_aggregator`

Destroys parallelizable optional aggregator and returns its limit.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 {
    <b>let</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> { <a href="aggregator.md#0x1_aggregator">aggregator</a>, integer } = <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>;
    <b>let</b> limit = <a href="aggregator.md#0x1_aggregator_limit">aggregator::limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<a href="aggregator.md#0x1_aggregator">aggregator</a>));
    <a href="aggregator.md#0x1_aggregator_destroy">aggregator::destroy</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>));
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(integer);
    limit
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_integer"></a>

## Function `destroy_optional_integer`

Destroys non-parallelizable optional aggregator and returns its limit.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 {
    <b>let</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> { <a href="aggregator.md#0x1_aggregator">aggregator</a>, integer } = <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>;
    <b>let</b> limit = <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&integer));
    <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(integer));
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);
    limit
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to optional aggregator, aborting on exceeding the <code>limit</code>.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>, value: u128) {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) {
        <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);
        <a href="aggregator.md#0x1_aggregator_add">aggregator::add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>, value);
    } <b>else</b> {
        <b>let</b> integer = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);
        <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer, value);
    }
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from optional aggregator, aborting on going below zero.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub">sub</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub">sub</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>, value: u128) {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) {
        <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);
        <a href="aggregator.md#0x1_aggregator_sub">aggregator::sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>, value);
    } <b>else</b> {
        <b>let</b> integer = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);
        <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer, value);
    }
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_read"></a>

## Function `read`

Returns the value stored in optional aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) {
        <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);
        <a href="aggregator.md#0x1_aggregator_read">aggregator::read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>)
    } <b>else</b> {
        <b>let</b> integer = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);
        <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer)
    }
}
</code></pre>



</details>

<a id="0x1_optional_aggregator_is_parallelizable"></a>

## Function `is_parallelizable`

Returns true if optional aggregator uses parallelizable implementation.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>When creating a new integer instance, it guarantees that the limit assigned is a value passed into the function as an argument, and the value field becomes zero.</td>
<td>High</td>
<td>The new_integer function sets the limit field to the argument passed in, and the value field is set to zero.</td>
<td>Formally verified via <a href="#high-level-req-1">new_integer</a>.</td>
</tr>

<tr>
<td>2</td>
<td>For a given integer instance it should always be possible to: (1) return the limit value of the integer resource, (2) return the current value stored in that particular instance, and (3) destroy the integer instance.</td>
<td>Low</td>
<td>The following functions should not abort if the Integer instance exists: limit(), read_integer(), destroy_integer().</td>
<td>Formally verified via: <a href="#high-level-req-2.1">read_integer</a>, <a href="#high-level-req-2.2">limit</a>, and <a href="#high-level-req-2.3">destroy_integer</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Every successful switch must end with the aggregator type changed from non-parallelizable to parallelizable or vice versa.</td>
<td>High</td>
<td>The switch function run, if successful, should always change the aggregator type.</td>
<td>Formally verified via <a href="#high-level-req-3">switch_and_zero_out</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_OptionalAggregator"></a>

### Struct `OptionalAggregator`


<pre><code><b>struct</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> <b>has</b> store
</code></pre>



<dl>
<dt>
<code><a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="aggregator.md#0x1_aggregator_Aggregator">aggregator::Aggregator</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>integer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) &lt;==&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(integer);
<b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(integer) &lt;==&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);
<b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(integer) ==&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(integer).value &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(integer).limit;
<b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) ==&gt; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &lt;=
    <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>));
</code></pre>



<a id="@Specification_1_new_integer"></a>

### Function `new_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit: u128): <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result.limit == limit;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> result.value == 0;
</code></pre>



<a id="@Specification_1_add_integer"></a>

### Function `add_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)
</code></pre>


Check for overflow.


<pre><code><b>aborts_if</b> value &gt; (integer.limit - integer.value);
<b>aborts_if</b> integer.value + value &gt; <a href="optional_aggregator.md#0x1_optional_aggregator_MAX_U128">MAX_U128</a>;
<b>ensures</b> integer.value &lt;= integer.limit;
<b>ensures</b> integer.value == <b>old</b>(integer.value) + value;
</code></pre>



<a id="@Specification_1_sub_integer"></a>

### Function `sub_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)
</code></pre>




<pre><code><b>aborts_if</b> value &gt; integer.value;
<b>ensures</b> integer.value == <b>old</b>(integer.value) - value;
</code></pre>



<a id="@Specification_1_limit"></a>

### Function `limit`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(integer: &<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_read_integer"></a>

### Function `read_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer: &<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.1" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_destroy_integer"></a>

### Function `destroy_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer: <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.3" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new">new</a>(parallelizable: bool): <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>
</code></pre>




<pre><code><b>aborts_if</b> parallelizable && !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);
<b>ensures</b> parallelizable ==&gt; <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(result);
<b>ensures</b> !parallelizable ==&gt; !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(result);
<b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(result) == 0;
<b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(result) &lt;= <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_limit">optional_aggregator_limit</a>(result);
</code></pre>



<a id="@Specification_1_switch"></a>

### Function `switch`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch">switch</a>(_optional_aggregator: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)
</code></pre>




<pre><code><b>aborts_if</b> <b>true</b>;
</code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy">destroy</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)
</code></pre>




<a id="@Specification_1_destroy_optional_aggregator"></a>

### Function `destroy_optional_aggregator`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128
</code></pre>


The aggregator exists and the integer does not exist when destroy the aggregator.


<pre><code><b>ensures</b> result == <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>));
</code></pre>



<a id="@Specification_1_destroy_optional_integer"></a>

### Function `destroy_optional_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128
</code></pre>


The integer exists and the aggregator does not exist when destroy the integer.


<pre><code><b>ensures</b> result == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit;
</code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_value"></a>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 {
   <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) {
       <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))
   } <b>else</b> {
       <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value
   }
}
</code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_limit"></a>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_limit">optional_aggregator_limit</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 {
   <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) {
       <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))
   } <b>else</b> {
       <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit
   }
}
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)
</code></pre>




<pre><code><b>include</b> <a href="optional_aggregator.md#0x1_optional_aggregator_AddAbortsIf">AddAbortsIf</a>;
<b>ensures</b> ((<a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) == <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) + value));
</code></pre>




<a id="0x1_optional_aggregator_AddAbortsIf"></a>


<pre><code><b>schema</b> <a href="optional_aggregator.md#0x1_optional_aggregator_AddAbortsIf">AddAbortsIf</a> {
    <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>;
    value: u128;
    <b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) && (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))
        + value &gt; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)));
    <b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) && (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))
        + value &gt; <a href="optional_aggregator.md#0x1_optional_aggregator_MAX_U128">MAX_U128</a>);
    <b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &&
        (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value + value &gt; <a href="optional_aggregator.md#0x1_optional_aggregator_MAX_U128">MAX_U128</a>);
    <b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &&
        (value &gt; (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit - <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value));
}
</code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub">sub</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)
</code></pre>




<pre><code><b>include</b> <a href="optional_aggregator.md#0x1_optional_aggregator_SubAbortsIf">SubAbortsIf</a>;
<b>ensures</b> ((<a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) == <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) - value));
</code></pre>




<a id="0x1_optional_aggregator_SubAbortsIf"></a>


<pre><code><b>schema</b> <a href="optional_aggregator.md#0x1_optional_aggregator_SubAbortsIf">SubAbortsIf</a> {
    <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>;
    value: u128;
    <b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) && (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))
        &lt; value);
    <b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &&
        (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value &lt; value);
}
</code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128
</code></pre>




<pre><code><b>ensures</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) ==&gt; result == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value;
<b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) ==&gt;
    result == <a href="aggregator.md#0x1_aggregator_spec_read">aggregator::spec_read</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>));
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
