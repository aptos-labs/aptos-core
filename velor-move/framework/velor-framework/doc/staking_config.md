
<a id="0x1_staking_config"></a>

# Module `0x1::staking_config`

Provides the configuration for staking and rewards


-  [Resource `StakingConfig`](#0x1_staking_config_StakingConfig)
-  [Resource `StakingRewardsConfig`](#0x1_staking_config_StakingRewardsConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_staking_config_initialize)
-  [Function `reward_rate`](#0x1_staking_config_reward_rate)
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
    -  [Function `reward_rate`](#@Specification_1_reward_rate)
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


<pre><code><b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="../../velor-stdlib/doc/math_fixed64.md#0x1_math_fixed64">0x1::math_fixed64</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_staking_config_StakingConfig"></a>

## Resource `StakingConfig`

Validator set configurations that will be stored with the @velor_framework account.


<pre><code><b>struct</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



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

Staking reward configurations that will be stored with the @velor_framework account.


<pre><code><b>struct</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>

</dd>
<dt>
<code>min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
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
<code>rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_staking_config_MAX_U64"></a>



<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_staking_config_EDEPRECATED_FUNCTION"></a>

The function has been deprecated.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>: u64 = 10;
</code></pre>



<a id="0x1_staking_config_BPS_DENOMINATOR"></a>

Denominator of number in basis points. 1 bps(basis points) = 0.01%.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_BPS_DENOMINATOR">BPS_DENOMINATOR</a>: u64 = 10000;
</code></pre>



<a id="0x1_staking_config_EDISABLED_FUNCTION"></a>

The function is disabled or hasn't been enabled.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EDISABLED_FUNCTION">EDISABLED_FUNCTION</a>: u64 = 11;
</code></pre>



<a id="0x1_staking_config_EINVALID_LAST_REWARDS_RATE_PERIOD_START"></a>

Specified start time of last rewards rate period is invalid, which must be not late than the current timestamp.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_LAST_REWARDS_RATE_PERIOD_START">EINVALID_LAST_REWARDS_RATE_PERIOD_START</a>: u64 = 7;
</code></pre>



<a id="0x1_staking_config_EINVALID_MIN_REWARDS_RATE"></a>

Specified min rewards rate is invalid, which must be within [0, rewards_rate].


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_MIN_REWARDS_RATE">EINVALID_MIN_REWARDS_RATE</a>: u64 = 6;
</code></pre>



<a id="0x1_staking_config_EINVALID_REWARDS_RATE"></a>

Specified rewards rate is invalid, which must be within [0, MAX_REWARDS_RATE].


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE">EINVALID_REWARDS_RATE</a>: u64 = 5;
</code></pre>



<a id="0x1_staking_config_EINVALID_REWARDS_RATE_DECREASE_RATE"></a>

Specified rewards rate decrease rate is invalid, which must be not greater than BPS_DENOMINATOR.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE_DECREASE_RATE">EINVALID_REWARDS_RATE_DECREASE_RATE</a>: u64 = 8;
</code></pre>



<a id="0x1_staking_config_EINVALID_REWARDS_RATE_PERIOD"></a>

Specified rewards rate period is invalid. It must be larger than 0 and cannot be changed if configured.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE_PERIOD">EINVALID_REWARDS_RATE_PERIOD</a>: u64 = 9;
</code></pre>



<a id="0x1_staking_config_EINVALID_STAKE_RANGE"></a>

Specified stake range is invalid. Max must be greater than min.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_STAKE_RANGE">EINVALID_STAKE_RANGE</a>: u64 = 3;
</code></pre>



<a id="0x1_staking_config_EINVALID_VOTING_POWER_INCREASE_LIMIT"></a>

The voting power increase limit percentage must be within (0, 50].


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EINVALID_VOTING_POWER_INCREASE_LIMIT">EINVALID_VOTING_POWER_INCREASE_LIMIT</a>: u64 = 4;
</code></pre>



<a id="0x1_staking_config_EZERO_LOCKUP_DURATION"></a>

Stake lockup duration cannot be zero.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EZERO_LOCKUP_DURATION">EZERO_LOCKUP_DURATION</a>: u64 = 1;
</code></pre>



<a id="0x1_staking_config_EZERO_REWARDS_RATE_DENOMINATOR"></a>

Reward rate denominator cannot be zero.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_EZERO_REWARDS_RATE_DENOMINATOR">EZERO_REWARDS_RATE_DENOMINATOR</a>: u64 = 2;
</code></pre>



<a id="0x1_staking_config_MAX_REWARDS_RATE"></a>

Limit the maximum value of <code>rewards_rate</code> in order to avoid any arithmetic overflow.


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>: u64 = 1000000;
</code></pre>



<a id="0x1_staking_config_ONE_YEAR_IN_SECS"></a>

1 year => 365 * 24 * 60 * 60


<pre><code><b>const</b> <a href="staking_config.md#0x1_staking_config_ONE_YEAR_IN_SECS">ONE_YEAR_IN_SECS</a>: u64 = 31536000;
</code></pre>



<a id="0x1_staking_config_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="staking_config.md#0x1_staking_config_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="staking_config.md#0x1_staking_config_initialize">initialize</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    minimum_stake: u64,
    maximum_stake: u64,
    recurring_lockup_duration_secs: u64,
    allow_validator_set_change: bool,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
    voting_power_increase_limit: u64,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);

    // This can fail <a href="genesis.md#0x1_genesis">genesis</a> but is necessary so that <a href="../../velor-stdlib/doc/any.md#0x1_any">any</a> misconfigurations can be corrected before <a href="genesis.md#0x1_genesis">genesis</a> succeeds
    <a href="staking_config.md#0x1_staking_config_validate_required_stake">validate_required_stake</a>(minimum_stake, maximum_stake);

    <b>assert</b>!(recurring_lockup_duration_secs &gt; 0, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EZERO_LOCKUP_DURATION">EZERO_LOCKUP_DURATION</a>));
    <b>assert</b>!(
        rewards_rate_denominator &gt; 0,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EZERO_REWARDS_RATE_DENOMINATOR">EZERO_REWARDS_RATE_DENOMINATOR</a>),
    );
    <b>assert</b>!(
        voting_power_increase_limit &gt; 0 && voting_power_increase_limit &lt;= 50,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_VOTING_POWER_INCREASE_LIMIT">EINVALID_VOTING_POWER_INCREASE_LIMIT</a>),
    );

    // `rewards_rate` which is the numerator is limited <b>to</b> be `&lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>` in order <b>to</b> avoid the arithmetic
    // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted <b>to</b> get the desired rewards
    // rate (i.e., rewards_rate / rewards_rate_denominator).
    <b>assert</b>!(rewards_rate &lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE">EINVALID_REWARDS_RATE</a>));

    // We <b>assert</b> that (rewards_rate / rewards_rate_denominator &lt;= 1).
    <b>assert</b>!(rewards_rate &lt;= rewards_rate_denominator, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE">EINVALID_REWARDS_RATE</a>));

    <b>move_to</b>(velor_framework, <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
        minimum_stake,
        maximum_stake,
        recurring_lockup_duration_secs,
        allow_validator_set_change,
        rewards_rate,
        rewards_rate_denominator,
        voting_power_increase_limit,
    });

    // Initialize <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> <b>with</b> the given rewards_rate and rewards_rate_denominator,
    // <b>while</b> setting min_rewards_rate and rewards_rate_decrease_rate <b>to</b> 0.
    <a href="staking_config.md#0x1_staking_config_initialize_rewards">initialize_rewards</a>(
        velor_framework,
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_rational">fixed_point64::create_from_rational</a>((rewards_rate <b>as</b> u128), (rewards_rate_denominator <b>as</b> u128)),
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_rational">fixed_point64::create_from_rational</a>(0, 1000),
        <a href="staking_config.md#0x1_staking_config_ONE_YEAR_IN_SECS">ONE_YEAR_IN_SECS</a>,
        0,
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_rational">fixed_point64::create_from_rational</a>(0, 1000),
    );
}
</code></pre>



</details>

<a id="0x1_staking_config_reward_rate"></a>

## Function `reward_rate`

Return the reward rate of this epoch as a tuple (numerator, denominator).


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_reward_rate">reward_rate</a>(): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_reward_rate">reward_rate</a>(): (u64, u64) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>, <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
    <a href="staking_config.md#0x1_staking_config_get_reward_rate">get_reward_rate</a>(<b>borrow_global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework))
}
</code></pre>



</details>

<a id="0x1_staking_config_initialize_rewards"></a>

## Function `initialize_rewards`

Initialize rewards configurations.
Can only be called as part of the Velor governance proposal process established by the VelorGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_initialize_rewards">initialize_rewards</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, rewards_rate_period_in_secs: u64, last_rewards_rate_period_start_in_secs: u64, rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_initialize_rewards">initialize_rewards</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    rewards_rate: FixedPoint64,
    min_rewards_rate: FixedPoint64,
    rewards_rate_period_in_secs: u64,
    last_rewards_rate_period_start_in_secs: u64,
    rewards_rate_decrease_rate: FixedPoint64,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);

    <a href="staking_config.md#0x1_staking_config_validate_rewards_config">validate_rewards_config</a>(
        rewards_rate,
        min_rewards_rate,
        rewards_rate_period_in_secs,
        rewards_rate_decrease_rate,
    );
    <b>assert</b>!(
        <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt;= last_rewards_rate_period_start_in_secs,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_LAST_REWARDS_RATE_PERIOD_START">EINVALID_LAST_REWARDS_RATE_PERIOD_START</a>)
    );

    <b>move_to</b>(velor_framework, <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> {
        rewards_rate,
        min_rewards_rate,
        rewards_rate_period_in_secs,
        last_rewards_rate_period_start_in_secs,
        rewards_rate_decrease_rate,
    });
}
</code></pre>



