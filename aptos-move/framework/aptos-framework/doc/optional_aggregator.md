
<a id="0x1_optional_aggregator"></a>

# Module `0x1::optional_aggregator`

This module provides an interface to aggregate integers either via<br/> aggregator (parallelizable) or via normal integers.


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


<pre><code>use 0x1::aggregator;<br/>use 0x1::aggregator_factory;<br/>use 0x1::error;<br/>use 0x1::option;<br/></code></pre>



<a id="0x1_optional_aggregator_Integer"></a>

## Struct `Integer`

Wrapper around integer with a custom overflow limit. Supports add, subtract and read just like <code>Aggregator</code>.


<pre><code>struct Integer has store<br/></code></pre>



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


<pre><code>struct OptionalAggregator has store<br/></code></pre>



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


<pre><code>const EAGGREGATOR_OVERFLOW: u64 &#61; 1;<br/></code></pre>



<a id="0x1_optional_aggregator_EAGGREGATOR_UNDERFLOW"></a>

Aggregator feature is not supported. Raised by native code.


<pre><code>const EAGGREGATOR_UNDERFLOW: u64 &#61; 2;<br/></code></pre>



<a id="0x1_optional_aggregator_new_integer"></a>

## Function `new_integer`

Creates a new integer which overflows on exceeding a <code>limit</code>.


<pre><code>fun new_integer(limit: u128): optional_aggregator::Integer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun new_integer(limit: u128): Integer &#123;<br/>    Integer &#123;<br/>        value: 0,<br/>        limit,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_add_integer"></a>

## Function `add_integer`

Adds <code>value</code> to integer. Aborts on overflowing the limit.


<pre><code>fun add_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_integer(integer: &amp;mut Integer, value: u128) &#123;<br/>    assert!(<br/>        value &lt;&#61; (integer.limit &#45; integer.value),<br/>        error::out_of_range(EAGGREGATOR_OVERFLOW)<br/>    );<br/>    integer.value &#61; integer.value &#43; value;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_sub_integer"></a>

## Function `sub_integer`

Subtracts <code>value</code> from integer. Aborts on going below zero.


<pre><code>fun sub_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun sub_integer(integer: &amp;mut Integer, value: u128) &#123;<br/>    assert!(value &lt;&#61; integer.value, error::out_of_range(EAGGREGATOR_UNDERFLOW));<br/>    integer.value &#61; integer.value &#45; value;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_limit"></a>

## Function `limit`

Returns an overflow limit of integer.


<pre><code>fun limit(integer: &amp;optional_aggregator::Integer): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun limit(integer: &amp;Integer): u128 &#123;<br/>    integer.limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_read_integer"></a>

## Function `read_integer`

Returns a value stored in this integer.


<pre><code>fun read_integer(integer: &amp;optional_aggregator::Integer): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun read_integer(integer: &amp;Integer): u128 &#123;<br/>    integer.value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_integer"></a>

## Function `destroy_integer`

Destroys an integer.


