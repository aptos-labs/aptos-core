
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


<pre><code>use 0x1::aggregator;
use 0x1::aggregator_factory;
use 0x1::error;
use 0x1::option;
</code></pre>



<a id="0x1_optional_aggregator_Integer"></a>

## Struct `Integer`

Wrapper around integer with a custom overflow limit. Supports add, subtract and read just like <code>Aggregator</code>.


<pre><code>struct Integer has store
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


<pre><code>struct OptionalAggregator has store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>aggregator: option::Option&lt;aggregator::Aggregator&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>integer: option::Option&lt;optional_aggregator::Integer&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_optional_aggregator_EAGGREGATOR_OVERFLOW"></a>

The value of aggregator underflows (goes below zero). Raised by native code.


<pre><code>const EAGGREGATOR_OVERFLOW: u64 &#61; 1;
</code></pre>



<a id="0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code>const EAGGREGATOR_UNDERFLOW: u64 &#61; 2;
</code></pre>



<a id="0x1_optional_aggregator_new_integer"></a>

## Function `new_integer`

Creates a new integer which overflows on exceeding a <code>limit</code>.


<pre><code>fun new_integer(limit: u128): optional_aggregator::Integer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun new_integer(limit: u128): Integer &#123;
    Integer &#123;
        value: 0,
        limit,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_add_integer"></a>

## Function `add_integer`

Adds <code>value</code> to integer. Aborts on overflowing the limit.


<pre><code>fun add_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_integer(integer: &amp;mut Integer, value: u128) &#123;
    assert!(
        value &lt;&#61; (integer.limit &#45; integer.value),
        error::out_of_range(EAGGREGATOR_OVERFLOW)
    );
    integer.value &#61; integer.value &#43; value;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_sub_integer"></a>

## Function `sub_integer`

Subtracts <code>value</code> from integer. Aborts on going below zero.