</details>

<a id="0x1_staking_config_get"></a>

## Function `get`



<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get">get</a>(): <a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get">get</a>(): <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
    *<b>borrow_global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework)
}
</code></pre>



</details>

<a id="0x1_staking_config_get_allow_validator_set_change"></a>

## Function `get_allow_validator_set_change`

Return whether validator set changes are allowed


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">get_allow_validator_set_change</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">get_allow_validator_set_change</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>): bool {
    config.allow_validator_set_change
}
</code></pre>



</details>

<a id="0x1_staking_config_get_required_stake"></a>

## Function `get_required_stake`

Return the required min/max stake.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_required_stake">get_required_stake</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_required_stake">get_required_stake</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>): (u64, u64) {
    (config.minimum_stake, config.maximum_stake)
}
</code></pre>



</details>

<a id="0x1_staking_config_get_recurring_lockup_duration"></a>

## Function `get_recurring_lockup_duration`

Return the recurring lockup duration that every validator is automatically renewed for (unless they unlock and
withdraw all funds).


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">get_recurring_lockup_duration</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_recurring_lockup_duration">get_recurring_lockup_duration</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>): u64 {
    config.recurring_lockup_duration_secs
}
</code></pre>



</details>

<a id="0x1_staking_config_get_reward_rate"></a>

