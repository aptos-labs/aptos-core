
<a id="0x1_genesis"></a>

# Module `0x1::genesis`



-  [Struct `AccountMap`](#0x1_genesis_AccountMap)
-  [Struct `VestingPoolsMap`](#0x1_genesis_VestingPoolsMap)
-  [Struct `EmployeeAccountMap`](#0x1_genesis_EmployeeAccountMap)
-  [Struct `ValidatorConfiguration`](#0x1_genesis_ValidatorConfiguration)
-  [Struct `ValidatorConfigurationWithCommission`](#0x1_genesis_ValidatorConfigurationWithCommission)
-  [Struct `DelegatorConfiguration`](#0x1_genesis_DelegatorConfiguration)
-  [Struct `PboDelegatorConfiguration`](#0x1_genesis_PboDelegatorConfiguration)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_genesis_initialize)
-  [Function `initialize_supra_coin`](#0x1_genesis_initialize_supra_coin)
-  [Function `initialize_supra_native_automation`](#0x1_genesis_initialize_supra_native_automation)
-  [Function `initialize_core_resources_and_supra_coin`](#0x1_genesis_initialize_core_resources_and_supra_coin)
-  [Function `create_accounts`](#0x1_genesis_create_accounts)
-  [Function `create_account`](#0x1_genesis_create_account)
-  [Function `create_multiple_multisig_accounts_with_schema`](#0x1_genesis_create_multiple_multisig_accounts_with_schema)
-  [Function `create_multisig_account_with_balance`](#0x1_genesis_create_multisig_account_with_balance)
-  [Function `create_employee_validators`](#0x1_genesis_create_employee_validators)
-  [Function `create_initialize_validators_with_commission`](#0x1_genesis_create_initialize_validators_with_commission)
-  [Function `create_initialize_validators`](#0x1_genesis_create_initialize_validators)
-  [Function `create_initialize_validator`](#0x1_genesis_create_initialize_validator)
-  [Function `create_pbo_delegation_pools`](#0x1_genesis_create_pbo_delegation_pools)
-  [Function `create_pbo_delegation_pool`](#0x1_genesis_create_pbo_delegation_pool)
-  [Function `assert_validator_addresses_check`](#0x1_genesis_assert_validator_addresses_check)
-  [Function `create_vesting_without_staking_pools`](#0x1_genesis_create_vesting_without_staking_pools)
-  [Function `initialize_validator`](#0x1_genesis_initialize_validator)
-  [Function `set_genesis_end`](#0x1_genesis_set_genesis_end)
-  [Function `initialize_for_verification`](#0x1_genesis_initialize_for_verification)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initialize_supra_coin`](#@Specification_1_initialize_supra_coin)
    -  [Function `create_initialize_validators_with_commission`](#@Specification_1_create_initialize_validators_with_commission)
    -  [Function `create_initialize_validators`](#@Specification_1_create_initialize_validators)
    -  [Function `create_initialize_validator`](#@Specification_1_create_initialize_validator)
    -  [Function `create_pbo_delegation_pools`](#@Specification_1_create_pbo_delegation_pools)
    -  [Function `create_pbo_delegation_pool`](#@Specification_1_create_pbo_delegation_pool)
    -  [Function `assert_validator_addresses_check`](#@Specification_1_assert_validator_addresses_check)
    -  [Function `create_vesting_without_staking_pools`](#@Specification_1_create_vesting_without_staking_pools)
    -  [Function `set_genesis_end`](#@Specification_1_set_genesis_end)
    -  [Function `initialize_for_verification`](#@Specification_1_initialize_for_verification)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aggregator_factory.md#0x1_aggregator_factory">0x1::aggregator_factory</a>;
<b>use</b> <a href="automation_registry.md#0x1_automation_registry">0x1::automation_registry</a>;
<b>use</b> <a href="block.md#0x1_block">0x1::block</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="evm_config.md#0x1_evm_config">0x1::evm_config</a>;
<b>use</b> <a href="execution_config.md#0x1_execution_config">0x1::execution_config</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;
<b>use</b> <a href="gas_schedule.md#0x1_gas_schedule">0x1::gas_schedule</a>;
<b>use</b> <a href="multisig_account.md#0x1_multisig_account">0x1::multisig_account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="pbo_delegation_pool.md#0x1_pbo_delegation_pool">0x1::pbo_delegation_pool</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;
<b>use</b> <a href="staking_contract.md#0x1_staking_contract">0x1::staking_contract</a>;
<b>use</b> <a href="state_storage.md#0x1_state_storage">0x1::state_storage</a>;
<b>use</b> <a href="storage_gas.md#0x1_storage_gas">0x1::storage_gas</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="supra_account.md#0x1_supra_account">0x1::supra_account</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="supra_config.md#0x1_supra_config">0x1::supra_config</a>;
<b>use</b> <a href="supra_governance.md#0x1_supra_governance">0x1::supra_governance</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;
<b>use</b> <a href="transaction_validation.md#0x1_transaction_validation">0x1::transaction_validation</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="version.md#0x1_version">0x1::version</a>;
<b>use</b> <a href="vesting.md#0x1_vesting">0x1::vesting</a>;
<b>use</b> <a href="vesting_without_staking.md#0x1_vesting_without_staking">0x1::vesting_without_staking</a>;
</code></pre>



<a id="0x1_genesis_AccountMap"></a>

## Struct `AccountMap`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_VestingPoolsMap"></a>

## Struct `VestingPoolsMap`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_VestingPoolsMap">VestingPoolsMap</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>admin_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vpool_locking_percentage: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_numerators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>withdrawal_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>shareholders: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cliff_period_in_seconds: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>period_duration_in_seconds: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_EmployeeAccountMap"></a>

## Struct `EmployeeAccountMap`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>validator: <a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a></code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule_numerator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiary_resetter: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_ValidatorConfiguration"></a>

## Struct `ValidatorConfiguration`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>operator_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>voter_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>stake_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>consensus_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>full_node_network_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_ValidatorConfigurationWithCommission"></a>

## Struct `ValidatorConfigurationWithCommission`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validator_config: <a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a></code>
</dt>
<dd>

</dd>
<dt>
<code>commission_percentage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>join_during_genesis: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_DelegatorConfiguration"></a>

## Struct `DelegatorConfiguration`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_DelegatorConfiguration">DelegatorConfiguration</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>validator: <a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a></code>
</dt>
<dd>

</dd>
<dt>
<code>delegator_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delegator_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_PboDelegatorConfiguration"></a>

## Struct `PboDelegatorConfiguration`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">PboDelegatorConfiguration</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>delegator_config: <a href="genesis.md#0x1_genesis_DelegatorConfiguration">genesis::DelegatorConfiguration</a></code>
</dt>
<dd>

</dd>
<dt>
<code>multisig_admin: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_schedule_numerators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_schedule_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_startup_time_from_now: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>unlock_period_duration: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_genesis_EACCOUNT_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a id="0x1_genesis_ENO_SHAREHOLDERS"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>: u64 = 5;
</code></pre>



<a id="0x1_genesis_EDENOMINATOR_IS_ZERO"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EDENOMINATOR_IS_ZERO">EDENOMINATOR_IS_ZERO</a>: u64 = 8;
</code></pre>



<a id="0x1_genesis_EACCOUNT_NOT_REGISTERED_FOR_COIN"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EACCOUNT_NOT_REGISTERED_FOR_COIN">EACCOUNT_NOT_REGISTERED_FOR_COIN</a>: u64 = 9;
</code></pre>



<a id="0x1_genesis_EDUPLICATE_ACCOUNT"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>: u64 = 1;
</code></pre>



<a id="0x1_genesis_ENUMERATOR_GREATER_THAN_DENOMINATOR"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_ENUMERATOR_GREATER_THAN_DENOMINATOR">ENUMERATOR_GREATER_THAN_DENOMINATOR</a>: u64 = 7;
</code></pre>



<a id="0x1_genesis_ENUMERATOR_IS_ZERO"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_ENUMERATOR_IS_ZERO">ENUMERATOR_IS_ZERO</a>: u64 = 4;
</code></pre>



<a id="0x1_genesis_EPERCENTAGE_INVALID"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EPERCENTAGE_INVALID">EPERCENTAGE_INVALID</a>: u64 = 6;
</code></pre>



<a id="0x1_genesis_EVESTING_SCHEDULE_IS_ZERO"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EVESTING_SCHEDULE_IS_ZERO">EVESTING_SCHEDULE_IS_ZERO</a>: u64 = 3;
</code></pre>



<a id="0x1_genesis_VESTING_CONTRACT_SEED"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_VESTING_CONTRACT_SEED">VESTING_CONTRACT_SEED</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [86, 69, 83, 84, 73, 78, 71, 95, 87, 73, 72, 79, 85, 84, 95, 83, 84, 65, 75, 73, 78, 71, 95, 83, 69, 69, 68];
</code></pre>



<a id="0x1_genesis_initialize"></a>

## Function `initialize`

Genesis step 1: Initialize supra framework account and core modules on chain.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize">initialize</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="supra_config.md#0x1_supra_config">supra_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, genesis_timestamp_in_microseconds: u64, <a href="evm_config.md#0x1_evm_config">evm_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize">initialize</a>(
    <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    initial_version: u64,
    <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="supra_config.md#0x1_supra_config">supra_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    epoch_interval_microsecs: u64,
    minimum_stake: u64,
    maximum_stake: u64,
    recurring_lockup_duration_secs: u64,
    allow_validator_set_change: bool,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
    voting_power_increase_limit: u64,
    genesis_timestamp_in_microseconds: u64,
    <a href="evm_config.md#0x1_evm_config">evm_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    // Initialize the supra framework <a href="account.md#0x1_account">account</a>. This is the <a href="account.md#0x1_account">account</a> <b>where</b> system resources and modules will be
    // deployed <b>to</b>. This will be entirely managed by on-chain governance and no entities have the key or privileges
    // <b>to</b> <b>use</b> this <a href="account.md#0x1_account">account</a>.
    <b>let</b> (supra_framework_account, supra_framework_signer_cap) = <a href="account.md#0x1_account_create_framework_reserved_account">account::create_framework_reserved_account</a>(
        @supra_framework
    );
    // Initialize <a href="account.md#0x1_account">account</a> configs on aptos framework <a href="account.md#0x1_account">account</a>.
    <a href="account.md#0x1_account_initialize">account::initialize</a>(&supra_framework_account);

    <a href="transaction_validation.md#0x1_transaction_validation_initialize">transaction_validation::initialize</a>(
        &supra_framework_account,
        b"script_prologue",
        b"module_prologue",
        b"multi_agent_script_prologue",
        b"epilogue",
    );

    // Give the decentralized on-chain governance control over the core framework <a href="account.md#0x1_account">account</a>.
    <a href="supra_governance.md#0x1_supra_governance_store_signer_cap">supra_governance::store_signer_cap</a>(&supra_framework_account, @supra_framework, supra_framework_signer_cap);

    // put reserved framework reserved accounts under supra governance
    <b>let</b> framework_reserved_addresses = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;[@0x2, @0x3, @0x4, @0x5, @0x6, @0x7, @0x8, @0x9, @0xa];
    <b>while</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&framework_reserved_addresses)) {
        <b>let</b> <b>address</b> = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>&lt;<b>address</b>&gt;(&<b>mut</b> framework_reserved_addresses);
        <b>let</b> (_, framework_signer_cap) = <a href="account.md#0x1_account_create_framework_reserved_account">account::create_framework_reserved_account</a>(<b>address</b>);
        <a href="supra_governance.md#0x1_supra_governance_store_signer_cap">supra_governance::store_signer_cap</a>(&supra_framework_account, <b>address</b>, framework_signer_cap);
    };

    <a href="consensus_config.md#0x1_consensus_config_initialize">consensus_config::initialize</a>(&supra_framework_account, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>);
    <a href="execution_config.md#0x1_execution_config_set">execution_config::set</a>(&supra_framework_account, <a href="execution_config.md#0x1_execution_config">execution_config</a>);
    <a href="supra_config.md#0x1_supra_config_initialize">supra_config::initialize</a>(&supra_framework_account, <a href="supra_config.md#0x1_supra_config">supra_config</a>);
    <a href="version.md#0x1_version_initialize">version::initialize</a>(&supra_framework_account, initial_version);
    <a href="stake.md#0x1_stake_initialize">stake::initialize</a>(&supra_framework_account);
    <a href="staking_config.md#0x1_staking_config_initialize">staking_config::initialize</a>(
        &supra_framework_account,
        minimum_stake,
        maximum_stake,
        recurring_lockup_duration_secs,
        allow_validator_set_change,
        rewards_rate,
        rewards_rate_denominator,
        voting_power_increase_limit,
    );
    <a href="storage_gas.md#0x1_storage_gas_initialize">storage_gas::initialize</a>(&supra_framework_account);
    <a href="gas_schedule.md#0x1_gas_schedule_initialize">gas_schedule::initialize</a>(&supra_framework_account, <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>);

    // Ensure we can create aggregators for supply, but not enable it for common <b>use</b> just yet.
    <a href="aggregator_factory.md#0x1_aggregator_factory_initialize_aggregator_factory">aggregator_factory::initialize_aggregator_factory</a>(&supra_framework_account);
    <a href="coin.md#0x1_coin_initialize_supply_config">coin::initialize_supply_config</a>(&supra_framework_account);

    <a href="chain_id.md#0x1_chain_id_initialize">chain_id::initialize</a>(&supra_framework_account, <a href="chain_id.md#0x1_chain_id">chain_id</a>);
    <a href="reconfiguration.md#0x1_reconfiguration_initialize">reconfiguration::initialize</a>(&supra_framework_account);
    <a href="block.md#0x1_block_initialize">block::initialize</a>(&supra_framework_account, epoch_interval_microsecs);
    <a href="state_storage.md#0x1_state_storage_initialize">state_storage::initialize</a>(&supra_framework_account);
    <a href="timestamp.md#0x1_timestamp_set_time_has_started">timestamp::set_time_has_started</a>(&supra_framework_account, genesis_timestamp_in_microseconds);
    <a href="evm_config.md#0x1_evm_config_initialize">evm_config::initialize</a>(&supra_framework_account, <a href="evm_config.md#0x1_evm_config">evm_config</a>);
}
</code></pre>



</details>

<a id="0x1_genesis_initialize_supra_coin"></a>

## Function `initialize_supra_coin`

Genesis step 2: Initialize Supra coin.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_supra_coin">initialize_supra_coin</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_supra_coin">initialize_supra_coin</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> (burn_cap, mint_cap) = <a href="supra_coin.md#0x1_supra_coin_initialize">supra_coin::initialize</a>(supra_framework);
    <a href="coin.md#0x1_coin_create_coin_conversion_map">coin::create_coin_conversion_map</a>(supra_framework);
    <a href="coin.md#0x1_coin_create_pairing">coin::create_pairing</a>&lt;SupraCoin&gt;(supra_framework);
    // Give <a href="stake.md#0x1_stake">stake</a> <b>module</b> MintCapability&lt;SupraCoin&gt; so it can mint rewards.
    <a href="stake.md#0x1_stake_store_supra_coin_mint_cap">stake::store_supra_coin_mint_cap</a>(supra_framework, mint_cap);
    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> BurnCapability&lt;SupraCoin&gt; so it can burn gas.
    <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_burn_cap">transaction_fee::store_supra_coin_burn_cap</a>(supra_framework, burn_cap);
    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> MintCapability&lt;SupraCoin&gt; so it can mint refunds.
    <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_mint_cap">transaction_fee::store_supra_coin_mint_cap</a>(supra_framework, mint_cap);
}
</code></pre>



</details>

<a id="0x1_genesis_initialize_supra_native_automation"></a>

## Function `initialize_supra_native_automation`

Genesis step 3: Initialize Supra Native Automation.


<pre><code><b>public</b> <b>fun</b> <a href="genesis.md#0x1_genesis_initialize_supra_native_automation">initialize_supra_native_automation</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, task_duration_cap_in_secs: u64, registry_max_gas_cap: u64, automation_base_fee_in_quants_per_sec: u64, flat_registration_fee_in_quants: u64, congestion_threshold_percentage: u8, congestion_base_fee_in_quants_per_sec: u64, congestion_exponent: u8, task_capacity: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="genesis.md#0x1_genesis_initialize_supra_native_automation">initialize_supra_native_automation</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    task_duration_cap_in_secs: u64,
    registry_max_gas_cap: u64,
    automation_base_fee_in_quants_per_sec: u64,
    flat_registration_fee_in_quants: u64,
    congestion_threshold_percentage: u8,
    congestion_base_fee_in_quants_per_sec: u64,
    congestion_exponent: u8,
    task_capacity: u16,
) {
    <b>let</b> epoch_interval_secs = <a href="block.md#0x1_block_get_epoch_interval_secs">block::get_epoch_interval_secs</a>();
    <a href="automation_registry.md#0x1_automation_registry_initialize">automation_registry::initialize</a>(
        supra_framework,
        epoch_interval_secs,
        task_duration_cap_in_secs,
        registry_max_gas_cap,
        automation_base_fee_in_quants_per_sec,
        flat_registration_fee_in_quants,
        congestion_threshold_percentage,
        congestion_base_fee_in_quants_per_sec,
        congestion_exponent,
        task_capacity,
    )
}
</code></pre>



</details>

<a id="0x1_genesis_initialize_core_resources_and_supra_coin"></a>

## Function `initialize_core_resources_and_supra_coin`

Only called for testnets and e2e tests.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_core_resources_and_supra_coin">initialize_core_resources_and_supra_coin</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, core_resources_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_core_resources_and_supra_coin">initialize_core_resources_and_supra_coin</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    core_resources_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <b>let</b> (burn_cap, mint_cap) = <a href="supra_coin.md#0x1_supra_coin_initialize">supra_coin::initialize</a>(supra_framework);
    <a href="coin.md#0x1_coin_create_coin_conversion_map">coin::create_coin_conversion_map</a>(supra_framework);
    <a href="coin.md#0x1_coin_create_pairing">coin::create_pairing</a>&lt;SupraCoin&gt;(supra_framework);
    // Give <a href="stake.md#0x1_stake">stake</a> <b>module</b> MintCapability&lt;SupraCoin&gt; so it can mint rewards.
    <a href="stake.md#0x1_stake_store_supra_coin_mint_cap">stake::store_supra_coin_mint_cap</a>(supra_framework, mint_cap);
    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> BurnCapability&lt;SupraCoin&gt; so it can burn gas.
    <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_burn_cap">transaction_fee::store_supra_coin_burn_cap</a>(supra_framework, burn_cap);
    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> MintCapability&lt;SupraCoin&gt; so it can mint refunds.
    <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_mint_cap">transaction_fee::store_supra_coin_mint_cap</a>(supra_framework, mint_cap);

    <b>let</b> core_resources = <a href="account.md#0x1_account_create_account">account::create_account</a>(@core_resources);
    <a href="supra_account.md#0x1_supra_account_register_supra">supra_account::register_supra</a>(&core_resources); // register Supra store
    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(&core_resources, core_resources_auth_key);
    <a href="supra_coin.md#0x1_supra_coin_configure_accounts_for_test">supra_coin::configure_accounts_for_test</a>(supra_framework, &core_resources, mint_cap);
}
</code></pre>



</details>

<a id="0x1_genesis_create_accounts"></a>

## Function `create_accounts`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_accounts">create_accounts</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">genesis::AccountMap</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_accounts">create_accounts</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a>&gt;) {
    <b>let</b> unique_accounts = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&accounts, |account_map| {
        <b>let</b> account_map: &<a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a> = account_map;
        <b>assert</b>!(
            !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&unique_accounts, &account_map.account_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>),
        );
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> unique_accounts, account_map.account_address);

        <a href="genesis.md#0x1_genesis_create_account">create_account</a>(
            supra_framework,
            account_map.account_address,
            account_map.balance,
        );
    });
}
</code></pre>



</details>

<a id="0x1_genesis_create_account"></a>

## Function `create_account`

This creates an funds an account if it doesn't exist.
If it exists, it just returns the signer.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_account">create_account</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, account_address: <b>address</b>, balance: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_account">create_account</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, account_address: <b>address</b>, balance: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>if</b> (<a href="account.md#0x1_account_exists_at">account::exists_at</a>(account_address)) {
        <a href="create_signer.md#0x1_create_signer">create_signer</a>(account_address)
    } <b>else</b> {
        <b>let</b> <a href="account.md#0x1_account">account</a> = <a href="account.md#0x1_account_create_account">account::create_account</a>(account_address);
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;SupraCoin&gt;(&<a href="account.md#0x1_account">account</a>);
        <a href="supra_coin.md#0x1_supra_coin_mint">supra_coin::mint</a>(supra_framework, account_address, balance);
        <a href="account.md#0x1_account">account</a>
    }
}
</code></pre>



</details>

<a id="0x1_genesis_create_multiple_multisig_accounts_with_schema"></a>

## Function `create_multiple_multisig_accounts_with_schema`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_multiple_multisig_accounts_with_schema">create_multiple_multisig_accounts_with_schema</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>, additional_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, timeout_duration: u64, balance: u64, num_of_accounts: u32): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_multiple_multisig_accounts_with_schema">create_multiple_multisig_accounts_with_schema</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owner: <b>address</b>,
    additional_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    timeout_duration: u64,
    balance: u64,
    num_of_accounts: u32
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; {
    <b>let</b> counter = 0;
    <b>let</b> result = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (counter &lt; num_of_accounts) {
        <b>let</b> account_addr = <a href="genesis.md#0x1_genesis_create_multisig_account_with_balance">create_multisig_account_with_balance</a>(supra_framework, owner, additional_owners,
            num_signatures_required, metadata_keys, metadata_values, timeout_duration, balance);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> result, account_addr);
        counter = counter + 1;
    };
    result
}
</code></pre>



</details>

<a id="0x1_genesis_create_multisig_account_with_balance"></a>

## Function `create_multisig_account_with_balance`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_multisig_account_with_balance">create_multisig_account_with_balance</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>, additional_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, timeout_duration: u64, balance: u64): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_multisig_account_with_balance">create_multisig_account_with_balance</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owner: <b>address</b>,
    additional_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    timeout_duration: u64,
    balance: u64,
): <b>address</b> {
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(owner), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_all">vector::all</a>(&additional_owners, |ao_addr|{ <a href="account.md#0x1_account_exists_at">account::exists_at</a>(*ao_addr) }),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>)
    );
    <b>let</b> addr = <a href="multisig_account.md#0x1_multisig_account_get_next_multisig_account_address">multisig_account::get_next_multisig_account_address</a>(owner);
    <b>let</b> owner_signer = <a href="create_signer.md#0x1_create_signer">create_signer</a>(owner);
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners">multisig_account::create_with_owners</a>(
        &owner_signer,
        additional_owners,
        num_signatures_required,
        metadata_keys,
        metadata_values,
        timeout_duration
    );
    <a href="supra_coin.md#0x1_supra_coin_mint">supra_coin::mint</a>(supra_framework, addr, balance);
    <a href="account.md#0x1_account_increment_sequence_number">account::increment_sequence_number</a>(owner);
    addr
}
</code></pre>



</details>

<a id="0x1_genesis_create_employee_validators"></a>

## Function `create_employee_validators`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_employee_validators">create_employee_validators</a>(employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">genesis::EmployeeAccountMap</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_employee_validators">create_employee_validators</a>(
    employee_vesting_start: u64,
    employee_vesting_period_duration: u64,
    employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a>&gt;,
) {
    <b>let</b> unique_accounts = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&employees, |employee_group| {
        <b>let</b> j = 0;
        <b>let</b> employee_group: &<a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a> = employee_group;
        <b>let</b> num_employees_in_group = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&employee_group.accounts);

        <b>let</b> buy_ins = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>();

        <b>while</b> (j &lt; num_employees_in_group) {
            <b>let</b> <a href="account.md#0x1_account">account</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&employee_group.accounts, j);
            <b>assert</b>!(
                !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&unique_accounts, <a href="account.md#0x1_account">account</a>),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>),
            );
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> unique_accounts, *<a href="account.md#0x1_account">account</a>);

            <b>let</b> employee = <a href="create_signer.md#0x1_create_signer">create_signer</a>(*<a href="account.md#0x1_account">account</a>);
            <b>let</b> total = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(*<a href="account.md#0x1_account">account</a>);
            <b>let</b> coins = <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;SupraCoin&gt;(&employee, total);
            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> buy_ins, *<a href="account.md#0x1_account">account</a>, coins);

            j = j + 1;
        };

        <b>let</b> j = 0;
        <b>let</b> num_vesting_events = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&employee_group.vesting_schedule_numerator);
        <b>let</b> schedule = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

        <b>while</b> (j &lt; num_vesting_events) {
            <b>let</b> numerator = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&employee_group.vesting_schedule_numerator, j);
            <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_rational">fixed_point32::create_from_rational</a>(
                *numerator,
                employee_group.vesting_schedule_denominator
            );
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> schedule, <a href="event.md#0x1_event">event</a>);

            j = j + 1;
        };

        <b>let</b> vesting_schedule = <a href="vesting.md#0x1_vesting_create_vesting_schedule">vesting::create_vesting_schedule</a>(
            schedule,
            employee_vesting_start,
            employee_vesting_period_duration,
        );

        <b>let</b> admin = employee_group.validator.validator_config.owner_address;
        <b>let</b> admin_signer = &<a href="create_signer.md#0x1_create_signer">create_signer</a>(admin);
        <b>let</b> contract_address = <a href="vesting.md#0x1_vesting_create_vesting_contract">vesting::create_vesting_contract</a>(
            admin_signer,
            &employee_group.accounts,
            buy_ins,
            vesting_schedule,
            admin,
            employee_group.validator.validator_config.operator_address,
            employee_group.validator.validator_config.voter_address,
            employee_group.validator.commission_percentage,
            x"",
        );
        <b>let</b> pool_address = <a href="vesting.md#0x1_vesting_stake_pool_address">vesting::stake_pool_address</a>(contract_address);

        <b>if</b> (employee_group.beneficiary_resetter != @0x0) {
            <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">vesting::set_beneficiary_resetter</a>(admin_signer, contract_address, employee_group.beneficiary_resetter);
        };

        <b>let</b> validator = &employee_group.validator.validator_config;
        <b>assert</b>!(
            <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.owner_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),
        );
        <b>assert</b>!(
            <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.operator_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),
        );
        <b>assert</b>!(
            <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.voter_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),
        );
        <b>if</b> (employee_group.validator.join_during_genesis) {
            <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address, validator);
        };
    });
}
</code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators_with_commission"></a>

## Function `create_initialize_validators_with_commission`

DEPRECATED



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, use_staking_contract: bool, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    use_staking_contract: bool,
    validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a>&gt;,
) {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&validators, |validator| {
        <b>let</b> validator: &<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a> = validator;
        <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(supra_framework, validator, use_staking_contract);
    });
}
</code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators"></a>

## Function `create_initialize_validators`

DEPRECATED

Sets up the initial validator set for the network.
The validator "owner" accounts, and their authentication
Addresses (and keys) are encoded in the <code>owners</code>
Each validator signs consensus messages with the private key corresponding to the Ed25519
public key in <code>consensus_pubkeys</code>.
Finally, each validator must specify the network address
(see types/src/network_address/mod.rs) for itself and its full nodes.

Network address fields are a vector per account, where each entry is a vector of addresses
encoded in a single BCS byte array.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators">create_initialize_validators</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators">create_initialize_validators</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a>&gt;) {
    <b>let</b> validators_with_commission = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_reverse">vector::for_each_reverse</a>(validators, |validator| {
        <b>let</b> validator_with_commission = <a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a> {
            validator_config: validator,
            commission_percentage: 0,
            join_during_genesis: <b>true</b>,
        };
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validators_with_commission, validator_with_commission);
    });

    <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(supra_framework, <b>false</b>, validators_with_commission);
}
</code></pre>



