
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
-  [Function `switch_and_zero_out`](#0x1_optional_aggregator_switch_and_zero_out)
-  [Function `switch_to_integer_and_zero_out`](#0x1_optional_aggregator_switch_to_integer_and_zero_out)
-  [Function `switch_to_aggregator_and_zero_out`](#0x1_optional_aggregator_switch_to_aggregator_and_zero_out)
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
    -  [Function `switch_and_zero_out`](#@Specification_1_switch_and_zero_out)
    -  [Function `switch_to_integer_and_zero_out`](#@Specification_1_switch_to_integer_and_zero_out)
    -  [Function `switch_to_aggregator_and_zero_out`](#@Specification_1_switch_to_aggregator_and_zero_out)
    -  [Function `destroy`](#@Specification_1_destroy)
    -  [Function `destroy_optional_aggregator`](#@Specification_1_destroy_optional_aggregator)
    -  [Function `destroy_optional_integer`](#@Specification_1_destroy_optional_integer)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `sub`](#@Specification_1_sub)
    -  [Function `read`](#@Specification_1_read)


<pre><code><b>use</b> <a href="aggregator.md#0x1_aggregator">0x1::aggregator</a>;<br /><b>use</b> <a href="aggregator_factory.md#0x1_aggregator_factory">0x1::aggregator_factory</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /></code></pre>



<a id="0x1_optional_aggregator_Integer"></a>

## Struct `Integer`

Wrapper around integer with a custom overflow limit. Supports add, subtract and read just like <code>Aggregator</code>.


<pre><code><b>struct</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> <b>has</b> store<br /></code></pre>



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


<pre><code><b>struct</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> <b>has</b> store<br /></code></pre>



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


<a id="0x1_optional_aggregator_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by native code.


<pre><code><b>const</b> <a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code><b>const</b> <a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_optional_aggregator_new_integer"></a>

## Function `new_integer`

Creates a new integer which overflows on exceeding a <code>limit</code>.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit: u128): <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit: u128): <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> &#123;<br />    <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> &#123;<br />        value: 0,<br />        limit,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_add_integer"></a>

## Function `add_integer`

Adds <code>value</code> to integer. Aborts on overflowing the limit.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>, value: u128) &#123;<br />    <b>assert</b>!(<br />        value &lt;&#61; (integer.limit &#45; integer.value),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_OVERFLOW">EAGGREGATOR_OVERFLOW</a>)<br />    );<br />    integer.value &#61; integer.value &#43; value;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_sub_integer"></a>

## Function `sub_integer`

Subtracts <code>value</code> from integer. Aborts on going below zero.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>, value: u128) &#123;<br />    <b>assert</b>!(value &lt;&#61; integer.value, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW">EAGGREGATOR_UNDERFLOW</a>));<br />    integer.value &#61; integer.value &#45; value;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_limit"></a>

## Function `limit`

Returns an overflow limit of integer.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(integer: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(integer: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>): u128 &#123;<br />    integer.limit<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_read_integer"></a>

## Function `read_integer`

Returns a value stored in this integer.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>): u128 &#123;<br />    integer.value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_integer"></a>

## Function `destroy_integer`

Destroys an integer.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer: <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer: <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a>) &#123;<br />    <b>let</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">Integer</a> &#123; value: _, limit: _ &#125; &#61; integer;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_new"></a>

## Function `new`

Creates a new optional aggregator.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new">new</a>(limit: u128, parallelizable: bool): <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new">new</a>(limit: u128, parallelizable: bool): <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> &#123;<br />    <b>if</b> (parallelizable) &#123;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> &#123;<br />            <a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="aggregator_factory.md#0x1_aggregator_factory_create_aggregator_internal">aggregator_factory::create_aggregator_internal</a>(limit)),<br />            integer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />        &#125;<br />    &#125; <b>else</b> &#123;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> &#123;<br />            <a href="aggregator.md#0x1_aggregator">aggregator</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),<br />            integer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit)),<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_switch"></a>

## Function `switch`

Switches between parallelizable and non&#45;parallelizable implementations.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch">switch</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch">switch</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>) &#123;<br />    <b>let</b> value &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br />    <a href="optional_aggregator.md#0x1_optional_aggregator_switch_and_zero_out">switch_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br />    <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>, value);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_switch_and_zero_out"></a>

## Function `switch_and_zero_out`

Switches between parallelizable and non&#45;parallelizable implementations, setting
the value of the new optional aggregator to zero.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_and_zero_out">switch_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_and_zero_out">switch_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>) &#123;<br />    <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#123;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_integer_and_zero_out">switch_to_integer_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br />    &#125; <b>else</b> &#123;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_aggregator_and_zero_out">switch_to_aggregator_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_switch_to_integer_and_zero_out"></a>

## Function `switch_to_integer_and_zero_out`

Switches from parallelizable to non&#45;parallelizable implementation, zero&#45;initializing
the value.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_integer_and_zero_out">switch_to_integer_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_integer_and_zero_out">switch_to_integer_and_zero_out</a>(<br />    <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a><br />): u128 &#123;<br />    <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />    <b>let</b> limit &#61; <a href="aggregator.md#0x1_aggregator_limit">aggregator::limit</a>(&amp;<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />    <a href="aggregator.md#0x1_aggregator_destroy">aggregator::destroy</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />    <b>let</b> integer &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer, integer);<br />    limit<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_switch_to_aggregator_and_zero_out"></a>

## Function `switch_to_aggregator_and_zero_out`

Switches from non&#45;parallelizable to parallelizable implementation, zero&#45;initializing
the value.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_aggregator_and_zero_out">switch_to_aggregator_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_aggregator_and_zero_out">switch_to_aggregator_and_zero_out</a>(<br />    <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a><br />): u128 &#123;<br />    <b>let</b> integer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);<br />    <b>let</b> limit &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(&amp;integer);<br />    <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer);<br />    <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> &#61; <a href="aggregator_factory.md#0x1_aggregator_factory_create_aggregator_internal">aggregator_factory::create_aggregator_internal</a>(limit);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>, <a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />    limit<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy"></a>

## Function `destroy`

Destroys optional aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy">destroy</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy">destroy</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>) &#123;<br />    <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#123;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br />    &#125; <b>else</b> &#123;<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_aggregator"></a>

## Function `destroy_optional_aggregator`

Destroys parallelizable optional aggregator and returns its limit.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 &#123;<br />    <b>let</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> &#123; <a href="aggregator.md#0x1_aggregator">aggregator</a>, integer &#125; &#61; <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>;<br />    <b>let</b> limit &#61; <a href="aggregator.md#0x1_aggregator_limit">aggregator::limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="aggregator.md#0x1_aggregator">aggregator</a>));<br />    <a href="aggregator.md#0x1_aggregator_destroy">aggregator::destroy</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(integer);<br />    limit<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_integer"></a>

## Function `destroy_optional_integer`

Destroys non&#45;parallelizable optional aggregator and returns its limit.


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 &#123;<br />    <b>let</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> &#123; <a href="aggregator.md#0x1_aggregator">aggregator</a>, integer &#125; &#61; <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>;<br />    <b>let</b> limit &#61; <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;integer));<br />    <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(integer));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_none">option::destroy_none</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />    limit<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to optional aggregator, aborting on exceeding the <code>limit</code>.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>, value: u128) &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &#123;<br />        <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />        <a href="aggregator.md#0x1_aggregator_add">aggregator::add</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>, value);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> integer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer, value);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from optional aggregator, aborting on going below zero.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub">sub</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub">sub</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>, value: u128) &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &#123;<br />        <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />        <a href="aggregator.md#0x1_aggregator_sub">aggregator::sub</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>, value);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> integer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow_mut">option::borrow_mut</a>(&amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer, value);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_read"></a>

## Function `read`

Returns the value stored in optional aggregator.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &#123;<br />        <b>let</b> <a href="aggregator.md#0x1_aggregator">aggregator</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br />        <a href="aggregator.md#0x1_aggregator_read">aggregator::read</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> integer &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer);<br />        <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_optional_aggregator_is_parallelizable"></a>

## Function `is_parallelizable`

Returns true if optional aggregator uses parallelizable implementation.


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): bool &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)<br />&#125;<br /></code></pre>



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
<td>Every successful switch must end with the aggregator type changed from non&#45;parallelizable to parallelizable or vice versa.</td>
<td>High</td>
<td>The switch function run, if successful, should always change the aggregator type.</td>
<td>Formally verified via <a href="#high-level-req-3">switch_and_zero_out</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_OptionalAggregator"></a>

### Struct `OptionalAggregator`


<pre><code><b>struct</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a> <b>has</b> store<br /></code></pre>



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



<pre><code><b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) &lt;&#61;&#61;&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(integer);<br /><b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(integer) &lt;&#61;&#61;&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>);<br /><b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(integer) &#61;&#61;&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(integer).value &lt;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(integer).limit;<br /><b>invariant</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>) &#61;&#61;&gt; <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &lt;&#61;<br />    <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="aggregator.md#0x1_aggregator">aggregator</a>));<br /></code></pre>