<pre><code>fun destroy_integer(integer: optional_aggregator::Integer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_integer(integer: Integer) &#123;<br/>    let Integer &#123; value: _, limit: _ &#125; &#61; integer;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_new"></a>

## Function `new`

Creates a new optional aggregator.


<pre><code>public(friend) fun new(limit: u128, parallelizable: bool): optional_aggregator::OptionalAggregator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun new(limit: u128, parallelizable: bool): OptionalAggregator &#123;<br/>    if (parallelizable) &#123;<br/>        OptionalAggregator &#123;<br/>            aggregator: option::some(aggregator_factory::create_aggregator_internal(limit)),<br/>            integer: option::none(),<br/>        &#125;<br/>    &#125; else &#123;<br/>        OptionalAggregator &#123;<br/>            aggregator: option::none(),<br/>            integer: option::some(new_integer(limit)),<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_switch"></a>

## Function `switch`

Switches between parallelizable and non&#45;parallelizable implementations.


<pre><code>public fun switch(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun switch(optional_aggregator: &amp;mut OptionalAggregator) &#123;<br/>    let value &#61; read(optional_aggregator);<br/>    switch_and_zero_out(optional_aggregator);<br/>    add(optional_aggregator, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_switch_and_zero_out"></a>

## Function `switch_and_zero_out`

Switches between parallelizable and non&#45;parallelizable implementations, setting<br/> the value of the new optional aggregator to zero.


<pre><code>fun switch_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun switch_and_zero_out(optional_aggregator: &amp;mut OptionalAggregator) &#123;<br/>    if (is_parallelizable(optional_aggregator)) &#123;<br/>        switch_to_integer_and_zero_out(optional_aggregator);<br/>    &#125; else &#123;<br/>        switch_to_aggregator_and_zero_out(optional_aggregator);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_switch_to_integer_and_zero_out"></a>

## Function `switch_to_integer_and_zero_out`

Switches from parallelizable to non&#45;parallelizable implementation, zero&#45;initializing<br/> the value.


<pre><code>fun switch_to_integer_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun switch_to_integer_and_zero_out(<br/>    optional_aggregator: &amp;mut OptionalAggregator<br/>): u128 &#123;<br/>    let aggregator &#61; option::extract(&amp;mut optional_aggregator.aggregator);<br/>    let limit &#61; aggregator::limit(&amp;aggregator);<br/>    aggregator::destroy(aggregator);<br/>    let integer &#61; new_integer(limit);<br/>    option::fill(&amp;mut optional_aggregator.integer, integer);<br/>    limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_switch_to_aggregator_and_zero_out"></a>

## Function `switch_to_aggregator_and_zero_out`

Switches from non&#45;parallelizable to parallelizable implementation, zero&#45;initializing<br/> the value.


<pre><code>fun switch_to_aggregator_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun switch_to_aggregator_and_zero_out(<br/>    optional_aggregator: &amp;mut OptionalAggregator<br/>): u128 &#123;<br/>    let integer &#61; option::extract(&amp;mut optional_aggregator.integer);<br/>    let limit &#61; limit(&amp;integer);<br/>    destroy_integer(integer);<br/>    let aggregator &#61; aggregator_factory::create_aggregator_internal(limit);<br/>    option::fill(&amp;mut optional_aggregator.aggregator, aggregator);<br/>    limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy"></a>

## Function `destroy`

Destroys optional aggregator.


<pre><code>public fun destroy(optional_aggregator: optional_aggregator::OptionalAggregator)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy(optional_aggregator: OptionalAggregator) &#123;<br/>    if (is_parallelizable(&amp;optional_aggregator)) &#123;<br/>        destroy_optional_aggregator(optional_aggregator);<br/>    &#125; else &#123;<br/>        destroy_optional_integer(optional_aggregator);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_aggregator"></a>

## Function `destroy_optional_aggregator`

Destroys parallelizable optional aggregator and returns its limit.


<pre><code>fun destroy_optional_aggregator(optional_aggregator: optional_aggregator::OptionalAggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_optional_aggregator(optional_aggregator: OptionalAggregator): u128 &#123;<br/>    let OptionalAggregator &#123; aggregator, integer &#125; &#61; optional_aggregator;<br/>    let limit &#61; aggregator::limit(option::borrow(&amp;aggregator));<br/>    aggregator::destroy(option::destroy_some(aggregator));<br/>    option::destroy_none(integer);<br/>    limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_destroy_optional_integer"></a>

## Function `destroy_optional_integer`

Destroys non&#45;parallelizable optional aggregator and returns its limit.


<pre><code>fun destroy_optional_integer(optional_aggregator: optional_aggregator::OptionalAggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_optional_integer(optional_aggregator: OptionalAggregator): u128 &#123;<br/>    let OptionalAggregator &#123; aggregator, integer &#125; &#61; optional_aggregator;<br/>    let limit &#61; limit(option::borrow(&amp;integer));<br/>    destroy_integer(option::destroy_some(integer));<br/>    option::destroy_none(aggregator);<br/>    limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_add"></a>

## Function `add`

Adds <code>value</code> to optional aggregator, aborting on exceeding the <code>limit</code>.


<pre><code>public fun add(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add(optional_aggregator: &amp;mut OptionalAggregator, value: u128) &#123;<br/>    if (option::is_some(&amp;optional_aggregator.aggregator)) &#123;<br/>        let aggregator &#61; option::borrow_mut(&amp;mut optional_aggregator.aggregator);<br/>        aggregator::add(aggregator, value);<br/>    &#125; else &#123;<br/>        let integer &#61; option::borrow_mut(&amp;mut optional_aggregator.integer);<br/>        add_integer(integer, value);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_sub"></a>

## Function `sub`

Subtracts <code>value</code> from optional aggregator, aborting on going below zero.


<pre><code>public fun sub(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sub(optional_aggregator: &amp;mut OptionalAggregator, value: u128) &#123;<br/>    if (option::is_some(&amp;optional_aggregator.aggregator)) &#123;<br/>        let aggregator &#61; option::borrow_mut(&amp;mut optional_aggregator.aggregator);<br/>        aggregator::sub(aggregator, value);<br/>    &#125; else &#123;<br/>        let integer &#61; option::borrow_mut(&amp;mut optional_aggregator.integer);<br/>        sub_integer(integer, value);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_read"></a>

## Function `read`

Returns the value stored in optional aggregator.


<pre><code>public fun read(optional_aggregator: &amp;optional_aggregator::OptionalAggregator): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun read(optional_aggregator: &amp;OptionalAggregator): u128 &#123;<br/>    if (option::is_some(&amp;optional_aggregator.aggregator)) &#123;<br/>        let aggregator &#61; option::borrow(&amp;optional_aggregator.aggregator);<br/>        aggregator::read(aggregator)<br/>    &#125; else &#123;<br/>        let integer &#61; option::borrow(&amp;optional_aggregator.integer);<br/>        read_integer(integer)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_optional_aggregator_is_parallelizable"></a>

## Function `is_parallelizable`

Returns true if optional aggregator uses parallelizable implementation.


<pre><code>public fun is_parallelizable(optional_aggregator: &amp;optional_aggregator::OptionalAggregator): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_parallelizable(optional_aggregator: &amp;OptionalAggregator): bool &#123;<br/>    option::is_some(&amp;optional_aggregator.aggregator)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;When creating a new integer instance, it guarantees that the limit assigned is a value passed into the function as an argument, and the value field becomes zero.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The new_integer function sets the limit field to the argument passed in, and the value field is set to zero.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;new_integer&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;For a given integer instance it should always be possible to: (1) return the limit value of the integer resource, (2) return the current value stored in that particular instance, and (3) destroy the integer instance.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The following functions should not abort if the Integer instance exists: limit(), read_integer(), destroy_integer().&lt;/td&gt;<br/>&lt;td&gt;Formally verified via: &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.1&quot;&gt;read_integer&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.2&quot;&gt;limit&lt;/a&gt;, and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.3&quot;&gt;destroy_integer&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Every successful switch must end with the aggregator type changed from non&#45;parallelizable to parallelizable or vice versa.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The switch function run, if successful, should always change the aggregator type.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;switch_and_zero_out&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_OptionalAggregator"></a>

### Struct `OptionalAggregator`


<pre><code>struct OptionalAggregator has store<br/></code></pre>



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



<pre><code>invariant option::is_some(aggregator) &lt;&#61;&#61;&gt; option::is_none(integer);<br/>invariant option::is_some(integer) &lt;&#61;&#61;&gt; option::is_none(aggregator);<br/>invariant option::is_some(integer) &#61;&#61;&gt; option::borrow(integer).value &lt;&#61; option::borrow(integer).limit;<br/>invariant option::is_some(aggregator) &#61;&#61;&gt; aggregator::spec_aggregator_get_val(option::borrow(aggregator)) &lt;&#61;<br/>    aggregator::spec_get_limit(option::borrow(aggregator));<br/></code></pre>



<a id="@Specification_1_new_integer"></a>

### Function `new_integer`


<pre><code>fun new_integer(limit: u128): optional_aggregator::Integer<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.limit &#61;&#61; limit;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures result.value &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_add_integer"></a>

### Function `add_integer`


<pre><code>fun add_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)<br/></code></pre>


Check for overflow.


<pre><code>aborts_if value &gt; (integer.limit &#45; integer.value);<br/>aborts_if integer.value &#43; value &gt; MAX_U128;<br/>ensures integer.value &lt;&#61; integer.limit;<br/>ensures integer.value &#61;&#61; old(integer.value) &#43; value;<br/></code></pre>



<a id="@Specification_1_sub_integer"></a>

### Function `sub_integer`


<pre><code>fun sub_integer(integer: &amp;mut optional_aggregator::Integer, value: u128)<br/></code></pre>




<pre><code>aborts_if value &gt; integer.value;<br/>ensures integer.value &#61;&#61; old(integer.value) &#45; value;<br/></code></pre>



<a id="@Specification_1_limit"></a>

### Function `limit`


<pre><code>fun limit(integer: &amp;optional_aggregator::Integer): u128<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_read_integer"></a>

### Function `read_integer`


<pre><code>fun read_integer(integer: &amp;optional_aggregator::Integer): u128<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_destroy_integer"></a>

### Function `destroy_integer`


<pre><code>fun destroy_integer(integer: optional_aggregator::Integer)<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_new"></a>

### Function `new`


<pre><code>public(friend) fun new(limit: u128, parallelizable: bool): optional_aggregator::OptionalAggregator<br/></code></pre>




<pre><code>aborts_if parallelizable &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>ensures parallelizable &#61;&#61;&gt; is_parallelizable(result);<br/>ensures !parallelizable &#61;&#61;&gt; !is_parallelizable(result);<br/>ensures optional_aggregator_value(result) &#61;&#61; 0;<br/>ensures optional_aggregator_value(result) &lt;&#61; optional_aggregator_limit(result);<br/></code></pre>



<a id="@Specification_1_switch"></a>

### Function `switch`


<pre><code>public fun switch(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)<br/></code></pre>




<pre><code>let vec_ref &#61; optional_aggregator.integer.vec;<br/>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) !&#61; 0;<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) &#61;&#61; 0;<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>ensures optional_aggregator_value(optional_aggregator) &#61;&#61; optional_aggregator_value(old(optional_aggregator));<br/></code></pre>



<a id="@Specification_1_switch_and_zero_out"></a>

### Function `switch_and_zero_out`


<pre><code>fun switch_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator)<br/></code></pre>


Option&lt;Integer&gt; does not exist When Option&lt;Aggregator&gt; exists.<br/> Option&lt;Integer&gt; exists when Option&lt;Aggregator&gt; does not exist.<br/> The AggregatorFactory is under the @aptos_framework when Option&lt;Aggregator&gt; does not exist.


<pre><code>let vec_ref &#61; optional_aggregator.integer.vec;<br/>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) !&#61; 0;<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; len(vec_ref) &#61;&#61; 0;<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures is_parallelizable(old(optional_aggregator)) &#61;&#61;&gt; !is_parallelizable(optional_aggregator);<br/>ensures !is_parallelizable(old(optional_aggregator)) &#61;&#61;&gt; is_parallelizable(optional_aggregator);<br/>ensures optional_aggregator_value(optional_aggregator) &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_switch_to_integer_and_zero_out"></a>

### Function `switch_to_integer_and_zero_out`


<pre><code>fun switch_to_integer_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128<br/></code></pre>


The aggregator exists and the integer dosex not exist when Switches from parallelizable to non&#45;parallelizable implementation.


<pre><code>let limit &#61; aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator));<br/>aborts_if len(optional_aggregator.aggregator.vec) &#61;&#61; 0;<br/>aborts_if len(optional_aggregator.integer.vec) !&#61; 0;<br/>ensures !is_parallelizable(optional_aggregator);<br/>ensures option::borrow(optional_aggregator.integer).limit &#61;&#61; limit;<br/>ensures option::borrow(optional_aggregator.integer).value &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_switch_to_aggregator_and_zero_out"></a>

### Function `switch_to_aggregator_and_zero_out`


<pre><code>fun switch_to_aggregator_and_zero_out(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator): u128<br/></code></pre>


The integer exists and the aggregator does not exist when Switches from non&#45;parallelizable to parallelizable implementation.<br/> The AggregatorFactory is under the @aptos_framework.


<pre><code>let limit &#61; option::borrow(optional_aggregator.integer).limit;<br/>aborts_if len(optional_aggregator.integer.vec) &#61;&#61; 0;<br/>aborts_if !exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>aborts_if len(optional_aggregator.aggregator.vec) !&#61; 0;<br/>ensures is_parallelizable(optional_aggregator);<br/>ensures aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator)) &#61;&#61; limit;<br/>ensures aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator)) &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code>public fun destroy(optional_aggregator: optional_aggregator::OptionalAggregator)<br/></code></pre>




<pre><code>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; len(optional_aggregator.integer.vec) !&#61; 0;<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp; len(optional_aggregator.integer.vec) &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_destroy_optional_aggregator"></a>

### Function `destroy_optional_aggregator`


<pre><code>fun destroy_optional_aggregator(optional_aggregator: optional_aggregator::OptionalAggregator): u128<br/></code></pre>


The aggregator exists and the integer does not exist when destroy the aggregator.


<pre><code>aborts_if len(optional_aggregator.aggregator.vec) &#61;&#61; 0;<br/>aborts_if len(optional_aggregator.integer.vec) !&#61; 0;<br/>ensures result &#61;&#61; aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator));<br/></code></pre>



<a id="@Specification_1_destroy_optional_integer"></a>

### Function `destroy_optional_integer`


<pre><code>fun destroy_optional_integer(optional_aggregator: optional_aggregator::OptionalAggregator): u128<br/></code></pre>


The integer exists and the aggregator does not exist when destroy the integer.


<pre><code>aborts_if len(optional_aggregator.integer.vec) &#61;&#61; 0;<br/>aborts_if len(optional_aggregator.aggregator.vec) !&#61; 0;<br/>ensures result &#61;&#61; option::borrow(optional_aggregator.integer).limit;<br/></code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_value"></a>


<pre><code>fun optional_aggregator_value(optional_aggregator: OptionalAggregator): u128 &#123;<br/>   if (is_parallelizable(optional_aggregator)) &#123;<br/>       aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))<br/>   &#125; else &#123;<br/>       option::borrow(optional_aggregator.integer).value<br/>   &#125;<br/>&#125;<br/></code></pre>




<a id="0x1_optional_aggregator_optional_aggregator_limit"></a>


<pre><code>fun optional_aggregator_limit(optional_aggregator: OptionalAggregator): u128 &#123;<br/>   if (is_parallelizable(optional_aggregator)) &#123;<br/>       aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator))<br/>   &#125; else &#123;<br/>       option::borrow(optional_aggregator.integer).limit<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)<br/></code></pre>




<pre><code>include AddAbortsIf;<br/>ensures ((optional_aggregator_value(optional_aggregator) &#61;&#61; optional_aggregator_value(old(optional_aggregator)) &#43; value));<br/></code></pre>




<a id="0x1_optional_aggregator_AddAbortsIf"></a>


<pre><code>schema AddAbortsIf &#123;<br/>optional_aggregator: OptionalAggregator;<br/>value: u128;<br/>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))<br/>    &#43; value &gt; aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator)));<br/>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))<br/>    &#43; value &gt; MAX_U128);<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp;<br/>    (option::borrow(optional_aggregator.integer).value &#43; value &gt; MAX_U128);<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp;<br/>    (value &gt; (option::borrow(optional_aggregator.integer).limit &#45; option::borrow(optional_aggregator.integer).value));<br/>&#125;<br/></code></pre>



<a id="@Specification_1_sub"></a>

### Function `sub`


<pre><code>public fun sub(optional_aggregator: &amp;mut optional_aggregator::OptionalAggregator, value: u128)<br/></code></pre>




<pre><code>include SubAbortsIf;<br/>ensures ((optional_aggregator_value(optional_aggregator) &#61;&#61; optional_aggregator_value(old(optional_aggregator)) &#45; value));<br/></code></pre>




<a id="0x1_optional_aggregator_SubAbortsIf"></a>


<pre><code>schema SubAbortsIf &#123;<br/>optional_aggregator: OptionalAggregator;<br/>value: u128;<br/>aborts_if is_parallelizable(optional_aggregator) &amp;&amp; (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))<br/>    &lt; value);<br/>aborts_if !is_parallelizable(optional_aggregator) &amp;&amp;<br/>    (option::borrow(optional_aggregator.integer).value &lt; value);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_read"></a>

### Function `read`


<pre><code>public fun read(optional_aggregator: &amp;optional_aggregator::OptionalAggregator): u128<br/></code></pre>




<pre><code>ensures !is_parallelizable(optional_aggregator) &#61;&#61;&gt; result &#61;&#61; option::borrow(optional_aggregator.integer).value;<br/>ensures is_parallelizable(optional_aggregator) &#61;&#61;&gt;<br/>    result &#61;&#61; aggregator::spec_read(option::borrow(optional_aggregator.aggregator));<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