</details>

<a id="0x1_genesis_create_initialize_validator"></a>

## Function `create_initialize_validator`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, commission_config: &<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>, use_staking_contract: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    commission_config: &<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a>,
    use_staking_contract: bool,
) {
    <b>let</b> validator = &commission_config.validator_config;

    <b>let</b> owner = &<a href="genesis.md#0x1_genesis_create_account">create_account</a>(supra_framework, validator.owner_address, validator.stake_amount);
    <a href="genesis.md#0x1_genesis_create_account">create_account</a>(supra_framework, validator.operator_address, 0);
    <a href="genesis.md#0x1_genesis_create_account">create_account</a>(supra_framework, validator.voter_address, 0);

    // Initialize the <a href="stake.md#0x1_stake">stake</a> pool and join the validator set.
    <b>let</b> pool_address = <b>if</b> (use_staking_contract) {
        <a href="staking_contract.md#0x1_staking_contract_create_staking_contract">staking_contract::create_staking_contract</a>(
            owner,
            validator.operator_address,
            validator.voter_address,
            validator.stake_amount,
            commission_config.commission_percentage,
            x"",
        );
        <a href="staking_contract.md#0x1_staking_contract_stake_pool_address">staking_contract::stake_pool_address</a>(validator.owner_address, validator.operator_address)
    } <b>else</b> {
        <a href="stake.md#0x1_stake_initialize_stake_owner">stake::initialize_stake_owner</a>(
            owner,
            validator.stake_amount,
            validator.operator_address,
            validator.voter_address,
        );
        validator.owner_address
    };

    <b>if</b> (commission_config.join_during_genesis) {
        <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address, validator);
    };
}
</code></pre>



