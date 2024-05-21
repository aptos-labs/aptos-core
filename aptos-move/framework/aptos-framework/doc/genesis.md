
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


<pre><code>use 0x1::account;<br/>use 0x1::aggregator_factory;<br/>use 0x1::aptos_coin;<br/>use 0x1::aptos_governance;<br/>use 0x1::block;<br/>use 0x1::chain_id;<br/>use 0x1::chain_status;<br/>use 0x1::coin;<br/>use 0x1::consensus_config;<br/>use 0x1::create_signer;<br/>use 0x1::error;<br/>use 0x1::execution_config;<br/>use 0x1::features;<br/>use 0x1::fixed_point32;<br/>use 0x1::gas_schedule;<br/>use 0x1::reconfiguration;<br/>use 0x1::simple_map;<br/>use 0x1::stake;<br/>use 0x1::staking_config;<br/>use 0x1::staking_contract;<br/>use 0x1::state_storage;<br/>use 0x1::storage_gas;<br/>use 0x1::timestamp;<br/>use 0x1::transaction_fee;<br/>use 0x1::transaction_validation;<br/>use 0x1::vector;<br/>use 0x1::version;<br/>use 0x1::vesting;<br/></code></pre>



<a id="0x1_genesis_AccountMap"></a>

## Struct `AccountMap`



<pre><code>struct AccountMap has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: address</code>
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



<pre><code>struct EmployeeAccountMap has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>accounts: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>validator: genesis::ValidatorConfigurationWithCommission</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule_numerator: vector&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vesting_schedule_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>beneficiary_resetter: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_ValidatorConfiguration"></a>

## Struct `ValidatorConfiguration`



<pre><code>struct ValidatorConfiguration has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>operator_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>voter_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>consensus_pubkey: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proof_of_possession: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>full_node_network_addresses: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_genesis_ValidatorConfigurationWithCommission"></a>

## Struct `ValidatorConfigurationWithCommission`



<pre><code>struct ValidatorConfigurationWithCommission has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validator_config: genesis::ValidatorConfiguration</code>
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



<pre><code>const EACCOUNT_DOES_NOT_EXIST: u64 &#61; 2;<br/></code></pre>



<a id="0x1_genesis_EDUPLICATE_ACCOUNT"></a>



<pre><code>const EDUPLICATE_ACCOUNT: u64 &#61; 1;<br/></code></pre>



<a id="0x1_genesis_initialize"></a>

## Function `initialize`

Genesis step 1: Initialize aptos framework account and core modules on chain.


