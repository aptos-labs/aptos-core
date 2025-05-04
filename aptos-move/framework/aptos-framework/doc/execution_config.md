
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


<pre><code><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;
<b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_execution_config_ExecutionConfig"></a>

## Resource `ExecutionConfig`



<pre><code><b>struct</b> <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> <b>has</b> drop, store, key
</code></pre>



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


<pre><code><b>const</b> <a href="execution_config.md#0x1_execution_config_EINVALID_CONFIG">EINVALID_CONFIG</a>: u64 = 1;
</code></pre>



<a id="0x1_execution_config_set"></a>

## Function `set`

Deprecated by <code><a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&config) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="execution_config.md#0x1_execution_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));

    <b>if</b> (<b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework)) {
        <b>let</b> config_ref = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework).config;
        *config_ref = config;
    } <b>else</b> {
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> { config });
    };
    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so validator nodes can sync on the updated configs.
    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
}
</code></pre>



</details>

<a id="0x1_execution_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update on-chain execution configs for the next epoch.
Example usage:
```
aptos_framework::execution_config::set_for_next_epoch(&framework_signer, some_config_bytes);
aptos_framework::aptos_governance::reconfigure(&framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&config) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="execution_config.md#0x1_execution_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> { config });
}
</code></pre>



</details>

<a id="0x1_execution_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="execution_config.md#0x1_execution_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="execution_config.md#0x1_execution_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;()) {
        <b>let</b> config = <a href="config_buffer.md#0x1_config_buffer_extract_v2">config_buffer::extract_v2</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;();
        <b>if</b> (<b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework) = config;
        } <b>else</b> {
            <b>move_to</b>(framework, config);
        };
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


Ensure the caller is admin
When setting now time must be later than last_reconfiguration_time.


<pre><code><b>pragma</b> verify_duration_estimate = 600;
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_genesis">chain_status::is_genesis</a>();
<b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework);
<b>requires</b> len(config) &gt; 0;
<b>include</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() ==&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;
<b>include</b> <a href="aptos_coin.md#0x1_aptos_coin_ExistsAptosCoin">aptos_coin::ExistsAptosCoin</a>;
<b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);
<b>requires</b> <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &gt;= <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();
<b>ensures</b> <b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="execution_config.md#0x1_execution_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>include</b> <a href="config_buffer.md#0x1_config_buffer_SetForNextEpochAbortsIf">config_buffer::SetForNextEpochAbortsIf</a>;
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="execution_config.md#0x1_execution_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> @aptos_framework == std::signer::address_of(framework);
<b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">ExecutionConfig</a>&gt;;
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