</details>

<a id="0x1_genesis_create_pbo_delegation_pools"></a>

## Function `create_pbo_delegation_pools`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_pbo_delegation_pools">create_pbo_delegation_pools</a>(pbo_delegator_configs: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">genesis::PboDelegatorConfiguration</a>&gt;, delegation_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_pbo_delegation_pools">create_pbo_delegation_pools</a>(
    pbo_delegator_configs: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">PboDelegatorConfiguration</a>&gt;,
    delegation_percentage: u64,
) {
    <b>let</b> unique_accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>assert</b>!(
        delegation_percentage != 0 && delegation_percentage &lt;= 100,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EPERCENTAGE_INVALID">EPERCENTAGE_INVALID</a>)
    );
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&pbo_delegator_configs, |pbo_delegator_config| {
        <b>let</b> pbo_delegator_config: &<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">PboDelegatorConfiguration</a> = pbo_delegator_config;
        <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&unique_accounts, &pbo_delegator_config.delegator_config.owner_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>));
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> unique_accounts, pbo_delegator_config.delegator_config.owner_address);
        <a href="genesis.md#0x1_genesis_create_pbo_delegation_pool">create_pbo_delegation_pool</a>(pbo_delegator_config, delegation_percentage);
    });
}
</code></pre>



</details>

