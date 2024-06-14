
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


<pre><code><b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;<br /><b>use</b> <a href="dkg.md#0x1_dkg">0x1::dkg</a>;<br /><b>use</b> <a href="execution_config.md#0x1_execution_config">0x1::execution_config</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="gas_schedule.md#0x1_gas_schedule">0x1::gas_schedule</a>;<br /><b>use</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config">0x1::jwk_consensus_config</a>;<br /><b>use</b> <a href="jwks.md#0x1_jwks">0x1::jwks</a>;<br /><b>use</b> <a href="keyless_account.md#0x1_keyless_account">0x1::keyless_account</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config">0x1::randomness_api_v0_config</a>;<br /><b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;<br /><b>use</b> <a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum">0x1::randomness_config_seqnum</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state">0x1::reconfiguration_state</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;<br /><b>use</b> <a href="version.md#0x1_version">0x1::version</a>;<br /></code></pre>



<a id="0x1_reconfiguration_with_dkg_try_start"></a>

## Function `try_start`

Trigger a reconfiguration with DKG.
Do nothing if one is already in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">try_start</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">try_start</a>() &#123;<br />    <b>let</b> incomplete_dkg_session &#61; <a href="dkg.md#0x1_dkg_incomplete_session">dkg::incomplete_session</a>();<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;incomplete_dkg_session)) &#123;<br />        <b>let</b> session &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;incomplete_dkg_session);<br />        <b>if</b> (<a href="dkg.md#0x1_dkg_session_dealer_epoch">dkg::session_dealer_epoch</a>(session) &#61;&#61; <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>()) &#123;<br />            <b>return</b><br />        &#125;<br />    &#125;;<br />    <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">reconfiguration_state::on_reconfig_start</a>();<br />    <b>let</b> cur_epoch &#61; <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>();<br />    <a href="dkg.md#0x1_dkg_start">dkg::start</a>(<br />        cur_epoch,<br />        <a href="randomness_config.md#0x1_randomness_config_current">randomness_config::current</a>(),<br />        <a href="stake.md#0x1_stake_cur_validator_consensus_infos">stake::cur_validator_consensus_infos</a>(),<br />        <a href="stake.md#0x1_stake_next_validator_consensus_infos">stake::next_validator_consensus_infos</a>(),<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish"></a>

## Function `finish`

Clear incomplete DKG session, if it exists.
Apply buffered on&#45;chain configs (except for ValidatorSet, which is done inside <code><a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>()</code>).
Re&#45;enable validator set changes.
Run the default reconfiguration to enter the new epoch.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <a href="dkg.md#0x1_dkg_try_clear_incomplete_session">dkg::try_clear_incomplete_session</a>(framework);<br />    <a href="consensus_config.md#0x1_consensus_config_on_new_epoch">consensus_config::on_new_epoch</a>(framework);<br />    <a href="execution_config.md#0x1_execution_config_on_new_epoch">execution_config::on_new_epoch</a>(framework);<br />    <a href="gas_schedule.md#0x1_gas_schedule_on_new_epoch">gas_schedule::on_new_epoch</a>(framework);<br />    std::version::on_new_epoch(framework);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_on_new_epoch">features::on_new_epoch</a>(framework);<br />    <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">jwk_consensus_config::on_new_epoch</a>(framework);<br />    <a href="jwks.md#0x1_jwks_on_new_epoch">jwks::on_new_epoch</a>(framework);<br />    <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">keyless_account::on_new_epoch</a>(framework);<br />    <a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum_on_new_epoch">randomness_config_seqnum::on_new_epoch</a>(framework);<br />    <a href="randomness_config.md#0x1_randomness_config_on_new_epoch">randomness_config::on_new_epoch</a>(framework);<br />    <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_on_new_epoch">randomness_api_v0_config::on_new_epoch</a>(framework);<br />    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish_with_dkg_result"></a>

## Function `finish_with_dkg_result`

Complete the current reconfiguration with DKG.
Abort if no DKG is in progress.


<pre><code><b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish_with_dkg_result">finish_with_dkg_result</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish_with_dkg_result">finish_with_dkg_result</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) &#123;<br />    <a href="dkg.md#0x1_dkg_finish">dkg::finish</a>(dkg_result);<br />    <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(<a href="account.md#0x1_account">account</a>);<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_0_try_start"></a>

### Function `try_start`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">try_start</a>()<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>requires</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">reconfiguration::Configuration</a>&gt;(@aptos_framework);<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>(<br />) &#61;&#61;&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;<br /><b>aborts_if</b> <b>false</b>;<br /><b>pragma</b> verify_duration_estimate &#61; 600;<br /></code></pre>



<a id="@Specification_0_finish"></a>

### Function `finish`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">finish</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1500;<br /><b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">FinishRequirement</a>;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_reconfiguration_with_dkg_FinishRequirement"></a>


<pre><code><b>schema</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">FinishRequirement</a> &#123;<br />framework: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>requires</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework) &#61;&#61; @aptos_framework;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>requires</b> <b>exists</b>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_Features">features::Features</a>&gt;(@std);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="version.md#0x1_version_Version">version::Version</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">gas_schedule::GasScheduleV2</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">execution_config::ExecutionConfig</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">consensus_config::ConsensusConfig</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwks.md#0x1_jwks_SupportedOIDCProviders">jwks::SupportedOIDCProviders</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_config_seqnum.md#0x1_randomness_config_seqnum_RandomnessConfigSeqNum">randomness_config_seqnum::RandomnessConfigSeqNum</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">randomness_api_v0_config::AllowCustomMaxGasFlag</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">randomness_api_v0_config::RequiredGasDeposit</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">jwk_consensus_config::JWKConsensusConfig</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>&gt;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>&gt;;<br />&#125;<br /></code></pre>



<a id="@Specification_0_finish_with_dkg_result"></a>

### Function `finish_with_dkg_result`


<pre><code><b>fun</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish_with_dkg_result">finish_with_dkg_result</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 1500;<br /><b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">FinishRequirement</a> &#123;<br />    framework: <a href="account.md#0x1_account">account</a><br />&#125;;<br /><b>requires</b> <a href="dkg.md#0x1_dkg_has_incomplete_session">dkg::has_incomplete_session</a>();<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
