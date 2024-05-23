
<a id="0x1_aggregator_factory"></a>

# Module `0x1::aggregator_factory`

This module provides foundations to create aggregators. Currently only<br/> Aptos Framework (0x1) can create them, so this module helps to wrap<br/> the constructor of <code>Aggregator</code> struct so that only a system account<br/> can initialize one. In the future, this might change and aggregators<br/> can be enabled for the public.


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


<pre><code>use 0x1::aggregator;<br/>use 0x1::error;<br/>use 0x1::system_addresses;<br/>use 0x1::table;<br/></code></pre>



<a id="0x1_aggregator_factory_AggregatorFactory"></a>

## Resource `AggregatorFactory`

Creates new aggregators. Used to control the numbers of aggregators in the<br/> system and who can create them. At the moment, only Aptos Framework (0x1)<br/> account can.


<pre><code>struct AggregatorFactory has key<br/></code></pre>



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


<pre><code>const EAGGREGATOR_FACTORY_NOT_FOUND: u64 &#61; 1;<br/></code></pre>



<a id="0x1_aggregator_factory_initialize_aggregator_factory"></a>

## Function `initialize_aggregator_factory`

Creates a new factory for aggregators. Can only be called during genesis.


<pre><code>public(friend) fun initialize_aggregator_factory(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize_aggregator_factory(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    let aggregator_factory &#61; AggregatorFactory &#123;<br/>        phantom_table: table::new()<br/>    &#125;;<br/>    move_to(aptos_framework, aggregator_factory);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_factory_create_aggregator_internal"></a>

## Function `create_aggregator_internal`

Creates a new aggregator instance which overflows on exceeding a <code>limit</code>.


<pre><code>public(friend) fun create_aggregator_internal(limit: u128): aggregator::Aggregator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_aggregator_internal(limit: u128): Aggregator acquires AggregatorFactory &#123;<br/>    assert!(<br/>        exists&lt;AggregatorFactory&gt;(@aptos_framework),<br/>        error::not_found(EAGGREGATOR_FACTORY_NOT_FOUND)<br/>    );<br/><br/>    let aggregator_factory &#61; borrow_global_mut&lt;AggregatorFactory&gt;(@aptos_framework);<br/>    new_aggregator(aggregator_factory, limit)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_factory_create_aggregator"></a>

## Function `create_aggregator`

This is currently a function closed for public. This can be updated in the future by on&#45;chain governance<br/> to allow any signer to call.


<pre><code>public fun create_aggregator(account: &amp;signer, limit: u128): aggregator::Aggregator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_aggregator(account: &amp;signer, limit: u128): Aggregator acquires AggregatorFactory &#123;<br/>    // Only Aptos Framework (0x1) account can call this for now.<br/>    system_addresses::assert_aptos_framework(account);<br/>    create_aggregator_internal(limit)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aggregator_factory_new_aggregator"></a>

## Function `new_aggregator`

Returns a new aggregator.


<pre><code>fun new_aggregator(aggregator_factory: &amp;mut aggregator_factory::AggregatorFactory, limit: u128): aggregator::Aggregator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun new_aggregator(aggregator_factory: &amp;mut AggregatorFactory, limit: u128): Aggregator;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;During the module&apos;s initialization, it guarantees that the Aptos framework is the caller and that the AggregatorFactory resource will move under the Aptos framework account.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The initialize function is responsible for establishing the initial state of the module by creating the AggregatorFactory resource, indicating its presence within the module&apos;s context. Subsequently, the resource transfers to the Aptos framework account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;initialize_aggregator_factory&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;To create a new aggregator instance, the aggregator factory must already be initialized and exist under the Aptos account.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_aggregator_internal function asserts that AggregatorFactory exists for the Aptos account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;CreateAggregatorInternalAbortsIf&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Only the Aptos framework address may create an aggregator instance currently.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The create_aggregator function ensures that the address calling it is the Aptos framework address.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;create_aggregator&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The creation of new aggregators should be done correctly.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The native new_aggregator function correctly creates a new aggregator.&lt;/td&gt;<br/>&lt;td&gt;The new_aggregator native function has been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize_aggregator_factory"></a>

### Function `initialize_aggregator_factory`


<pre><code>public(friend) fun initialize_aggregator_factory(aptos_framework: &amp;signer)<br/></code></pre>


Make sure the caller is @aptos_framework.<br/> AggregatorFactory is not under the caller before creating the resource.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if addr !&#61; @aptos_framework;<br/>aborts_if exists&lt;AggregatorFactory&gt;(addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures exists&lt;AggregatorFactory&gt;(addr);<br/></code></pre>



<a id="@Specification_1_create_aggregator_internal"></a>

### Function `create_aggregator_internal`


<pre><code>public(friend) fun create_aggregator_internal(limit: u128): aggregator::Aggregator<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
include CreateAggregatorInternalAbortsIf;<br/>ensures aggregator::spec_get_limit(result) &#61;&#61; limit;<br/>ensures aggregator::spec_aggregator_get_val(result) &#61;&#61; 0;<br/></code></pre>




<a id="0x1_aggregator_factory_CreateAggregatorInternalAbortsIf"></a>


<pre><code>schema CreateAggregatorInternalAbortsIf &#123;<br/>aborts_if !exists&lt;AggregatorFactory&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_aggregator"></a>

### Function `create_aggregator`


<pre><code>public fun create_aggregator(account: &amp;signer, limit: u128): aggregator::Aggregator<br/></code></pre>


Make sure the caller is @aptos_framework.<br/> AggregatorFactory existed under the @aptos_framework when Creating a new aggregator.


<pre><code>let addr &#61; signer::address_of(account);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>aborts_if !exists&lt;AggregatorFactory&gt;(@aptos_framework);<br/></code></pre>




<a id="0x1_aggregator_factory_spec_new_aggregator"></a>


<pre><code>native fun spec_new_aggregator(limit: u128): Aggregator;<br/></code></pre>



<a id="@Specification_1_new_aggregator"></a>

### Function `new_aggregator`


<pre><code>fun new_aggregator(aggregator_factory: &amp;mut aggregator_factory::AggregatorFactory, limit: u128): aggregator::Aggregator<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_new_aggregator(limit);<br/>ensures aggregator::spec_get_limit(result) &#61;&#61; limit;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