<a id="0x1_genesis_create_pbo_delegation_pool"></a>

## Function `create_pbo_delegation_pool`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_pbo_delegation_pool">create_pbo_delegation_pool</a>(pbo_delegator_config: &<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">genesis::PboDelegatorConfiguration</a>, delegation_percentage: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_pbo_delegation_pool">create_pbo_delegation_pool</a>(
    pbo_delegator_config: &<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">PboDelegatorConfiguration</a>,
    delegation_percentage: u64,
) {
    <b>assert</b>!(
        delegation_percentage != 0 && delegation_percentage &lt;= 100,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EPERCENTAGE_INVALID">EPERCENTAGE_INVALID</a>)
    );
    <b>let</b> unique_accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&pbo_delegator_config.delegator_config.delegator_addresses, |delegator_address| {
        <b>let</b> delegator_address: &<b>address</b> = delegator_address;
        <b>assert</b>!(
            !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&unique_accounts, delegator_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>),
        );
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> unique_accounts, *delegator_address);
    });
    <b>let</b> owner_signer = <a href="create_signer.md#0x1_create_signer">create_signer</a>(pbo_delegator_config.delegator_config.owner_address);
    // get a list of delegator addresses, withdraw the <a href="coin.md#0x1_coin">coin</a> from them and merge them into a single <a href="account.md#0x1_account">account</a>
    <b>let</b> delegator_addresses = pbo_delegator_config.delegator_config.delegator_addresses;
    <b>let</b> coinInitialization = <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;SupraCoin&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(delegator_addresses, |delegator_address| {
        <b>let</b> delegator = &<a href="create_signer.md#0x1_create_signer">create_signer</a>(delegator_address);
        <b>let</b> total = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(delegator_address);
        <b>let</b> withdraw_amount = total * delegation_percentage / 100;
        <b>let</b> coins = <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;SupraCoin&gt;(delegator, withdraw_amount);
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> coinInitialization, coins);
    });
    <a href="pbo_delegation_pool.md#0x1_pbo_delegation_pool_initialize_delegation_pool">pbo_delegation_pool::initialize_delegation_pool</a>(
        &owner_signer,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(pbo_delegator_config.multisig_admin),
        pbo_delegator_config.delegator_config.validator.commission_percentage,
        pbo_delegator_config.delegator_config.delegation_pool_creation_seed,
        pbo_delegator_config.delegator_config.delegator_addresses,
        pbo_delegator_config.delegator_config.delegator_stakes,
        coinInitialization,
        pbo_delegator_config.unlock_schedule_numerators,
        pbo_delegator_config.unlock_schedule_denominator,
        pbo_delegator_config.unlock_startup_time_from_now + <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        pbo_delegator_config.unlock_period_duration,
    );

    <b>let</b> pool_address = <a href="pbo_delegation_pool.md#0x1_pbo_delegation_pool_get_owned_pool_address">pbo_delegation_pool::get_owned_pool_address</a>(
        pbo_delegator_config.delegator_config.owner_address
    );
    <b>let</b> validator = pbo_delegator_config.delegator_config.validator.validator_config;
    <a href="pbo_delegation_pool.md#0x1_pbo_delegation_pool_set_operator">pbo_delegation_pool::set_operator</a>(&owner_signer, validator.operator_address);
    <a href="pbo_delegation_pool.md#0x1_pbo_delegation_pool_set_delegated_voter">pbo_delegation_pool::set_delegated_voter</a>(&owner_signer, validator.voter_address);
    <a href="genesis.md#0x1_genesis_assert_validator_addresses_check">assert_validator_addresses_check</a>(&validator);

    <b>if</b> (pbo_delegator_config.delegator_config.validator.join_during_genesis) {
        <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address, &validator);
    };
}
</code></pre>