<a id="@Specification_1_new_integer"></a>

### Function `new_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new_integer">new_integer</a>(limit: u128): <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result.limit &#61;&#61; limit;<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> result.value &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_add_integer"></a>

### Function `add_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add_integer">add_integer</a>(integer: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)<br /></code></pre>


Check for overflow.


<pre><code><b>aborts_if</b> value &gt; (integer.limit &#45; integer.value);<br /><b>aborts_if</b> integer.value &#43; value &gt; MAX_U128;<br /><b>ensures</b> integer.value &lt;&#61; integer.limit;<br /><b>ensures</b> integer.value &#61;&#61; <b>old</b>(integer.value) &#43; value;<br /></code></pre>



<a id="@Specification_1_sub_integer"></a>

### Function `sub_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub_integer">sub_integer</a>(integer: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>, value: u128)<br /></code></pre>




<pre><code><b>aborts_if</b> value &gt; integer.value;<br /><b>ensures</b> integer.value &#61;&#61; <b>old</b>(integer.value) &#45; value;<br /></code></pre>



<a id="@Specification_1_limit"></a>

### Function `limit`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_limit">limit</a>(integer: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-2.2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_read_integer"></a>

### Function `read_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read_integer">read_integer</a>(integer: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>): u128<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-2.1" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_destroy_integer"></a>

### Function `destroy_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_integer">destroy_integer</a>(integer: <a href="optional_aggregator.md#0x1_optional_aggregator_Integer">optional_aggregator::Integer</a>)<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-2.3" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_new">new</a>(limit: u128, parallelizable: bool): <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a><br /></code></pre>




<pre><code><b>aborts_if</b> parallelizable &amp;&amp; !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br /><b>ensures</b> parallelizable &#61;&#61;&gt; <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(result);<br /><b>ensures</b> !parallelizable &#61;&#61;&gt; !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(result);<br /><b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(result) &#61;&#61; 0;<br /><b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(result) &lt;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_limit">optional_aggregator_limit</a>(result);<br /></code></pre>



<a id="@Specification_1_switch"></a>

### Function `switch`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch">switch</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)<br /></code></pre>




<pre><code><b>let</b> vec_ref &#61; <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec;<br /><b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; len(vec_ref) !&#61; 0;<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; len(vec_ref) &#61;&#61; 0;<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br /><b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &#61;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>));<br /></code></pre>



<a id="@Specification_1_switch_and_zero_out"></a>

### Function `switch_and_zero_out`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_and_zero_out">switch_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)<br /></code></pre>


Option&lt;Integer&gt; does not exist When Option&lt;Aggregator&gt; exists.
Option&lt;Integer&gt; exists when Option&lt;Aggregator&gt; does not exist.
The AggregatorFactory is under the @aptos_framework when Option&lt;Aggregator&gt; does not exist.


<pre><code><b>let</b> vec_ref &#61; <a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec;<br /><b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; len(vec_ref) !&#61; 0;<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; len(vec_ref) &#61;&#61; 0;<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#61;&#61;&gt; !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br /><b>ensures</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#61;&#61;&gt; <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br /><b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_switch_to_integer_and_zero_out"></a>

### Function `switch_to_integer_and_zero_out`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_integer_and_zero_out">switch_to_integer_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>


The aggregator exists and the integer dosex not exist when Switches from parallelizable to non&#45;parallelizable implementation.


<pre><code><b>let</b> limit &#61; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>));<br /><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>.vec) &#61;&#61; 0;<br /><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec) !&#61; 0;<br /><b>ensures</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit &#61;&#61; limit;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_switch_to_aggregator_and_zero_out"></a>

### Function `switch_to_aggregator_and_zero_out`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_switch_to_aggregator_and_zero_out">switch_to_aggregator_and_zero_out</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>


The integer exists and the aggregator does not exist when Switches from non&#45;parallelizable to parallelizable implementation.
The AggregatorFactory is under the @aptos_framework.


<pre><code><b>let</b> limit &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit;<br /><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec) &#61;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>.vec) !&#61; 0;<br /><b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>);<br /><b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &#61;&#61; limit;<br /><b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)) &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy">destroy</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec) !&#61; 0;<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec) &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_destroy_optional_aggregator"></a>

