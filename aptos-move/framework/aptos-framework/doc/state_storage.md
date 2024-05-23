
<a id="0x1_state_storage"></a>

# Module `0x1::state_storage`



-  [Struct `Usage`](#0x1_state_storage_Usage)
-  [Resource `StateStorageUsage`](#0x1_state_storage_StateStorageUsage)
-  [Resource `GasParameter`](#0x1_state_storage_GasParameter)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_state_storage_initialize)
-  [Function `on_new_block`](#0x1_state_storage_on_new_block)
-  [Function `current_items_and_bytes`](#0x1_state_storage_current_items_and_bytes)
-  [Function `get_state_storage_usage_only_at_epoch_beginning`](#0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning)
-  [Function `on_reconfig`](#0x1_state_storage_on_reconfig)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `on_new_block`](#@Specification_1_on_new_block)
    -  [Function `current_items_and_bytes`](#@Specification_1_current_items_and_bytes)
    -  [Function `get_state_storage_usage_only_at_epoch_beginning`](#@Specification_1_get_state_storage_usage_only_at_epoch_beginning)
    -  [Function `on_reconfig`](#@Specification_1_on_reconfig)


<pre><code>use 0x1::error;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_state_storage_Usage"></a>

## Struct `Usage`



<pre><code>struct Usage has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>items: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>bytes: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_state_storage_StateStorageUsage"></a>

## Resource `StateStorageUsage`

This is updated at the beginning of each epoch, reflecting the storage<br/> usage after the last txn of the previous epoch is committed.


<pre><code>struct StateStorageUsage has store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>usage: state_storage::Usage</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_state_storage_GasParameter"></a>

## Resource `GasParameter`



<pre><code>struct GasParameter has store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>usage: state_storage::Usage</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_state_storage_ESTATE_STORAGE_USAGE"></a>



<pre><code>const ESTATE_STORAGE_USAGE: u64 &#61; 0;<br/></code></pre>



<a id="0x1_state_storage_initialize"></a>

## Function `initialize`



<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(<br/>        !exists&lt;StateStorageUsage&gt;(@aptos_framework),<br/>        error::already_exists(ESTATE_STORAGE_USAGE)<br/>    );<br/>    move_to(aptos_framework, StateStorageUsage &#123;<br/>        epoch: 0,<br/>        usage: Usage &#123;<br/>            items: 0,<br/>            bytes: 0,<br/>        &#125;<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_state_storage_on_new_block"></a>

## Function `on_new_block`



<pre><code>public(friend) fun on_new_block(epoch: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_block(epoch: u64) acquires StateStorageUsage &#123;<br/>    assert!(<br/>        exists&lt;StateStorageUsage&gt;(@aptos_framework),<br/>        error::not_found(ESTATE_STORAGE_USAGE)<br/>    );<br/>    let usage &#61; borrow_global_mut&lt;StateStorageUsage&gt;(@aptos_framework);<br/>    if (epoch !&#61; usage.epoch) &#123;<br/>        usage.epoch &#61; epoch;<br/>        usage.usage &#61; get_state_storage_usage_only_at_epoch_beginning();<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_state_storage_current_items_and_bytes"></a>

## Function `current_items_and_bytes`



<pre><code>public(friend) fun current_items_and_bytes(): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun current_items_and_bytes(): (u64, u64) acquires StateStorageUsage &#123;<br/>    assert!(<br/>        exists&lt;StateStorageUsage&gt;(@aptos_framework),<br/>        error::not_found(ESTATE_STORAGE_USAGE)<br/>    );<br/>    let usage &#61; borrow_global&lt;StateStorageUsage&gt;(@aptos_framework);<br/>    (usage.usage.items, usage.usage.bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_state_storage_get_state_storage_usage_only_at_epoch_beginning"></a>

## Function `get_state_storage_usage_only_at_epoch_beginning`

Warning: the result returned is based on the base state view held by the<br/> VM for the entire block or chunk of transactions, it&apos;s only deterministic<br/> if called from the first transaction of the block because the execution layer<br/> guarantees a fresh state view then.


<pre><code>fun get_state_storage_usage_only_at_epoch_beginning(): state_storage::Usage<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun get_state_storage_usage_only_at_epoch_beginning(): Usage;<br/></code></pre>



</details>

<a id="0x1_state_storage_on_reconfig"></a>

## Function `on_reconfig`



<pre><code>public(friend) fun on_reconfig()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_reconfig() &#123;<br/>    abort 0<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;Given the blockchain is in an operating state, the resources for tracking state storage usage and gas parameters must exist for the Aptos framework address.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The initialize function ensures only the Aptos framework address can call it.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;module&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;During the initialization of the module, it is guaranteed that the resource for tracking state storage usage will be moved under the Aptos framework account with default initial values.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The resource for tracking state storage usage may only be initialized with specific values and published under the aptos_framework account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;initialize&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The initialization function is only called once, during genesis.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The initialize function ensures StateStorageUsage does not already exist.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;initialize&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;During the initialization of the module, it is guaranteed that the resource for tracking state storage usage will be moved under the Aptos framework account with default initial values.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The resource for tracking state storage usage may only be initialized with specific values and published under the aptos_framework account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;initialize&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;The structure for tracking state storage usage should exist for it to be updated at the beginning of each new block and for retrieving the values of structure members.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The functions on_new_block and current_items_and_bytes verify that the StateStorageUsage structure exists before performing any further operations.&lt;/td&gt;<br/>&lt;td&gt;Formally Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.1&quot;&gt;current_items_and_bytes&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.2&quot;&gt;on_new_block&lt;/a&gt;, and the &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.3&quot;&gt;global invariant&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt; and &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;StateStorageUsage&gt;(@aptos_framework);<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;GasParameter&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>


ensure caller is admin.<br/> aborts if StateStorageUsage already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
aborts_if !system_addresses::is_aptos_framework_address(addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if exists&lt;StateStorageUsage&gt;(@aptos_framework);<br/>ensures exists&lt;StateStorageUsage&gt;(@aptos_framework);<br/>let post state_usage &#61; global&lt;StateStorageUsage&gt;(@aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
ensures state_usage.epoch &#61;&#61; 0 &amp;&amp; state_usage.usage.bytes &#61;&#61; 0 &amp;&amp; state_usage.usage.items &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_on_new_block"></a>

### Function `on_new_block`


<pre><code>public(friend) fun on_new_block(epoch: u64)<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
requires chain_status::is_operating();<br/>aborts_if false;<br/>ensures epoch &#61;&#61; global&lt;StateStorageUsage&gt;(@aptos_framework).epoch;<br/></code></pre>



<a id="@Specification_1_current_items_and_bytes"></a>

### Function `current_items_and_bytes`


<pre><code>public(friend) fun current_items_and_bytes(): (u64, u64)<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if !exists&lt;StateStorageUsage&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_get_state_storage_usage_only_at_epoch_beginning"></a>

### Function `get_state_storage_usage_only_at_epoch_beginning`


<pre><code>fun get_state_storage_usage_only_at_epoch_beginning(): state_storage::Usage<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_on_reconfig"></a>

### Function `on_reconfig`


<pre><code>public(friend) fun on_reconfig()<br/></code></pre>




<pre><code>aborts_if true;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