<pre><code>fun sub_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun sub_integer(integer: &amp;mut Integer, value: u128) &#123;
    assert!(value &lt;&#61; integer.value, error::out_of_range(EAGGREGATOR_UNDERFLOW));
    integer.value &#61; integer.value &#45; value;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_limit"></a>

## Function `limit`

Returns an overflow limit of integer.


<pre><code>fun limit(integer: &amp;optional_aggregator::Integer): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun limit(integer: &amp;Integer): u128 &#123;
    integer.limit
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_read_integer"></a>

## Function `read_integer`

Returns a value stored in this integer.


<pre><code>fun read_integer(integer: &amp;optional_aggregator::Integer): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun read_integer(integer: &amp;Integer): u128 &#123;
    integer.value
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_integer"></a>

## Function `destroy_integer`

Destroys an integer.


<pre><code>fun destroy_integer(integer: optional_aggregator::Integer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_integer(integer: Integer) &#123;
    let Integer &#123; value: _, limit: _ &#125; &#61; integer;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_new"></a>

## Function `new`

Creates a new optional aggregator.


<pre><code>public(friend) fun new(limit: u128, parallelizable: bool): optional_aggregator::OptionalAggregator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun new(limit: u128, parallelizable: bool): OptionalAggregator &#123;
    if (parallelizable) &#123;
        OptionalAggregator &#123;
            aggregator: option::some(aggregator_factory::create_aggregator_internal(limit)),
            integer: option::none(),
        &#125;
    &#125; else &#123;
        OptionalAggregator &#123;
            aggregator: option::none(),
            integer: option::some(new_integer(limit)),
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_switch"></a>

## Function `switch`

Switches between parallelizable and non-parallelizable implementations.


<pre><code>public fun switch(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun switch(optional_aggregator: &amp;mut OptionalAggregator) &#123;
    let value &#61; read(optional_aggregator);
    switch_and_zero_out(optional_aggregator);
    add(optional_aggregator, value);
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_switch_and_zero_out"></a>

## Function `switch_and_zero_out`

Switches between parallelizable and non-parallelizable implementations, setting
the value of the new optional aggregator to zero.


<pre><code>fun switch_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun switch_and_zero_out(optional_aggregator: &amp;mut OptionalAggregator) &#123;
    if (is_parallelizable(optional_aggregator)) &#123;
        switch_to_integer_and_zero_out(optional_aggregator);
    &#125; else &#123;
        switch_to_aggregator_and_zero_out(optional_aggregator);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_switch_to_integer_and_zero_out"></a>

## Function `switch_to_integer_and_zero_out`

Switches from parallelizable to non-parallelizable implementation, zero-initializing
the value.


<pre><code>fun switch_to_integer_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun switch_to_integer_and_zero_out(
    optional_aggregator: &amp;mut OptionalAggregator
): u128 &#123;
    let aggregator &#61; option::extract(&amp;mut optional_aggregator.aggregator);
    let limit &#61; aggregator::limit(&amp;aggregator);
    aggregator::destroy(aggregator);
    let integer &#61; new_integer(limit);
    option::fill(&amp;mut optional_aggregator.integer, integer);
    limit
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_switch_to_aggregator_and_zero_out"></a>

## Function `switch_to_aggregator_and_zero_out`

Switches from non-parallelizable to parallelizable implementation, zero-initializing
the value.


<pre><code>fun switch_to_aggregator_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun switch_to_aggregator_and_zero_out(
    optional_aggregator: &amp;mut OptionalAggregator
): u128 &#123;
    let integer &#61; option::extract(&amp;mut optional_aggregator.integer);
    let limit &#61; limit(&amp;integer);
    destroy_integer(integer);
    let aggregator &#61; aggregator_factory::create_aggregator_internal(limit);
    option::fill(&amp;mut optional_aggregator.aggregator, aggregator);
    limit
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy"></a>

## Function `destroy`

Destroys optional aggregator.


<pre><code>public fun destroy(optional_aggregator: optional_aggregator::OptionalAggregator)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy(optional_aggregator: OptionalAggregator) &#123;
    if (is_parallelizable(&amp;optional_aggregator)) &#123;
        destroy_optional_aggregator(optional_aggregator);
    &#125; else &#123;
        destroy_optional_integer(optional_aggregator);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_aggregator"></a>

## Function `destroy_optional_aggregator`

Destroys parallelizable optional aggregator and returns its limit.


<pre><code>fun destroy_optional_aggregator(optional_aggregator: optional_aggregator::OptionalAggregator): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_optional_aggregator(optional_aggregator: OptionalAggregator): u128 &#123;
    let OptionalAggregator &#123; aggregator, integer &#125; &#61; optional_aggregator;
    let limit &#61; aggregator::limit(option::borrow(&amp;aggregator));
    aggregator::destroy(option::destroy_some(aggregator));
    option::destroy_none(integer);
    limit
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_integer"></a>

## Function `destroy_optional_integer`

Destroys non-parallelizable optional aggregator and returns its limit.


<pre><code>fun destroy_optional_integer(optional_aggregator: optional_aggregator::OptionalAggregator): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_optional_integer(optional_aggregator: OptionalAggregator): u128 &#123;
    let OptionalAggregator &#123; aggregator, integer &#125; &#61; optional_aggregator;
    let limit &#61; limit(option::borrow(&amp;integer));
    destroy_integer(option::destroy_some(integer));
    option::destroy_none(aggregator);
    limit
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to optional aggregator, aborting on exceeding the <code>limit</code>.


<pre><code>public fun add(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add(optional_aggregator: &amp;mut OptionalAggregator, value: u128) &#123;
    if (option::is_some(&amp;optional_aggregator.aggregator)) &#123;
        let aggregator &#61; option::borrow_mut(&amp;mut optional_aggregator.aggregator);
        aggregator::add(aggregator, value);
    &#125; else &#123;
        let integer &#61; option::borrow_mut(&amp;mut optional_aggregator.integer);
        add_integer(integer, value);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from optional aggregator, aborting on going below zero.


<pre><code>public fun sub(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sub(optional_aggregator: &amp;mut OptionalAggregator, value: u128) &#123;
    if (option::is_some(&amp;optional_aggregator.aggregator)) &#123;
        let aggregator &#61; option::borrow_mut(&amp;mut optional_aggregator.aggregator);
        aggregator::sub(aggregator, value);
    &#125; else &#123;
        let integer &#61; option::borrow_mut(&amp;mut optional_aggregator.integer);
        sub_integer(integer, value);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_read"></a>

## Function `read`

Returns the value stored in optional aggregator.


<pre><code>public fun read(optional_aggregator: &amp;optional_aggregator::OptionalAggregator): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read(optional_aggregator: &amp;OptionalAggregator): u128 &#123;
    if (option::is_some(&amp;optional_aggregator.aggregator)) &#123;
        let aggregator &#61; option::borrow(&amp;optional_aggregator.aggregator);
        aggregator::read(aggregator)
    &#125; else &#123;
        let integer &#61; option::borrow(&amp;optional_aggregator.integer);
        read_integer(integer)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_optional_aggregator_is_parallelizable"></a>

## Function `is_parallelizable`

Returns true if optional aggregator uses parallelizable implementation.


<pre><code>public fun is_parallelizable(optional_aggregator: &amp;optional_aggregator::OptionalAggregator): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_parallelizable(optional_aggregator: &amp;OptionalAggregator): bool &#123;
    option::is_some(&amp;optional_aggregator.aggregator)
&#125;
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_OptionalAggregator"></a>

### Struct `OptionalAggregator`


<pre><code>struct OptionalAggregator has store
</code></pre>



<dl>
<dt>
<code>aggregator: option::Option&lt;aggregator::Aggregator&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>integer: option::Option&lt;optional_aggregator::Integer&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant option::is_some(aggregator) &lt;&#61;&#61;&gt; option::is_none(integer);
invariant option::is_some(integer) &lt;&#61;&#61;&gt; option::is_none(aggregator);
invariant option::is_some(integer) &#61;&#61;&gt; option::borrow(integer).value &lt;&#61; option::borrow(integer).limit;
invariant option::is_some(aggregator) &#61;&#61;&gt; aggregator::spec_aggregator_get_val(option::borrow(aggregator)) &lt;&#61;
    aggregator::spec_get_limit(option::borrow(aggregator));
</code></pre>



<a id="@Specification_1_new_integer"></a>

### Function `new_integer`


<pre><code>fun new_integer(limit: u128): optional_aggregator::Integer
</code></pre>




<pre><code>aborts_if false;
ensures result.limit &#61;&#61; limit;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures result.value &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_add_integer"></a>

### Function `add_integer`


<pre><code>fun add_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)
</code></pre>


Check for overflow.


<pre><code>aborts_if value &gt; (integer.limit &#45; integer.value);
aborts_if integer.value &#43; value &gt; MAX_U128;
ensures integer.value &lt;&#61; integer.limit;
ensures integer.value &#61;&#61; old(integer.value) &#43; value;
</code></pre>



<a id="@Specification_1_sub_integer"></a>

### Function `sub_integer`


<pre><code>fun sub_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)
</code></pre>




<pre><code>aborts_if value &gt; integer.value;
ensures integer.value &#61;&#61; old(integer.value) &#45; value;
</code></pre>



<a id="@Specification_1_limit"></a>

### Function `limit`


<pre><code>fun limit(integer: &amp;optional_aggregator::Integer): u128
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
aborts_if false;
</code></pre>



<a id="@Specification_1_read_integer"></a>

### Function `read_integer`


<pre><code>fun read_integer(integer: &amp;optional_aggregator::Integer): u128
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.1" href="#high-level-req">high-level requirement 2</a>:
aborts_if false;
</code></pre>



<a id="@Specification_1_destroy_integer"></a>

### Function `destroy_integer`


<pre><code>fun destroy_integer(integer: optional_aggregator::Integer)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.3" href="#high-level-req">high-level requirement 2</a>:
aborts_if false;
</code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public(friend) fun new(limit: u128, parallelizable: bool): optional_aggregator::OptionalAggregator
</code></pre>




<pre><code>aborts_if parallelizable &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
ensures parallelizable &#61;&#61;&gt; is_parallelizable(result);
ensures !parallelizable &#61;&#61;&gt; !is_parallelizable(result);
ensures optional_aggregator_value(result) &#61;&#61; 0;
ensures optional_aggregator_value(result) &lt;&#61; optional_aggregator_limit(result);
</code></pre>



<a id="@Specification_1_switch"></a>

### Function `switch`


<pre><code>public fun switch(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)
</code></pre>




<pre><code>let vec_ref &#61; optional_aggregator.integer.vec;
aborts_if is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) !&#61; 0;
aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) &#61;&#61; 0;
aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
ensures optional_aggregator_value(optional_aggregator) &#61;&#61; optional_aggregator_value(old(optional_aggregator));
</code></pre>



<a id="@Specification_1_switch_and_zero_out"></a>

### Function `switch_and_zero_out`


<pre><code>fun switch_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)
</code></pre>


Option<Integer> does not exist When Option<Aggregator> exists.
Option<Integer> exists when Option<Aggregator> does not exist.
The AggregatorFactory is under the @aptos_framework when Option<Aggregator> does not exist.


<pre><code>let vec_ref &#61; optional_aggregator.integer.vec;
aborts_if is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) !&#61; 0;
aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) &#61;&#61; 0;
aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures is_parallelizable(old(optional_aggregator)) &#61;&#61;&gt; !is_parallelizable(optional_aggregator);
ensures !is_parallelizable(old(optional_aggregator)) &#61;&#61;&gt; is_parallelizable(optional_aggregator);
ensures optional_aggregator_value(optional_aggregator) &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_switch_to_integer_and_zero_out"></a>