</details>

<a id="0x1_genesis_assert_validator_addresses_check"></a>

## Function `assert_validator_addresses_check`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_assert_validator_addresses_check">assert_validator_addresses_check</a>(validator: &<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_assert_validator_addresses_check">assert_validator_addresses_check</a>(validator: &<a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a>) {
    <b>assert</b>!(
        <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.owner_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),
    );
    <b>assert</b>!(
        <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.operator_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),
    );
    <b>assert</b>!(
        <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.voter_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),
    );
}
</code></pre>



</details>

<a id="0x1_genesis_create_vesting_without_staking_pools"></a>

## Function `create_vesting_without_staking_pools`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_vesting_without_staking_pools">create_vesting_without_staking_pools</a>(vesting_pool_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_VestingPoolsMap">genesis::VestingPoolsMap</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_vesting_without_staking_pools">create_vesting_without_staking_pools</a>(
    vesting_pool_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_VestingPoolsMap">VestingPoolsMap</a>&gt;
) {
    <b>let</b> unique_accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&vesting_pool_map, |pool_config|{
        <b>let</b> pool_config: &<a href="genesis.md#0x1_genesis_VestingPoolsMap">VestingPoolsMap</a> = pool_config;
        <b>let</b> schedule = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
        <b>let</b> schedule_length = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&pool_config.vesting_numerators);
        <b>assert</b>!(schedule_length != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EVESTING_SCHEDULE_IS_ZERO">EVESTING_SCHEDULE_IS_ZERO</a>));
        <b>assert</b>!(pool_config.vesting_denominator != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EDENOMINATOR_IS_ZERO">EDENOMINATOR_IS_ZERO</a>));
        <b>assert</b>!(pool_config.vpool_locking_percentage != 0 && pool_config.vpool_locking_percentage &lt;= 100,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EPERCENTAGE_INVALID">EPERCENTAGE_INVALID</a>));
        //check the sum of numerator are &lt;= denominator.
        <b>let</b> sum = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_fold">vector::fold</a>(pool_config.vesting_numerators, 0, |acc, x| acc + x);
        // Check that total of all fraction in `vesting_schedule` is not greater than 1
        <b>assert</b>!(sum &lt;= pool_config.vesting_denominator,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_ENUMERATOR_GREATER_THAN_DENOMINATOR">ENUMERATOR_GREATER_THAN_DENOMINATOR</a>));
        //<b>assert</b> that withdrawal_address is registered <b>to</b> receive SupraCoin
        <b>assert</b>!(
            <a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;SupraCoin&gt;(pool_config.withdrawal_address),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_NOT_REGISTERED_FOR_COIN">EACCOUNT_NOT_REGISTERED_FOR_COIN</a>)
        );
        //assertion on admin_address?
        <b>let</b> admin = <a href="create_signer.md#0x1_create_signer">create_signer</a>(pool_config.admin_address);

        //Create the <a href="vesting.md#0x1_vesting">vesting</a> schedule
        <b>let</b> j = 0;
        <b>while</b> (j &lt; schedule_length) {
            <b>let</b> numerator = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&pool_config.vesting_numerators, j);
            <b>assert</b>!(numerator != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_ENUMERATOR_IS_ZERO">ENUMERATOR_IS_ZERO</a>));
            <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_rational">fixed_point32::create_from_rational</a>(numerator, pool_config.vesting_denominator);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> schedule, <a href="event.md#0x1_event">event</a>);
            j = j + 1;
        };

        <b>let</b> vesting_schedule = <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_schedule">vesting_without_staking::create_vesting_schedule</a>(
            schedule,
            <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() + pool_config.cliff_period_in_seconds,
            pool_config.period_duration_in_seconds,
        );

        <b>let</b> buy_ins = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>();
        <b>let</b> num_shareholders = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&pool_config.shareholders);
        <b>assert</b>!(num_shareholders != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="genesis.md#0x1_genesis_ENO_SHAREHOLDERS">ENO_SHAREHOLDERS</a>));
        <b>let</b> j = 0;
        <b>while</b> (j &lt; num_shareholders) {
            <b>let</b> shareholder = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&pool_config.shareholders, j);
            <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&unique_accounts, &shareholder), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>));
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> unique_accounts, shareholder);
            <b>let</b> shareholder_signer = <a href="create_signer.md#0x1_create_signer">create_signer</a>(shareholder);
            <b>let</b> amount = <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;SupraCoin&gt;(shareholder);
            <b>let</b> amount_to_extract = (amount * (pool_config.vpool_locking_percentage <b>as</b> u64)) / 100;
            <b>let</b> coin_share = <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;SupraCoin&gt;(&shareholder_signer, amount_to_extract);
            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> buy_ins, shareholder, coin_share);
            j = j + 1;
        };
        <a href="vesting_without_staking.md#0x1_vesting_without_staking_create_vesting_contract">vesting_without_staking::create_vesting_contract</a>(
            &admin,
            buy_ins,
            vesting_schedule,
            pool_config.withdrawal_address,
            <a href="genesis.md#0x1_genesis_VESTING_CONTRACT_SEED">VESTING_CONTRACT_SEED</a>
        );
    });
}
</code></pre>



