
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


<pre><code>use 0x1::account;
use 0x1::aggregator_factory;
use 0x1::aptos_coin;
use 0x1::aptos_governance;
use 0x1::block;
use 0x1::chain_id;
use 0x1::chain_status;
use 0x1::coin;
use 0x1::consensus_config;
use 0x1::create_signer;
use 0x1::error;
use 0x1::execution_config;
use 0x1::features;
use 0x1::fixed_point32;
use 0x1::gas_schedule;
use 0x1::reconfiguration;
use 0x1::simple_map;
use 0x1::stake;
use 0x1::staking_config;
use 0x1::staking_contract;
use 0x1::state_storage;
use 0x1::storage_gas;
use 0x1::timestamp;
use 0x1::transaction_fee;
use 0x1::transaction_validation;
use 0x1::vector;
use 0x1::version;
use 0x1::vesting;
</code></pre>



<a id="0x1_genesis_AccountMap"></a>

## Struct `AccountMap`



<pre><code>struct AccountMap has drop
</code></pre>



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



<pre><code>struct EmployeeAccountMap has copy, drop
</code></pre>



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



<pre><code>struct ValidatorConfiguration has copy, drop
</code></pre>



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



<pre><code>struct ValidatorConfigurationWithCommission has copy, drop
</code></pre>



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



<pre><code>const EACCOUNT_DOES_NOT_EXIST: u64 &#61; 2;
</code></pre>



<a id="0x1_genesis_EDUPLICATE_ACCOUNT"></a>



<pre><code>const EDUPLICATE_ACCOUNT: u64 &#61; 1;
</code></pre>



<a id="0x1_genesis_initialize"></a>

## Function `initialize`

Genesis step 1: Initialize aptos framework account and core modules on chain.