<pre><code>fun initialize(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize(<br/>    gas_schedule: vector&lt;u8&gt;,<br/>    chain_id: u8,<br/>    initial_version: u64,<br/>    consensus_config: vector&lt;u8&gt;,<br/>    execution_config: vector&lt;u8&gt;,<br/>    epoch_interval_microsecs: u64,<br/>    minimum_stake: u64,<br/>    maximum_stake: u64,<br/>    recurring_lockup_duration_secs: u64,<br/>    allow_validator_set_change: bool,<br/>    rewards_rate: u64,<br/>    rewards_rate_denominator: u64,<br/>    voting_power_increase_limit: u64,<br/>) &#123;<br/>    // Initialize the aptos framework account. This is the account where system resources and modules will be<br/>    // deployed to. This will be entirely managed by on&#45;chain governance and no entities have the key or privileges<br/>    // to use this account.<br/>    let (aptos_framework_account, aptos_framework_signer_cap) &#61; account::create_framework_reserved_account(@aptos_framework);<br/>    // Initialize account configs on aptos framework account.<br/>    account::initialize(&amp;aptos_framework_account);<br/><br/>    transaction_validation::initialize(<br/>        &amp;aptos_framework_account,<br/>        b&quot;script_prologue&quot;,<br/>        b&quot;module_prologue&quot;,<br/>        b&quot;multi_agent_script_prologue&quot;,<br/>        b&quot;epilogue&quot;,<br/>    );<br/><br/>    // Give the decentralized on&#45;chain governance control over the core framework account.<br/>    aptos_governance::store_signer_cap(&amp;aptos_framework_account, @aptos_framework, aptos_framework_signer_cap);<br/><br/>    // put reserved framework reserved accounts under aptos governance<br/>    let framework_reserved_addresses &#61; vector&lt;address&gt;[@0x2, @0x3, @0x4, @0x5, @0x6, @0x7, @0x8, @0x9, @0xa];<br/>    while (!vector::is_empty(&amp;framework_reserved_addresses)) &#123;<br/>        let address &#61; vector::pop_back&lt;address&gt;(&amp;mut framework_reserved_addresses);<br/>        let (_, framework_signer_cap) &#61; account::create_framework_reserved_account(address);<br/>        aptos_governance::store_signer_cap(&amp;aptos_framework_account, address, framework_signer_cap);<br/>    &#125;;<br/><br/>    consensus_config::initialize(&amp;aptos_framework_account, consensus_config);<br/>    execution_config::set(&amp;aptos_framework_account, execution_config);<br/>    version::initialize(&amp;aptos_framework_account, initial_version);<br/>    stake::initialize(&amp;aptos_framework_account);<br/>    staking_config::initialize(<br/>        &amp;aptos_framework_account,<br/>        minimum_stake,<br/>        maximum_stake,<br/>        recurring_lockup_duration_secs,<br/>        allow_validator_set_change,<br/>        rewards_rate,<br/>        rewards_rate_denominator,<br/>        voting_power_increase_limit,<br/>    );<br/>    storage_gas::initialize(&amp;aptos_framework_account);<br/>    gas_schedule::initialize(&amp;aptos_framework_account, gas_schedule);<br/><br/>    // Ensure we can create aggregators for supply, but not enable it for common use just yet.<br/>    aggregator_factory::initialize_aggregator_factory(&amp;aptos_framework_account);<br/>    coin::initialize_supply_config(&amp;aptos_framework_account);<br/><br/>    chain_id::initialize(&amp;aptos_framework_account, chain_id);<br/>    reconfiguration::initialize(&amp;aptos_framework_account);<br/>    block::initialize(&amp;aptos_framework_account, epoch_interval_microsecs);<br/>    state_storage::initialize(&amp;aptos_framework_account);<br/>    timestamp::set_time_has_started(&amp;aptos_framework_account);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_initialize_aptos_coin"></a>

## Function `initialize_aptos_coin`

Genesis step 2: Initialize Aptos coin.


<pre><code>fun initialize_aptos_coin(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_aptos_coin(aptos_framework: &amp;signer) &#123;<br/>    let (burn_cap, mint_cap) &#61; aptos_coin::initialize(aptos_framework);<br/><br/>    coin::create_coin_conversion_map(aptos_framework);<br/>    coin::create_pairing&lt;AptosCoin&gt;(aptos_framework);<br/><br/>    // Give stake module MintCapability&lt;AptosCoin&gt; so it can mint rewards.<br/>    stake::store_aptos_coin_mint_cap(aptos_framework, mint_cap);<br/>    // Give transaction_fee module BurnCapability&lt;AptosCoin&gt; so it can burn gas.<br/>    transaction_fee::store_aptos_coin_burn_cap(aptos_framework, burn_cap);<br/>    // Give transaction_fee module MintCapability&lt;AptosCoin&gt; so it can mint refunds.<br/>    transaction_fee::store_aptos_coin_mint_cap(aptos_framework, mint_cap);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_initialize_core_resources_and_aptos_coin"></a>

## Function `initialize_core_resources_and_aptos_coin`

Only called for testnets and e2e tests.


<pre><code>fun initialize_core_resources_and_aptos_coin(aptos_framework: &amp;signer, core_resources_auth_key: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_core_resources_and_aptos_coin(<br/>    aptos_framework: &amp;signer,<br/>    core_resources_auth_key: vector&lt;u8&gt;,<br/>) &#123;<br/>    let (burn_cap, mint_cap) &#61; aptos_coin::initialize(aptos_framework);<br/><br/>    coin::create_coin_conversion_map(aptos_framework);<br/>    coin::create_pairing&lt;AptosCoin&gt;(aptos_framework);<br/><br/>    // Give stake module MintCapability&lt;AptosCoin&gt; so it can mint rewards.<br/>    stake::store_aptos_coin_mint_cap(aptos_framework, mint_cap);<br/>    // Give transaction_fee module BurnCapability&lt;AptosCoin&gt; so it can burn gas.<br/>    transaction_fee::store_aptos_coin_burn_cap(aptos_framework, burn_cap);<br/>    // Give transaction_fee module MintCapability&lt;AptosCoin&gt; so it can mint refunds.<br/>    transaction_fee::store_aptos_coin_mint_cap(aptos_framework, mint_cap);<br/><br/>    let core_resources &#61; account::create_account(@core_resources);<br/>    account::rotate_authentication_key_internal(&amp;core_resources, core_resources_auth_key);<br/>    aptos_coin::configure_accounts_for_test(aptos_framework, &amp;core_resources, mint_cap);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_create_accounts"></a>

## Function `create_accounts`



<pre><code>fun create_accounts(aptos_framework: &amp;signer, accounts: vector&lt;genesis::AccountMap&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_accounts(aptos_framework: &amp;signer, accounts: vector&lt;AccountMap&gt;) &#123;<br/>    let unique_accounts &#61; vector::empty();<br/>    vector::for_each_ref(&amp;accounts, &#124;account_map&#124; &#123;<br/>        let account_map: &amp;AccountMap &#61; account_map;<br/>        assert!(<br/>            !vector::contains(&amp;unique_accounts, &amp;account_map.account_address),<br/>            error::already_exists(EDUPLICATE_ACCOUNT),<br/>        );<br/>        vector::push_back(&amp;mut unique_accounts, account_map.account_address);<br/><br/>        create_account(<br/>            aptos_framework,<br/>            account_map.account_address,<br/>            account_map.balance,<br/>        );<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_create_account"></a>

## Function `create_account`

This creates an funds an account if it doesn't exist.
If it exists, it just returns the signer.


<pre><code>fun create_account(aptos_framework: &amp;signer, account_address: address, balance: u64): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_account(aptos_framework: &amp;signer, account_address: address, balance: u64): signer &#123;<br/>    if (account::exists_at(account_address)) &#123;<br/>        create_signer(account_address)<br/>    &#125; else &#123;<br/>        let account &#61; account::create_account(account_address);<br/>        coin::register&lt;AptosCoin&gt;(&amp;account);<br/>        aptos_coin::mint(aptos_framework, account_address, balance);<br/>        account<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_create_employee_validators"></a>

## Function `create_employee_validators`



<pre><code>fun create_employee_validators(employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: vector&lt;genesis::EmployeeAccountMap&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_employee_validators(<br/>    employee_vesting_start: u64,<br/>    employee_vesting_period_duration: u64,<br/>    employees: vector&lt;EmployeeAccountMap&gt;,<br/>) &#123;<br/>    let unique_accounts &#61; vector::empty();<br/><br/>    vector::for_each_ref(&amp;employees, &#124;employee_group&#124; &#123;<br/>        let j &#61; 0;<br/>        let employee_group: &amp;EmployeeAccountMap &#61; employee_group;<br/>        let num_employees_in_group &#61; vector::length(&amp;employee_group.accounts);<br/><br/>        let buy_ins &#61; simple_map::create();<br/><br/>        while (j &lt; num_employees_in_group) &#123;<br/>            let account &#61; vector::borrow(&amp;employee_group.accounts, j);<br/>            assert!(<br/>                !vector::contains(&amp;unique_accounts, account),<br/>                error::already_exists(EDUPLICATE_ACCOUNT),<br/>            );<br/>            vector::push_back(&amp;mut unique_accounts, &#42;account);<br/><br/>            let employee &#61; create_signer(&#42;account);<br/>            let total &#61; coin::balance&lt;AptosCoin&gt;(&#42;account);<br/>            let coins &#61; coin::withdraw&lt;AptosCoin&gt;(&amp;employee, total);<br/>            simple_map::add(&amp;mut buy_ins, &#42;account, coins);<br/><br/>            j &#61; j &#43; 1;<br/>        &#125;;<br/><br/>        let j &#61; 0;<br/>        let num_vesting_events &#61; vector::length(&amp;employee_group.vesting_schedule_numerator);<br/>        let schedule &#61; vector::empty();<br/><br/>        while (j &lt; num_vesting_events) &#123;<br/>            let numerator &#61; vector::borrow(&amp;employee_group.vesting_schedule_numerator, j);<br/>            let event &#61; fixed_point32::create_from_rational(&#42;numerator, employee_group.vesting_schedule_denominator);<br/>            vector::push_back(&amp;mut schedule, event);<br/><br/>            j &#61; j &#43; 1;<br/>        &#125;;<br/><br/>        let vesting_schedule &#61; vesting::create_vesting_schedule(<br/>            schedule,<br/>            employee_vesting_start,<br/>            employee_vesting_period_duration,<br/>        );<br/><br/>        let admin &#61; employee_group.validator.validator_config.owner_address;<br/>        let admin_signer &#61; &amp;create_signer(admin);<br/>        let contract_address &#61; vesting::create_vesting_contract(<br/>            admin_signer,<br/>            &amp;employee_group.accounts,<br/>            buy_ins,<br/>            vesting_schedule,<br/>            admin,<br/>            employee_group.validator.validator_config.operator_address,<br/>            employee_group.validator.validator_config.voter_address,<br/>            employee_group.validator.commission_percentage,<br/>            x&quot;&quot;,<br/>        );<br/>        let pool_address &#61; vesting::stake_pool_address(contract_address);<br/><br/>        if (employee_group.beneficiary_resetter !&#61; @0x0) &#123;<br/>            vesting::set_beneficiary_resetter(admin_signer, contract_address, employee_group.beneficiary_resetter);<br/>        &#125;;<br/><br/>        let validator &#61; &amp;employee_group.validator.validator_config;<br/>        assert!(<br/>            account::exists_at(validator.owner_address),<br/>            error::not_found(EACCOUNT_DOES_NOT_EXIST),<br/>        );<br/>        assert!(<br/>            account::exists_at(validator.operator_address),<br/>            error::not_found(EACCOUNT_DOES_NOT_EXIST),<br/>        );<br/>        assert!(<br/>            account::exists_at(validator.voter_address),<br/>            error::not_found(EACCOUNT_DOES_NOT_EXIST),<br/>        );<br/>        if (employee_group.validator.join_during_genesis) &#123;<br/>            initialize_validator(pool_address, validator);<br/>        &#125;;<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators_with_commission"></a>

## Function `create_initialize_validators_with_commission`



<pre><code>fun create_initialize_validators_with_commission(aptos_framework: &amp;signer, use_staking_contract: bool, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_initialize_validators_with_commission(<br/>    aptos_framework: &amp;signer,<br/>    use_staking_contract: bool,<br/>    validators: vector&lt;ValidatorConfigurationWithCommission&gt;,<br/>) &#123;<br/>    vector::for_each_ref(&amp;validators, &#124;validator&#124; &#123;<br/>        let validator: &amp;ValidatorConfigurationWithCommission &#61; validator;<br/>        create_initialize_validator(aptos_framework, validator, use_staking_contract);<br/>    &#125;);<br/><br/>    // Destroy the aptos framework account&apos;s ability to mint coins now that we&apos;re done with setting up the initial<br/>    // validators.<br/>    aptos_coin::destroy_mint_cap(aptos_framework);<br/><br/>    stake::on_new_epoch();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators"></a>

## Function `create_initialize_validators`

Sets up the initial validator set for the network.
The validator "owner" accounts, and their authentication
Addresses (and keys) are encoded in the <code>owners</code>
Each validator signs consensus messages with the private key corresponding to the Ed25519
public key in <code>consensus_pubkeys</code>.
Finally, each validator must specify the network address
(see types/src/network_address/mod.rs) for itself and its full nodes.

Network address fields are a vector per account, where each entry is a vector of addresses
encoded in a single BCS byte array.


<pre><code>fun create_initialize_validators(aptos_framework: &amp;signer, validators: vector&lt;genesis::ValidatorConfiguration&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_initialize_validators(aptos_framework: &amp;signer, validators: vector&lt;ValidatorConfiguration&gt;) &#123;<br/>    let validators_with_commission &#61; vector::empty();<br/>    vector::for_each_reverse(validators, &#124;validator&#124; &#123;<br/>        let validator_with_commission &#61; ValidatorConfigurationWithCommission &#123;<br/>            validator_config: validator,<br/>            commission_percentage: 0,<br/>            join_during_genesis: true,<br/>        &#125;;<br/>        vector::push_back(&amp;mut validators_with_commission, validator_with_commission);<br/>    &#125;);<br/><br/>    create_initialize_validators_with_commission(aptos_framework, false, validators_with_commission);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_create_initialize_validator"></a>

## Function `create_initialize_validator`



<pre><code>fun create_initialize_validator(aptos_framework: &amp;signer, commission_config: &amp;genesis::ValidatorConfigurationWithCommission, use_staking_contract: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_initialize_validator(<br/>    aptos_framework: &amp;signer,<br/>    commission_config: &amp;ValidatorConfigurationWithCommission,<br/>    use_staking_contract: bool,<br/>) &#123;<br/>    let validator &#61; &amp;commission_config.validator_config;<br/><br/>    let owner &#61; &amp;create_account(aptos_framework, validator.owner_address, validator.stake_amount);<br/>    create_account(aptos_framework, validator.operator_address, 0);<br/>    create_account(aptos_framework, validator.voter_address, 0);<br/><br/>    // Initialize the stake pool and join the validator set.<br/>    let pool_address &#61; if (use_staking_contract) &#123;<br/>        staking_contract::create_staking_contract(<br/>            owner,<br/>            validator.operator_address,<br/>            validator.voter_address,<br/>            validator.stake_amount,<br/>            commission_config.commission_percentage,<br/>            x&quot;&quot;,<br/>        );<br/>        staking_contract::stake_pool_address(validator.owner_address, validator.operator_address)<br/>    &#125; else &#123;<br/>        stake::initialize_stake_owner(<br/>            owner,<br/>            validator.stake_amount,<br/>            validator.operator_address,<br/>            validator.voter_address,<br/>        );<br/>        validator.owner_address<br/>    &#125;;<br/><br/>    if (commission_config.join_during_genesis) &#123;<br/>        initialize_validator(pool_address, validator);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_initialize_validator"></a>

## Function `initialize_validator`



<pre><code>fun initialize_validator(pool_address: address, validator: &amp;genesis::ValidatorConfiguration)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_validator(pool_address: address, validator: &amp;ValidatorConfiguration) &#123;<br/>    let operator &#61; &amp;create_signer(validator.operator_address);<br/><br/>    stake::rotate_consensus_key(<br/>        operator,<br/>        pool_address,<br/>        validator.consensus_pubkey,<br/>        validator.proof_of_possession,<br/>    );<br/>    stake::update_network_and_fullnode_addresses(<br/>        operator,<br/>        pool_address,<br/>        validator.network_addresses,<br/>        validator.full_node_network_addresses,<br/>    );<br/>    stake::join_validator_set_internal(operator, pool_address);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_set_genesis_end"></a>

## Function `set_genesis_end`

The last step of genesis.


<pre><code>fun set_genesis_end(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun set_genesis_end(aptos_framework: &amp;signer) &#123;<br/>    chain_status::set_genesis_end(aptos_framework);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_genesis_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>&#35;[verify_only]<br/>fun initialize_for_verification(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64, accounts: vector&lt;genesis::AccountMap&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: vector&lt;genesis::EmployeeAccountMap&gt;, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_for_verification(<br/>    gas_schedule: vector&lt;u8&gt;,<br/>    chain_id: u8,<br/>    initial_version: u64,<br/>    consensus_config: vector&lt;u8&gt;,<br/>    execution_config: vector&lt;u8&gt;,<br/>    epoch_interval_microsecs: u64,<br/>    minimum_stake: u64,<br/>    maximum_stake: u64,<br/>    recurring_lockup_duration_secs: u64,<br/>    allow_validator_set_change: bool,<br/>    rewards_rate: u64,<br/>    rewards_rate_denominator: u64,<br/>    voting_power_increase_limit: u64,<br/>    aptos_framework: &amp;signer,<br/>    min_voting_threshold: u128,<br/>    required_proposer_stake: u64,<br/>    voting_duration_secs: u64,<br/>    accounts: vector&lt;AccountMap&gt;,<br/>    employee_vesting_start: u64,<br/>    employee_vesting_period_duration: u64,<br/>    employees: vector&lt;EmployeeAccountMap&gt;,<br/>    validators: vector&lt;ValidatorConfigurationWithCommission&gt;<br/>) &#123;<br/>    initialize(<br/>        gas_schedule,<br/>        chain_id,<br/>        initial_version,<br/>        consensus_config,<br/>        execution_config,<br/>        epoch_interval_microsecs,<br/>        minimum_stake,<br/>        maximum_stake,<br/>        recurring_lockup_duration_secs,<br/>        allow_validator_set_change,<br/>        rewards_rate,<br/>        rewards_rate_denominator,<br/>        voting_power_increase_limit<br/>    );<br/>    features::change_feature_flags_for_verification(aptos_framework, vector[1, 2], vector[]);<br/>    initialize_aptos_coin(aptos_framework);<br/>    aptos_governance::initialize_for_verification(<br/>        aptos_framework,<br/>        min_voting_threshold,<br/>        required_proposer_stake,<br/>        voting_duration_secs<br/>    );<br/>    create_accounts(aptos_framework, accounts);<br/>    create_employee_validators(employee_vesting_start, employee_vesting_period_duration, employees);<br/>    create_initialize_validators_with_commission(aptos_framework, true, validators);<br/>    set_genesis_end(aptos_framework);<br/>&#125;<br/></code></pre>



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
<td>Addresses ranging from 0x0 - 0xa should be reserved for the framework and part of aptos governance.</td>
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


<pre><code>pragma verify &#61; true;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>fun initialize(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>include InitalizeRequires;<br/>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if exists&lt;account::Account&gt;(@0x0);<br/>aborts_if exists&lt;account::Account&gt;(@0x2);<br/>aborts_if exists&lt;account::Account&gt;(@0x3);<br/>aborts_if exists&lt;account::Account&gt;(@0x4);<br/>aborts_if exists&lt;account::Account&gt;(@0x5);<br/>aborts_if exists&lt;account::Account&gt;(@0x6);<br/>aborts_if exists&lt;account::Account&gt;(@0x7);<br/>aborts_if exists&lt;account::Account&gt;(@0x8);<br/>aborts_if exists&lt;account::Account&gt;(@0x9);<br/>aborts_if exists&lt;account::Account&gt;(@0xa);<br/>ensures exists&lt;account::Account&gt;(@0x0);<br/>ensures exists&lt;account::Account&gt;(@0x2);<br/>ensures exists&lt;account::Account&gt;(@0x3);<br/>ensures exists&lt;account::Account&gt;(@0x4);<br/>ensures exists&lt;account::Account&gt;(@0x5);<br/>ensures exists&lt;account::Account&gt;(@0x6);<br/>ensures exists&lt;account::Account&gt;(@0x7);<br/>ensures exists&lt;account::Account&gt;(@0x8);<br/>ensures exists&lt;account::Account&gt;(@0x9);<br/>ensures exists&lt;account::Account&gt;(@0xa);<br/>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures exists&lt;aptos_governance::GovernanceResponsbility&gt;(@aptos_framework);<br/>ensures exists&lt;consensus_config::ConsensusConfig&gt;(@aptos_framework);<br/>ensures exists&lt;execution_config::ExecutionConfig&gt;(@aptos_framework);<br/>ensures exists&lt;version::Version&gt;(@aptos_framework);<br/>ensures exists&lt;stake::ValidatorSet&gt;(@aptos_framework);<br/>ensures exists&lt;stake::ValidatorPerformance&gt;(@aptos_framework);<br/>ensures exists&lt;storage_gas::StorageGasConfig&gt;(@aptos_framework);<br/>ensures exists&lt;storage_gas::StorageGas&gt;(@aptos_framework);<br/>ensures exists&lt;gas_schedule::GasScheduleV2&gt;(@aptos_framework);<br/>ensures exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);<br/>ensures exists&lt;coin::SupplyConfig&gt;(@aptos_framework);<br/>ensures exists&lt;chain_id::ChainId&gt;(@aptos_framework);<br/>ensures exists&lt;reconfiguration::Configuration&gt;(@aptos_framework);<br/>ensures exists&lt;block::BlockResource&gt;(@aptos_framework);<br/>ensures exists&lt;state_storage::StateStorageUsage&gt;(@aptos_framework);<br/>ensures exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>ensures exists&lt;account::Account&gt;(@aptos_framework);<br/>ensures exists&lt;version::SetVersionCapability&gt;(@aptos_framework);<br/>ensures exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_initialize_aptos_coin"></a>

### Function `initialize_aptos_coin`


<pre><code>fun initialize_aptos_coin(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
requires !exists&lt;stake::AptosCoinCapabilities&gt;(@aptos_framework);<br/>ensures exists&lt;stake::AptosCoinCapabilities&gt;(@aptos_framework);<br/>requires exists&lt;transaction_fee::AptosCoinCapabilities&gt;(@aptos_framework);<br/>ensures exists&lt;transaction_fee::AptosCoinCapabilities&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_create_initialize_validators_with_commission"></a>

### Function `create_initialize_validators_with_commission`


<pre><code>fun create_initialize_validators_with_commission(aptos_framework: &amp;signer, use_staking_contract: bool, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include stake::ResourceRequirement;<br/>include stake::GetReconfigStartTimeRequirement;<br/>include CompareTimeRequires;<br/>include aptos_coin::ExistsAptosCoin;<br/></code></pre>



<a id="@Specification_1_create_initialize_validators"></a>

### Function `create_initialize_validators`


<pre><code>fun create_initialize_validators(aptos_framework: &amp;signer, validators: vector&lt;genesis::ValidatorConfiguration&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include stake::ResourceRequirement;<br/>include stake::GetReconfigStartTimeRequirement;<br/>include CompareTimeRequires;<br/>include aptos_coin::ExistsAptosCoin;<br/></code></pre>



<a id="@Specification_1_create_initialize_validator"></a>

### Function `create_initialize_validator`


<pre><code>fun create_initialize_validator(aptos_framework: &amp;signer, commission_config: &amp;genesis::ValidatorConfigurationWithCommission, use_staking_contract: bool)<br/></code></pre>




<pre><code>include stake::ResourceRequirement;<br/></code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code>fun set_genesis_end(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>pragma delegate_invariants_to_caller;<br/>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
requires len(global&lt;stake::ValidatorSet&gt;(@aptos_framework).active_validators) &gt;&#61; 1;<br/>// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
let addr &#61; std::signer::address_of(aptos_framework);<br/>aborts_if addr !&#61; @aptos_framework;<br/>aborts_if exists&lt;chain_status::GenesisEndMarker&gt;(@aptos_framework);<br/>ensures global&lt;chain_status::GenesisEndMarker&gt;(@aptos_framework) &#61;&#61; chain_status::GenesisEndMarker &#123;&#125;;<br/></code></pre>




<a id="0x1_genesis_InitalizeRequires"></a>


<pre><code>schema InitalizeRequires &#123;<br/>execution_config: vector&lt;u8&gt;;<br/>requires !exists&lt;account::Account&gt;(@aptos_framework);<br/>requires chain_status::is_operating();<br/>requires len(execution_config) &gt; 0;<br/>requires exists&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>include CompareTimeRequires;<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>&#125;<br/></code></pre>




<a id="0x1_genesis_CompareTimeRequires"></a>


<pre><code>schema CompareTimeRequires &#123;<br/>let staking_rewards_config &#61; global&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);<br/>requires staking_rewards_config.last_rewards_rate_period_start_in_secs &lt;&#61; timestamp::spec_now_seconds();<br/>&#125;<br/></code></pre>



<a id="@Specification_1_initialize_for_verification"></a>

### Function `initialize_for_verification`


<pre><code>&#35;[verify_only]<br/>fun initialize_for_verification(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64, accounts: vector&lt;genesis::AccountMap&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: vector&lt;genesis::EmployeeAccountMap&gt;, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include InitalizeRequires;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
