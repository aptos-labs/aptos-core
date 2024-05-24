
<a id="0x1_execution_config"></a>

# Module `0x1::execution_config`

Maintains the execution config for the blockchain. The config is stored in a
Reconfiguration, and may be updated by root.


-  [Resource `ExecutionConfig`](#0x1_execution_config_ExecutionConfig)
-  [Constants](#@Constants_0)
-  [Function `set`](#0x1_execution_config_set)
-  [Function `set_for_next_epoch`](#0x1_execution_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_execution_config_on_new_epoch)
-  [Specification](#@Specification_1)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)


<pre><code><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_execution_config_ExecutionConfig"></a>

## Resource `ExecutionConfig`



<pre><code><b>struct</b> <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> <b>has</b> drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_execution_config_EINVALID_CONFIG"></a>

The provided on chain config bytes are empty or invalid


<pre><code><b>const</b> <a href="execution_config.md#0x1_execution_config_EINVALID_CONFIG">EINVALID_CONFIG</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_execution_config_set"></a>

## Function `set`

Deprecated by <code><a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;config) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="execution_config.md#0x1_execution_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> config_ref &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework).config;<br />        &#42;config_ref &#61; config;<br />    &#125; <b>else</b> &#123;<br />        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> &#123; config &#125;);<br />    &#125;;<br />    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so validator nodes can sync on the updated configs.<br />    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_execution_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on&#45;chain governance to update on&#45;chain execution configs for the next epoch.
Example usage:
```
aptos_framework::execution_config::set_for_next_epoch(&amp;framework_signer, some_config_bytes);
aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;config) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="execution_config.md#0x1_execution_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> &#123; config &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_execution_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="execution_config.md#0x1_execution_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="execution_config.md#0x1_execution_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;()) &#123;<br />        <b>let</b> config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework) &#61; config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, config);<br />        &#125;;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>


Ensure the caller is admin
When setting now time must be later than last_reconfiguration_time.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 600;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_genesis">chain_status::is_genesis</a>();<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework);<br /><b>requires</b> len(config) &gt; 0;<br /><b>include</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() &#61;&#61;&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;<br /><b>include</b> <a href="aptos_coin.md#0x1_aptos_coin_ExistsAptosCoin">aptos_coin::ExistsAptosCoin</a>;<br /><b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);<br /><b>requires</b> <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &gt;&#61; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();<br /><b>ensures</b> <b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>include</b> <a href="config_buffer.md#0x1_config_buffer_SetForNextEpochAbortsIf">config_buffer::SetForNextEpochAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="execution_config.md#0x1_execution_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> @aptos_framework &#61;&#61; std::signer::address_of(framework);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