## Function `get_reward_rate`

Return the reward rate of this epoch.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_reward_rate">get_reward_rate</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_reward_rate">get_reward_rate</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>): (u64, u64) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> {
    <b>if</b> (<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_periodical_reward_rate_decrease_enabled">features::periodical_reward_rate_decrease_enabled</a>()) {
        <b>let</b> epoch_rewards_rate = <b>borrow_global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework).rewards_rate;
        <b>if</b> (<a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_is_zero">fixed_point64::is_zero</a>(epoch_rewards_rate)) {
            (0u64, 1u64)
        } <b>else</b> {
            // Maximize denominator for higher precision.
            // Restriction: nominator &lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a> && denominator &lt;= <a href="staking_config.md#0x1_staking_config_MAX_U64">MAX_U64</a>
            <b>let</b> denominator = <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_divide_u128">fixed_point64::divide_u128</a>((<a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a> <b>as</b> u128), epoch_rewards_rate);
            <b>if</b> (denominator &gt; <a href="staking_config.md#0x1_staking_config_MAX_U64">MAX_U64</a>) {
                denominator = <a href="staking_config.md#0x1_staking_config_MAX_U64">MAX_U64</a>
            };
            <b>let</b> nominator = (<a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_multiply_u128">fixed_point64::multiply_u128</a>(denominator, epoch_rewards_rate) <b>as</b> u64);
            (nominator, (denominator <b>as</b> u64))
        }
    } <b>else</b> {
        (config.rewards_rate, config.rewards_rate_denominator)
    }
}
</code></pre>



</details>

<a id="0x1_staking_config_get_voting_power_increase_limit"></a>

## Function `get_voting_power_increase_limit`

Return the joining limit %.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_voting_power_increase_limit">get_voting_power_increase_limit</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_voting_power_increase_limit">get_voting_power_increase_limit</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>): u64 {
    config.voting_power_increase_limit
}
</code></pre>



</details>

<a id="0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate"></a>

## Function `calculate_and_save_latest_epoch_rewards_rate`

Calculate and save the latest rewards rate.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate">calculate_and_save_latest_epoch_rewards_rate</a>(): <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate">calculate_and_save_latest_epoch_rewards_rate</a>(): FixedPoint64 <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> {
    <b>assert</b>!(<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_periodical_reward_rate_decrease_enabled">features::periodical_reward_rate_decrease_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="staking_config.md#0x1_staking_config_EDISABLED_FUNCTION">EDISABLED_FUNCTION</a>));
    <b>let</b> staking_rewards_config = <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_rewards_config">calculate_and_save_latest_rewards_config</a>();
    staking_rewards_config.rewards_rate
}
</code></pre>



</details>

<a id="0x1_staking_config_calculate_and_save_latest_rewards_config"></a>

## Function `calculate_and_save_latest_rewards_config`

Calculate and return the up-to-date StakingRewardsConfig.


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_rewards_config">calculate_and_save_latest_rewards_config</a>(): <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_rewards_config">calculate_and_save_latest_rewards_config</a>(): <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> {
    <b>let</b> staking_rewards_config = <b>borrow_global_mut</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
    <b>let</b> current_time_in_secs = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>assert</b>!(
        current_time_in_secs &gt;= staking_rewards_config.last_rewards_rate_period_start_in_secs,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_LAST_REWARDS_RATE_PERIOD_START">EINVALID_LAST_REWARDS_RATE_PERIOD_START</a>)
    );
    <b>if</b> (current_time_in_secs - staking_rewards_config.last_rewards_rate_period_start_in_secs &lt; staking_rewards_config.rewards_rate_period_in_secs) {
        <b>return</b> *staking_rewards_config
    };
    // Rewards rate decrease rate cannot be greater than 100%. Otherwise rewards rate will be negative.
    <b>assert</b>!(
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_ceil">fixed_point64::ceil</a>(staking_rewards_config.rewards_rate_decrease_rate) &lt;= 1,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE_DECREASE_RATE">EINVALID_REWARDS_RATE_DECREASE_RATE</a>)
    );
    <b>let</b> new_rate = <a href="../../velor-stdlib/doc/math_fixed64.md#0x1_math_fixed64_mul_div">math_fixed64::mul_div</a>(
        staking_rewards_config.rewards_rate,
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_sub">fixed_point64::sub</a>(
            <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_u128">fixed_point64::create_from_u128</a>(1),
            staking_rewards_config.rewards_rate_decrease_rate,
        ),
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_u128">fixed_point64::create_from_u128</a>(1),
    );
    new_rate = <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_max">fixed_point64::max</a>(new_rate, staking_rewards_config.min_rewards_rate);

    staking_rewards_config.rewards_rate = new_rate;
    staking_rewards_config.last_rewards_rate_period_start_in_secs =
        staking_rewards_config.last_rewards_rate_period_start_in_secs +
        staking_rewards_config.rewards_rate_period_in_secs;
    <b>return</b> *staking_rewards_config
}
</code></pre>



