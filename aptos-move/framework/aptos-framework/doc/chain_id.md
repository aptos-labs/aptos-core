
<a id="0x1_chain_id"></a>

# Module `0x1::chain_id`

The chain id distinguishes between different chains (e.g., testnet and the main network).<br/> One important role is to prevent transactions intended for one chain from being executed on another.<br/> This code provides a container for storing a chain id and functions to initialize and get it.


-  [Resource `ChainId`](#0x1_chain_id_ChainId)
-  [Function `initialize`](#0x1_chain_id_initialize)
-  [Function `get`](#0x1_chain_id_get)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_0_initialize)
    -  [Function `get`](#@Specification_0_get)


<pre><code>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_chain_id_ChainId"></a>

## Resource `ChainId`



<pre><code>struct ChainId has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_chain_id_initialize"></a>

## Function `initialize`

Only called during genesis.<br/> Publish the chain ID <code>id</code> of this instance under the SystemAddresses address


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, id: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, id: u8) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, ChainId &#123; id &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_chain_id_get"></a>

## Function `get`

Return the chain ID of this instance.


<pre><code>&#35;[view]<br/>public fun get(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get(): u8 acquires ChainId &#123;<br/>    borrow_global&lt;ChainId&gt;(@aptos_framework).id<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;During genesis, the ChainId resource should be created and moved under the Aptos framework account with the specified chain id.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The chain_id::initialize function is responsible for generating the ChainId resource and then storing it under the aptos_framework account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;initialize&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The chain id can only be fetched if the chain id resource exists under the Aptos framework account.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The chain_id::get function fetches the chain id by borrowing the ChainId resource from the aptos_framework account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;get&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_0_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, id: u8)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if addr !&#61; @aptos_framework;<br/>aborts_if exists&lt;ChainId&gt;(@aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures exists&lt;ChainId&gt;(addr);<br/>ensures global&lt;ChainId&gt;(addr).id &#61;&#61; id;<br/></code></pre>



<a id="@Specification_0_get"></a>

### Function `get`


<pre><code>&#35;[view]<br/>public fun get(): u8<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if !exists&lt;ChainId&gt;(@aptos_framework);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