### Function `switch_to_integer_and_zero_out`


<pre><code>fun switch_to_integer_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128
</code></pre>


The aggregator exists and the integer dosex not exist when Switches from parallelizable to non-parallelizable implementation.


<pre><code>let limit &#61; aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator));
aborts_if len(optional_aggregator.aggregator.vec) &#61;&#61; 0;
aborts_if len(optional_aggregator.integer.vec) !&#61; 0;
ensures !is_parallelizable(optional_aggregator);
ensures option::borrow(optional_aggregator.integer).limit &#61;&#61; limit;
ensures option::borrow(optional_aggregator.integer).value &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_switch_to_aggregator_and_zero_out"></a>

### Function `switch_to_aggregator_and_zero_out`


<pre><code>fun switch_to_aggregator_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128
</code></pre>


The integer exists and the aggregator does not exist when Switches from non-parallelizable to parallelizable implementation.
The AggregatorFactory is under the @aptos_framework.


<pre><code>let limit &#61; option::borrow(optional_aggregator.integer).limit;
aborts_if len(optional_aggregator.integer.vec) &#61;&#61; 0;
aborts_if !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
aborts_if len(optional_aggregator.aggregator.vec) !&#61; 0;
ensures is_parallelizable(optional_aggregator);
ensures aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator)) &#61;&#61; limit;
ensures aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator)) &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code>public fun destroy(optional_aggregator: optional_aggregator::OptionalAggregator)
</code></pre>