</details>

<a id="0x1_genesis_initialize_validator"></a>

## Function `initialize_validator`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address: <b>address</b>, validator: &<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address: <b>address</b>, validator: &<a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a>) {
    <b>let</b> operator = &<a href="create_signer.md#0x1_create_signer">create_signer</a>(validator.operator_address);

    <a href="stake.md#0x1_stake_rotate_consensus_key_genesis">stake::rotate_consensus_key_genesis</a>(
        operator,
        pool_address,
        validator.consensus_pubkey,
    );
    <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">stake::update_network_and_fullnode_addresses</a>(
        operator,
        pool_address,
        validator.network_addresses,
        validator.full_node_network_addresses,
    );
    <a href="stake.md#0x1_stake_join_validator_set_internal">stake::join_validator_set_internal</a>(operator, pool_address);
}
</code></pre>



</details>

<a id="0x1_genesis_set_genesis_end"></a>

## Function `set_genesis_end`

The last step of genesis.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    // Destroy the mint <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> owned by the framework <a href="account.md#0x1_account">account</a>. The <a href="stake.md#0x1_stake">stake</a> and <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a>
    // modules should be the only holders of this <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>, which they will <b>use</b> <b>to</b>
    // mint <a href="block.md#0x1_block">block</a> rewards and storage refunds, respectively.
    <a href="supra_coin.md#0x1_supra_coin_destroy_mint_cap">supra_coin::destroy_mint_cap</a>(supra_framework);
    <a href="stake.md#0x1_stake_on_new_epoch">stake::on_new_epoch</a>();
    <a href="chain_status.md#0x1_chain_status_set_genesis_end">chain_status::set_genesis_end</a>(supra_framework);
}
</code></pre>



