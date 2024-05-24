
<a id="0x1_genesis"></a>

# Module `0x1::genesis`



-  [Struct `AccountMap`](#0x1_genesis_AccountMap)
-  [Struct `EmployeeAccountMap`](#0x1_genesis_EmployeeAccountMap)
-  [Struct `ValidatorConfiguration`](#0x1_genesis_ValidatorConfiguration)
-  [Struct `ValidatorConfigurationWithCommission`](#0x1_genesis_ValidatorConfigurationWithCommission)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_genesis_initialize)
-  [Function `initialize_aptos_coin`](#0x1_genesis_initialize_aptos_coin)
-  [Function `initialize_core_resources_and_aptos_coin`](#0x1_genesis_initialize_core_resources_and_aptos_coin)
-  [Function `create_accounts`](#0x1_genesis_create_accounts)
-  [Function `create_account`](#0x1_genesis_create_account)
-  [Function `create_employee_validators`](#0x1_genesis_create_employee_validators)
-  [Function `create_initialize_validators_with_commission`](#0x1_genesis_create_initialize_validators_with_commission)
-  [Function `create_initialize_validators`](#0x1_genesis_create_initialize_validators)
-  [Function `create_initialize_validator`](#0x1_genesis_create_initialize_validator)
-  [Function `initialize_validator`](#0x1_genesis_initialize_validator)
-  [Function `set_genesis_end`](#0x1_genesis_set_genesis_end)
-  [Function `initialize_for_verification`](#0x1_genesis_initialize_for_verification)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initialize_aptos_coin`](#@Specification_1_initialize_aptos_coin)
    -  [Function `create_initialize_validators_with_commission`](#@Specification_1_create_initialize_validators_with_commission)
    -  [Function `create_initialize_validators`](#@Specification_1_create_initialize_validators)
    -  [Function `create_initialize_validator`](#@Specification_1_create_initialize_validator)
    -  [Function `set_genesis_end`](#@Specification_1_set_genesis_end)
    -  [Function `initialize_for_verification`](#@Specification_1_initialize_for_verification)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aggregator_factory.md#0x1_aggregator_factory">0x1::aggregator_factory</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="aptos_governance.md#0x1_aptos_governance">0x1::aptos_governance</a>;<br /><b>use</b> <a href="block.md#0x1_block">0x1::block</a>;<br /><b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;<br /><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;<br /><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="execution_config.md#0x1_execution_config">0x1::execution_config</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32">0x1::fixed_point32</a>;<br /><b>use</b> <a href="gas_schedule.md#0x1_gas_schedule">0x1::gas_schedule</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;<br /><b>use</b> <a href="staking_contract.md#0x1_staking_contract">0x1::staking_contract</a>;<br /><b>use</b> <a href="state_storage.md#0x1_state_storage">0x1::state_storage</a>;<br /><b>use</b> <a href="storage_gas.md#0x1_storage_gas">0x1::storage_gas</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;<br /><b>use</b> <a href="transaction_validation.md#0x1_transaction_validation">0x1::transaction_validation</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /><b>use</b> <a href="version.md#0x1_version">0x1::version</a>;<br /><b>use</b> <a href="vesting.md#0x1_vesting">0x1::vesting</a>;<br /></code></pre>



<a id="0x1_genesis_AccountMap"></a>

## Struct `AccountMap`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a> <b>has</b> drop<br /></code></pre>



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

<a id="0x1_genesis_EmployeeAccountMap"></a>

## Struct `EmployeeAccountMap`



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a> <b>has</b> <b>copy</b>, drop<br /></code></pre>



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



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a> <b>has</b> <b>copy</b>, drop<br /></code></pre>



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
<code>proof_of_possession: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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



<pre><code><b>struct</b> <a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a> <b>has</b> <b>copy</b>, drop<br /></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_genesis_EACCOUNT_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_genesis_EDUPLICATE_ACCOUNT"></a>



<pre><code><b>const</b> <a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_genesis_initialize"></a>

## Function `initialize`

Genesis step 1: Initialize aptos framework account and core modules on chain.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize">initialize</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize">initialize</a>(<br />    <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />    initial_version: u64,<br />    <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    epoch_interval_microsecs: u64,<br />    minimum_stake: u64,<br />    maximum_stake: u64,<br />    recurring_lockup_duration_secs: u64,<br />    allow_validator_set_change: bool,<br />    rewards_rate: u64,<br />    rewards_rate_denominator: u64,<br />    voting_power_increase_limit: u64,<br />) &#123;<br />    // Initialize the aptos framework <a href="account.md#0x1_account">account</a>. This is the <a href="account.md#0x1_account">account</a> <b>where</b> system resources and modules will be<br />    // deployed <b>to</b>. This will be entirely managed by on&#45;chain governance and no entities have the key or privileges<br />    // <b>to</b> <b>use</b> this <a href="account.md#0x1_account">account</a>.<br />    <b>let</b> (aptos_framework_account, aptos_framework_signer_cap) &#61; <a href="account.md#0x1_account_create_framework_reserved_account">account::create_framework_reserved_account</a>(@aptos_framework);<br />    // Initialize <a href="account.md#0x1_account">account</a> configs on aptos framework <a href="account.md#0x1_account">account</a>.<br />    <a href="account.md#0x1_account_initialize">account::initialize</a>(&amp;aptos_framework_account);<br /><br />    <a href="transaction_validation.md#0x1_transaction_validation_initialize">transaction_validation::initialize</a>(<br />        &amp;aptos_framework_account,<br />        b&quot;script_prologue&quot;,<br />        b&quot;module_prologue&quot;,<br />        b&quot;multi_agent_script_prologue&quot;,<br />        b&quot;epilogue&quot;,<br />    );<br /><br />    // Give the decentralized on&#45;chain governance control over the core framework <a href="account.md#0x1_account">account</a>.<br />    <a href="aptos_governance.md#0x1_aptos_governance_store_signer_cap">aptos_governance::store_signer_cap</a>(&amp;aptos_framework_account, @aptos_framework, aptos_framework_signer_cap);<br /><br />    // put reserved framework reserved accounts under aptos governance<br />    <b>let</b> framework_reserved_addresses &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;[@0x2, @0x3, @0x4, @0x5, @0x6, @0x7, @0x8, @0x9, @0xa];<br />    <b>while</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;framework_reserved_addresses)) &#123;<br />        <b>let</b> <b>address</b> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>&lt;<b>address</b>&gt;(&amp;<b>mut</b> framework_reserved_addresses);<br />        <b>let</b> (_, framework_signer_cap) &#61; <a href="account.md#0x1_account_create_framework_reserved_account">account::create_framework_reserved_account</a>(<b>address</b>);<br />        <a href="aptos_governance.md#0x1_aptos_governance_store_signer_cap">aptos_governance::store_signer_cap</a>(&amp;aptos_framework_account, <b>address</b>, framework_signer_cap);<br />    &#125;;<br /><br />    <a href="consensus_config.md#0x1_consensus_config_initialize">consensus_config::initialize</a>(&amp;aptos_framework_account, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>);<br />    <a href="execution_config.md#0x1_execution_config_set">execution_config::set</a>(&amp;aptos_framework_account, <a href="execution_config.md#0x1_execution_config">execution_config</a>);<br />    <a href="version.md#0x1_version_initialize">version::initialize</a>(&amp;aptos_framework_account, initial_version);<br />    <a href="stake.md#0x1_stake_initialize">stake::initialize</a>(&amp;aptos_framework_account);<br />    <a href="staking_config.md#0x1_staking_config_initialize">staking_config::initialize</a>(<br />        &amp;aptos_framework_account,<br />        minimum_stake,<br />        maximum_stake,<br />        recurring_lockup_duration_secs,<br />        allow_validator_set_change,<br />        rewards_rate,<br />        rewards_rate_denominator,<br />        voting_power_increase_limit,<br />    );<br />    <a href="storage_gas.md#0x1_storage_gas_initialize">storage_gas::initialize</a>(&amp;aptos_framework_account);<br />    <a href="gas_schedule.md#0x1_gas_schedule_initialize">gas_schedule::initialize</a>(&amp;aptos_framework_account, <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>);<br /><br />    // Ensure we can create aggregators for supply, but not enable it for common <b>use</b> just yet.<br />    <a href="aggregator_factory.md#0x1_aggregator_factory_initialize_aggregator_factory">aggregator_factory::initialize_aggregator_factory</a>(&amp;aptos_framework_account);<br />    <a href="coin.md#0x1_coin_initialize_supply_config">coin::initialize_supply_config</a>(&amp;aptos_framework_account);<br /><br />    <a href="chain_id.md#0x1_chain_id_initialize">chain_id::initialize</a>(&amp;aptos_framework_account, <a href="chain_id.md#0x1_chain_id">chain_id</a>);<br />    <a href="reconfiguration.md#0x1_reconfiguration_initialize">reconfiguration::initialize</a>(&amp;aptos_framework_account);<br />    <a href="block.md#0x1_block_initialize">block::initialize</a>(&amp;aptos_framework_account, epoch_interval_microsecs);<br />    <a href="state_storage.md#0x1_state_storage_initialize">state_storage::initialize</a>(&amp;aptos_framework_account);<br />    <a href="timestamp.md#0x1_timestamp_set_time_has_started">timestamp::set_time_has_started</a>(&amp;aptos_framework_account);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_initialize_aptos_coin"></a>

## Function `initialize_aptos_coin`

Genesis step 2: Initialize Aptos coin.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_aptos_coin">initialize_aptos_coin</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_aptos_coin">initialize_aptos_coin</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>let</b> (burn_cap, mint_cap) &#61; <a href="aptos_coin.md#0x1_aptos_coin_initialize">aptos_coin::initialize</a>(aptos_framework);<br /><br />    <a href="coin.md#0x1_coin_create_coin_conversion_map">coin::create_coin_conversion_map</a>(aptos_framework);<br />    <a href="coin.md#0x1_coin_create_pairing">coin::create_pairing</a>&lt;AptosCoin&gt;(aptos_framework);<br /><br />    // Give <a href="stake.md#0x1_stake">stake</a> <b>module</b> MintCapability&lt;AptosCoin&gt; so it can mint rewards.<br />    <a href="stake.md#0x1_stake_store_aptos_coin_mint_cap">stake::store_aptos_coin_mint_cap</a>(aptos_framework, mint_cap);<br />    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> BurnCapability&lt;AptosCoin&gt; so it can burn gas.<br />    <a href="transaction_fee.md#0x1_transaction_fee_store_aptos_coin_burn_cap">transaction_fee::store_aptos_coin_burn_cap</a>(aptos_framework, burn_cap);<br />    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> MintCapability&lt;AptosCoin&gt; so it can mint refunds.<br />    <a href="transaction_fee.md#0x1_transaction_fee_store_aptos_coin_mint_cap">transaction_fee::store_aptos_coin_mint_cap</a>(aptos_framework, mint_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_initialize_core_resources_and_aptos_coin"></a>

## Function `initialize_core_resources_and_aptos_coin`

Only called for testnets and e2e tests.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_core_resources_and_aptos_coin">initialize_core_resources_and_aptos_coin</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, core_resources_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_core_resources_and_aptos_coin">initialize_core_resources_and_aptos_coin</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    core_resources_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) &#123;<br />    <b>let</b> (burn_cap, mint_cap) &#61; <a href="aptos_coin.md#0x1_aptos_coin_initialize">aptos_coin::initialize</a>(aptos_framework);<br /><br />    <a href="coin.md#0x1_coin_create_coin_conversion_map">coin::create_coin_conversion_map</a>(aptos_framework);<br />    <a href="coin.md#0x1_coin_create_pairing">coin::create_pairing</a>&lt;AptosCoin&gt;(aptos_framework);<br /><br />    // Give <a href="stake.md#0x1_stake">stake</a> <b>module</b> MintCapability&lt;AptosCoin&gt; so it can mint rewards.<br />    <a href="stake.md#0x1_stake_store_aptos_coin_mint_cap">stake::store_aptos_coin_mint_cap</a>(aptos_framework, mint_cap);<br />    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> BurnCapability&lt;AptosCoin&gt; so it can burn gas.<br />    <a href="transaction_fee.md#0x1_transaction_fee_store_aptos_coin_burn_cap">transaction_fee::store_aptos_coin_burn_cap</a>(aptos_framework, burn_cap);<br />    // Give <a href="transaction_fee.md#0x1_transaction_fee">transaction_fee</a> <b>module</b> MintCapability&lt;AptosCoin&gt; so it can mint refunds.<br />    <a href="transaction_fee.md#0x1_transaction_fee_store_aptos_coin_mint_cap">transaction_fee::store_aptos_coin_mint_cap</a>(aptos_framework, mint_cap);<br /><br />    <b>let</b> core_resources &#61; <a href="account.md#0x1_account_create_account">account::create_account</a>(@core_resources);<br />    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(&amp;core_resources, core_resources_auth_key);<br />    <a href="aptos_coin.md#0x1_aptos_coin_configure_accounts_for_test">aptos_coin::configure_accounts_for_test</a>(aptos_framework, &amp;core_resources, mint_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_create_accounts"></a>

## Function `create_accounts`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_accounts">create_accounts</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">genesis::AccountMap</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_accounts">create_accounts</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a>&gt;) &#123;<br />    <b>let</b> unique_accounts &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;accounts, &#124;account_map&#124; &#123;<br />        <b>let</b> account_map: &amp;<a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a> &#61; account_map;<br />        <b>assert</b>!(<br />            !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&amp;unique_accounts, &amp;account_map.account_address),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>),<br />        );<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> unique_accounts, account_map.account_address);<br /><br />        <a href="genesis.md#0x1_genesis_create_account">create_account</a>(<br />            aptos_framework,<br />            account_map.account_address,<br />            account_map.balance,<br />        );<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_create_account"></a>

## Function `create_account`

This creates an funds an account if it doesn&apos;t exist.
If it exists, it just returns the signer.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_account">create_account</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, account_address: <b>address</b>, balance: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_account">create_account</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, account_address: <b>address</b>, balance: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#123;<br />    <b>if</b> (<a href="account.md#0x1_account_exists_at">account::exists_at</a>(account_address)) &#123;<br />        <a href="create_signer.md#0x1_create_signer">create_signer</a>(account_address)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> <a href="account.md#0x1_account">account</a> &#61; <a href="account.md#0x1_account_create_account">account::create_account</a>(account_address);<br />        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&amp;<a href="account.md#0x1_account">account</a>);<br />        <a href="aptos_coin.md#0x1_aptos_coin_mint">aptos_coin::mint</a>(aptos_framework, account_address, balance);<br />        <a href="account.md#0x1_account">account</a><br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_create_employee_validators"></a>

## Function `create_employee_validators`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_employee_validators">create_employee_validators</a>(employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">genesis::EmployeeAccountMap</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_employee_validators">create_employee_validators</a>(<br />    employee_vesting_start: u64,<br />    employee_vesting_period_duration: u64,<br />    employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a>&gt;,<br />) &#123;<br />    <b>let</b> unique_accounts &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;employees, &#124;employee_group&#124; &#123;<br />        <b>let</b> j &#61; 0;<br />        <b>let</b> employee_group: &amp;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a> &#61; employee_group;<br />        <b>let</b> num_employees_in_group &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;employee_group.accounts);<br /><br />        <b>let</b> buy_ins &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>();<br /><br />        <b>while</b> (j &lt; num_employees_in_group) &#123;<br />            <b>let</b> <a href="account.md#0x1_account">account</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;employee_group.accounts, j);<br />            <b>assert</b>!(<br />                !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&amp;unique_accounts, <a href="account.md#0x1_account">account</a>),<br />                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="genesis.md#0x1_genesis_EDUPLICATE_ACCOUNT">EDUPLICATE_ACCOUNT</a>),<br />            );<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> unique_accounts, &#42;<a href="account.md#0x1_account">account</a>);<br /><br />            <b>let</b> employee &#61; <a href="create_signer.md#0x1_create_signer">create_signer</a>(&#42;<a href="account.md#0x1_account">account</a>);<br />            <b>let</b> total &#61; <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;AptosCoin&gt;(&#42;<a href="account.md#0x1_account">account</a>);<br />            <b>let</b> coins &#61; <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;AptosCoin&gt;(&amp;employee, total);<br />            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> buy_ins, &#42;<a href="account.md#0x1_account">account</a>, coins);<br /><br />            j &#61; j &#43; 1;<br />        &#125;;<br /><br />        <b>let</b> j &#61; 0;<br />        <b>let</b> num_vesting_events &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;employee_group.vesting_schedule_numerator);<br />        <b>let</b> schedule &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br /><br />        <b>while</b> (j &lt; num_vesting_events) &#123;<br />            <b>let</b> numerator &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;employee_group.vesting_schedule_numerator, j);<br />            <b>let</b> <a href="event.md#0x1_event">event</a> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32_create_from_rational">fixed_point32::create_from_rational</a>(&#42;numerator, employee_group.vesting_schedule_denominator);<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> schedule, <a href="event.md#0x1_event">event</a>);<br /><br />            j &#61; j &#43; 1;<br />        &#125;;<br /><br />        <b>let</b> vesting_schedule &#61; <a href="vesting.md#0x1_vesting_create_vesting_schedule">vesting::create_vesting_schedule</a>(<br />            schedule,<br />            employee_vesting_start,<br />            employee_vesting_period_duration,<br />        );<br /><br />        <b>let</b> admin &#61; employee_group.validator.validator_config.owner_address;<br />        <b>let</b> admin_signer &#61; &amp;<a href="create_signer.md#0x1_create_signer">create_signer</a>(admin);<br />        <b>let</b> contract_address &#61; <a href="vesting.md#0x1_vesting_create_vesting_contract">vesting::create_vesting_contract</a>(<br />            admin_signer,<br />            &amp;employee_group.accounts,<br />            buy_ins,<br />            vesting_schedule,<br />            admin,<br />            employee_group.validator.validator_config.operator_address,<br />            employee_group.validator.validator_config.voter_address,<br />            employee_group.validator.commission_percentage,<br />            x&quot;&quot;,<br />        );<br />        <b>let</b> pool_address &#61; <a href="vesting.md#0x1_vesting_stake_pool_address">vesting::stake_pool_address</a>(contract_address);<br /><br />        <b>if</b> (employee_group.beneficiary_resetter !&#61; @0x0) &#123;<br />            <a href="vesting.md#0x1_vesting_set_beneficiary_resetter">vesting::set_beneficiary_resetter</a>(admin_signer, contract_address, employee_group.beneficiary_resetter);<br />        &#125;;<br /><br />        <b>let</b> validator &#61; &amp;employee_group.validator.validator_config;<br />        <b>assert</b>!(<br />            <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.owner_address),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),<br />        );<br />        <b>assert</b>!(<br />            <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.operator_address),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),<br />        );<br />        <b>assert</b>!(<br />            <a href="account.md#0x1_account_exists_at">account::exists_at</a>(validator.voter_address),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="genesis.md#0x1_genesis_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>),<br />        );<br />        <b>if</b> (employee_group.validator.join_during_genesis) &#123;<br />            <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address, validator);<br />        &#125;;<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators_with_commission"></a>

## Function `create_initialize_validators_with_commission`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, use_staking_contract: bool, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    use_staking_contract: bool,<br />    validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a>&gt;,<br />) &#123;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;validators, &#124;validator&#124; &#123;<br />        <b>let</b> validator: &amp;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a> &#61; validator;<br />        <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(aptos_framework, validator, use_staking_contract);<br />    &#125;);<br /><br />    // Destroy the aptos framework <a href="account.md#0x1_account">account</a>&apos;s ability <b>to</b> mint coins now that we&apos;re done <b>with</b> setting up the initial<br />    // validators.<br />    <a href="aptos_coin.md#0x1_aptos_coin_destroy_mint_cap">aptos_coin::destroy_mint_cap</a>(aptos_framework);<br /><br />    <a href="stake.md#0x1_stake_on_new_epoch">stake::on_new_epoch</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators"></a>

## Function `create_initialize_validators`

Sets up the initial validator set for the network.
The validator &quot;owner&quot; accounts, and their authentication
Addresses (and keys) are encoded in the <code>owners</code>
Each validator signs consensus messages with the private key corresponding to the Ed25519
public key in <code>consensus_pubkeys</code>.
Finally, each validator must specify the network address
(see types/src/network_address/mod.rs) for itself and its full nodes.

Network address fields are a vector per account, where each entry is a vector of addresses
encoded in a single BCS byte array.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators">create_initialize_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators">create_initialize_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a>&gt;) &#123;<br />    <b>let</b> validators_with_commission &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_reverse">vector::for_each_reverse</a>(validators, &#124;validator&#124; &#123;<br />        <b>let</b> validator_with_commission &#61; <a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a> &#123;<br />            validator_config: validator,<br />            commission_percentage: 0,<br />            join_during_genesis: <b>true</b>,<br />        &#125;;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> validators_with_commission, validator_with_commission);<br />    &#125;);<br /><br />    <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(aptos_framework, <b>false</b>, validators_with_commission);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_create_initialize_validator"></a>

## Function `create_initialize_validator`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, commission_config: &amp;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>, use_staking_contract: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    commission_config: &amp;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a>,<br />    use_staking_contract: bool,<br />) &#123;<br />    <b>let</b> validator &#61; &amp;commission_config.validator_config;<br /><br />    <b>let</b> owner &#61; &amp;<a href="genesis.md#0x1_genesis_create_account">create_account</a>(aptos_framework, validator.owner_address, validator.stake_amount);<br />    <a href="genesis.md#0x1_genesis_create_account">create_account</a>(aptos_framework, validator.operator_address, 0);<br />    <a href="genesis.md#0x1_genesis_create_account">create_account</a>(aptos_framework, validator.voter_address, 0);<br /><br />    // Initialize the <a href="stake.md#0x1_stake">stake</a> pool and join the validator set.<br />    <b>let</b> pool_address &#61; <b>if</b> (use_staking_contract) &#123;<br />        <a href="staking_contract.md#0x1_staking_contract_create_staking_contract">staking_contract::create_staking_contract</a>(<br />            owner,<br />            validator.operator_address,<br />            validator.voter_address,<br />            validator.stake_amount,<br />            commission_config.commission_percentage,<br />            x&quot;&quot;,<br />        );<br />        <a href="staking_contract.md#0x1_staking_contract_stake_pool_address">staking_contract::stake_pool_address</a>(validator.owner_address, validator.operator_address)<br />    &#125; <b>else</b> &#123;<br />        <a href="stake.md#0x1_stake_initialize_stake_owner">stake::initialize_stake_owner</a>(<br />            owner,<br />            validator.stake_amount,<br />            validator.operator_address,<br />            validator.voter_address,<br />        );<br />        validator.owner_address<br />    &#125;;<br /><br />    <b>if</b> (commission_config.join_during_genesis) &#123;<br />        <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address, validator);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_initialize_validator"></a>

## Function `initialize_validator`



<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address: <b>address</b>, validator: &amp;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_validator">initialize_validator</a>(pool_address: <b>address</b>, validator: &amp;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">ValidatorConfiguration</a>) &#123;<br />    <b>let</b> operator &#61; &amp;<a href="create_signer.md#0x1_create_signer">create_signer</a>(validator.operator_address);<br /><br />    <a href="stake.md#0x1_stake_rotate_consensus_key">stake::rotate_consensus_key</a>(<br />        operator,<br />        pool_address,<br />        validator.consensus_pubkey,<br />        validator.proof_of_possession,<br />    );<br />    <a href="stake.md#0x1_stake_update_network_and_fullnode_addresses">stake::update_network_and_fullnode_addresses</a>(<br />        operator,<br />        pool_address,<br />        validator.network_addresses,<br />        validator.full_node_network_addresses,<br />    );<br />    <a href="stake.md#0x1_stake_join_validator_set_internal">stake::join_validator_set_internal</a>(operator, pool_address);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_set_genesis_end"></a>

## Function `set_genesis_end`

The last step of genesis.


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="chain_status.md#0x1_chain_status_set_genesis_end">chain_status::set_genesis_end</a>(aptos_framework);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_genesis_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>&#35;[verify_only]<br /><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_for_verification">initialize_for_verification</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">genesis::AccountMap</a>&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">genesis::EmployeeAccountMap</a>&gt;, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_for_verification">initialize_for_verification</a>(<br />    <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8,<br />    initial_version: u64,<br />    <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    epoch_interval_microsecs: u64,<br />    minimum_stake: u64,<br />    maximum_stake: u64,<br />    recurring_lockup_duration_secs: u64,<br />    allow_validator_set_change: bool,<br />    rewards_rate: u64,<br />    rewards_rate_denominator: u64,<br />    voting_power_increase_limit: u64,<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    min_voting_threshold: u128,<br />    required_proposer_stake: u64,<br />    voting_duration_secs: u64,<br />    accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">AccountMap</a>&gt;,<br />    employee_vesting_start: u64,<br />    employee_vesting_period_duration: u64,<br />    employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">EmployeeAccountMap</a>&gt;,<br />    validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">ValidatorConfigurationWithCommission</a>&gt;<br />) &#123;<br />    <a href="genesis.md#0x1_genesis_initialize">initialize</a>(<br />        <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>,<br />        <a href="chain_id.md#0x1_chain_id">chain_id</a>,<br />        initial_version,<br />        <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>,<br />        <a href="execution_config.md#0x1_execution_config">execution_config</a>,<br />        epoch_interval_microsecs,<br />        minimum_stake,<br />        maximum_stake,<br />        recurring_lockup_duration_secs,<br />        allow_validator_set_change,<br />        rewards_rate,<br />        rewards_rate_denominator,<br />        voting_power_increase_limit<br />    );<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_change_feature_flags_for_verification">features::change_feature_flags_for_verification</a>(aptos_framework, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 2], <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);<br />    <a href="genesis.md#0x1_genesis_initialize_aptos_coin">initialize_aptos_coin</a>(aptos_framework);<br />    <a href="aptos_governance.md#0x1_aptos_governance_initialize_for_verification">aptos_governance::initialize_for_verification</a>(<br />        aptos_framework,<br />        min_voting_threshold,<br />        required_proposer_stake,<br />        voting_duration_secs<br />    );<br />    <a href="genesis.md#0x1_genesis_create_accounts">create_accounts</a>(aptos_framework, accounts);<br />    <a href="genesis.md#0x1_genesis_create_employee_validators">create_employee_validators</a>(employee_vesting_start, employee_vesting_period_duration, employees);<br />    <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(aptos_framework, <b>true</b>, validators);<br />    <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(aptos_framework);<br />&#125;<br /></code></pre>



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
<td>All the core resources and modules should be created during genesis and owned by the Aptos framework account.</td>
<td>Critical</td>
<td>Resources created during genesis initialization: GovernanceResponsbility, ConsensusConfig, ExecutionConfig, Version, SetVersionCapability, ValidatorSet, ValidatorPerformance, StakingConfig, StorageGasConfig, StorageGas, GasScheduleV2, AggregatorFactory, SupplyConfig, ChainId, Configuration, BlockResource, StateStorageUsage, CurrentTimeMicroseconds. If some of the resources were to be owned by a malicious account, it could lead to the compromise of the chain, as these are core resources. It should be formally verified by a post condition to ensure that all the critical resources are owned by the Aptos framework.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Addresses ranging from 0x0 &#45; 0xa should be reserved for the framework and part of aptos governance.</td>
<td>Critical</td>
<td>The function genesis::initialize calls account::create_framework_reserved_account for addresses 0x0, 0x2, 0x3, 0x4, ..., 0xa which creates an account and authentication_key for them. This should be formally verified by ensuring that at the beginning of the genesis::initialize function no Account resource exists for the reserved addresses, and at the end of the function, an Account resource exists.</td>
<td>Formally verified via <a href="#high-level-req-2">initialize</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The Aptos coin should be initialized during genesis and only the Aptos framework account should own the mint and burn capabilities for the APT token.</td>
<td>Critical</td>
<td>Both mint and burn capabilities are wrapped inside the stake::AptosCoinCapabilities and transaction_fee::AptosCoinCapabilities resources which are stored under the aptos framework account.</td>
<td>Formally verified via <a href="#high-level-req-3">initialize_aptos_coin</a>.</td>
</tr>

<tr>
<td>4</td>
<td>An initial set of validators should exist before the end of genesis.</td>
<td>Low</td>
<td>To ensure that there will be a set of validators available to validate the genesis block, the length of the ValidatorSet.active_validators vector should be &gt; 0.</td>
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


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize">initialize</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="genesis.md#0x1_genesis_InitalizeRequires">InitalizeRequires</a>;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x0);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x2);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x3);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x4);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x5);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x6);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x7);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x8);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x9);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0xa);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x0);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x2);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x3);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x4);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x5);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x6);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x7);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x8);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0x9);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@0xa);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">aptos_governance::GovernanceResponsbility</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">consensus_config::ConsensusConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="execution_config.md#0x1_execution_config_ExecutionConfig">execution_config::ExecutionConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="version.md#0x1_version_Version">version::Version</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorPerformance">stake::ValidatorPerformance</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">storage_gas::StorageGas</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">gas_schedule::GasScheduleV2</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_SupplyConfig">coin::SupplyConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">chain_id::ChainId</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">reconfiguration::Configuration</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="block.md#0x1_block_BlockResource">block::BlockResource</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">state_storage::StateStorageUsage</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">version::SetVersionCapability</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_initialize_aptos_coin"></a>

### Function `initialize_aptos_coin`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_aptos_coin">initialize_aptos_coin</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>requires</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_AptosCoinCapabilities">stake::AptosCoinCapabilities</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_AptosCoinCapabilities">stake::AptosCoinCapabilities</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">transaction_fee::AptosCoinCapabilities</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">transaction_fee::AptosCoinCapabilities</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_create_initialize_validators_with_commission"></a>

### Function `create_initialize_validators_with_commission`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators_with_commission">create_initialize_validators_with_commission</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, use_staking_contract: bool, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a>;<br /><b>include</b> <a href="aptos_coin.md#0x1_aptos_coin_ExistsAptosCoin">aptos_coin::ExistsAptosCoin</a>;<br /></code></pre>



<a id="@Specification_1_create_initialize_validators"></a>

### Function `create_initialize_validators`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validators">create_initialize_validators</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfiguration">genesis::ValidatorConfiguration</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a>;<br /><b>include</b> <a href="aptos_coin.md#0x1_aptos_coin_ExistsAptosCoin">aptos_coin::ExistsAptosCoin</a>;<br /></code></pre>



<a id="@Specification_1_create_initialize_validator"></a>

### Function `create_initialize_validator`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_create_initialize_validator">create_initialize_validator</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, commission_config: &amp;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>, use_staking_contract: bool)<br /></code></pre>




<pre><code><b>include</b> <a href="stake.md#0x1_stake_ResourceRequirement">stake::ResourceRequirement</a>;<br /></code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code><b>fun</b> <a href="genesis.md#0x1_genesis_set_genesis_end">set_genesis_end</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> delegate_invariants_to_caller;<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>requires</b> len(<b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework).active_validators) &gt;&#61; 1;<br />// This enforces <a id="high-level-req-5" href="#high-level-req">high&#45;level requirement 5</a>:
<b>let</b> addr &#61; std::signer::address_of(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">chain_status::GenesisEndMarker</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>global</b>&lt;<a href="chain_status.md#0x1_chain_status_GenesisEndMarker">chain_status::GenesisEndMarker</a>&gt;(@aptos_framework) &#61;&#61; <a href="chain_status.md#0x1_chain_status_GenesisEndMarker">chain_status::GenesisEndMarker</a> &#123;&#125;;<br /></code></pre>




<a id="0x1_genesis_InitalizeRequires"></a>


<pre><code><b>schema</b> <a href="genesis.md#0x1_genesis_InitalizeRequires">InitalizeRequires</a> &#123;<br /><a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>requires</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(@aptos_framework);<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>requires</b> len(<a href="execution_config.md#0x1_execution_config">execution_config</a>) &gt; 0;<br /><b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>include</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a>;<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br />&#125;<br /></code></pre>




<a id="0x1_genesis_CompareTimeRequires"></a>


<pre><code><b>schema</b> <a href="genesis.md#0x1_genesis_CompareTimeRequires">CompareTimeRequires</a> &#123;<br /><b>let</b> staking_rewards_config &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework);<br /><b>requires</b> staking_rewards_config.last_rewards_rate_period_start_in_secs &lt;&#61; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();<br />&#125;<br /></code></pre>



<a id="@Specification_1_initialize_for_verification"></a>

### Function `initialize_for_verification`


<pre><code>&#35;[verify_only]<br /><b>fun</b> <a href="genesis.md#0x1_genesis_initialize_for_verification">initialize_for_verification</a>(<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="chain_id.md#0x1_chain_id">chain_id</a>: u8, initial_version: u64, <a href="consensus_config.md#0x1_consensus_config">consensus_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="execution_config.md#0x1_execution_config">execution_config</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64, accounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_AccountMap">genesis::AccountMap</a>&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_EmployeeAccountMap">genesis::EmployeeAccountMap</a>&gt;, validators: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="genesis.md#0x1_genesis_ValidatorConfigurationWithCommission">genesis::ValidatorConfigurationWithCommission</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="genesis.md#0x1_genesis_InitalizeRequires">InitalizeRequires</a>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