<pre><code>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; len(optional_aggregator.integer.vec) !&#61; 0;
aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; len(optional_aggregator.integer.vec) &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_destroy_optional_aggregator"></a>

### Function `destroy_optional_aggregator`


<pre><code>fun destroy_optional_aggregator(optional_aggregator: optional_aggregator::OptionalAggregator): u128
</code></pre>


The aggregator exists and the integer does not exist when destroy the aggregator.


<pre><code>aborts_if len(optional_aggregator.aggregator.vec) &#61;&#61; 0;
aborts_if len(optional_aggregator.integer.vec) !&#61; 0;
ensures result &#61;&#61; aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator));
</code></pre>



<a id="@Specification_1_destroy_optional_integer"></a>

### Function `destroy_optional_integer`


<pre><code>fun destroy_optional_integer(optional_aggregator: optional_aggregator::OptionalAggregator): u128
</code></pre>


The integer exists and the aggregator does not exist when destroy the integer.


<pre><code>aborts_if len(optional_aggregator.integer.vec) &#61;&#61; 0;
aborts_if len(optional_aggregator.aggregator.vec) !&#61; 0;
ensures result &#61;&#61; option::borrow(optional_aggregator.integer).limit;
</code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_value"></a>


<pre><code>fun optional_aggregator_value(optional_aggregator: OptionalAggregator): u128 &#123;
   if (is_parallelizable(optional_aggregator)) &#123;
       aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
   &#125; else &#123;
       option::borrow(optional_aggregator.integer).value
   &#125;