<pre><code>fun initialize(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize(
    gas_schedule: vector&lt;u8&gt;,
    chain_id: u8,
    initial_version: u64,
    consensus_config: vector&lt;u8&gt;,
    execution_config: vector&lt;u8&gt;,
    epoch_interval_microsecs: u64,
    minimum_stake: u64,
    maximum_stake: u64,
    recurring_lockup_duration_secs: u64,
    allow_validator_set_change: bool,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
    voting_power_increase_limit: u64,
) &#123;
    // Initialize the aptos framework account. This is the account where system resources and modules will be
    // deployed to. This will be entirely managed by on&#45;chain governance and no entities have the key or privileges
    // to use this account.
    let (aptos_framework_account, aptos_framework_signer_cap) &#61; account::create_framework_reserved_account(@aptos_framework);
    // Initialize account configs on aptos framework account.
    account::initialize(&amp;aptos_framework_account);

    transaction_validation::initialize(
        &amp;aptos_framework_account,
        b&quot;script_prologue&quot;,
        b&quot;module_prologue&quot;,
        b&quot;multi_agent_script_prologue&quot;,
        b&quot;epilogue&quot;,
    );

    // Give the decentralized on&#45;chain governance control over the core framework account.
    aptos_governance::store_signer_cap(&amp;aptos_framework_account, @aptos_framework, aptos_framework_signer_cap);

    // put reserved framework reserved accounts under aptos governance
    let framework_reserved_addresses &#61; vector&lt;address&gt;[@0x2, @0x3, @0x4, @0x5, @0x6, @0x7, @0x8, @0x9, @0xa];
    while (!vector::is_empty(&amp;framework_reserved_addresses)) &#123;
        let address &#61; vector::pop_back&lt;address&gt;(&amp;mut framework_reserved_addresses);
        let (_, framework_signer_cap) &#61; account::create_framework_reserved_account(address);
        aptos_governance::store_signer_cap(&amp;aptos_framework_account, address, framework_signer_cap);
    &#125;;

    consensus_config::initialize(&amp;aptos_framework_account, consensus_config);
    execution_config::set(&amp;aptos_framework_account, execution_config);
    version::initialize(&amp;aptos_framework_account, initial_version);
    stake::initialize(&amp;aptos_framework_account);
    staking_config::initialize(
        &amp;aptos_framework_account,
        minimum_stake,
        maximum_stake,
        recurring_lockup_duration_secs,
        allow_validator_set_change,
        rewards_rate,
        rewards_rate_denominator,
        voting_power_increase_limit,
    );
    storage_gas::initialize(&amp;aptos_framework_account);
    gas_schedule::initialize(&amp;aptos_framework_account, gas_schedule);

    // Ensure we can create aggregators for supply, but not enable it for common use just yet.
    aggregator_factory::initialize_aggregator_factory(&amp;aptos_framework_account);
    coin::initialize_supply_config(&amp;aptos_framework_account);

    chain_id::initialize(&amp;aptos_framework_account, chain_id);
    reconfiguration::initialize(&amp;aptos_framework_account);
    block::initialize(&amp;aptos_framework_account, epoch_interval_microsecs);
    state_storage::initialize(&amp;aptos_framework_account);
    timestamp::set_time_has_started(&amp;aptos_framework_account);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_initialize_aptos_coin"></a>

## Function `initialize_aptos_coin`

Genesis step 2: Initialize Aptos coin.


<pre><code>fun initialize_aptos_coin(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_aptos_coin(aptos_framework: &amp;signer) &#123;
    let (burn_cap, mint_cap) &#61; aptos_coin::initialize(aptos_framework);

    coin::create_coin_conversion_map(aptos_framework);
    coin::create_pairing&lt;AptosCoin&gt;(aptos_framework);

    // Give stake module MintCapability&lt;AptosCoin&gt; so it can mint rewards.
    stake::store_aptos_coin_mint_cap(aptos_framework, mint_cap);
    // Give transaction_fee module BurnCapability&lt;AptosCoin&gt; so it can burn gas.
    transaction_fee::store_aptos_coin_burn_cap(aptos_framework, burn_cap);
    // Give transaction_fee module MintCapability&lt;AptosCoin&gt; so it can mint refunds.
    transaction_fee::store_aptos_coin_mint_cap(aptos_framework, mint_cap);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_initialize_core_resources_and_aptos_coin"></a>

## Function `initialize_core_resources_and_aptos_coin`

Only called for testnets and e2e tests.


<pre><code>fun initialize_core_resources_and_aptos_coin(aptos_framework: &amp;signer, core_resources_auth_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_core_resources_and_aptos_coin(
    aptos_framework: &amp;signer,
    core_resources_auth_key: vector&lt;u8&gt;,
) &#123;
    let (burn_cap, mint_cap) &#61; aptos_coin::initialize(aptos_framework);

    coin::create_coin_conversion_map(aptos_framework);
    coin::create_pairing&lt;AptosCoin&gt;(aptos_framework);

    // Give stake module MintCapability&lt;AptosCoin&gt; so it can mint rewards.
    stake::store_aptos_coin_mint_cap(aptos_framework, mint_cap);
    // Give transaction_fee module BurnCapability&lt;AptosCoin&gt; so it can burn gas.
    transaction_fee::store_aptos_coin_burn_cap(aptos_framework, burn_cap);
    // Give transaction_fee module MintCapability&lt;AptosCoin&gt; so it can mint refunds.
    transaction_fee::store_aptos_coin_mint_cap(aptos_framework, mint_cap);

    let core_resources &#61; account::create_account(@core_resources);
    account::rotate_authentication_key_internal(&amp;core_resources, core_resources_auth_key);
    aptos_coin::configure_accounts_for_test(aptos_framework, &amp;core_resources, mint_cap);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_create_accounts"></a>

## Function `create_accounts`



<pre><code>fun create_accounts(aptos_framework: &amp;signer, accounts: vector&lt;genesis::AccountMap&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_accounts(aptos_framework: &amp;signer, accounts: vector&lt;AccountMap&gt;) &#123;
    let unique_accounts &#61; vector::empty();
    vector::for_each_ref(&amp;accounts, &#124;account_map&#124; &#123;
        let account_map: &amp;AccountMap &#61; account_map;
        assert!(
            !vector::contains(&amp;unique_accounts, &amp;account_map.account_address),
            error::already_exists(EDUPLICATE_ACCOUNT),
        );
        vector::push_back(&amp;mut unique_accounts, account_map.account_address);

        create_account(
            aptos_framework,
            account_map.account_address,
            account_map.balance,
        );
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_create_account"></a>

## Function `create_account`

This creates an funds an account if it doesn't exist.
If it exists, it just returns the signer.


<pre><code>fun create_account(aptos_framework: &amp;signer, account_address: address, balance: u64): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_account(aptos_framework: &amp;signer, account_address: address, balance: u64): signer &#123;
    if (account::exists_at(account_address)) &#123;
        create_signer(account_address)
    &#125; else &#123;
        let account &#61; account::create_account(account_address);
        coin::register&lt;AptosCoin&gt;(&amp;account);
        aptos_coin::mint(aptos_framework, account_address, balance);
        account
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_genesis_create_employee_validators"></a>

## Function `create_employee_validators`



<pre><code>fun create_employee_validators(employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: vector&lt;genesis::EmployeeAccountMap&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_employee_validators(
    employee_vesting_start: u64,
    employee_vesting_period_duration: u64,
    employees: vector&lt;EmployeeAccountMap&gt;,
) &#123;
    let unique_accounts &#61; vector::empty();

    vector::for_each_ref(&amp;employees, &#124;employee_group&#124; &#123;
        let j &#61; 0;
        let employee_group: &amp;EmployeeAccountMap &#61; employee_group;
        let num_employees_in_group &#61; vector::length(&amp;employee_group.accounts);

        let buy_ins &#61; simple_map::create();

        while (j &lt; num_employees_in_group) &#123;
            let account &#61; vector::borrow(&amp;employee_group.accounts, j);
            assert!(
                !vector::contains(&amp;unique_accounts, account),
                error::already_exists(EDUPLICATE_ACCOUNT),
            );
            vector::push_back(&amp;mut unique_accounts, &#42;account);

            let employee &#61; create_signer(&#42;account);
            let total &#61; coin::balance&lt;AptosCoin&gt;(&#42;account);
            let coins &#61; coin::withdraw&lt;AptosCoin&gt;(&amp;employee, total);
            simple_map::add(&amp;mut buy_ins, &#42;account, coins);

            j &#61; j &#43; 1;
        &#125;;

        let j &#61; 0;
        let num_vesting_events &#61; vector::length(&amp;employee_group.vesting_schedule_numerator);
        let schedule &#61; vector::empty();

        while (j &lt; num_vesting_events) &#123;
            let numerator &#61; vector::borrow(&amp;employee_group.vesting_schedule_numerator, j);
            let event &#61; fixed_point32::create_from_rational(&#42;numerator, employee_group.vesting_schedule_denominator);
            vector::push_back(&amp;mut schedule, event);

            j &#61; j &#43; 1;
        &#125;;

        let vesting_schedule &#61; vesting::create_vesting_schedule(
            schedule,
            employee_vesting_start,
            employee_vesting_period_duration,
        );

        let admin &#61; employee_group.validator.validator_config.owner_address;
        let admin_signer &#61; &amp;create_signer(admin);
        let contract_address &#61; vesting::create_vesting_contract(
            admin_signer,
            &amp;employee_group.accounts,
            buy_ins,
            vesting_schedule,
            admin,
            employee_group.validator.validator_config.operator_address,
            employee_group.validator.validator_config.voter_address,
            employee_group.validator.commission_percentage,
            x&quot;&quot;,
        );
        let pool_address &#61; vesting::stake_pool_address(contract_address);

        if (employee_group.beneficiary_resetter !&#61; @0x0) &#123;
            vesting::set_beneficiary_resetter(admin_signer, contract_address, employee_group.beneficiary_resetter);
        &#125;;

        let validator &#61; &amp;employee_group.validator.validator_config;
        assert!(
            account::exists_at(validator.owner_address),
            error::not_found(EACCOUNT_DOES_NOT_EXIST),
        );
        assert!(
            account::exists_at(validator.operator_address),
            error::not_found(EACCOUNT_DOES_NOT_EXIST),
        );
        assert!(
            account::exists_at(validator.voter_address),
            error::not_found(EACCOUNT_DOES_NOT_EXIST),
        );
        if (employee_group.validator.join_during_genesis) &#123;
            initialize_validator(pool_address, validator);
        &#125;;
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_create_initialize_validators_with_commission"></a>

## Function `create_initialize_validators_with_commission`



<pre><code>fun create_initialize_validators_with_commission(aptos_framework: &amp;signer, use_staking_contract: bool, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_initialize_validators_with_commission(
    aptos_framework: &amp;signer,
    use_staking_contract: bool,
    validators: vector&lt;ValidatorConfigurationWithCommission&gt;,
) &#123;
    vector::for_each_ref(&amp;validators, &#124;validator&#124; &#123;
        let validator: &amp;ValidatorConfigurationWithCommission &#61; validator;
        create_initialize_validator(aptos_framework, validator, use_staking_contract);
    &#125;);

    // Destroy the aptos framework account&apos;s ability to mint coins now that we&apos;re done with setting up the initial
    // validators.
    aptos_coin::destroy_mint_cap(aptos_framework);

    stake::on_new_epoch();
&#125;
</code></pre>



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


<pre><code>fun create_initialize_validators(aptos_framework: &amp;signer, validators: vector&lt;genesis::ValidatorConfiguration&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_initialize_validators(aptos_framework: &amp;signer, validators: vector&lt;ValidatorConfiguration&gt;) &#123;
    let validators_with_commission &#61; vector::empty();
    vector::for_each_reverse(validators, &#124;validator&#124; &#123;
        let validator_with_commission &#61; ValidatorConfigurationWithCommission &#123;
            validator_config: validator,
            commission_percentage: 0,
            join_during_genesis: true,
        &#125;;
        vector::push_back(&amp;mut validators_with_commission, validator_with_commission);
    &#125;);

    create_initialize_validators_with_commission(aptos_framework, false, validators_with_commission);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_create_initialize_validator"></a>

## Function `create_initialize_validator`



<pre><code>fun create_initialize_validator(aptos_framework: &amp;signer, commission_config: &amp;genesis::ValidatorConfigurationWithCommission, use_staking_contract: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_initialize_validator(
    aptos_framework: &amp;signer,
    commission_config: &amp;ValidatorConfigurationWithCommission,
    use_staking_contract: bool,
) &#123;
    let validator &#61; &amp;commission_config.validator_config;

    let owner &#61; &amp;create_account(aptos_framework, validator.owner_address, validator.stake_amount);
    create_account(aptos_framework, validator.operator_address, 0);
    create_account(aptos_framework, validator.voter_address, 0);

    // Initialize the stake pool and join the validator set.
    let pool_address &#61; if (use_staking_contract) &#123;
        staking_contract::create_staking_contract(
            owner,
            validator.operator_address,
            validator.voter_address,
            validator.stake_amount,
            commission_config.commission_percentage,
            x&quot;&quot;,
        );
        staking_contract::stake_pool_address(validator.owner_address, validator.operator_address)
    &#125; else &#123;
        stake::initialize_stake_owner(
            owner,
            validator.stake_amount,
            validator.operator_address,
            validator.voter_address,
        );
        validator.owner_address
    &#125;;

    if (commission_config.join_during_genesis) &#123;
        initialize_validator(pool_address, validator);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_genesis_initialize_validator"></a>

## Function `initialize_validator`



<pre><code>fun initialize_validator(pool_address: address, validator: &amp;genesis::ValidatorConfiguration)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_validator(pool_address: address, validator: &amp;ValidatorConfiguration) &#123;
    let operator &#61; &amp;create_signer(validator.operator_address);

    stake::rotate_consensus_key(
        operator,
        pool_address,
        validator.consensus_pubkey,
        validator.proof_of_possession,
    );
    stake::update_network_and_fullnode_addresses(
        operator,
        pool_address,
        validator.network_addresses,
        validator.full_node_network_addresses,
    );
    stake::join_validator_set_internal(operator, pool_address);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_set_genesis_end"></a>

## Function `set_genesis_end`

The last step of genesis.


<pre><code>fun set_genesis_end(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun set_genesis_end(aptos_framework: &amp;signer) &#123;
    chain_status::set_genesis_end(aptos_framework);
&#125;
</code></pre>



</details>

<a id="0x1_genesis_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>&#35;[verify_only]
fun initialize_for_verification(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64, accounts: vector&lt;genesis::AccountMap&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: vector&lt;genesis::EmployeeAccountMap&gt;, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_for_verification(
    gas_schedule: vector&lt;u8&gt;,
    chain_id: u8,
    initial_version: u64,
    consensus_config: vector&lt;u8&gt;,
    execution_config: vector&lt;u8&gt;,
    epoch_interval_microsecs: u64,
    minimum_stake: u64,
    maximum_stake: u64,
    recurring_lockup_duration_secs: u64,
    allow_validator_set_change: bool,
    rewards_rate: u64,
    rewards_rate_denominator: u64,
    voting_power_increase_limit: u64,
    aptos_framework: &amp;signer,
    min_voting_threshold: u128,
    required_proposer_stake: u64,
    voting_duration_secs: u64,
    accounts: vector&lt;AccountMap&gt;,
    employee_vesting_start: u64,
    employee_vesting_period_duration: u64,
    employees: vector&lt;EmployeeAccountMap&gt;,
    validators: vector&lt;ValidatorConfigurationWithCommission&gt;
) &#123;
    initialize(
        gas_schedule,
        chain_id,
        initial_version,
        consensus_config,
        execution_config,
        epoch_interval_microsecs,
        minimum_stake,
        maximum_stake,
        recurring_lockup_duration_secs,
        allow_validator_set_change,
        rewards_rate,
        rewards_rate_denominator,
        voting_power_increase_limit
    );
    features::change_feature_flags_for_verification(aptos_framework, vector[1, 2], vector[]);
    initialize_aptos_coin(aptos_framework);
    aptos_governance::initialize_for_verification(
        aptos_framework,
        min_voting_threshold,
        required_proposer_stake,
        voting_duration_secs
    );
    create_accounts(aptos_framework, accounts);
    create_employee_validators(employee_vesting_start, employee_vesting_period_duration, employees);
    create_initialize_validators_with_commission(aptos_framework, true, validators);
    set_genesis_end(aptos_framework);
&#125;
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


<pre><code>pragma verify &#61; true;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>fun initialize(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
include InitalizeRequires;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if exists&lt;account::Account&gt;(@0x0);
aborts_if exists&lt;account::Account&gt;(@0x2);
aborts_if exists&lt;account::Account&gt;(@0x3);
aborts_if exists&lt;account::Account&gt;(@0x4);
aborts_if exists&lt;account::Account&gt;(@0x5);
aborts_if exists&lt;account::Account&gt;(@0x6);
aborts_if exists&lt;account::Account&gt;(@0x7);
aborts_if exists&lt;account::Account&gt;(@0x8);
aborts_if exists&lt;account::Account&gt;(@0x9);
aborts_if exists&lt;account::Account&gt;(@0xa);
ensures exists&lt;account::Account&gt;(@0x0);
ensures exists&lt;account::Account&gt;(@0x2);
ensures exists&lt;account::Account&gt;(@0x3);
ensures exists&lt;account::Account&gt;(@0x4);
ensures exists&lt;account::Account&gt;(@0x5);
ensures exists&lt;account::Account&gt;(@0x6);
ensures exists&lt;account::Account&gt;(@0x7);
ensures exists&lt;account::Account&gt;(@0x8);
ensures exists&lt;account::Account&gt;(@0x9);
ensures exists&lt;account::Account&gt;(@0xa);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures exists&lt;aptos_governance::GovernanceResponsbility&gt;(@aptos_framework);
ensures exists&lt;consensus_config::ConsensusConfig&gt;(@aptos_framework);
ensures exists&lt;execution_config::ExecutionConfig&gt;(@aptos_framework);
ensures exists&lt;version::Version&gt;(@aptos_framework);
ensures exists&lt;stake::ValidatorSet&gt;(@aptos_framework);
ensures exists&lt;stake::ValidatorPerformance&gt;(@aptos_framework);
ensures exists&lt;storage_gas::StorageGasConfig&gt;(@aptos_framework);
ensures exists&lt;storage_gas::StorageGas&gt;(@aptos_framework);
ensures exists&lt;gas_schedule::GasScheduleV2&gt;(@aptos_framework);
ensures exists&lt;aggregator_factory::AggregatorFactory&gt;(@aptos_framework);
ensures exists&lt;coin::SupplyConfig&gt;(@aptos_framework);
ensures exists&lt;chain_id::ChainId&gt;(@aptos_framework);
ensures exists&lt;reconfiguration::Configuration&gt;(@aptos_framework);
ensures exists&lt;block::BlockResource&gt;(@aptos_framework);
ensures exists&lt;state_storage::StateStorageUsage&gt;(@aptos_framework);
ensures exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
ensures exists&lt;account::Account&gt;(@aptos_framework);
ensures exists&lt;version::SetVersionCapability&gt;(@aptos_framework);
ensures exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_initialize_aptos_coin"></a>

### Function `initialize_aptos_coin`


<pre><code>fun initialize_aptos_coin(aptos_framework: &amp;signer)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
requires !exists&lt;stake::AptosCoinCapabilities&gt;(@aptos_framework);
ensures exists&lt;stake::AptosCoinCapabilities&gt;(@aptos_framework);
requires exists&lt;transaction_fee::AptosCoinCapabilities&gt;(@aptos_framework);
ensures exists&lt;transaction_fee::AptosCoinCapabilities&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_create_initialize_validators_with_commission"></a>

### Function `create_initialize_validators_with_commission`


<pre><code>fun create_initialize_validators_with_commission(aptos_framework: &amp;signer, use_staking_contract: bool, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
include stake::ResourceRequirement;
include stake::GetReconfigStartTimeRequirement;
include CompareTimeRequires;
include aptos_coin::ExistsAptosCoin;
</code></pre>



<a id="@Specification_1_create_initialize_validators"></a>

### Function `create_initialize_validators`


<pre><code>fun create_initialize_validators(aptos_framework: &amp;signer, validators: vector&lt;genesis::ValidatorConfiguration&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
include stake::ResourceRequirement;
include stake::GetReconfigStartTimeRequirement;
include CompareTimeRequires;
include aptos_coin::ExistsAptosCoin;
</code></pre>



<a id="@Specification_1_create_initialize_validator"></a>

### Function `create_initialize_validator`


<pre><code>fun create_initialize_validator(aptos_framework: &amp;signer, commission_config: &amp;genesis::ValidatorConfigurationWithCommission, use_staking_contract: bool)
</code></pre>




<pre><code>include stake::ResourceRequirement;
</code></pre>



<a id="@Specification_1_set_genesis_end"></a>

### Function `set_genesis_end`


<pre><code>fun set_genesis_end(aptos_framework: &amp;signer)
</code></pre>




<pre><code>pragma delegate_invariants_to_caller;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
requires len(global&lt;stake::ValidatorSet&gt;(@aptos_framework).active_validators) &gt;&#61; 1;
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
let addr &#61; std::signer::address_of(aptos_framework);
aborts_if addr !&#61; @aptos_framework;
aborts_if exists&lt;chain_status::GenesisEndMarker&gt;(@aptos_framework);
ensures global&lt;chain_status::GenesisEndMarker&gt;(@aptos_framework) &#61;&#61; chain_status::GenesisEndMarker &#123;&#125;;
</code></pre>




<a id="0x1_genesis_InitalizeRequires"></a>


<pre><code>schema InitalizeRequires &#123;
    execution_config: vector&lt;u8&gt;;
    requires !exists&lt;account::Account&gt;(@aptos_framework);
    requires chain_status::is_operating();
    requires len(execution_config) &gt; 0;
    requires exists&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);
    requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
    requires exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
    include CompareTimeRequires;
    include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
&#125;
</code></pre>




<a id="0x1_genesis_CompareTimeRequires"></a>


<pre><code>schema CompareTimeRequires &#123;
    let staking_rewards_config &#61; global&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);
    requires staking_rewards_config.last_rewards_rate_period_start_in_secs &lt;&#61; timestamp::spec_now_seconds();
&#125;
</code></pre>



<a id="@Specification_1_initialize_for_verification"></a>

### Function `initialize_for_verification`


<pre><code>&#35;[verify_only]
fun initialize_for_verification(gas_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, execution_config: vector&lt;u8&gt;, epoch_interval_microsecs: u64, minimum_stake: u64, maximum_stake: u64, recurring_lockup_duration_secs: u64, allow_validator_set_change: bool, rewards_rate: u64, rewards_rate_denominator: u64, voting_power_increase_limit: u64, aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64, accounts: vector&lt;genesis::AccountMap&gt;, employee_vesting_start: u64, employee_vesting_period_duration: u64, employees: vector&lt;genesis::EmployeeAccountMap&gt;, validators: vector&lt;genesis::ValidatorConfigurationWithCommission&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
include InitalizeRequires;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
