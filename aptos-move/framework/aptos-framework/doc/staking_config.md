
<a id="0x1_staking_config"></a>

# Module `0x1::staking_config`

Provides the configuration for staking and rewards


-  [Resource `StakingConfig`](#0x1_staking_config_StakingConfig)
-  [Resource `StakingRewardsConfig`](#0x1_staking_config_StakingRewardsConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_staking_config_initialize)
-  [Function `initialize_rewards`](#0x1_staking_config_initialize_rewards)
-  [Function `get`](#0x1_staking_config_get)
-  [Function `get_allow_validator_set_change`](#0x1_staking_config_get_allow_validator_set_change)
-  [Function `get_required_stake`](#0x1_staking_config_get_required_stake)
-  [Function `get_recurring_lockup_duration`](#0x1_staking_config_get_recurring_lockup_duration)
-  [Function `get_reward_rate`](#0x1_staking_config_get_reward_rate)
-  [Function `get_voting_power_increase_limit`](#0x1_staking_config_get_voting_power_increase_limit)
-  [Function `calculate_and_save_latest_epoch_rewards_rate`](#0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate)
-  [Function `calculate_and_save_latest_rewards_config`](#0x1_staking_config_calculate_and_save_latest_rewards_config)
-  [Function `update_required_stake`](#0x1_staking_config_update_required_stake)
-  [Function `update_recurring_lockup_duration_secs`](#0x1_staking_config_update_recurring_lockup_duration_secs)
-  [Function `update_rewards_rate`](#0x1_staking_config_update_rewards_rate)
-  [Function `update_rewards_config`](#0x1_staking_config_update_rewards_config)
-  [Function `update_voting_power_increase_limit`](#0x1_staking_config_update_voting_power_increase_limit)
-  [Function `validate_required_stake`](#0x1_staking_config_validate_required_stake)
-  [Function `validate_rewards_config`](#0x1_staking_config_validate_rewards_config)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Resource `StakingConfig`](#@Specification_1_StakingConfig)
    -  [Resource `StakingRewardsConfig`](#@Specification_1_StakingRewardsConfig)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initialize_rewards`](#@Specification_1_initialize_rewards)
    -  [Function `get`](#@Specification_1_get)
    -  [Function `get_reward_rate`](#@Specification_1_get_reward_rate)
    -  [Function `calculate_and_save_latest_epoch_rewards_rate`](#@Specification_1_calculate_and_save_latest_epoch_rewards_rate)
    -  [Function `calculate_and_save_latest_rewards_config`](#@Specification_1_calculate_and_save_latest_rewards_config)
    -  [Function `update_required_stake`](#@Specification_1_update_required_stake)
    -  [Function `update_recurring_lockup_duration_secs`](#@Specification_1_update_recurring_lockup_duration_secs)
    -  [Function `update_rewards_rate`](#@Specification_1_update_rewards_rate)
    -  [Function `update_rewards_config`](#@Specification_1_update_rewards_config)
    -  [Function `update_voting_power_increase_limit`](#@Specification_1_update_voting_power_increase_limit)
    -  [Function `validate_required_stake`](#@Specification_1_validate_required_stake)
    -  [Function `validate_rewards_config`](#@Specification_1_validate_rewards_config)


<pre><code>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::fixed_point64;<br/>use 0x1::math_fixed64;<br/>use 0x1::system_addresses;<br/>use 0x1::timestamp;<br/></code></pre>



<a id="0x1_staking_config_StakingConfig"></a>

## Resource `StakingConfig`

Validator set configurations that will be stored with the @aptos_framework account.


<pre><code>struct StakingConfig has copy, drop, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>minimum_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>recurring_lockup_duration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>allow_validator_set_change: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_power_increase_limit: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_staking_config_StakingRewardsConfig"></a>

## Resource `StakingRewardsConfig`

Staking reward configurations that will be stored with the @aptos_framework account.


<pre><code>struct StakingRewardsConfig has copy, drop, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>rewards_rate: fixed_point64::FixedPoint64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_rewards_rate: fixed_point64::FixedPoint64</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate_period_in_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_rewards_rate_period_start_in_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate_decrease_rate: fixed_point64::FixedPoint64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_staking_config_MAX_U64"></a>



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_staking_config_BPS_DENOMINATOR"></a>

Denominator of number in basis points. 1 bps(basis points) &#61; 0.01%.


<pre><code>const BPS_DENOMINATOR: u64 &#61; 10000;<br/></code></pre>



<a id="0x1_staking_config_EDEPRECATED_FUNCTION"></a>

The function has been deprecated.


<pre><code>const EDEPRECATED_FUNCTION: u64 &#61; 10;<br/></code></pre>



<a id="0x1_staking_config_EDISABLED_FUNCTION"></a>

The function is disabled or hasn&apos;t been enabled.


<pre><code>const EDISABLED_FUNCTION: u64 &#61; 11;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_LAST_REWARDS_RATE_PERIOD_START"></a>

Specified start time of last rewards rate period is invalid, which must be not late than the current timestamp.


<pre><code>const EINVALID_LAST_REWARDS_RATE_PERIOD_START: u64 &#61; 7;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_MIN_REWARDS_RATE"></a>

Specified min rewards rate is invalid, which must be within [0, rewards_rate].


<pre><code>const EINVALID_MIN_REWARDS_RATE: u64 &#61; 6;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_REWARDS_RATE"></a>

Specified rewards rate is invalid, which must be within [0, MAX_REWARDS_RATE].


<pre><code>const EINVALID_REWARDS_RATE: u64 &#61; 5;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_REWARDS_RATE_DECREASE_RATE"></a>

Specified rewards rate decrease rate is invalid, which must be not greater than BPS_DENOMINATOR.


<pre><code>const EINVALID_REWARDS_RATE_DECREASE_RATE: u64 &#61; 8;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_REWARDS_RATE_PERIOD"></a>

Specified rewards rate period is invalid. It must be larger than 0 and cannot be changed if configured.


<pre><code>const EINVALID_REWARDS_RATE_PERIOD: u64 &#61; 9;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_STAKE_RANGE"></a>

Specified stake range is invalid. Max must be greater than min.


<pre><code>const EINVALID_STAKE_RANGE: u64 &#61; 3;<br/></code></pre>



<a id="0x1_staking_config_EINVALID_VOTING_POWER_INCREASE_LIMIT"></a>

The voting power increase limit percentage must be within (0, 50].


<pre><code>const EINVALID_VOTING_POWER_INCREASE_LIMIT: u64 &#61; 4;<br/></code></pre>



<a id="0x1_staking_config_EZERO_LOCKUP_DURATION"></a>

Stake lockup duration cannot be zero.


<pre><code>const EZERO_LOCKUP_DURATION: u64 &#61; 1;<br/></code></pre>



<a id="0x1_staking_config_EZERO_REWARDS_RATE_DENOMINATOR"></a>

Reward rate denominator cannot be zero.


<pre><code>const EZERO_REWARDS_RATE_DENOMINATOR: u64 &#61; 2;<br/></code></pre>



<a id="0x1_staking_config_MAX_REWARDS_RATE"></a>

Limit the maximum value of <code>rewards_rate</code> in order to avoid any arithmetic overflow.


<pre><code>const MAX_REWARDS_RATE: u64 &#61; 1000000;<br/></code></pre>



<a id="0x1_staking_config_ONE_YEAR_IN_SECS"></a>

1 year &#61;&gt; 365 &#42; 24 &#42; 60 &#42; 60


<pre><code>const ONE_YEAR_IN_SECS: u64 &#61; 31536000;<br/></code></pre>



<a id="0x1_staking_config_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(<br/>    aptos_framework: &amp;signer,<br/>    minimum_stake: u64,<br/>    maximum_stake: u64,<br/>    recurring_lockup_duration_secs: u64,<br/>    allow_validator_set_change: bool,<br/>    rewards_rate: u64,<br/>    rewards_rate_denominator: u64,<br/>    voting_power_increase_limit: u64,<br/>) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    // This can fail genesis but is necessary so that any misconfigurations can be corrected before genesis succeeds<br/>    validate_required_stake(minimum_stake, maximum_stake);<br/><br/>    assert!(recurring_lockup_duration_secs &gt; 0, error::invalid_argument(EZERO_LOCKUP_DURATION));<br/>    assert!(<br/>        rewards_rate_denominator &gt; 0,<br/>        error::invalid_argument(EZERO_REWARDS_RATE_DENOMINATOR),<br/>    );<br/>    assert!(<br/>        voting_power_increase_limit &gt; 0 &amp;&amp; voting_power_increase_limit &lt;&#61; 50,<br/>        error::invalid_argument(EINVALID_VOTING_POWER_INCREASE_LIMIT),<br/>    );<br/><br/>    // `rewards_rate` which is the numerator is limited to be `&lt;&#61; MAX_REWARDS_RATE` in order to avoid the arithmetic<br/>    // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards<br/>    // rate (i.e., rewards_rate / rewards_rate_denominator).<br/>    assert!(rewards_rate &lt;&#61; MAX_REWARDS_RATE, error::invalid_argument(EINVALID_REWARDS_RATE));<br/><br/>    // We assert that (rewards_rate / rewards_rate_denominator &lt;&#61; 1).<br/>    assert!(rewards_rate &lt;&#61; rewards_rate_denominator, error::invalid_argument(EINVALID_REWARDS_RATE));<br/><br/>    move_to(aptos_framework, StakingConfig &#123;<br/>        minimum_stake,<br/>        maximum_stake,<br/>        recurring_lockup_duration_secs,<br/>        allow_validator_set_change,<br/>        rewards_rate,<br/>        rewards_rate_denominator,<br/>        voting_power_increase_limit,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_initialize_rewards"></a>

## Function `initialize_rewards`

Initialize rewards configurations.<br/> Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun initialize_rewards(aptos_framework: &amp;signer, rewards_rate: fixed_point64::FixedPoint64, min_rewards_rate: fixed_point64::FixedPoint64, rewards_rate_period_in_secs: u64, last_rewards_rate_period_start_in_secs: u64, rewards_rate_decrease_rate: fixed_point64::FixedPoint64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_rewards(<br/>    aptos_framework: &amp;signer,<br/>    rewards_rate: FixedPoint64,<br/>    min_rewards_rate: FixedPoint64,<br/>    rewards_rate_period_in_secs: u64,<br/>    last_rewards_rate_period_start_in_secs: u64,<br/>    rewards_rate_decrease_rate: FixedPoint64,<br/>) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    validate_rewards_config(<br/>        rewards_rate,<br/>        min_rewards_rate,<br/>        rewards_rate_period_in_secs,<br/>        rewards_rate_decrease_rate,<br/>    );<br/>    assert!(<br/>        timestamp::now_seconds() &gt;&#61; last_rewards_rate_period_start_in_secs,<br/>        error::invalid_argument(EINVALID_LAST_REWARDS_RATE_PERIOD_START)<br/>    );<br/><br/>    move_to(aptos_framework, StakingRewardsConfig &#123;<br/>        rewards_rate,<br/>        min_rewards_rate,<br/>        rewards_rate_period_in_secs,<br/>        last_rewards_rate_period_start_in_secs,<br/>        rewards_rate_decrease_rate,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_get"></a>

## Function `get`



<pre><code>public fun get(): staking_config::StakingConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get(): StakingConfig acquires StakingConfig &#123;<br/>    &#42;borrow_global&lt;StakingConfig&gt;(@aptos_framework)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_get_allow_validator_set_change"></a>

## Function `get_allow_validator_set_change`

Return whether validator set changes are allowed


<pre><code>public fun get_allow_validator_set_change(config: &amp;staking_config::StakingConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_allow_validator_set_change(config: &amp;StakingConfig): bool &#123;<br/>    config.allow_validator_set_change<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_get_required_stake"></a>

## Function `get_required_stake`

Return the required min/max stake.


<pre><code>public fun get_required_stake(config: &amp;staking_config::StakingConfig): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_required_stake(config: &amp;StakingConfig): (u64, u64) &#123;<br/>    (config.minimum_stake, config.maximum_stake)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_get_recurring_lockup_duration"></a>

## Function `get_recurring_lockup_duration`

Return the recurring lockup duration that every validator is automatically renewed for (unless they unlock and<br/> withdraw all funds).


<pre><code>public fun get_recurring_lockup_duration(config: &amp;staking_config::StakingConfig): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_recurring_lockup_duration(config: &amp;StakingConfig): u64 &#123;<br/>    config.recurring_lockup_duration_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_get_reward_rate"></a>

## Function `get_reward_rate`

Return the reward rate of this epoch.


<pre><code>public fun get_reward_rate(config: &amp;staking_config::StakingConfig): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_reward_rate(config: &amp;StakingConfig): (u64, u64) acquires StakingRewardsConfig &#123;<br/>    if (features::periodical_reward_rate_decrease_enabled()) &#123;<br/>        let epoch_rewards_rate &#61; borrow_global&lt;StakingRewardsConfig&gt;(@aptos_framework).rewards_rate;<br/>        if (fixed_point64::is_zero(epoch_rewards_rate)) &#123;<br/>            (0u64, 1u64)<br/>        &#125; else &#123;<br/>            // Maximize denominator for higher precision.<br/>            // Restriction: nominator &lt;&#61; MAX_REWARDS_RATE &amp;&amp; denominator &lt;&#61; MAX_U64<br/>            let denominator &#61; fixed_point64::divide_u128((MAX_REWARDS_RATE as u128), epoch_rewards_rate);<br/>            if (denominator &gt; MAX_U64) &#123;<br/>                denominator &#61; MAX_U64<br/>            &#125;;<br/>            let nominator &#61; (fixed_point64::multiply_u128(denominator, epoch_rewards_rate) as u64);<br/>            (nominator, (denominator as u64))<br/>        &#125;<br/>    &#125; else &#123;<br/>        (config.rewards_rate, config.rewards_rate_denominator)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_get_voting_power_increase_limit"></a>

## Function `get_voting_power_increase_limit`

Return the joining limit %.


<pre><code>public fun get_voting_power_increase_limit(config: &amp;staking_config::StakingConfig): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_voting_power_increase_limit(config: &amp;StakingConfig): u64 &#123;<br/>    config.voting_power_increase_limit<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate"></a>

## Function `calculate_and_save_latest_epoch_rewards_rate`

Calculate and save the latest rewards rate.


<pre><code>public(friend) fun calculate_and_save_latest_epoch_rewards_rate(): fixed_point64::FixedPoint64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun calculate_and_save_latest_epoch_rewards_rate(): FixedPoint64 acquires StakingRewardsConfig &#123;<br/>    assert!(features::periodical_reward_rate_decrease_enabled(), error::invalid_state(EDISABLED_FUNCTION));<br/>    let staking_rewards_config &#61; calculate_and_save_latest_rewards_config();<br/>    staking_rewards_config.rewards_rate<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_calculate_and_save_latest_rewards_config"></a>

## Function `calculate_and_save_latest_rewards_config`

Calculate and return the up&#45;to&#45;date StakingRewardsConfig.


<pre><code>fun calculate_and_save_latest_rewards_config(): staking_config::StakingRewardsConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_and_save_latest_rewards_config(): StakingRewardsConfig acquires StakingRewardsConfig &#123;<br/>    let staking_rewards_config &#61; borrow_global_mut&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/>    let current_time_in_secs &#61; timestamp::now_seconds();<br/>    assert!(<br/>        current_time_in_secs &gt;&#61; staking_rewards_config.last_rewards_rate_period_start_in_secs,<br/>        error::invalid_argument(EINVALID_LAST_REWARDS_RATE_PERIOD_START)<br/>    );<br/>    if (current_time_in_secs &#45; staking_rewards_config.last_rewards_rate_period_start_in_secs &lt; staking_rewards_config.rewards_rate_period_in_secs) &#123;<br/>        return &#42;staking_rewards_config<br/>    &#125;;<br/>    // Rewards rate decrease rate cannot be greater than 100%. Otherwise rewards rate will be negative.<br/>    assert!(<br/>        fixed_point64::ceil(staking_rewards_config.rewards_rate_decrease_rate) &lt;&#61; 1,<br/>        error::invalid_argument(EINVALID_REWARDS_RATE_DECREASE_RATE)<br/>    );<br/>    let new_rate &#61; math_fixed64::mul_div(<br/>        staking_rewards_config.rewards_rate,<br/>        fixed_point64::sub(<br/>            fixed_point64::create_from_u128(1),<br/>            staking_rewards_config.rewards_rate_decrease_rate,<br/>        ),<br/>        fixed_point64::create_from_u128(1),<br/>    );<br/>    new_rate &#61; fixed_point64::max(new_rate, staking_rewards_config.min_rewards_rate);<br/><br/>    staking_rewards_config.rewards_rate &#61; new_rate;<br/>    staking_rewards_config.last_rewards_rate_period_start_in_secs &#61;<br/>        staking_rewards_config.last_rewards_rate_period_start_in_secs &#43;<br/>        staking_rewards_config.rewards_rate_period_in_secs;<br/>    return &#42;staking_rewards_config<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_update_required_stake"></a>

## Function `update_required_stake`

Update the min and max stake amounts.<br/> Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun update_required_stake(aptos_framework: &amp;signer, minimum_stake: u64, maximum_stake: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_required_stake(<br/>    aptos_framework: &amp;signer,<br/>    minimum_stake: u64,<br/>    maximum_stake: u64,<br/>) acquires StakingConfig &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    validate_required_stake(minimum_stake, maximum_stake);<br/><br/>    let staking_config &#61; borrow_global_mut&lt;StakingConfig&gt;(@aptos_framework);<br/>    staking_config.minimum_stake &#61; minimum_stake;<br/>    staking_config.maximum_stake &#61; maximum_stake;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_update_recurring_lockup_duration_secs"></a>

## Function `update_recurring_lockup_duration_secs`

Update the recurring lockup duration.<br/> Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun update_recurring_lockup_duration_secs(aptos_framework: &amp;signer, new_recurring_lockup_duration_secs: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_recurring_lockup_duration_secs(<br/>    aptos_framework: &amp;signer,<br/>    new_recurring_lockup_duration_secs: u64,<br/>) acquires StakingConfig &#123;<br/>    assert!(new_recurring_lockup_duration_secs &gt; 0, error::invalid_argument(EZERO_LOCKUP_DURATION));<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    let staking_config &#61; borrow_global_mut&lt;StakingConfig&gt;(@aptos_framework);<br/>    staking_config.recurring_lockup_duration_secs &#61; new_recurring_lockup_duration_secs;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_update_rewards_rate"></a>

## Function `update_rewards_rate`

DEPRECATING<br/> Update the rewards rate.<br/> Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun update_rewards_rate(aptos_framework: &amp;signer, new_rewards_rate: u64, new_rewards_rate_denominator: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_rewards_rate(<br/>    aptos_framework: &amp;signer,<br/>    new_rewards_rate: u64,<br/>    new_rewards_rate_denominator: u64,<br/>) acquires StakingConfig &#123;<br/>    assert!(!features::periodical_reward_rate_decrease_enabled(), error::invalid_state(EDEPRECATED_FUNCTION));<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(<br/>        new_rewards_rate_denominator &gt; 0,<br/>        error::invalid_argument(EZERO_REWARDS_RATE_DENOMINATOR),<br/>    );<br/>    // `rewards_rate` which is the numerator is limited to be `&lt;&#61; MAX_REWARDS_RATE` in order to avoid the arithmetic<br/>    // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted to get the desired rewards<br/>    // rate (i.e., rewards_rate / rewards_rate_denominator).<br/>    assert!(new_rewards_rate &lt;&#61; MAX_REWARDS_RATE, error::invalid_argument(EINVALID_REWARDS_RATE));<br/><br/>    // We assert that (rewards_rate / rewards_rate_denominator &lt;&#61; 1).<br/>    assert!(new_rewards_rate &lt;&#61; new_rewards_rate_denominator, error::invalid_argument(EINVALID_REWARDS_RATE));<br/><br/>    let staking_config &#61; borrow_global_mut&lt;StakingConfig&gt;(@aptos_framework);<br/>    staking_config.rewards_rate &#61; new_rewards_rate;<br/>    staking_config.rewards_rate_denominator &#61; new_rewards_rate_denominator;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_update_rewards_config"></a>

## Function `update_rewards_config`



<pre><code>public fun update_rewards_config(aptos_framework: &amp;signer, rewards_rate: fixed_point64::FixedPoint64, min_rewards_rate: fixed_point64::FixedPoint64, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: fixed_point64::FixedPoint64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_rewards_config(<br/>    aptos_framework: &amp;signer,<br/>    rewards_rate: FixedPoint64,<br/>    min_rewards_rate: FixedPoint64,<br/>    rewards_rate_period_in_secs: u64,<br/>    rewards_rate_decrease_rate: FixedPoint64,<br/>) acquires StakingRewardsConfig &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    validate_rewards_config(<br/>        rewards_rate,<br/>        min_rewards_rate,<br/>        rewards_rate_period_in_secs,<br/>        rewards_rate_decrease_rate,<br/>    );<br/><br/>    let staking_rewards_config &#61; borrow_global_mut&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/>    // Currently rewards_rate_period_in_secs is not allowed to be changed because this could bring complicated<br/>    // logics. At the moment the argument is just a placeholder for future use.<br/>    assert!(<br/>        rewards_rate_period_in_secs &#61;&#61; staking_rewards_config.rewards_rate_period_in_secs,<br/>        error::invalid_argument(EINVALID_REWARDS_RATE_PERIOD),<br/>    );<br/>    staking_rewards_config.rewards_rate &#61; rewards_rate;<br/>    staking_rewards_config.min_rewards_rate &#61; min_rewards_rate;<br/>    staking_rewards_config.rewards_rate_period_in_secs &#61; rewards_rate_period_in_secs;<br/>    staking_rewards_config.rewards_rate_decrease_rate &#61; rewards_rate_decrease_rate;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_update_voting_power_increase_limit"></a>

## Function `update_voting_power_increase_limit`

Update the joining limit %.<br/> Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.


<pre><code>public fun update_voting_power_increase_limit(aptos_framework: &amp;signer, new_voting_power_increase_limit: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_voting_power_increase_limit(<br/>    aptos_framework: &amp;signer,<br/>    new_voting_power_increase_limit: u64,<br/>) acquires StakingConfig &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(<br/>        new_voting_power_increase_limit &gt; 0 &amp;&amp; new_voting_power_increase_limit &lt;&#61; 50,<br/>        error::invalid_argument(EINVALID_VOTING_POWER_INCREASE_LIMIT),<br/>    );<br/><br/>    let staking_config &#61; borrow_global_mut&lt;StakingConfig&gt;(@aptos_framework);<br/>    staking_config.voting_power_increase_limit &#61; new_voting_power_increase_limit;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_validate_required_stake"></a>

## Function `validate_required_stake`



<pre><code>fun validate_required_stake(minimum_stake: u64, maximum_stake: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_required_stake(minimum_stake: u64, maximum_stake: u64) &#123;<br/>    assert!(minimum_stake &lt;&#61; maximum_stake &amp;&amp; maximum_stake &gt; 0, error::invalid_argument(EINVALID_STAKE_RANGE));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_config_validate_rewards_config"></a>

## Function `validate_rewards_config`



<pre><code>fun validate_rewards_config(rewards_rate: fixed_point64::FixedPoint64, min_rewards_rate: fixed_point64::FixedPoint64, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: fixed_point64::FixedPoint64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_rewards_config(<br/>    rewards_rate: FixedPoint64,<br/>    min_rewards_rate: FixedPoint64,<br/>    rewards_rate_period_in_secs: u64,<br/>    rewards_rate_decrease_rate: FixedPoint64,<br/>) &#123;<br/>    // Bound rewards rate to avoid arithmetic overflow.<br/>    assert!(<br/>        less_or_equal(rewards_rate, fixed_point64::create_from_u128((1u128))),<br/>        error::invalid_argument(EINVALID_REWARDS_RATE)<br/>    );<br/>    assert!(<br/>        less_or_equal(min_rewards_rate, rewards_rate),<br/>        error::invalid_argument(EINVALID_MIN_REWARDS_RATE)<br/>    );<br/>    // Rewards rate decrease rate cannot be greater than 100%. Otherwise rewards rate will be negative.<br/>    assert!(<br/>        fixed_point64::ceil(rewards_rate_decrease_rate) &lt;&#61; 1,<br/>        error::invalid_argument(EINVALID_REWARDS_RATE_DECREASE_RATE)<br/>    );<br/>    // This field, rewards_rate_period_in_secs must be greater than 0.<br/>    // TODO: rewards_rate_period_in_secs should be longer than the epoch duration but reading epoch duration causes a circular dependency.<br/>    assert!(<br/>        rewards_rate_period_in_secs &gt; 0,<br/>        error::invalid_argument(EINVALID_REWARDS_RATE_PERIOD),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The ability to initialize the staking config and staking rewards resources, as well as the ability to update the staking config and staking rewards should only be available to the Aptos framework account.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The function initialize and initialize_rewards are used to initialize the StakingConfig and StakingRewardConfig resources. Updating the resources, can be done using the update_required_stake, update_recurring_lockup_duration_secs, update_rewards_rate, update_rewards_config, update_voting_power_increase_limit functions, which ensure that the signer is aptos_framework using the assert_aptos_framework function.&lt;/td&gt;<br/>&lt;td&gt;Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.1&quot;&gt;initialize&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.2&quot;&gt;initialize_rewards&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.3&quot;&gt;update_required_stake&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.4&quot;&gt;update_recurring_lockup_duration_secs&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.5&quot;&gt;update_rewards_rate&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.6&quot;&gt;update_rewards_config&lt;/a&gt;, and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.7&quot;&gt;update_voting_power_increase_limit&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The voting power increase, in a staking config resource, should always be greater than 0 and less or equal to 50.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;During the initialization and update of the staking config, the value of voting_power_increase_limit is ensured to be in the range of (0 to 50].&lt;/td&gt;<br/>&lt;td&gt;Ensured via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.1&quot;&gt;initialize&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.2&quot;&gt;update_voting_power_increase_limit&lt;/a&gt;. Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.3&quot;&gt;StakingConfig&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The recurring lockup duration, in a staking config resource, should always be greater than 0.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;During the initialization and update of the staking config, the value of recurring_lockup_duration_secs is ensured to be greater than 0.&lt;/td&gt;<br/>&lt;td&gt;Ensured via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3.1&quot;&gt;initialize&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3.2&quot;&gt;update_recurring_lockup_duration_secs&lt;/a&gt;. Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3.3&quot;&gt;StakingConfig&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The calculation of rewards should not be possible if the last reward rate period just started.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The function calculate_and_save_latest_rewards_config ensures that last_rewards_rate_period_start_in_secs is greater or equal to the current timestamp.&lt;/td&gt;<br/>&lt;td&gt;Formally verified in &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;StakingRewardsConfigEnabledRequirement&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;The rewards rate should always be less than or equal to 100%.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;When initializing and updating the rewards rate, it is ensured that the rewards_rate is less or equal to MAX_REWARDS_RATE, otherwise rewards rate will be negative.&lt;/td&gt;<br/>&lt;td&gt;Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5&quot;&gt;StakingConfig&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;The reward rate&apos;s denominator should never be 0.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;While initializing and updating the rewards rate, rewards_rate_denominator is ensured to be greater than 0.&lt;/td&gt;<br/>&lt;td&gt;Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;6&quot;&gt;StakingConfig&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;The reward rate&apos;s nominator and dominator ratio should always be less or equal to 1.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;When initializing and updating the rewards rate, it is ensured that rewards_rate is less or equal to rewards_rate_denominator.&lt;/td&gt;<br/>&lt;td&gt;Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7&quot;&gt;StakingConfig&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;StakingConfig&gt;(@aptos_framework);<br/>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_StakingConfig"></a>

### Resource `StakingConfig`


<pre><code>struct StakingConfig has copy, drop, key<br/></code></pre>



<dl>
<dt>
<code>minimum_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>recurring_lockup_duration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>allow_validator_set_change: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_power_increase_limit: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
invariant rewards_rate &lt;&#61; MAX_REWARDS_RATE;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;6&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 6&lt;/a&gt;:
invariant rewards_rate_denominator &gt; 0;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
invariant rewards_rate &lt;&#61; rewards_rate_denominator;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
invariant recurring_lockup_duration_secs &gt; 0;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
invariant voting_power_increase_limit &gt; 0 &amp;&amp; voting_power_increase_limit &lt;&#61; 50;<br/></code></pre>



<a id="@Specification_1_StakingRewardsConfig"></a>

### Resource `StakingRewardsConfig`


<pre><code>struct StakingRewardsConfig has copy, drop, key<br/></code></pre>



<dl>
<dt>
<code>rewards_rate: fixed_point64::FixedPoint64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_rewards_rate: fixed_point64::FixedPoint64</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate_period_in_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_rewards_rate_period_start_in_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>rewards_rate_decrease_rate: fixed_point64::FixedPoint64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant fixed_point64::spec_less_or_equal(<br/>    rewards_rate,<br/>    fixed_point64::spec_create_from_u128((1u128)));<br/>invariant fixed_point64::spec_less_or_equal(min_rewards_rate, rewards_rate);<br/>invariant rewards_rate_period_in_secs &gt; 0;<br/>invariant fixed_point64::spec_ceil(rewards_rate_decrease_rate) &lt;&#61; 1;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)<br/></code></pre>


Caller must be @aptos_framework.<br/> The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.<br/> The rewards_rate_denominator must greater than zero.<br/> Only this %0&#45;%50 of current total voting power is allowed to join the validator set in each epoch.<br/> The <code>rewards_rate</code> which is the numerator is limited to be <code>&lt;&#61; MAX_REWARDS_RATE</code> in order to avoid the arithmetic overflow in the rewards calculation.<br/> rewards_rate/rewards_rate_denominator &lt;&#61; 1.<br/> StakingConfig does not exist under the aptos_framework before creating it.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>aborts_if minimum_stake &gt; maximum_stake &#124;&#124; maximum_stake &#61;&#61; 0;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if recurring_lockup_duration_secs &#61;&#61; 0;<br/>aborts_if rewards_rate_denominator &#61;&#61; 0;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if voting_power_increase_limit &#61;&#61; 0 &#124;&#124; voting_power_increase_limit &gt; 50;<br/>aborts_if rewards_rate &gt; MAX_REWARDS_RATE;<br/>aborts_if rewards_rate &gt; rewards_rate_denominator;<br/>aborts_if exists&lt;StakingConfig&gt;(addr);<br/>ensures exists&lt;StakingConfig&gt;(addr);<br/></code></pre>



<a id="@Specification_1_initialize_rewards"></a>

### Function `initialize_rewards`


<pre><code>public fun initialize_rewards(aptos_framework: &amp;signer, rewards_rate: fixed_point64::FixedPoint64, min_rewards_rate: fixed_point64::FixedPoint64, rewards_rate_period_in_secs: u64, last_rewards_rate_period_start_in_secs: u64, rewards_rate_decrease_rate: fixed_point64::FixedPoint64)<br/></code></pre>


Caller must be @aptos_framework.<br/> last_rewards_rate_period_start_in_secs cannot be later than now.<br/> Abort at any condition in StakingRewardsConfigValidationAborts.<br/> StakingRewardsConfig does not exist under the aptos_framework before creating it.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>aborts_if last_rewards_rate_period_start_in_secs &gt; timestamp::spec_now_seconds();<br/>include StakingRewardsConfigValidationAbortsIf;<br/>aborts_if exists&lt;StakingRewardsConfig&gt;(addr);<br/>ensures exists&lt;StakingRewardsConfig&gt;(addr);<br/></code></pre>



<a id="@Specification_1_get"></a>

### Function `get`


<pre><code>public fun get(): staking_config::StakingConfig<br/></code></pre>




<pre><code>aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_get_reward_rate"></a>

### Function `get_reward_rate`


<pre><code>public fun get_reward_rate(config: &amp;staking_config::StakingConfig): (u64, u64)<br/></code></pre>




<pre><code>include StakingRewardsConfigRequirement;<br/>ensures (features::spec_periodical_reward_rate_decrease_enabled() &amp;&amp;<br/>    (global&lt;StakingRewardsConfig&gt;(@aptos_framework).rewards_rate.value as u64) !&#61; 0) &#61;&#61;&gt;<br/>        result_1 &lt;&#61; MAX_REWARDS_RATE &amp;&amp; result_2 &lt;&#61; MAX_U64;<br/></code></pre>



<a id="@Specification_1_calculate_and_save_latest_epoch_rewards_rate"></a>

### Function `calculate_and_save_latest_epoch_rewards_rate`


<pre><code>public(friend) fun calculate_and_save_latest_epoch_rewards_rate(): fixed_point64::FixedPoint64<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>aborts_if !exists&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/>aborts_if !features::spec_periodical_reward_rate_decrease_enabled();<br/>include StakingRewardsConfigRequirement;<br/></code></pre>



<a id="@Specification_1_calculate_and_save_latest_rewards_config"></a>

### Function `calculate_and_save_latest_rewards_config`


<pre><code>fun calculate_and_save_latest_rewards_config(): staking_config::StakingRewardsConfig<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>requires features::spec_periodical_reward_rate_decrease_enabled();<br/>include StakingRewardsConfigRequirement;<br/>aborts_if !exists&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_update_required_stake"></a>

### Function `update_required_stake`


<pre><code>public fun update_required_stake(aptos_framework: &amp;signer, minimum_stake: u64, maximum_stake: u64)<br/></code></pre>


Caller must be @aptos_framework.<br/> The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.<br/> The StakingConfig is under @aptos_framework.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>aborts_if minimum_stake &gt; maximum_stake &#124;&#124; maximum_stake &#61;&#61; 0;<br/>aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);<br/>ensures global&lt;StakingConfig&gt;(@aptos_framework).minimum_stake &#61;&#61; minimum_stake &amp;&amp;<br/>    global&lt;StakingConfig&gt;(@aptos_framework).maximum_stake &#61;&#61; maximum_stake;<br/></code></pre>



<a id="@Specification_1_update_recurring_lockup_duration_secs"></a>

### Function `update_recurring_lockup_duration_secs`


<pre><code>public fun update_recurring_lockup_duration_secs(aptos_framework: &amp;signer, new_recurring_lockup_duration_secs: u64)<br/></code></pre>


Caller must be @aptos_framework.<br/> The new_recurring_lockup_duration_secs must greater than zero.<br/> The StakingConfig is under @aptos_framework.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if new_recurring_lockup_duration_secs &#61;&#61; 0;<br/>aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);<br/>ensures global&lt;StakingConfig&gt;(@aptos_framework).recurring_lockup_duration_secs &#61;&#61; new_recurring_lockup_duration_secs;<br/></code></pre>



<a id="@Specification_1_update_rewards_rate"></a>

### Function `update_rewards_rate`


<pre><code>public fun update_rewards_rate(aptos_framework: &amp;signer, new_rewards_rate: u64, new_rewards_rate_denominator: u64)<br/></code></pre>


Caller must be @aptos_framework.<br/> The new_rewards_rate_denominator must greater than zero.<br/> The StakingConfig is under @aptos_framework.<br/> The <code>rewards_rate</code> which is the numerator is limited to be <code>&lt;&#61; MAX_REWARDS_RATE</code> in order to avoid the arithmetic overflow in the rewards calculation.<br/> rewards_rate/rewards_rate_denominator &lt;&#61; 1.


<pre><code>aborts_if features::spec_periodical_reward_rate_decrease_enabled();<br/>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>aborts_if new_rewards_rate_denominator &#61;&#61; 0;<br/>aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);<br/>aborts_if new_rewards_rate &gt; MAX_REWARDS_RATE;<br/>aborts_if new_rewards_rate &gt; new_rewards_rate_denominator;<br/>let post staking_config &#61; global&lt;StakingConfig&gt;(@aptos_framework);<br/>ensures staking_config.rewards_rate &#61;&#61; new_rewards_rate;<br/>ensures staking_config.rewards_rate_denominator &#61;&#61; new_rewards_rate_denominator;<br/></code></pre>



<a id="@Specification_1_update_rewards_config"></a>

### Function `update_rewards_config`


<pre><code>public fun update_rewards_config(aptos_framework: &amp;signer, rewards_rate: fixed_point64::FixedPoint64, min_rewards_rate: fixed_point64::FixedPoint64, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: fixed_point64::FixedPoint64)<br/></code></pre>


Caller must be @aptos_framework.<br/> StakingRewardsConfig is under the @aptos_framework.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include StakingRewardsConfigRequirement;<br/>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.6&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>aborts_if global&lt;StakingRewardsConfig&gt;(@aptos_framework).rewards_rate_period_in_secs !&#61; rewards_rate_period_in_secs;<br/>include StakingRewardsConfigValidationAbortsIf;<br/>aborts_if !exists&lt;StakingRewardsConfig&gt;(addr);<br/>let post staking_rewards_config &#61; global&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/>ensures staking_rewards_config.rewards_rate &#61;&#61; rewards_rate;<br/>ensures staking_rewards_config.min_rewards_rate &#61;&#61; min_rewards_rate;<br/>ensures staking_rewards_config.rewards_rate_period_in_secs &#61;&#61; rewards_rate_period_in_secs;<br/>ensures staking_rewards_config.rewards_rate_decrease_rate &#61;&#61; rewards_rate_decrease_rate;<br/></code></pre>



<a id="@Specification_1_update_voting_power_increase_limit"></a>

### Function `update_voting_power_increase_limit`


<pre><code>public fun update_voting_power_increase_limit(aptos_framework: &amp;signer, new_voting_power_increase_limit: u64)<br/></code></pre>


Caller must be @aptos_framework.<br/> Only this %0&#45;%50 of current total voting power is allowed to join the validator set in each epoch.<br/> The StakingConfig is under @aptos_framework.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.7&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if addr !&#61; @aptos_framework;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if new_voting_power_increase_limit &#61;&#61; 0 &#124;&#124; new_voting_power_increase_limit &gt; 50;<br/>aborts_if !exists&lt;StakingConfig&gt;(@aptos_framework);<br/>ensures global&lt;StakingConfig&gt;(@aptos_framework).voting_power_increase_limit &#61;&#61; new_voting_power_increase_limit;<br/></code></pre>



<a id="@Specification_1_validate_required_stake"></a>

### Function `validate_required_stake`


<pre><code>fun validate_required_stake(minimum_stake: u64, maximum_stake: u64)<br/></code></pre>


The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.


<pre><code>aborts_if minimum_stake &gt; maximum_stake &#124;&#124; maximum_stake &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_validate_rewards_config"></a>

### Function `validate_rewards_config`


<pre><code>fun validate_rewards_config(rewards_rate: fixed_point64::FixedPoint64, min_rewards_rate: fixed_point64::FixedPoint64, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: fixed_point64::FixedPoint64)<br/></code></pre>


Abort at any condition in StakingRewardsConfigValidationAborts.


<pre><code>include StakingRewardsConfigValidationAbortsIf;<br/></code></pre>


rewards_rate must be within [0, 1].<br/> min_rewards_rate must be not greater than rewards_rate.<br/> rewards_rate_period_in_secs must be greater than 0.<br/> rewards_rate_decrease_rate must be within [0,1].


<a id="0x1_staking_config_StakingRewardsConfigValidationAbortsIf"></a>


<pre><code>schema StakingRewardsConfigValidationAbortsIf &#123;<br/>rewards_rate: FixedPoint64;<br/>min_rewards_rate: FixedPoint64;<br/>rewards_rate_period_in_secs: u64;<br/>rewards_rate_decrease_rate: FixedPoint64;<br/>aborts_if fixed_point64::spec_greater(<br/>    rewards_rate,<br/>    fixed_point64::spec_create_from_u128((1u128)));<br/>aborts_if fixed_point64::spec_greater(min_rewards_rate, rewards_rate);<br/>aborts_if rewards_rate_period_in_secs &#61;&#61; 0;<br/>aborts_if fixed_point64::spec_ceil(rewards_rate_decrease_rate) &gt; 1;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_config_StakingRewardsConfigRequirement"></a>


<pre><code>schema StakingRewardsConfigRequirement &#123;<br/>requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>include features::spec_periodical_reward_rate_decrease_enabled() &#61;&#61;&gt; StakingRewardsConfigEnabledRequirement;<br/>&#125;<br/></code></pre>




<a id="0x1_staking_config_StakingRewardsConfigEnabledRequirement"></a>


<pre><code>schema StakingRewardsConfigEnabledRequirement &#123;<br/>requires exists&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/>let staking_rewards_config &#61; global&lt;StakingRewardsConfig&gt;(@aptos_framework);<br/>let rewards_rate &#61; staking_rewards_config.rewards_rate;<br/>let min_rewards_rate &#61; staking_rewards_config.min_rewards_rate;<br/>let rewards_rate_period_in_secs &#61; staking_rewards_config.rewards_rate_period_in_secs;<br/>let last_rewards_rate_period_start_in_secs &#61; staking_rewards_config.last_rewards_rate_period_start_in_secs;<br/>let rewards_rate_decrease_rate &#61; staking_rewards_config.rewards_rate_decrease_rate;<br/>requires fixed_point64::spec_less_or_equal(<br/>    rewards_rate,<br/>    fixed_point64::spec_create_from_u128((1u128)));<br/>requires fixed_point64::spec_less_or_equal(min_rewards_rate, rewards_rate);<br/>requires rewards_rate_period_in_secs &gt; 0;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
    requires last_rewards_rate_period_start_in_secs &lt;&#61; timestamp::spec_now_seconds();<br/>requires fixed_point64::spec_ceil(rewards_rate_decrease_rate) &lt;&#61; 1;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
