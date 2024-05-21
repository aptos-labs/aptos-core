
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


<pre><code>use 0x1::consensus_config;<br/>use 0x1::dkg;<br/>use 0x1::execution_config;<br/>use 0x1::features;<br/>use 0x1::gas_schedule;<br/>use 0x1::jwk_consensus_config;<br/>use 0x1::jwks;<br/>use 0x1::keyless_account;<br/>use 0x1::option;<br/>use 0x1::randomness_api_v0_config;<br/>use 0x1::randomness_config;<br/>use 0x1::randomness_config_seqnum;<br/>use 0x1::reconfiguration;<br/>use 0x1::reconfiguration_state;<br/>use 0x1::stake;<br/>use 0x1::system_addresses;<br/>use 0x1::validator_consensus_info;<br/>use 0x1::version;<br/></code></pre>



<a id="0x1_reconfiguration_with_dkg_try_start"></a>

## Function `try_start`

Trigger a reconfiguration with DKG.
Do nothing if one is already in progress.


<pre><code>public(friend) fun try_start()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun try_start() &#123;<br/>    let incomplete_dkg_session &#61; dkg::incomplete_session();<br/>    if (option::is_some(&amp;incomplete_dkg_session)) &#123;<br/>        let session &#61; option::borrow(&amp;incomplete_dkg_session);<br/>        if (dkg::session_dealer_epoch(session) &#61;&#61; reconfiguration::current_epoch()) &#123;<br/>            return<br/>        &#125;<br/>    &#125;;<br/>    reconfiguration_state::on_reconfig_start();<br/>    let cur_epoch &#61; reconfiguration::current_epoch();<br/>    dkg::start(<br/>        cur_epoch,<br/>        randomness_config::current(),<br/>        stake::cur_validator_consensus_infos(),<br/>        stake::next_validator_consensus_infos(),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish"></a>

## Function `finish`

Clear incomplete DKG session, if it exists.
Apply buffered on-chain configs (except for ValidatorSet, which is done inside <code>reconfiguration::reconfigure()</code>).
Re-enable validator set changes.
Run the default reconfiguration to enter the new epoch.


<pre><code>public(friend) fun finish(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun finish(framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    dkg::try_clear_incomplete_session(framework);<br/>    consensus_config::on_new_epoch(framework);<br/>    execution_config::on_new_epoch(framework);<br/>    gas_schedule::on_new_epoch(framework);<br/>    std::version::on_new_epoch(framework);<br/>    features::on_new_epoch(framework);<br/>    jwk_consensus_config::on_new_epoch(framework);<br/>    jwks::on_new_epoch(framework);<br/>    keyless_account::on_new_epoch(framework);<br/>    randomness_config_seqnum::on_new_epoch(framework);<br/>    randomness_config::on_new_epoch(framework);<br/>    randomness_api_v0_config::on_new_epoch(framework);<br/>    reconfiguration::reconfigure();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish_with_dkg_result"></a>

## Function `finish_with_dkg_result`

Complete the current reconfiguration with DKG.
Abort if no DKG is in progress.


<pre><code>fun finish_with_dkg_result(account: &amp;signer, dkg_result: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun finish_with_dkg_result(account: &amp;signer, dkg_result: vector&lt;u8&gt;) &#123;<br/>    dkg::finish(dkg_result);<br/>    finish(account);<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/></code></pre>



<a id="@Specification_0_try_start"></a>

### Function `try_start`


<pre><code>public(friend) fun try_start()<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>requires exists&lt;reconfiguration::Configuration&gt;(@aptos_framework);<br/>requires chain_status::is_operating();<br/>include stake::ResourceRequirement;<br/>include stake::GetReconfigStartTimeRequirement;<br/>include features::spec_periodical_reward_rate_decrease_enabled(<br/>) &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;<br/>aborts_if false;<br/>pragma verify_duration_estimate &#61; 600;<br/></code></pre>



<a id="@Specification_0_finish"></a>

### Function `finish`


<pre><code>public(friend) fun finish(framework: &amp;signer)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1500;<br/>include FinishRequirement;<br/>aborts_if false;<br/></code></pre>




<a id="0x1_reconfiguration_with_dkg_FinishRequirement"></a>


<pre><code>schema FinishRequirement &#123;<br/>framework: signer;<br/>requires signer::address_of(framework) &#61;&#61; @aptos_framework;<br/>requires chain_status::is_operating();<br/>requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>include staking_config::StakingRewardsConfigRequirement;<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>requires exists&lt;features::Features&gt;(@std);<br/>include config_buffer::OnNewEpochRequirement&lt;version::Version&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;gas_schedule::GasScheduleV2&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;execution_config::ExecutionConfig&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;consensus_config::ConsensusConfig&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;jwks::SupportedOIDCProviders&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;randomness_config::RandomnessConfig&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;randomness_config_seqnum::RandomnessConfigSeqNum&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;randomness_api_v0_config::AllowCustomMaxGasFlag&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;randomness_api_v0_config::RequiredGasDeposit&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;jwk_consensus_config::JWKConsensusConfig&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;keyless_account::Configuration&gt;;<br/>include config_buffer::OnNewEpochRequirement&lt;keyless_account::Groth16VerificationKey&gt;;<br/>&#125;<br/></code></pre>



<a id="@Specification_0_finish_with_dkg_result"></a>

### Function `finish_with_dkg_result`


<pre><code>fun finish_with_dkg_result(account: &amp;signer, dkg_result: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1500;<br/>include FinishRequirement &#123;<br/>    framework: account<br/>&#125;;<br/>requires dkg::has_incomplete_session();<br/>aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