&#125;
</code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_limit"></a>


<pre><code>fun optional_aggregator_limit(optional_aggregator: OptionalAggregator): u128 &#123;
   if (is_parallelizable(optional_aggregator)) &#123;
       aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator))
   &#125; else &#123;
       option::borrow(optional_aggregator.integer).limit
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)
</code></pre>




<pre><code>include AddAbortsIf;
ensures ((optional_aggregator_value(optional_aggregator) &#61;&#61; optional_aggregator_value(old(optional_aggregator)) &#43; value));
</code></pre>




<a id="0x1_optional_aggregator_AddAbortsIf"></a>


<pre><code>schema AddAbortsIf &#123;
    optional_aggregator: OptionalAggregator;
    value: u128;
    aborts_if is_parallelizable(optional_aggregator) &amp;&amp; (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
        &#43; value &gt; aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator)));
    aborts_if is_parallelizable(optional_aggregator) &amp;&amp; (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
        &#43; value &gt; MAX_U128);
    aborts_if !is_parallelizable(optional_aggregator) &amp;&amp;
        (option::borrow(optional_aggregator.integer).value &#43; value &gt; MAX_U128);
    aborts_if !is_parallelizable(optional_aggregator) &amp;&amp;
        (value &gt; (option::borrow(optional_aggregator.integer).limit &#45; option::borrow(optional_aggregator.integer).value));
&#125;
</code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code>public fun sub(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)
</code></pre>




<pre><code>include SubAbortsIf;
ensures ((optional_aggregator_value(optional_aggregator) &#61;&#61; optional_aggregator_value(old(optional_aggregator)) &#45; value));
</code></pre>




<a id="0x1_optional_aggregator_SubAbortsIf"></a>


<pre><code>schema SubAbortsIf &#123;
    optional_aggregator: OptionalAggregator;
    value: u128;
    aborts_if is_parallelizable(optional_aggregator) &amp;&amp; (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
        &lt; value);
    aborts_if !is_parallelizable(optional_aggregator) &amp;&amp;
        (option::borrow(optional_aggregator.integer).value &lt; value);
&#125;
</code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code>public fun read(optional_aggregator: &amp;optional_aggregator::OptionalAggregator): u128
</code></pre>




<pre><code>ensures !is_parallelizable(optional_aggregator) &#61;&#61;&gt; result &#61;&#61; option::borrow(optional_aggregator.integer).value;
ensures is_parallelizable(optional_aggregator) &#61;&#61;&gt;
    result &#61;&#61; aggregator::spec_read(option::borrow(optional_aggregator.aggregator));
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
