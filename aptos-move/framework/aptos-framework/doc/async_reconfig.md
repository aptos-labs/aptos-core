
<a id="0x1_async_reconfig"></a>

# Module `0x1::async_reconfig`

Formal async reconfiguration state management to replace <code><a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg">reconfiguration_with_dkg</a>.<b>move</b></code>.

Every feature that requires end-of-epoch processing now has to specify the following procedures.
- A function <code>on_async_reconfig_start()</code> to start the processing.
- A function <code>ready_for_next_epoch()</code> to inform the framework whether the feature needs more time for processing.
- A function  <code>on_new_epoch()</code> to clean things up right before epoch change.


-  [Function `try_start`](#0x1_async_reconfig_try_start)
-  [Function `force_finish`](#0x1_async_reconfig_force_finish)
-  [Function `try_finish`](#0x1_async_reconfig_try_finish)


<pre><code><b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;
<b>use</b> <a href="dkg.md#0x1_dkg">0x1::dkg</a>;
<b>use</b> <a href="execution_config.md#0x1_execution_config">0x1::execution_config</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="gas_schedule.md#0x1_gas_schedule">0x1::gas_schedule</a>;
<b>use</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config">0x1::jwk_consensus_config</a>;
<b>use</b> <a href="jwks.md#0x1_jwks">0x1::jwks</a>;
<b>use</b> <a href="keyless_account.md#0x1_keyless_account">0x1::keyless_account</a>;
<b>use</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config">0x1::randomness_api_v0_config</a>;
<b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;
<b>use</b> <a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum">0x1::randomness_config_seqnum</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state">0x1::reconfiguration_state</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="version.md#0x1_version">0x1::version</a>;
</code></pre>



<a id="0x1_async_reconfig_try_start"></a>

## Function `try_start`

Trigger an async reconfig. More specifically,
- for every feature that requires end-of-epoch processing, call its <code>on_async_reconfig_start()</code> hook.

Do nothing if one is already in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="async_reconfig.md#0x1_async_reconfig_try_start">try_start</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="async_reconfig.md#0x1_async_reconfig_try_start">try_start</a>() {
    <b>if</b> (!<a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">reconfiguration_state::is_in_progress</a>()) {
        <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">reconfiguration_state::on_reconfig_start</a>();
        <a href="dkg.md#0x1_dkg_on_async_reconfig_start">dkg::on_async_reconfig_start</a>();
        // another_feature::on_async_reconfig_start();
    };
}
</code></pre>



</details>

<a id="0x1_async_reconfig_force_finish"></a>

## Function `force_finish`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="async_reconfig.md#0x1_async_reconfig_force_finish">force_finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="async_reconfig.md#0x1_async_reconfig_force_finish">force_finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <a href="dkg.md#0x1_dkg_on_new_epoch">dkg::on_new_epoch</a>(framework);
    // another_feature::on_new_epoch(framework);

    // Apply buffered config changes.
    <a href="consensus_config.md#0x1_consensus_config_on_new_epoch">consensus_config::on_new_epoch</a>(framework);
    <a href="execution_config.md#0x1_execution_config_on_new_epoch">execution_config::on_new_epoch</a>(framework);
    <a href="gas_schedule.md#0x1_gas_schedule_on_new_epoch">gas_schedule::on_new_epoch</a>(framework);
    std::version::on_new_epoch(framework);
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_on_new_epoch">features::on_new_epoch</a>(framework);
    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">jwk_consensus_config::on_new_epoch</a>(framework);
    <a href="jwks.md#0x1_jwks_on_new_epoch">jwks::on_new_epoch</a>(framework);
    <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">keyless_account::on_new_epoch</a>(framework);
    <a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum_on_new_epoch">randomness_config_seqnum::on_new_epoch</a>(framework);
    <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">randomness_config::on_new_epoch</a>(framework);
    <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_on_new_epoch">randomness_api_v0_config::on_new_epoch</a>(framework);
    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
}
</code></pre>



</details>

<a id="0x1_async_reconfig_try_finish"></a>

## Function `try_finish`

Complete the current reconfiguration with DKG if possible.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="async_reconfig.md#0x1_async_reconfig_try_finish">try_finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="async_reconfig.md#0x1_async_reconfig_try_finish">try_finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> ready_for_next_epoch = <b>true</b>;
    ready_for_next_epoch = ready_for_next_epoch && <a href="dkg.md#0x1_dkg_ready_for_next_epoch">dkg::ready_for_next_epoch</a>();
    // ready_for_next_epoch = ready_for_next_epoch && another_feature::ready_for_next_epoch();
    <b>if</b> (ready_for_next_epoch) {
        <a href="async_reconfig.md#0x1_async_reconfig_force_finish">force_finish</a>(framework);
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
