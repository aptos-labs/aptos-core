
<a id="0x1_reconfiguration_with_dkg"></a>

# Module `0x1::reconfiguration_with_dkg`

Reconfiguration with DKG helper functions.


-  [Function `try_start`](#0x1_reconfiguration_with_dkg_try_start)
-  [Function `finish`](#0x1_reconfiguration_with_dkg_finish)
-  [Function `finish_with_dkg_result`](#0x1_reconfiguration_with_dkg_finish_with_dkg_result)
-  [Specification](#@Specification_0)
    -  [Function `try_start`](#@Specification_0_try_start)
    -  [Function `finish`](#@Specification_0_finish)
    -  [Function `finish_with_dkg_result`](#@Specification_0_finish_with_dkg_result)


<pre><code><b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;
<b>use</b> <a href="dkg.md#0x1_dkg">0x1::dkg</a>;
<b>use</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config">0x1::evm_genesis_config</a>;
<b>use</b> <a href="execution_config.md#0x1_execution_config">0x1::execution_config</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="gas_schedule.md#0x1_gas_schedule">0x1::gas_schedule</a>;
<b>use</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config">0x1::jwk_consensus_config</a>;
<b>use</b> <a href="jwks.md#0x1_jwks">0x1::jwks</a>;
<b>use</b> <a href="keyless_account.md#0x1_keyless_account">0x1::keyless_account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config">0x1::randomness_api_v0_config</a>;
<b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;
<b>use</b> <a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum">0x1::randomness_config_seqnum</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state">0x1::reconfiguration_state</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="supra_config.md#0x1_supra_config">0x1::supra_config</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
<b>use</b> <a href="version.md#0x1_version">0x1::version</a>;
</code></pre>



<a id="0x1_reconfiguration_with_dkg_try_start"></a>

## Function `try_start`

Trigger a reconfiguration with DKG.
Do nothing if one is already in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">try_start</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">try_start</a>() {
    <b>let</b> incomplete_dkg_session = <a href="dkg.md#0x1_dkg_incomplete_session">dkg::incomplete_session</a>();
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&incomplete_dkg_session)) {
        <b>let</b> session = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&incomplete_dkg_session);
        <b>if</b> (<a href="dkg.md#0x1_dkg_session_dealer_epoch">dkg::session_dealer_epoch</a>(session) == <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>()) {
            <b>return</b>
        }
    };
    <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">reconfiguration_state::on_reconfig_start</a>();
    <b>let</b> cur_epoch = <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();
    <a href="dkg.md#0x1_dkg_start">dkg::start</a>(
        cur_epoch,
        <a href="randomness_config.md#0x1_randomness_config_current">randomness_config::current</a>(),
        <a href="stake.md#0x1_stake_cur_validator_consensus_infos">stake::cur_validator_consensus_infos</a>(),
        <a href="stake.md#0x1_stake_next_validator_consensus_infos">stake::next_validator_consensus_infos</a>()
    );
}
</code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish"></a>

## Function `finish`

Clear incomplete DKG session, if it exists.
Apply buffered on-chain configs (except for ValidatorSet, which is done inside <code><a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>()</code>).
Re-enable validator set changes.
Run the default reconfiguration to enter the new epoch.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(framework);
    <a href="dkg.md#0x1_dkg_try_clear_incomplete_session">dkg::try_clear_incomplete_session</a>(framework);
    <a href="consensus_config.md#0x1_consensus_config_on_new_epoch">consensus_config::on_new_epoch</a>(framework);
    <a href="execution_config.md#0x1_execution_config_on_new_epoch">execution_config::on_new_epoch</a>(framework);
    <a href="supra_config.md#0x1_supra_config_on_new_epoch">supra_config::on_new_epoch</a>(framework);
    <a href="gas_schedule.md#0x1_gas_schedule_on_new_epoch">gas_schedule::on_new_epoch</a>(framework);
    std::version::on_new_epoch(framework);
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_on_new_epoch">features::on_new_epoch</a>(framework);
    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">jwk_consensus_config::on_new_epoch</a>(framework);
    <a href="jwks.md#0x1_jwks_on_new_epoch">jwks::on_new_epoch</a>(framework);
    <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">keyless_account::on_new_epoch</a>(framework);
    <a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum_on_new_epoch">randomness_config_seqnum::on_new_epoch</a>(framework);
    <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">randomness_config::on_new_epoch</a>(framework);
    <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_on_new_epoch">randomness_api_v0_config::on_new_epoch</a>(framework);
    <a href="evm_genesis_config.md#0x1_evm_genesis_config_on_new_epoch">evm_genesis_config::on_new_epoch</a>(framework);
    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
}
</code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish_with_dkg_result"></a>

## Function `finish_with_dkg_result`

Complete the current reconfiguration with DKG.
Abort if no DKG is in progress.


<pre><code><b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish_with_dkg_result">finish_with_dkg_result</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish_with_dkg_result">finish_with_dkg_result</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="dkg.md#0x1_dkg_finish">dkg::finish</a>(dkg_result);
    <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(<a href="account.md#0x1_account">account</a>);
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_0_try_start"></a>

### Function `try_start`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">try_start</a>()
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>requires</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">reconfiguration::Configuration</a>&gt;(@supra_framework);
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;
<b>include</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>(
) ==&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;
<b>aborts_if</b> <b>false</b>;
<b>pragma</b> verify_duration_estimate = 600;
</code></pre>



<a id="@Specification_0_finish"></a>

### Function `finish`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1500;
<b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">FinishRequirement</a>;
<b>aborts_if</b> <b>false</b>;
</code></pre>




<a id="0x1_reconfiguration_with_dkg_FinishRequirement"></a>


<pre><code><b>schema</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">FinishRequirement</a> {
    framework: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>requires</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework) == @supra_framework;
    <b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
    <b>requires</b> <b>exists</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(@supra_framework);
    <b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
    <b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@supra_framework);
    <b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;
    <b>requires</b> <b>exists</b>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_Features">features::Features</a>&gt;(@std);
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="version.md#0x1_version_Version">version::Version</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">gas_schedule::GasScheduleV2</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">execution_config::ExecutionConfig</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">consensus_config::ConsensusConfig</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="supra_config.md#0x1_supra_config_SupraConfig">supra_config::SupraConfig</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">jwks::SupportedOIDCProviders</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum_RandomnessConfigSeqNum">randomness_config_seqnum::RandomnessConfigSeqNum</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">randomness_api_v0_config::AllowCustomMaxGasFlag</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">randomness_api_v0_config::RequiredGasDeposit</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>&gt;;
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>&gt;;
}
</code></pre>



<a id="@Specification_0_finish_with_dkg_result"></a>

### Function `finish_with_dkg_result`


<pre><code><b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish_with_dkg_result">finish_with_dkg_result</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 1500;
<b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">FinishRequirement</a> {
    framework: <a href="account.md#0x1_account">account</a>
};
<b>requires</b> <a href="dkg.md#0x1_dkg_has_incomplete_session">dkg::has_incomplete_session</a>();
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