</details>

<a id="0x1_staking_config_update_required_stake"></a>

## Function `update_required_stake`

Update the min and max stake amounts.
Can only be called as part of the Velor governance proposal process established by the VelorGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_required_stake">update_required_stake</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, minimum_stake: u64, maximum_stake: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_required_stake">update_required_stake</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    minimum_stake: u64,
    maximum_stake: u64,
) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <a href="staking_config.md#0x1_staking_config_validate_required_stake">validate_required_stake</a>(minimum_stake, maximum_stake);

    <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <b>borrow_global_mut</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
    <a href="staking_config.md#0x1_staking_config">staking_config</a>.minimum_stake = minimum_stake;
    <a href="staking_config.md#0x1_staking_config">staking_config</a>.maximum_stake = maximum_stake;
}
</code></pre>



</details>

<a id="0x1_staking_config_update_recurring_lockup_duration_secs"></a>

## Function `update_recurring_lockup_duration_secs`

Update the recurring lockup duration.
Can only be called as part of the Velor governance proposal process established by the VelorGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_recurring_lockup_duration_secs">update_recurring_lockup_duration_secs</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_recurring_lockup_duration_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_recurring_lockup_duration_secs">update_recurring_lockup_duration_secs</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_recurring_lockup_duration_secs: u64,
) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
    <b>assert</b>!(new_recurring_lockup_duration_secs &gt; 0, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EZERO_LOCKUP_DURATION">EZERO_LOCKUP_DURATION</a>));
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);

    <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <b>borrow_global_mut</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
    <a href="staking_config.md#0x1_staking_config">staking_config</a>.recurring_lockup_duration_secs = new_recurring_lockup_duration_secs;
}
</code></pre>



</details>

<a id="0x1_staking_config_update_rewards_rate"></a>

## Function `update_rewards_rate`

DEPRECATING
Update the rewards rate.
Can only be called as part of the Velor governance proposal process established by the VelorGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_rewards_rate">update_rewards_rate</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_rewards_rate: u64, new_rewards_rate_denominator: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_rewards_rate">update_rewards_rate</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_rewards_rate: u64,
    new_rewards_rate_denominator: u64,
) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
    <b>assert</b>!(!<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_periodical_reward_rate_decrease_enabled">features::periodical_reward_rate_decrease_enabled</a>(), <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="staking_config.md#0x1_staking_config_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>));
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>assert</b>!(
        new_rewards_rate_denominator &gt; 0,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EZERO_REWARDS_RATE_DENOMINATOR">EZERO_REWARDS_RATE_DENOMINATOR</a>),
    );
    // `rewards_rate` which is the numerator is limited <b>to</b> be `&lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>` in order <b>to</b> avoid the arithmetic
    // overflow in the rewards calculation. `rewards_rate_denominator` can be adjusted <b>to</b> get the desired rewards
    // rate (i.e., rewards_rate / rewards_rate_denominator).
    <b>assert</b>!(new_rewards_rate &lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE">EINVALID_REWARDS_RATE</a>));

    // We <b>assert</b> that (rewards_rate / rewards_rate_denominator &lt;= 1).
    <b>assert</b>!(new_rewards_rate &lt;= new_rewards_rate_denominator, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE">EINVALID_REWARDS_RATE</a>));

    <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <b>borrow_global_mut</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
    <a href="staking_config.md#0x1_staking_config">staking_config</a>.rewards_rate = new_rewards_rate;
    <a href="staking_config.md#0x1_staking_config">staking_config</a>.rewards_rate_denominator = new_rewards_rate_denominator;
}
</code></pre>



</details>

<a id="0x1_staking_config_update_rewards_config"></a>

## Function `update_rewards_config`



<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_rewards_config">update_rewards_config</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_rewards_config">update_rewards_config</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    rewards_rate: FixedPoint64,
    min_rewards_rate: FixedPoint64,
    rewards_rate_period_in_secs: u64,
    rewards_rate_decrease_rate: FixedPoint64,
) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);

    <a href="staking_config.md#0x1_staking_config_validate_rewards_config">validate_rewards_config</a>(
        rewards_rate,
        min_rewards_rate,
        rewards_rate_period_in_secs,
        rewards_rate_decrease_rate,
    );

    <b>let</b> staking_rewards_config = <b>borrow_global_mut</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
    // Currently rewards_rate_period_in_secs is not allowed <b>to</b> be changed because this could bring complicated
    // logics. At the moment the argument is just a placeholder for future <b>use</b>.
    <b>assert</b>!(
        rewards_rate_period_in_secs == staking_rewards_config.rewards_rate_period_in_secs,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE_PERIOD">EINVALID_REWARDS_RATE_PERIOD</a>),
    );
    staking_rewards_config.rewards_rate = rewards_rate;
    staking_rewards_config.min_rewards_rate = min_rewards_rate;
    staking_rewards_config.rewards_rate_period_in_secs = rewards_rate_period_in_secs;
    staking_rewards_config.rewards_rate_decrease_rate = rewards_rate_decrease_rate;
}
</code></pre>



