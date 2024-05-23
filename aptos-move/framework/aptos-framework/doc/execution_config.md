
<a id="0x1_execution_config"></a>

# Module `0x1::execution_config`

Maintains the execution config for the blockchain. The config is stored in a<br/> Reconfiguration, and may be updated by root.


-  [Resource `ExecutionConfig`](#0x1_execution_config_ExecutionConfig)
-  [Constants](#@Constants_0)
-  [Function `set`](#0x1_execution_config_set)
-  [Function `set_for_next_epoch`](#0x1_execution_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_execution_config_on_new_epoch)
-  [Specification](#@Specification_1)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)


<pre><code>use 0x1::chain_status;<br/>use 0x1::config_buffer;<br/>use 0x1::error;<br/>use 0x1::reconfiguration;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_execution_config_ExecutionConfig"></a>

## Resource `ExecutionConfig`



<pre><code>struct ExecutionConfig has drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_execution_config_EINVALID_CONFIG"></a>

The provided on chain config bytes are empty or invalid


<pre><code>const EINVALID_CONFIG: u64 &#61; 1;<br/></code></pre>



<a id="0x1_execution_config_set"></a>

## Function `set`

Deprecated by <code>set_for_next_epoch()</code>.<br/><br/> WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!<br/><br/> TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;) acquires ExecutionConfig &#123;<br/>    system_addresses::assert_aptos_framework(account);<br/>    chain_status::assert_genesis();<br/><br/>    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));<br/><br/>    if (exists&lt;ExecutionConfig&gt;(@aptos_framework)) &#123;<br/>        let config_ref &#61; &amp;mut borrow_global_mut&lt;ExecutionConfig&gt;(@aptos_framework).config;<br/>        &#42;config_ref &#61; config;<br/>    &#125; else &#123;<br/>        move_to(account, ExecutionConfig &#123; config &#125;);<br/>    &#125;;<br/>    // Need to trigger reconfiguration so validator nodes can sync on the updated configs.<br/>    reconfiguration::reconfigure();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_execution_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on&#45;chain governance to update on&#45;chain execution configs for the next epoch.<br/> Example usage:<br/> ```<br/> aptos_framework::execution_config::set_for_next_epoch(&amp;framework_signer, some_config_bytes);<br/> aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);<br/> ```


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(account);<br/>    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));<br/>    config_buffer::upsert(ExecutionConfig &#123; config &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_execution_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>ExecutionConfig</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires ExecutionConfig &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    if (config_buffer::does_exist&lt;ExecutionConfig&gt;()) &#123;<br/>        let config &#61; config_buffer::extract&lt;ExecutionConfig&gt;();<br/>        if (exists&lt;ExecutionConfig&gt;(@aptos_framework)) &#123;<br/>            &#42;borrow_global_mut&lt;ExecutionConfig&gt;(@aptos_framework) &#61; config;<br/>        &#125; else &#123;<br/>            move_to(framework, config);<br/>        &#125;;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;)<br/></code></pre>


Ensure the caller is admin<br/> When setting now time must be later than last_reconfiguration_time.


<pre><code>pragma verify_duration_estimate &#61; 600;<br/>let addr &#61; signer::address_of(account);<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>requires chain_status::is_genesis();<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);<br/>requires len(config) &gt; 0;<br/>include features::spec_periodical_reward_rate_decrease_enabled() &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;<br/>include aptos_coin::ExistsAptosCoin;<br/>requires system_addresses::is_aptos_framework_address(addr);<br/>requires timestamp::spec_now_microseconds() &gt;&#61; reconfiguration::last_reconfiguration_time();<br/>ensures exists&lt;ExecutionConfig&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>include config_buffer::SetForNextEpochAbortsIf;<br/></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);<br/>include config_buffer::OnNewEpochRequirement&lt;ExecutionConfig&gt;;<br/>aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