</details>

<a id="0x1_genesis_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>#[verify_only]
<b>fun</b> <a href="genesis.md#0x1_genesis_initialize_for_verification">initialize_for_verification</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="supra_config.md#0x1_supra_config">supra_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="evm_config.md#0x1_evm_config">evm_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, voting_duration_secs: u64, supra_min_voting_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">genesis::AccountMap</a>&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">genesis::EmployeeAccountMap</a>&gt;, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_for_verification">initialize_for_verification</a>(
    <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,
    initial_version: u64,
    <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="supra_config.md#0x1_supra_config">supra_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="evm_config.md#0x1_evm_config">evm_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    epoch_interval_microsecs: u64,
    minimum_stake: u64,
    maximum_stake: u64,
    recurring_lockup_duration_secs: u64,
    allow_validator_set_change: bool,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
    voting_power_increase_limit: u64,
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    // min_voting_threshold: u128,
    // required_proposer_stake: u64,
    voting_duration_secs: u64,
    supra_min_voting_threshold: u64,
    voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a>&gt;,
    employee_vesting_start: u64,
    employee_vesting_period_duration: u64,
    employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a>&gt;,
    validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a>&gt;
) {
    <a href="genesis.md#0x1_genesis_initialize">initialize</a>(
        <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>,
        <a href="chain_id.md#0x1_chain_id">chain_id</a>,
        initial_version,
        <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>,
        <a href="execution_config.md#0x1_execution_config">execution_config</a>,
        <a href="supra_config.md#0x1_supra_config">supra_config</a>,
        epoch_interval_microsecs,
        minimum_stake,
        maximum_stake,
        recurring_lockup_duration_secs,
        allow_validator_set_change,
        rewards_rate,
        rewards_rate_denominator,
        voting_power_increase_limit,
        0,
        <a href="evm_config.md#0x1_evm_config">evm_config</a>,

    );
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_change_feature_flags_for_verification">features::change_feature_flags_for_verification</a>(supra_framework, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 2, 11], <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <a href="genesis.md#0x1_genesis_initialize_supra_coin">initialize_supra_coin</a>(supra_framework);
    <a href="supra_governance.md#0x1_supra_governance_initialize_for_verification">supra_governance::initialize_for_verification</a>(
        supra_framework,
        // min_voting_threshold,
        // required_proposer_stake,
        voting_duration_secs,
        supra_min_voting_threshold,
        voters,
    );
    <a href="genesis.md#0x1_genesis_create_accounts">create_accounts</a>(supra_framework, accounts);
    <a href="genesis.md#0x1_genesis_create_employee_validators">create_employee_validators</a>(employee_vesting_start, employee_vesting_period_duration, employees);
    <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(supra_framework, <b>true</b>, validators);
    <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(supra_framework);
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
<td>All the core resources and modules should be created during genesis and owned by the Supra framework account.</td>
<td>Critical</td>
<td>Resources created during genesis initialization: GovernanceResponsbility, ConsensusConfig, ExecutionConfig, Version, SetVersionCapability, ValidatorSet, ValidatorPerformance, StakingConfig, StorageGasConfig, StorageGas, GasScheduleV2, AggregatorFactory, SupplyConfig, ChainId, Configuration, BlockResource, StateStorageUsage, CurrentTimeMicroseconds. If some of the resources were to be owned by a malicious account, it could lead to the compromise of the chain, as these are core resources. It should be formally verified by a post condition to ensure that all the critical resources are owned by the Supra framework.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Addresses ranging from 0x0 - 0xa should be reserved for the framework and part of supra governance.</td>
<td>Critical</td>
<td>The function genesis::initialize calls account::create_framework_reserved_account for addresses 0x0, 0x2, 0x3, 0x4, ..., 0xa which creates an account and authentication_key for them. This should be formally verified by ensuring that at the beginning of the genesis::initialize function no Account resource exists for the reserved addresses, and at the end of the function, an Account resource exists.</td>
<td>Formally verified via <a href="#high-level-req-2">initialize</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The Supra coin should be initialized during genesis and only the Supra framework account should own the mint and burn capabilities for the SUPRA token.</td>
<td>Critical</td>
<td>Both mint and burn capabilities are wrapped inside the stake::SupraCoinCapabilities and transaction_fee::SupraCoinCapabilities resources which are stored under the supra framework account.</td>
<td>Formally verified via <a href="#high-level-req-3">initialize_supra_coin</a>.</td>
</tr>

<tr>
<td>4</td>
<td>An initial set of validators should exist before the end of genesis.</td>
<td>Low</td>
<td>To ensure that there will be a set of validators available to validate the genesis block, the length of the ValidatorSet.active_validators vector should be > 0.</td>
<td>Formally verified via <a href="#high-level-req-4">set_genesis_end</a>.</td>
</tr>

<tr>
<td>5</td>
<td>The end of genesis should be marked on chain.</td>
<td>Low</td>
<td>The end of genesis is marked, on chain, via the chain_status::GenesisEndMarker resource. The ownership of this resource marks the operating state of the chain.</td>
<td>Formally verified via <a href="#high-level-req-5">set_genesis_end</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize">initialize</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="supra_config.md#0x1_supra_config">supra_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, genesis_timestamp_in_microseconds: u64, <a href="evm_config.md#0x1_evm_config">evm_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="genesis.md#0x1_genesis_InitalizeRequires">InitalizeRequires</a>;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x0);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x2);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x3);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x4);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x5);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x6);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x7);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x8);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x9);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0xa);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x0);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x2);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x3);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x4);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x5);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x6);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x7);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x8);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x9);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0xa);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">supra_governance::GovernanceResponsbility</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">consensus_config::ConsensusConfig</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">execution_config::ExecutionConfig</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="supra_config.md#0x1_supra_config_SupraConfig">supra_config::SupraConfig</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="version.md#0x1_version_Version">version::Version</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">storage_gas::StorageGas</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">gas_schedule::GasScheduleV2</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">coin::SupplyConfig</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">chain_id::ChainId</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">reconfiguration::Configuration</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">block::BlockResource</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">state_storage::StateStorageUsage</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">version::SetVersionCapability</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@supra_framework);
</code></pre>



<a id="@Specification_1_initialize_supra_coin"></a>

### Function `initialize_supra_coin`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_supra_coin">initialize_supra_coin</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>requires</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_SupraCoinCapabilities">stake::SupraCoinCapabilities</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_SupraCoinCapabilities">stake::SupraCoinCapabilities</a>&gt;(@supra_framework);
<b>requires</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">transaction_fee::SupraCoinCapabilities</a>&gt;(@supra_framework);
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">transaction_fee::SupraCoinCapabilities</a>&gt;(@supra_framework);
</code></pre>



<a id="@Specification_1_create_initialize_validators_with_commission"></a>

### Function `create_initialize_validators_with_commission`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, use_staking_contract: bool, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;
<b>include</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a>;
<b>include</b> <a href="supra_coin.md#0x1_supra_coin_ExistsSupraCoin">supra_coin::ExistsSupraCoin</a>;
</code></pre>



<a id="@Specification_1_create_initialize_validators"></a>

### Function `create_initialize_validators`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators">create_initialize_validators</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;
<b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;
<b>include</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a>;
<b>include</b> <a href="supra_coin.md#0x1_supra_coin_ExistsSupraCoin">supra_coin::ExistsSupraCoin</a>;
</code></pre>



<a id="@Specification_1_create_initialize_validator"></a>

### Function `create_initialize_validator`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, commission_config: &<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>, use_staking_contract: bool)
</code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;
</code></pre>



<a id="@Specification_1_create_pbo_delegation_pools"></a>

### Function `create_pbo_delegation_pools`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_pbo_delegation_pools">create_pbo_delegation_pools</a>(pbo_delegator_configs: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">genesis::PboDelegatorConfiguration</a>&gt;, delegation_percentage: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_create_pbo_delegation_pool"></a>

### Function `create_pbo_delegation_pool`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_pbo_delegation_pool">create_pbo_delegation_pool</a>(pbo_delegator_config: &<a href="genesis.md#0x1_genesis_PboDelegatorConfiguration">genesis::PboDelegatorConfiguration</a>, delegation_percentage: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_assert_validator_addresses_check"></a>

### Function `assert_validator_addresses_check`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_assert_validator_addresses_check">assert_validator_addresses_check</a>(validator: &<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_create_vesting_without_staking_pools"></a>

### Function `create_vesting_without_staking_pools`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_vesting_without_staking_pools">create_vesting_without_staking_pools</a>(vesting_pool_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_VestingPoolsMap">genesis::VestingPoolsMap</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_genesis_InitalizeRequires"></a>


<pre><code><b>schema</b> <a href="genesis.md#0x1_genesis_InitalizeRequires">InitalizeRequires</a> {
    <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <b>requires</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@supra_framework);
    <b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
    <b>requires</b> len(<a href="execution_config.md#0x1_execution_config">execution_config</a>) &gt; 0;
    <b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@supra_framework);
    <b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@supra_framework);
    <b>requires</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;SupraCoin&gt;&gt;(@supra_framework);
    <b>include</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a>;
    <b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;
}
</code></pre>