</details>

<a id="0x1_staking_config_update_voting_power_increase_limit"></a>

## Function `update_voting_power_increase_limit`

Update the joining limit %.
Can only be called as part of the Velor governance proposal process established by the VelorGovernance module.


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_voting_power_increase_limit">update_voting_power_increase_limit</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voting_power_increase_limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_voting_power_increase_limit">update_voting_power_increase_limit</a>(
    velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_voting_power_increase_limit: u64,
) <b>acquires</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_velor_framework">system_addresses::assert_velor_framework</a>(velor_framework);
    <b>assert</b>!(
        new_voting_power_increase_limit &gt; 0 && new_voting_power_increase_limit &lt;= 50,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_VOTING_POWER_INCREASE_LIMIT">EINVALID_VOTING_POWER_INCREASE_LIMIT</a>),
    );

    <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <b>borrow_global_mut</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
    <a href="staking_config.md#0x1_staking_config">staking_config</a>.voting_power_increase_limit = new_voting_power_increase_limit;
}
</code></pre>



</details>

<a id="0x1_staking_config_validate_required_stake"></a>

## Function `validate_required_stake`



<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_validate_required_stake">validate_required_stake</a>(minimum_stake: u64, maximum_stake: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_validate_required_stake">validate_required_stake</a>(minimum_stake: u64, maximum_stake: u64) {
    <b>assert</b>!(minimum_stake &lt;= maximum_stake && maximum_stake &gt; 0, <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_STAKE_RANGE">EINVALID_STAKE_RANGE</a>));
}
</code></pre>



</details>

<a id="0x1_staking_config_validate_rewards_config"></a>

## Function `validate_rewards_config`



<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_validate_rewards_config">validate_rewards_config</a>(rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_validate_rewards_config">validate_rewards_config</a>(
    rewards_rate: FixedPoint64,
    min_rewards_rate: FixedPoint64,
    rewards_rate_period_in_secs: u64,
    rewards_rate_decrease_rate: FixedPoint64,
) {
    // Bound rewards rate <b>to</b> avoid arithmetic overflow.
    <b>assert</b>!(
        less_or_equal(rewards_rate, <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_u128">fixed_point64::create_from_u128</a>((1u128))),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE">EINVALID_REWARDS_RATE</a>)
    );
    <b>assert</b>!(
        less_or_equal(min_rewards_rate, rewards_rate),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_MIN_REWARDS_RATE">EINVALID_MIN_REWARDS_RATE</a>)
    );
    // Rewards rate decrease rate cannot be greater than 100%. Otherwise rewards rate will be negative.
    <b>assert</b>!(
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_ceil">fixed_point64::ceil</a>(rewards_rate_decrease_rate) &lt;= 1,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE_DECREASE_RATE">EINVALID_REWARDS_RATE_DECREASE_RATE</a>)
    );
    // This field, rewards_rate_period_in_secs must be greater than 0.
    // TODO: rewards_rate_period_in_secs should be longer than the epoch duration but reading epoch duration causes a circular dependency.
    <b>assert</b>!(
        rewards_rate_period_in_secs &gt; 0,
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="staking_config.md#0x1_staking_config_EINVALID_REWARDS_RATE_PERIOD">EINVALID_REWARDS_RATE_PERIOD</a>),
    );
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The ability to initialize the staking config and staking rewards resources, as well as the ability to update the staking config and staking rewards should only be available to the Velor framework account.</td>
<td>Medium</td>
<td>The function initialize and initialize_rewards are used to initialize the StakingConfig and StakingRewardConfig resources. Updating the resources, can be done using the update_required_stake, update_recurring_lockup_duration_secs, update_rewards_rate, update_rewards_config, update_voting_power_increase_limit functions, which ensure that the signer is velor_framework using the assert_velor_framework function.</td>
<td>Verified via <a href="#high-level-req-1.1">initialize</a>, <a href="#high-level-req-1.2">initialize_rewards</a>, <a href="#high-level-req-1.3">update_required_stake</a>, <a href="#high-level-req-1.4">update_recurring_lockup_duration_secs</a>, <a href="#high-level-req-1.5">update_rewards_rate</a>, <a href="#high-level-req-1.6">update_rewards_config</a>, and <a href="#high-level-req-1.7">update_voting_power_increase_limit</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The voting power increase, in a staking config resource, should always be greater than 0 and less or equal to 50.</td>
<td>High</td>
<td>During the initialization and update of the staking config, the value of voting_power_increase_limit is ensured to be in the range of (0 to 50].</td>
<td>Ensured via <a href="#high-level-req-2.1">initialize</a> and <a href="#high-level-req-2.2">update_voting_power_increase_limit</a>. Formally verified via <a href="#high-level-req-2.3">StakingConfig</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The recurring lockup duration, in a staking config resource, should always be greater than 0.</td>
<td>Medium</td>
<td>During the initialization and update of the staking config, the value of recurring_lockup_duration_secs is ensured to be greater than 0.</td>
<td>Ensured via <a href="#high-level-req-3.1">initialize</a> and <a href="#high-level-req-3.2">update_recurring_lockup_duration_secs</a>. Formally verified via <a href="#high-level-req-3.3">StakingConfig</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The calculation of rewards should not be possible if the last reward rate period just started.</td>
<td>High</td>
<td>The function calculate_and_save_latest_rewards_config ensures that last_rewards_rate_period_start_in_secs is greater or equal to the current timestamp.</td>
<td>Formally verified in <a href="#high-level-req-4">StakingRewardsConfigEnabledRequirement</a>.</td>
</tr>

