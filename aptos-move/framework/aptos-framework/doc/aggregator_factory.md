
<a id="0x1_aggregator_factory"></a>

# Module `0x1::aggregator_factory`

This module provides foundations to create aggregators. Currently only
Aptos Framework (0x1) can create them, so this module helps to wrap
the constructor of <code>Aggregator</code> struct so that only a system account
can initialize one. In the future, this might change and aggregators
can be enabled for the public.


-  [Resource `AggregatorFactory`](#0x1_aggregator_factory_AggregatorFactory)
-  [Constants](#@Constants_0)
-  [Function `initialize_aggregator_factory`](#0x1_aggregator_factory_initialize_aggregator_factory)
-  [Function `create_aggregator_internal`](#0x1_aggregator_factory_create_aggregator_internal)
-  [Function `create_aggregator`](#0x1_aggregator_factory_create_aggregator)
-  [Function `new_aggregator`](#0x1_aggregator_factory_new_aggregator)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize_aggregator_factory`](#@Specification_1_initialize_aggregator_factory)
    -  [Function `create_aggregator_internal`](#@Specification_1_create_aggregator_internal)
    -  [Function `create_aggregator`](#@Specification_1_create_aggregator)
    -  [Function `new_aggregator`](#@Specification_1_new_aggregator)


<pre><code>use 0x1::aggregator;
use 0x1::error;
use 0x1::system_addresses;
use 0x1::table;
</code></pre>



<a id="0x1_aggregator_factory_AggregatorFactory"></a>

## Resource `AggregatorFactory`

Creates new aggregators. Used to control the numbers of aggregators in the
system and who can create them. At the moment, only Aptos Framework (0x1)
account can.


<pre><code>struct AggregatorFactory has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>phantom_table: table::Table&lt;address, u128&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aggregator_factory_EAGGREGATOR_FACTORY_NOT_FOUND"></a>

Aggregator factory is not published yet.


<pre><code>const EAGGREGATOR_FACTORY_NOT_FOUND: u64 &#61; 1;
</code></pre>



<a id="0x1_aggregator_factory_initialize_aggregator_factory"></a>

## Function `initialize_aggregator_factory`

Creates a new factory for aggregators. Can only be called during genesis.


<pre><code>public(friend) fun initialize_aggregator_factory(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_aggregator_factory(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    let aggregator_factory &#61; AggregatorFactory &#123;
        phantom_table: table::new()
    &#125;;
    move_to(aptos_framework, aggregator_factory);
&#125;
</code></pre>



</details>

<a id="0x1_aggregator_factory_create_aggregator_internal"></a>

## Function `create_aggregator_internal`

Creates a new aggregator instance which overflows on exceeding a <code>limit</code>.


<pre><code>public(friend) fun create_aggregator_internal(limit: u128): aggregator::Aggregator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_aggregator_internal(limit: u128): Aggregator acquires AggregatorFactory &#123;
    assert!(
        exists&lt;AggregatorFactory&gt;(@aptos_framework),
        error::not_found(EAGGREGATOR_FACTORY_NOT_FOUND)
    );

    let aggregator_factory &#61; borrow_global_mut&lt;AggregatorFactory&gt;(@aptos_framework);
    new_aggregator(aggregator_factory, limit)
&#125;
</code></pre>



</details>

<a id="0x1_aggregator_factory_create_aggregator"></a>

## Function `create_aggregator`

This is currently a function closed for public. This can be updated in the future by on-chain governance
to allow any signer to call.


<pre><code>public fun create_aggregator(account: &amp;signer, limit: u128): aggregator::Aggregator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_aggregator(account: &amp;signer, limit: u128): Aggregator acquires AggregatorFactory &#123;
    // Only Aptos Framework (0x1) account can call this for now.
    system_addresses::assert_aptos_framework(account);
    create_aggregator_internal(limit)
&#125;
</code></pre>



</details>

<a id="0x1_aggregator_factory_new_aggregator"></a>

## Function `new_aggregator`

Returns a new aggregator.


<pre><code>fun new_aggregator(aggregator_factory: &amp;mut aggregator_factory::AggregatorFactory, limit: u128): aggregator::Aggregator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun new_aggregator(aggregator_factory: &amp;mut AggregatorFactory, limit: u128): Aggregator;
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
<td>During the module's initialization, it guarantees that the Aptos framework is the caller and that the AggregatorFactory resource will move under the Aptos framework account.</td>
<td>High</td>
<td>The initialize function is responsible for establishing the initial state of the module by creating the AggregatorFactory resource, indicating its presence within the module's context. Subsequently, the resource transfers to the Aptos framework account.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize_aggregator_factory</a>.</td>
</tr>

<tr>
<td>2</td>
<td>To create a new aggregator instance, the aggregator factory must already be initialized and exist under the Aptos account.</td>
<td>High</td>
<td>The create_aggregator_internal function asserts that AggregatorFactory exists for the Aptos account.</td>
<td>Formally verified via <a href="#high-level-req-2">CreateAggregatorInternalAbortsIf</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Only the Aptos framework address may create an aggregator instance currently.</td>
<td>Low</td>
<td>The create_aggregator function ensures that the address calling it is the Aptos framework address.</td>
<td>Formally verified via <a href="#high-level-req-3">create_aggregator</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The creation of new aggregators should be done correctly.</td>
<td>High</td>
<td>The native new_aggregator function correctly creates a new aggregator.</td>
<td>The new_aggregator native function has been manually audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize_aggregator_factory"></a>

### Function `initialize_aggregator_factory`


<pre><code>public(friend) fun initialize_aggregator_factory(aptos_framework: &amp;signer)
</code></pre>


Make sure the caller is @aptos_framework.
AggregatorFactory is not under the caller before creating the resource.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if addr !&#61; @aptos_framework;
aborts_if exists&lt;AggregatorFactory&gt;(addr);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures exists&lt;AggregatorFactory&gt;(addr);
</code></pre>



<a id="@Specification_1_create_aggregator_internal"></a>

### Function `create_aggregator_internal`


<pre><code>public(friend) fun create_aggregator_internal(limit: u128): aggregator::Aggregator
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
include CreateAggregatorInternalAbortsIf;
ensures aggregator::spec_get_limit(result) &#61;&#61; limit;
ensures aggregator::spec_aggregator_get_val(result) &#61;&#61; 0;
</code></pre>




<a id="0x1_aggregator_factory_CreateAggregatorInternalAbortsIf"></a>


<pre><code>schema CreateAggregatorInternalAbortsIf &#123;
    aborts_if !exists&lt;AggregatorFactory&gt;(@aptos_framework);
&#125;
</code></pre>



<a id="@Specification_1_create_aggregator"></a>

### Function `create_aggregator`


<pre><code>public fun create_aggregator(account: &amp;signer, limit: u128): aggregator::Aggregator
</code></pre>


Make sure the caller is @aptos_framework.
AggregatorFactory existed under the @aptos_framework when Creating a new aggregator.


<pre><code>let addr &#61; signer::address_of(account);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
aborts_if addr !&#61; @aptos_framework;
aborts_if !exists&lt;AggregatorFactory&gt;(@aptos_framework);
</code></pre>




<a id="0x1_aggregator_factory_spec_new_aggregator"></a>


<pre><code>native fun spec_new_aggregator(limit: u128): Aggregator;
</code></pre>



<a id="@Specification_1_new_aggregator"></a>

### Function `new_aggregator`


<pre><code>fun new_aggregator(aggregator_factory: &amp;mut aggregator_factory::AggregatorFactory, limit: u128): aggregator::Aggregator
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_new_aggregator(limit);
ensures aggregator::spec_get_limit(result) &#61;&#61; limit;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