<a id="0x1_genesis_CompareTimeRequires"></a>


<pre><code><b>schema</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a> {
    <b>let</b> staking_rewards_config = <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@supra_framework);
    <b>requires</b> staking_rewards_config.last_rewards_rate_period_start_in_secs &lt;= <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
}
</code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> delegate_invariants_to_caller;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>requires</b> len(<b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@supra_framework).active_validators) &gt;= 1;
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
<b>let</b> addr = std::signer::address_of(supra_framework);
<b>aborts_if</b> addr != @supra_framework;
<b>aborts_if</b> <b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">chain_status::GenesisEndMarker</a>&gt;(@supra_framework);
<b>ensures</b> <b>global</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">chain_status::GenesisEndMarker</a>&gt;(@supra_framework) == <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">chain_status::GenesisEndMarker</a> {};
</code></pre>



<a id="@Specification_1_initialize_for_verification"></a>

### Function `initialize_for_verification`


<pre><code>#[verify_only]
<b>fun</b> <a href="genesis.md#0x1_genesis_initialize_for_verification">initialize_for_verification</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="supra_config.md#0x1_supra_config">supra_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="evm_config.md#0x1_evm_config">evm_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, voting_duration_secs: u64, supra_min_voting_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">genesis::AccountMap</a>&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">genesis::EmployeeAccountMap</a>&gt;, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>include</b> <a href="genesis.md#0x1_genesis_InitalizeRequires">InitalizeRequires</a>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