<tr>
<td>5</td>
<td>The rewards rate should always be less than or equal to 100%.</td>
<td>High</td>
<td>When initializing and updating the rewards rate, it is ensured that the rewards_rate is less or equal to MAX_REWARDS_RATE, otherwise rewards rate will be negative.</td>
<td>Verified via <a href="#high-level-req-5">StakingConfig</a>.</td>
</tr>

<tr>
<td>6</td>
<td>The reward rate's denominator should never be 0.</td>
<td>High</td>
<td>While initializing and updating the rewards rate, rewards_rate_denominator is ensured to be greater than 0.</td>
<td>Verified via <a href="#high-level-req-6">StakingConfig</a>.</td>
</tr>

<tr>
<td>7</td>
<td>The reward rate's nominator and dominator ratio should always be less or equal to 1.</td>
<td>High</td>
<td>When initializing and updating the rewards rate, it is ensured that rewards_rate is less or equal to rewards_rate_denominator.</td>
<td>Verified via <a href="#high-level-req-7">StakingConfig</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
<b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_StakingConfig"></a>

### Resource `StakingConfig`


<pre><code><b>struct</b> <a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



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



<pre><code>// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
<b>invariant</b> rewards_rate &lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;
// This enforces <a id="high-level-req-6" href="#high-level-req">high-level requirement 6</a>:
<b>invariant</b> rewards_rate_denominator &gt; 0;
// This enforces <a id="high-level-req-7" href="#high-level-req">high-level requirement 7</a>:
<b>invariant</b> rewards_rate &lt;= rewards_rate_denominator;
// This enforces <a id="high-level-req-3.3" href="#high-level-req">high-level requirement 3</a>:
<b>invariant</b> recurring_lockup_duration_secs &gt; 0;
// This enforces <a id="high-level-req-2.3" href="#high-level-req">high-level requirement 2</a>:
<b>invariant</b> voting_power_increase_limit &gt; 0 && voting_power_increase_limit &lt;= 50;
</code></pre>



<a id="@Specification_1_StakingRewardsConfig"></a>

### Resource `StakingRewardsConfig`


<pre><code><b>struct</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<dl>
<dt>
<code>rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>

</dd>
<dt>
<code>min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
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
<code>rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a></code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">fixed_point64::spec_less_or_equal</a>(
    rewards_rate,
    <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">fixed_point64::spec_create_from_u128</a>((1u128)));
<b>invariant</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">fixed_point64::spec_less_or_equal</a>(min_rewards_rate, rewards_rate);
<b>invariant</b> rewards_rate_period_in_secs &gt; 0;
<b>invariant</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_ceil">fixed_point64::spec_ceil</a>(rewards_rate_decrease_rate) &lt;= 1;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="staking_config.md#0x1_staking_config_initialize">initialize</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)
</code></pre>


Caller must be @velor_framework.
The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
The rewards_rate_denominator must greater than zero.
Only this %0-%50 of current total voting power is allowed to join the validator set in each epoch.
The <code>rewards_rate</code> which is the numerator is limited to be <code>&lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a></code> in order to avoid the arithmetic overflow in the rewards calculation.
rewards_rate/rewards_rate_denominator <= 1.
StakingConfig does not exist under the velor_framework before creating it.


<pre><code><b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
<b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
// This enforces <a id="high-level-req-1.1" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
<b>aborts_if</b> minimum_stake &gt; maximum_stake || maximum_stake == 0;
// This enforces <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> recurring_lockup_duration_secs == 0;
<b>aborts_if</b> rewards_rate_denominator == 0;
// This enforces <a id="high-level-req-2.1" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> voting_power_increase_limit == 0 || voting_power_increase_limit &gt; 50;
<b>aborts_if</b> rewards_rate &gt; <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;
<b>aborts_if</b> rewards_rate &gt; rewards_rate_denominator;
<b>aborts_if</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(addr);
</code></pre>



<a id="@Specification_1_reward_rate"></a>

### Function `reward_rate`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_reward_rate">reward_rate</a>(): (u64, u64)
</code></pre>




<pre><code><b>let</b> config = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">StakingRewardsConfigRequirement</a>;
<b>ensures</b> (<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() &&
    (<b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework).rewards_rate.value <b>as</b> u64) != 0) ==&gt;
    result_1 &lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a> && result_2 &lt;= <a href="staking_config.md#0x1_staking_config_MAX_U64">MAX_U64</a>;
</code></pre>



<a id="@Specification_1_initialize_rewards"></a>

