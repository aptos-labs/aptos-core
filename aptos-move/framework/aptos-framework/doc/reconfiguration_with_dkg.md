
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


<pre><code>use 0x1::consensus_config;
use 0x1::dkg;
use 0x1::execution_config;
use 0x1::features;
use 0x1::gas_schedule;
use 0x1::jwk_consensus_config;
use 0x1::jwks;
use 0x1::keyless_account;
use 0x1::option;
use 0x1::randomness_api_v0_config;
use 0x1::randomness_config;
use 0x1::randomness_config_seqnum;
use 0x1::reconfiguration;
use 0x1::reconfiguration_state;
use 0x1::stake;
use 0x1::system_addresses;
use 0x1::validator_consensus_info;
use 0x1::version;
</code></pre>



<a id="0x1_reconfiguration_with_dkg_try_start"></a>

## Function `try_start`

Trigger a reconfiguration with DKG.
Do nothing if one is already in progress.


<pre><code>public(friend) fun try_start()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun try_start() &#123;
    let incomplete_dkg_session &#61; dkg::incomplete_session();
    if (option::is_some(&amp;incomplete_dkg_session)) &#123;
        let session &#61; option::borrow(&amp;incomplete_dkg_session);
        if (dkg::session_dealer_epoch(session) &#61;&#61; reconfiguration::current_epoch()) &#123;
            return
        &#125;
    &#125;;
    reconfiguration_state::on_reconfig_start();
    let cur_epoch &#61; reconfiguration::current_epoch();
    dkg::start(
        cur_epoch,
        randomness_config::current(),
        stake::cur_validator_consensus_infos(),
        stake::next_validator_consensus_infos(),
    );
&#125;
</code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish"></a>

## Function `finish`

Clear incomplete DKG session, if it exists.
Apply buffered on-chain configs (except for ValidatorSet, which is done inside <code>reconfiguration::reconfigure()</code>).
Re-enable validator set changes.
Run the default reconfiguration to enter the new epoch.


<pre><code>public(friend) fun finish(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun finish(framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(framework);
    dkg::try_clear_incomplete_session(framework);
    consensus_config::on_new_epoch(framework);
    execution_config::on_new_epoch(framework);
    gas_schedule::on_new_epoch(framework);
    std::version::on_new_epoch(framework);
    features::on_new_epoch(framework);
    jwk_consensus_config::on_new_epoch(framework);
    jwks::on_new_epoch(framework);
    keyless_account::on_new_epoch(framework);
    randomness_config_seqnum::on_new_epoch(framework);
    randomness_config::on_new_epoch(framework);
    randomness_api_v0_config::on_new_epoch(framework);
    reconfiguration::reconfigure();
&#125;
</code></pre>



</details>

<a id="0x1_reconfiguration_with_dkg_finish_with_dkg_result"></a>

## Function `finish_with_dkg_result`

Complete the current reconfiguration with DKG.
Abort if no DKG is in progress.


<pre><code>fun finish_with_dkg_result(account: &amp;signer, dkg_result: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun finish_with_dkg_result(account: &amp;signer, dkg_result: vector&lt;u8&gt;) &#123;
    dkg::finish(dkg_result);
    finish(account);
&#125;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code>pragma verify &#61; true;
</code></pre>



<a id="@Specification_0_try_start"></a>

### Function `try_start`


<pre><code>public(friend) fun try_start()
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
requires exists&lt;reconfiguration::Configuration&gt;(@aptos_framework);
requires chain_status::is_operating();
include stake::ResourceRequirement;
include stake::GetReconfigStartTimeRequirement;
include features::spec_periodical_reward_rate_decrease_enabled(
) &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;
aborts_if false;
pragma verify_duration_estimate &#61; 600;
</code></pre>



<a id="@Specification_0_finish"></a>

### Function `finish`


<pre><code>public(friend) fun finish(framework: &amp;signer)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1500;
include FinishRequirement;
aborts_if false;
</code></pre>




<a id="0x1_reconfiguration_with_dkg_FinishRequirement"></a>


<pre><code>schema FinishRequirement &#123;
    framework: signer;
    requires signer::address_of(framework) &#61;&#61; @aptos_framework;
    requires chain_status::is_operating();
    requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
    include staking_config::StakingRewardsConfigRequirement;
    requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
    include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
    requires exists&lt;features::Features&gt;(@std);
    include config_buffer::OnNewEpochRequirement&lt;version::Version&gt;;
    include config_buffer::OnNewEpochRequirement&lt;gas_schedule::GasScheduleV2&gt;;
    include config_buffer::OnNewEpochRequirement&lt;execution_config::ExecutionConfig&gt;;
    include config_buffer::OnNewEpochRequirement&lt;consensus_config::ConsensusConfig&gt;;
    include config_buffer::OnNewEpochRequirement&lt;jwks::SupportedOIDCProviders&gt;;
    include config_buffer::OnNewEpochRequirement&lt;randomness_config::RandomnessConfig&gt;;
    include config_buffer::OnNewEpochRequirement&lt;randomness_config_seqnum::RandomnessConfigSeqNum&gt;;
    include config_buffer::OnNewEpochRequirement&lt;randomness_api_v0_config::AllowCustomMaxGasFlag&gt;;
    include config_buffer::OnNewEpochRequirement&lt;randomness_api_v0_config::RequiredGasDeposit&gt;;
    include config_buffer::OnNewEpochRequirement&lt;jwk_consensus_config::JWKConsensusConfig&gt;;
    include config_buffer::OnNewEpochRequirement&lt;keyless_account::Configuration&gt;;
    include config_buffer::OnNewEpochRequirement&lt;keyless_account::Groth16VerificationKey&gt;;
&#125;
</code></pre>



<a id="@Specification_0_finish_with_dkg_result"></a>

### Function `finish_with_dkg_result`


<pre><code>fun finish_with_dkg_result(account: &amp;signer, dkg_result: vector&lt;u8&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 1500;
include FinishRequirement &#123;
    framework: account
&#125;;
requires dkg::has_incomplete_session();
aborts_if false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