### Function `destroy_optional_aggregator`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_aggregator">destroy_optional_aggregator</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>


The aggregator exists and the integer does not exist when destroy the aggregator.


<pre><code><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>.vec) &#61;&#61; 0;<br /><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec) !&#61; 0;<br /><b>ensures</b> result &#61;&#61; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>));<br /></code></pre>



<a id="@Specification_1_destroy_optional_integer"></a>

### Function `destroy_optional_integer`


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_destroy_optional_integer">destroy_optional_integer</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>


The integer exists and the aggregator does not exist when destroy the integer.


<pre><code><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer.vec) &#61;&#61; 0;<br /><b>aborts_if</b> len(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>.vec) !&#61; 0;<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit;<br /></code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_value"></a>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 &#123;<br />   <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#123;<br />       <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))<br />   &#125; <b>else</b> &#123;<br />       <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value<br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_limit"></a>


<pre><code><b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_limit">optional_aggregator_limit</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>): u128 &#123;<br />   <b>if</b> (<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#123;<br />       <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))<br />   &#125; <b>else</b> &#123;<br />       <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_add">add</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)<br /></code></pre>




<pre><code><b>include</b> <a href="optional_aggregator.md#0x1_optional_aggregator_AddAbortsIf">AddAbortsIf</a>;<br /><b>ensures</b> ((<a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &#61;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#43; value));<br /></code></pre>




<a id="0x1_optional_aggregator_AddAbortsIf"></a>


<pre><code><b>schema</b> <a href="optional_aggregator.md#0x1_optional_aggregator_AddAbortsIf">AddAbortsIf</a> &#123;<br /><a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>;<br />value: u128;<br /><b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))<br />    &#43; value &gt; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>)));<br /><b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))<br />    &#43; value &gt; MAX_U128);<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp;<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value &#43; value &gt; MAX_U128);<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp;<br />    (value &gt; (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).limit &#45; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value));<br />&#125;<br /></code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_sub">sub</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<b>mut</b> <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>, value: u128)<br /></code></pre>




<pre><code><b>include</b> <a href="optional_aggregator.md#0x1_optional_aggregator_SubAbortsIf">SubAbortsIf</a>;<br /><b>ensures</b> ((<a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &#61;&#61; <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator_value</a>(<b>old</b>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>)) &#45; value));<br /></code></pre>




<a id="0x1_optional_aggregator_SubAbortsIf"></a>


<pre><code><b>schema</b> <a href="optional_aggregator.md#0x1_optional_aggregator_SubAbortsIf">SubAbortsIf</a> &#123;<br /><a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: <a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">OptionalAggregator</a>;<br />value: u128;<br /><b>aborts_if</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp; (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>))<br />    &lt; value);<br /><b>aborts_if</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &amp;&amp;<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value &lt; value);<br />&#125;<br /></code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code><b>public</b> <b>fun</b> <a href="optional_aggregator.md#0x1_optional_aggregator_read">read</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>: &amp;<a href="optional_aggregator.md#0x1_optional_aggregator_OptionalAggregator">optional_aggregator::OptionalAggregator</a>): u128<br /></code></pre>




<pre><code><b>ensures</b> !<a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &#61;&#61;&gt; result &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.integer).value;<br /><b>ensures</b> <a href="optional_aggregator.md#0x1_optional_aggregator_is_parallelizable">is_parallelizable</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>) &#61;&#61;&gt;<br />    result &#61;&#61; <a href="aggregator.md#0x1_aggregator_spec_read">aggregator::spec_read</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(<a href="optional_aggregator.md#0x1_optional_aggregator">optional_aggregator</a>.<a href="aggregator.md#0x1_aggregator">aggregator</a>));<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