### Function `initialize_rewards`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_initialize_rewards">initialize_rewards</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, rewards_rate_period_in_secs: u64, last_rewards_rate_period_start_in_secs: u64, rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>)
</code></pre>


Caller must be @velor_framework.
last_rewards_rate_period_start_in_secs cannot be later than now.
Abort at any condition in StakingRewardsConfigValidationAborts.
StakingRewardsConfig does not exist under the velor_framework before creating it.


<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-1.2" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
<b>aborts_if</b> last_rewards_rate_period_start_in_secs &gt; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigValidationAbortsIf">StakingRewardsConfigValidationAbortsIf</a>;
<b>aborts_if</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(addr);
</code></pre>



<a id="@Specification_1_get"></a>

### Function `get`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get">get</a>(): <a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
</code></pre>



<a id="@Specification_1_get_reward_rate"></a>

### Function `get_reward_rate`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_get_reward_rate">get_reward_rate</a>(config: &<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): (u64, u64)
</code></pre>




<pre><code><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">StakingRewardsConfigRequirement</a>;
<b>ensures</b> (<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() &&
    (<b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework).rewards_rate.value <b>as</b> u64) != 0) ==&gt;
        result_1 &lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a> && result_2 &lt;= <a href="staking_config.md#0x1_staking_config_MAX_U64">MAX_U64</a>;
</code></pre>



<a id="@Specification_1_calculate_and_save_latest_epoch_rewards_rate"></a>

### Function `calculate_and_save_latest_epoch_rewards_rate`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_epoch_rewards_rate">calculate_and_save_latest_epoch_rewards_rate</a>(): <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
<b>aborts_if</b> !<a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>();
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">StakingRewardsConfigRequirement</a>;
</code></pre>



<a id="@Specification_1_calculate_and_save_latest_rewards_config"></a>

### Function `calculate_and_save_latest_rewards_config`


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_calculate_and_save_latest_rewards_config">calculate_and_save_latest_rewards_config</a>(): <a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>requires</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>();
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">StakingRewardsConfigRequirement</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
</code></pre>



<a id="@Specification_1_update_required_stake"></a>

### Function `update_required_stake`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_required_stake">update_required_stake</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, minimum_stake: u64, maximum_stake: u64)
</code></pre>


Caller must be @velor_framework.
The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.
The StakingConfig is under @velor_framework.


<pre><code><b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-1.3" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
<b>aborts_if</b> minimum_stake &gt; maximum_stake || maximum_stake == 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>ensures</b> <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework).minimum_stake == minimum_stake &&
    <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework).maximum_stake == maximum_stake;
</code></pre>



<a id="@Specification_1_update_recurring_lockup_duration_secs"></a>

### Function `update_recurring_lockup_duration_secs`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_recurring_lockup_duration_secs">update_recurring_lockup_duration_secs</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_recurring_lockup_duration_secs: u64)
</code></pre>


Caller must be @velor_framework.
The new_recurring_lockup_duration_secs must greater than zero.
The StakingConfig is under @velor_framework.


<pre><code><b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-1.4" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> new_recurring_lockup_duration_secs == 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>ensures</b> <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework).recurring_lockup_duration_secs == new_recurring_lockup_duration_secs;
</code></pre>



<a id="@Specification_1_update_rewards_rate"></a>

### Function `update_rewards_rate`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_rewards_rate">update_rewards_rate</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_rewards_rate: u64, new_rewards_rate_denominator: u64)
</code></pre>


Caller must be @velor_framework.
The new_rewards_rate_denominator must greater than zero.
The StakingConfig is under @velor_framework.
The <code>rewards_rate</code> which is the numerator is limited to be <code>&lt;= <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a></code> in order to avoid the arithmetic overflow in the rewards calculation.
rewards_rate/rewards_rate_denominator <= 1.


<pre><code><b>aborts_if</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>();
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-1.5" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
<b>aborts_if</b> new_rewards_rate_denominator == 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>aborts_if</b> new_rewards_rate &gt; <a href="staking_config.md#0x1_staking_config_MAX_REWARDS_RATE">MAX_REWARDS_RATE</a>;
<b>aborts_if</b> new_rewards_rate &gt; new_rewards_rate_denominator;
<b>let</b> <b>post</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>ensures</b> <a href="staking_config.md#0x1_staking_config">staking_config</a>.rewards_rate == new_rewards_rate;
<b>ensures</b> <a href="staking_config.md#0x1_staking_config">staking_config</a>.rewards_rate_denominator == new_rewards_rate_denominator;
</code></pre>



<a id="@Specification_1_update_rewards_config"></a>

### Function `update_rewards_config`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_rewards_config">update_rewards_config</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>)
</code></pre>


Caller must be @velor_framework.
StakingRewardsConfig is under the @velor_framework.


<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">StakingRewardsConfigRequirement</a>;
<b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-1.6" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
<b>aborts_if</b> <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework).rewards_rate_period_in_secs != rewards_rate_period_in_secs;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigValidationAbortsIf">StakingRewardsConfigValidationAbortsIf</a>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(addr);
<b>let</b> <b>post</b> staking_rewards_config = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
<b>ensures</b> staking_rewards_config.rewards_rate == rewards_rate;
<b>ensures</b> staking_rewards_config.min_rewards_rate == min_rewards_rate;
<b>ensures</b> staking_rewards_config.rewards_rate_period_in_secs == rewards_rate_period_in_secs;
<b>ensures</b> staking_rewards_config.rewards_rate_decrease_rate == rewards_rate_decrease_rate;
</code></pre>



<a id="@Specification_1_update_voting_power_increase_limit"></a>

### Function `update_voting_power_increase_limit`


<pre><code><b>public</b> <b>fun</b> <a href="staking_config.md#0x1_staking_config_update_voting_power_increase_limit">update_voting_power_increase_limit</a>(velor_framework: &<a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voting_power_increase_limit: u64)
</code></pre>


Caller must be @velor_framework.
Only this %0-%50 of current total voting power is allowed to join the validator set in each epoch.
The StakingConfig is under @velor_framework.


<pre><code><b>let</b> addr = <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(velor_framework);
// This enforces <a id="high-level-req-1.7" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> addr != @velor_framework;
// This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> new_voting_power_increase_limit == 0 || new_voting_power_increase_limit &gt; 50;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework);
<b>ensures</b> <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">StakingConfig</a>&gt;(@velor_framework).voting_power_increase_limit == new_voting_power_increase_limit;
</code></pre>



<a id="@Specification_1_validate_required_stake"></a>

### Function `validate_required_stake`


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_validate_required_stake">validate_required_stake</a>(minimum_stake: u64, maximum_stake: u64)
</code></pre>


The maximum_stake must be greater than maximum_stake in the range of Specified stake and the maximum_stake greater than zero.


<pre><code><b>aborts_if</b> minimum_stake &gt; maximum_stake || maximum_stake == 0;
</code></pre>



<a id="@Specification_1_validate_rewards_config"></a>

### Function `validate_rewards_config`


<pre><code><b>fun</b> <a href="staking_config.md#0x1_staking_config_validate_rewards_config">validate_rewards_config</a>(rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, min_rewards_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, rewards_rate_period_in_secs: u64, rewards_rate_decrease_rate: <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>)
</code></pre>


Abort at any condition in StakingRewardsConfigValidationAborts.


<pre><code><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigValidationAbortsIf">StakingRewardsConfigValidationAbortsIf</a>;
</code></pre>


rewards_rate must be within [0, 1].
min_rewards_rate must be not greater than rewards_rate.
rewards_rate_period_in_secs must be greater than 0.
rewards_rate_decrease_rate must be within [0,1].


<a id="0x1_staking_config_StakingRewardsConfigValidationAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigValidationAbortsIf">StakingRewardsConfigValidationAbortsIf</a> {
    rewards_rate: FixedPoint64;
    min_rewards_rate: FixedPoint64;
    rewards_rate_period_in_secs: u64;
    rewards_rate_decrease_rate: FixedPoint64;
    <b>aborts_if</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_greater">fixed_point64::spec_greater</a>(
        rewards_rate,
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">fixed_point64::spec_create_from_u128</a>((1u128)));
    <b>aborts_if</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_greater">fixed_point64::spec_greater</a>(min_rewards_rate, rewards_rate);
    <b>aborts_if</b> rewards_rate_period_in_secs == 0;
    <b>aborts_if</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_ceil">fixed_point64::spec_ceil</a>(rewards_rate_decrease_rate) &gt; 1;
}
</code></pre>




<a id="0x1_staking_config_StakingRewardsConfigRequirement"></a>


<pre><code><b>schema</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">StakingRewardsConfigRequirement</a> {
    <b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@velor_framework);
    <b>include</b> <a href="../../velor-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() ==&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">StakingRewardsConfigEnabledRequirement</a>;
}
</code></pre>




<a id="0x1_staking_config_StakingRewardsConfigEnabledRequirement"></a>


<pre><code><b>schema</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">StakingRewardsConfigEnabledRequirement</a> {
    <b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
    <b>let</b> staking_rewards_config = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">StakingRewardsConfig</a>&gt;(@velor_framework);
    <b>let</b> rewards_rate = staking_rewards_config.rewards_rate;
    <b>let</b> min_rewards_rate = staking_rewards_config.min_rewards_rate;
    <b>let</b> rewards_rate_period_in_secs = staking_rewards_config.rewards_rate_period_in_secs;
    <b>let</b> last_rewards_rate_period_start_in_secs = staking_rewards_config.last_rewards_rate_period_start_in_secs;
    <b>let</b> rewards_rate_decrease_rate = staking_rewards_config.rewards_rate_decrease_rate;
    <b>requires</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">fixed_point64::spec_less_or_equal</a>(
        rewards_rate,
        <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_create_from_u128">fixed_point64::spec_create_from_u128</a>((1u128)));
    <b>requires</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_less_or_equal">fixed_point64::spec_less_or_equal</a>(min_rewards_rate, rewards_rate);
    <b>requires</b> rewards_rate_period_in_secs &gt; 0;
    // This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    <b>requires</b> last_rewards_rate_period_start_in_secs &lt;= <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
    <b>requires</b> <a href="../../velor-stdlib/doc/fixed_point64.md#0x1_fixed_point64_spec_ceil">fixed_point64::spec_ceil</a>(rewards_rate_decrease_rate) &lt;= 1;
}
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
