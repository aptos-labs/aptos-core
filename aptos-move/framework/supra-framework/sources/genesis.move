module supra_framework::genesis {
    use std::error;
    use std::fixed_point32;
    use std::option;
    use std::string::String;
    use std::vector;
    use aptos_std::simple_map;

    use supra_framework::account;
    use supra_framework::aggregator_factory;
    use supra_framework::automation_registry;
    use supra_framework::block;
    use supra_framework::chain_id;
    use supra_framework::chain_status;
    use supra_framework::coin;
    use supra_framework::consensus_config;
    use supra_framework::execution_config;
    use supra_framework::supra_config;
    use supra_framework::evm_config;
    use supra_framework::create_signer::create_signer;
    use supra_framework::gas_schedule;
    use supra_framework::multisig_account;
    use supra_framework::pbo_delegation_pool;
    use supra_framework::reconfiguration;
    use supra_framework::stake;
    use supra_framework::staking_config;
    use supra_framework::staking_contract;
    use supra_framework::state_storage;
    use supra_framework::storage_gas;
    use supra_framework::supra_account;
    use supra_framework::supra_coin::{Self, SupraCoin};
    use supra_framework::supra_governance;
    use supra_framework::timestamp;
    use supra_framework::transaction_fee;
    use supra_framework::transaction_validation;
    use supra_framework::version;
    use supra_framework::vesting;
    use supra_framework::vesting_without_staking;

    #[test_only]
    use aptos_std::ed25519;

    #[verify_only]
    use std::features;

    const VESTING_CONTRACT_SEED: vector<u8> = b"VESTING_WIHOUT_STAKING_SEED";

    const EDUPLICATE_ACCOUNT: u64 = 1;
    const EACCOUNT_DOES_NOT_EXIST: u64 = 2;
    const EVESTING_SCHEDULE_IS_ZERO: u64 = 3;
    const ENUMERATOR_IS_ZERO: u64 = 4;
    const ENO_SHAREHOLDERS: u64 = 5;
    const EPERCENTAGE_INVALID: u64 = 6;
    const ENUMERATOR_GREATER_THAN_DENOMINATOR: u64 = 7;
    const EDENOMINATOR_IS_ZERO: u64 = 8;
    const EACCOUNT_NOT_REGISTERED_FOR_COIN: u64 = 9;


    struct AccountMap has drop {
        account_address: address,
        balance: u64,
    }

    struct VestingPoolsMap has copy, drop {
        // Address of the admin of the vesting pool
        admin_address: address,
        // Percentage of account balance should be put in vesting pool
        vpool_locking_percentage: u8,
        vesting_numerators: vector<u64>,
        vesting_denominator: u64,
        // Withdrawal address for the pool
        withdrawal_address: address,
        // Shareholders in the vesting pool
        shareholders: vector<address>,
        // Cliff duration in seconds
        cliff_period_in_seconds: u64,
        // Each vesting period duration in seconds
        period_duration_in_seconds: u64,
    }

    struct EmployeeAccountMap has copy, drop {
        accounts: vector<address>,
        validator: ValidatorConfigurationWithCommission,
        vesting_schedule_numerator: vector<u64>,
        vesting_schedule_denominator: u64,
        beneficiary_resetter: address,
    }

    struct ValidatorConfiguration has copy, drop {
        owner_address: address,
        operator_address: address,
        voter_address: address,
        stake_amount: u64,
        consensus_pubkey: vector<u8>,
        network_addresses: vector<u8>,
        full_node_network_addresses: vector<u8>,
    }

    struct ValidatorConfigurationWithCommission has copy, drop {
        validator_config: ValidatorConfiguration,
        commission_percentage: u64,
        join_during_genesis: bool,
    }

    struct DelegatorConfiguration has copy, drop {
        owner_address: address,
        delegation_pool_creation_seed: vector<u8>,
        validator: ValidatorConfigurationWithCommission,
        delegator_addresses: vector<address>,
        delegator_stakes: vector<u64>,
    }

    struct PboDelegatorConfiguration has copy, drop {
        delegator_config: DelegatorConfiguration,
        //Address of the multisig admin of the pool
        multisig_admin: address,
        //Numerator for unlock fraction
        unlock_schedule_numerators: vector<u64>,
        //Denominator for unlock fraction
        unlock_schedule_denominator: u64,
        //Time from `timestamp::now_seconds()` to start unlocking schedule
        unlock_startup_time_from_now: u64,
        //Time for each unlock
        unlock_period_duration: u64,
    }

    /// Genesis step 1: Initialize supra framework account and core modules on chain.
    fun initialize(
        gas_schedule: vector<u8>,
        chain_id: u8,
        initial_version: u64,
        consensus_config: vector<u8>,
        execution_config: vector<u8>,
        supra_config: vector<u8>,
        epoch_interval_microsecs: u64,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
        genesis_timestamp_in_microseconds: u64,
        evm_config: vector<u8>,
    ) {
        // Initialize the supra framework account. This is the account where system resources and modules will be
        // deployed to. This will be entirely managed by on-chain governance and no entities have the key or privileges
        // to use this account.
        let (supra_framework_account, supra_framework_signer_cap) = account::create_framework_reserved_account(
            @supra_framework
        );
        // Initialize account configs on supra framework account.
        account::initialize(&supra_framework_account);

        transaction_validation::initialize(
            &supra_framework_account,
            b"script_prologue",
            b"module_prologue",
            b"multi_agent_script_prologue",
            b"epilogue",
        );

        // Give the decentralized on-chain governance control over the core framework account.
        supra_governance::store_signer_cap(&supra_framework_account, @supra_framework, supra_framework_signer_cap);

        // put reserved framework reserved accounts under supra governance
        let framework_reserved_addresses = vector<address>[@0x2, @0x3, @0x4, @0x5, @0x6, @0x7, @0x8, @0x9, @0xa];
        while (!vector::is_empty(&framework_reserved_addresses)) {
            let address = vector::pop_back<address>(&mut framework_reserved_addresses);
            let (_, framework_signer_cap) = account::create_framework_reserved_account(address);
            supra_governance::store_signer_cap(&supra_framework_account, address, framework_signer_cap);
        };

        consensus_config::initialize(&supra_framework_account, consensus_config);
        execution_config::set(&supra_framework_account, execution_config);
        supra_config::initialize(&supra_framework_account, supra_config);
        version::initialize(&supra_framework_account, initial_version);
        stake::initialize(&supra_framework_account);
        staking_config::initialize(
            &supra_framework_account,
            minimum_stake,
            maximum_stake,
            recurring_lockup_duration_secs,
            allow_validator_set_change,
            rewards_rate,
            rewards_rate_denominator,
            voting_power_increase_limit,
        );
        storage_gas::initialize(&supra_framework_account);
        gas_schedule::initialize(&supra_framework_account, gas_schedule);

        // Ensure we can create aggregators for supply, but not enable it for common use just yet.
        aggregator_factory::initialize_aggregator_factory(&supra_framework_account);
        coin::initialize_supply_config(&supra_framework_account);

        chain_id::initialize(&supra_framework_account, chain_id);
        reconfiguration::initialize(&supra_framework_account);
        block::initialize(&supra_framework_account, epoch_interval_microsecs);
        state_storage::initialize(&supra_framework_account);
        timestamp::set_time_has_started(&supra_framework_account, genesis_timestamp_in_microseconds);
        evm_config::initialize(&supra_framework_account, evm_config);
    }

    /// Genesis step 2: Initialize Supra coin.
    fun initialize_supra_coin(supra_framework: &signer) {
        let (burn_cap, mint_cap) = supra_coin::initialize(supra_framework);
        coin::create_coin_conversion_map(supra_framework);
        coin::create_pairing<SupraCoin>(supra_framework);
        // Give stake module MintCapability<SupraCoin> so it can mint rewards.
        stake::store_supra_coin_mint_cap(supra_framework, mint_cap);
        // Give transaction_fee module BurnCapability<SupraCoin> so it can burn gas.
        transaction_fee::store_supra_coin_burn_cap(supra_framework, burn_cap);
        // Give transaction_fee module MintCapability<SupraCoin> so it can mint refunds.
        transaction_fee::store_supra_coin_mint_cap(supra_framework, mint_cap);
    }

    /// Genesis step 3: Initialize Supra Native Automation.
    public fun initialize_supra_native_automation(
        supra_framework: &signer,
        task_duration_cap_in_secs: u64,
        registry_max_gas_cap: u64,
        automation_base_fee_in_quants_per_sec: u64,
        flat_registration_fee_in_quants: u64,
        congestion_threshold_percentage: u8,
        congestion_base_fee_in_quants_per_sec: u64,
        congestion_exponent: u8,
        task_capacity: u16,
    ) {
        let epoch_interval_secs = block::get_epoch_interval_secs();
        automation_registry::initialize(
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


    /// Only called for testnets and e2e tests.
    fun initialize_core_resources_and_supra_coin(
        supra_framework: &signer,
        core_resources_auth_key: vector<u8>,
    ) {
        let (burn_cap, mint_cap) = supra_coin::initialize(supra_framework);
        coin::create_coin_conversion_map(supra_framework);
        coin::create_pairing<SupraCoin>(supra_framework);
        // Give stake module MintCapability<SupraCoin> so it can mint rewards.
        stake::store_supra_coin_mint_cap(supra_framework, mint_cap);
        // Give transaction_fee module BurnCapability<SupraCoin> so it can burn gas.
        transaction_fee::store_supra_coin_burn_cap(supra_framework, burn_cap);
        // Give transaction_fee module MintCapability<SupraCoin> so it can mint refunds.
        transaction_fee::store_supra_coin_mint_cap(supra_framework, mint_cap);

        let core_resources = account::create_account(@core_resources);
        supra_account::register_supra(&core_resources); // register Supra store
        account::rotate_authentication_key_internal(&core_resources, core_resources_auth_key);
        supra_coin::configure_accounts_for_test(supra_framework, &core_resources, mint_cap);
    }

    fun create_accounts(supra_framework: &signer, accounts: vector<AccountMap>) {
        let unique_accounts = vector::empty();
        vector::for_each_ref(&accounts, |account_map| {
            let account_map: &AccountMap = account_map;
            assert!(
                !vector::contains(&unique_accounts, &account_map.account_address),
                error::already_exists(EDUPLICATE_ACCOUNT),
            );
            vector::push_back(&mut unique_accounts, account_map.account_address);

            create_account(
                supra_framework,
                account_map.account_address,
                account_map.balance,
            );
        });
    }

    /// This creates an funds an account if it doesn't exist.
    /// If it exists, it just returns the signer.
    fun create_account(supra_framework: &signer, account_address: address, balance: u64): signer {
        if (account::exists_at(account_address)) {
            create_signer(account_address)
        } else {
            let account = account::create_account(account_address);
            coin::register<SupraCoin>(&account);
            supra_coin::mint(supra_framework, account_address, balance);
            account
        }
    }


    fun create_multiple_multisig_accounts_with_schema(
        supra_framework: &signer,
        owner: address,
        additional_owners: vector<address>,
        num_signatures_required: u64,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
        timeout_duration: u64,
        balance: u64,
        num_of_accounts: u32
    ): vector<address> {
        let counter = 0;
        let result = vector::empty();
        while (counter < num_of_accounts) {
            let account_addr = create_multisig_account_with_balance(supra_framework, owner, additional_owners,
                num_signatures_required, metadata_keys, metadata_values, timeout_duration, balance);
            vector::push_back(&mut result, account_addr);
            counter = counter + 1;
        };
        result
    }

    fun create_multisig_account_with_balance(
        supra_framework: &signer,
        owner: address,
        additional_owners: vector<address>,
        num_signatures_required: u64,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
        timeout_duration: u64,
        balance: u64,
    ): address {
        assert!(account::exists_at(owner), error::invalid_argument(EACCOUNT_DOES_NOT_EXIST));
        assert!(
            vector::all(&additional_owners, |ao_addr|{ account::exists_at(*ao_addr) }),
            error::invalid_argument(EACCOUNT_DOES_NOT_EXIST)
        );
        let addr = multisig_account::get_next_multisig_account_address(owner);
        let owner_signer = create_signer(owner);
        multisig_account::create_with_owners(
            &owner_signer,
            additional_owners,
            num_signatures_required,
            metadata_keys,
            metadata_values,
            timeout_duration
        );
        supra_coin::mint(supra_framework, addr, balance);
        account::increment_sequence_number(owner);
        addr
    }

    fun create_employee_validators(
        employee_vesting_start: u64,
        employee_vesting_period_duration: u64,
        employees: vector<EmployeeAccountMap>,
    ) {
        let unique_accounts = vector::empty();

        vector::for_each_ref(&employees, |employee_group| {
            let j = 0;
            let employee_group: &EmployeeAccountMap = employee_group;
            let num_employees_in_group = vector::length(&employee_group.accounts);

            let buy_ins = simple_map::create();

            while (j < num_employees_in_group) {
                let account = vector::borrow(&employee_group.accounts, j);
                assert!(
                    !vector::contains(&unique_accounts, account),
                    error::already_exists(EDUPLICATE_ACCOUNT),
                );
                vector::push_back(&mut unique_accounts, *account);

                let employee = create_signer(*account);
                let total = coin::balance<SupraCoin>(*account);
                let coins = coin::withdraw<SupraCoin>(&employee, total);
                simple_map::add(&mut buy_ins, *account, coins);

                j = j + 1;
            };

            let j = 0;
            let num_vesting_events = vector::length(&employee_group.vesting_schedule_numerator);
            let schedule = vector::empty();

            while (j < num_vesting_events) {
                let numerator = vector::borrow(&employee_group.vesting_schedule_numerator, j);
                let event = fixed_point32::create_from_rational(
                    *numerator,
                    employee_group.vesting_schedule_denominator
                );
                vector::push_back(&mut schedule, event);

                j = j + 1;
            };

            let vesting_schedule = vesting::create_vesting_schedule(
                schedule,
                employee_vesting_start,
                employee_vesting_period_duration,
            );

            let admin = employee_group.validator.validator_config.owner_address;
            let admin_signer = &create_signer(admin);
            let contract_address = vesting::create_vesting_contract(
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
            let pool_address = vesting::stake_pool_address(contract_address);

            if (employee_group.beneficiary_resetter != @0x0) {
                vesting::set_beneficiary_resetter(admin_signer, contract_address, employee_group.beneficiary_resetter);
            };

            let validator = &employee_group.validator.validator_config;
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
            if (employee_group.validator.join_during_genesis) {
                initialize_validator(pool_address, validator);
            };
        });
    }

    /// DEPRECATED
    /// 
    fun create_initialize_validators_with_commission(
        supra_framework: &signer,
        use_staking_contract: bool,
        validators: vector<ValidatorConfigurationWithCommission>,
    ) {
        vector::for_each_ref(&validators, |validator| {
            let validator: &ValidatorConfigurationWithCommission = validator;
            create_initialize_validator(supra_framework, validator, use_staking_contract);
        });
    }

    /// DEPRECATED
    /// 
    /// Sets up the initial validator set for the network.
    /// The validator "owner" accounts, and their authentication
    /// Addresses (and keys) are encoded in the `owners`
    /// Each validator signs consensus messages with the private key corresponding to the Ed25519
    /// public key in `consensus_pubkeys`.
    /// Finally, each validator must specify the network address
    /// (see types/src/network_address/mod.rs) for itself and its full nodes.
    ///
    /// Network address fields are a vector per account, where each entry is a vector of addresses
    /// encoded in a single BCS byte array.
    fun create_initialize_validators(supra_framework: &signer, validators: vector<ValidatorConfiguration>) {
        let validators_with_commission = vector::empty();
        vector::for_each_reverse(validators, |validator| {
            let validator_with_commission = ValidatorConfigurationWithCommission {
                validator_config: validator,
                commission_percentage: 0,
                join_during_genesis: true,
            };
            vector::push_back(&mut validators_with_commission, validator_with_commission);
        });

        create_initialize_validators_with_commission(supra_framework, false, validators_with_commission);
    }

    fun create_initialize_validator(
        supra_framework: &signer,
        commission_config: &ValidatorConfigurationWithCommission,
        use_staking_contract: bool,
    ) {
        let validator = &commission_config.validator_config;

        let owner = &create_account(supra_framework, validator.owner_address, validator.stake_amount);
        create_account(supra_framework, validator.operator_address, 0);
        create_account(supra_framework, validator.voter_address, 0);

        // Initialize the stake pool and join the validator set.
        let pool_address = if (use_staking_contract) {
            staking_contract::create_staking_contract(
                owner,
                validator.operator_address,
                validator.voter_address,
                validator.stake_amount,
                commission_config.commission_percentage,
                x"",
            );
            staking_contract::stake_pool_address(validator.owner_address, validator.operator_address)
        } else {
            stake::initialize_stake_owner(
                owner,
                validator.stake_amount,
                validator.operator_address,
                validator.voter_address,
            );
            validator.owner_address
        };

        if (commission_config.join_during_genesis) {
            initialize_validator(pool_address, validator);
        };
    }

    fun create_pbo_delegation_pools(
        pbo_delegator_configs: vector<PboDelegatorConfiguration>,
        delegation_percentage: u64,
    ) {
        let unique_accounts: vector<address> = vector::empty();
        assert!(
            delegation_percentage != 0 && delegation_percentage <= 100,
            error::invalid_argument(EPERCENTAGE_INVALID)
        );
        vector::for_each_ref(&pbo_delegator_configs, |pbo_delegator_config| {
            let pbo_delegator_config: &PboDelegatorConfiguration = pbo_delegator_config;
            assert!(!vector::contains(&unique_accounts, &pbo_delegator_config.delegator_config.owner_address),
                error::invalid_argument(EDUPLICATE_ACCOUNT));
            vector::push_back(&mut unique_accounts, pbo_delegator_config.delegator_config.owner_address);
            create_pbo_delegation_pool(pbo_delegator_config, delegation_percentage);
        });
    }

    fun create_pbo_delegation_pool(
        pbo_delegator_config: &PboDelegatorConfiguration,
        delegation_percentage: u64,
    ) {
        assert!(
            delegation_percentage != 0 && delegation_percentage <= 100,
            error::invalid_argument(EPERCENTAGE_INVALID)
        );
        let unique_accounts: vector<address> = vector::empty();
        vector::for_each_ref(&pbo_delegator_config.delegator_config.delegator_addresses, |delegator_address| {
            let delegator_address: &address = delegator_address;
            assert!(
                !vector::contains(&unique_accounts, delegator_address),
                error::already_exists(EDUPLICATE_ACCOUNT),
            );
            vector::push_back(&mut unique_accounts, *delegator_address);
        });
        let owner_signer = create_signer(pbo_delegator_config.delegator_config.owner_address);
        // get a list of delegator addresses, withdraw the coin from them and merge them into a single account
        let delegator_addresses = pbo_delegator_config.delegator_config.delegator_addresses;
        let coinInitialization = coin::zero<SupraCoin>();
        vector::for_each(delegator_addresses, |delegator_address| {
            let delegator = &create_signer(delegator_address);
            let total = coin::balance<SupraCoin>(delegator_address);
            let withdraw_amount = total * delegation_percentage / 100;
            let coins = coin::withdraw<SupraCoin>(delegator, withdraw_amount);
            coin::merge(&mut coinInitialization, coins);
        });
        pbo_delegation_pool::initialize_delegation_pool(
            &owner_signer,
            option::some(pbo_delegator_config.multisig_admin),
            pbo_delegator_config.delegator_config.validator.commission_percentage,
            pbo_delegator_config.delegator_config.delegation_pool_creation_seed,
            pbo_delegator_config.delegator_config.delegator_addresses,
            pbo_delegator_config.delegator_config.delegator_stakes,
            coinInitialization,
            pbo_delegator_config.unlock_schedule_numerators,
            pbo_delegator_config.unlock_schedule_denominator,
            pbo_delegator_config.unlock_startup_time_from_now + timestamp::now_seconds(),
            pbo_delegator_config.unlock_period_duration,
        );

        let pool_address = pbo_delegation_pool::get_owned_pool_address(
            pbo_delegator_config.delegator_config.owner_address
        );
        let validator = pbo_delegator_config.delegator_config.validator.validator_config;
        pbo_delegation_pool::set_operator(&owner_signer, validator.operator_address);
        pbo_delegation_pool::set_delegated_voter(&owner_signer, validator.voter_address);
        assert_validator_addresses_check(&validator);

        if (pbo_delegator_config.delegator_config.validator.join_during_genesis) {
            initialize_validator(pool_address, &validator);
        };
    }

    fun assert_validator_addresses_check(validator: &ValidatorConfiguration) {
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
    }


    fun create_vesting_without_staking_pools(
        vesting_pool_map: vector<VestingPoolsMap>
    ) {
        let unique_accounts: vector<address> = vector::empty();
        vector::for_each_ref(&vesting_pool_map, |pool_config|{
            let pool_config: &VestingPoolsMap = pool_config;
            let schedule = vector::empty();
            let schedule_length = vector::length(&pool_config.vesting_numerators);
            assert!(schedule_length != 0, error::invalid_argument(EVESTING_SCHEDULE_IS_ZERO));
            assert!(pool_config.vesting_denominator != 0, error::invalid_argument(EDENOMINATOR_IS_ZERO));
            assert!(pool_config.vpool_locking_percentage != 0 && pool_config.vpool_locking_percentage <= 100,
                error::invalid_argument(EPERCENTAGE_INVALID));
            //check the sum of numerator are <= denominator.
            let sum = vector::fold(pool_config.vesting_numerators, 0, |acc, x| acc + x);
            // Check that total of all fraction in `vesting_schedule` is not greater than 1
            assert!(sum <= pool_config.vesting_denominator,
                error::invalid_argument(ENUMERATOR_GREATER_THAN_DENOMINATOR));
            //assert that withdrawal_address is registered to receive SupraCoin
            assert!(
                coin::is_account_registered<SupraCoin>(pool_config.withdrawal_address),
                error::invalid_argument(EACCOUNT_NOT_REGISTERED_FOR_COIN)
            );
            //assertion on admin_address?
            let admin = create_signer(pool_config.admin_address);

            //Create the vesting schedule
            let j = 0;
            while (j < schedule_length) {
                let numerator = *vector::borrow(&pool_config.vesting_numerators, j);
                assert!(numerator != 0, error::invalid_argument(ENUMERATOR_IS_ZERO));
                let event = fixed_point32::create_from_rational(numerator, pool_config.vesting_denominator);
                vector::push_back(&mut schedule, event);
                j = j + 1;
            };

            let vesting_schedule = vesting_without_staking::create_vesting_schedule(
                schedule,
                timestamp::now_seconds() + pool_config.cliff_period_in_seconds,
                pool_config.period_duration_in_seconds,
            );

            let buy_ins = simple_map::create();
            let num_shareholders = vector::length(&pool_config.shareholders);
            assert!(num_shareholders != 0, error::invalid_argument(ENO_SHAREHOLDERS));
            let j = 0;
            while (j < num_shareholders) {
                let shareholder = *vector::borrow(&pool_config.shareholders, j);
                assert!(!vector::contains(&unique_accounts, &shareholder), error::already_exists(EDUPLICATE_ACCOUNT));
                vector::push_back(&mut unique_accounts, shareholder);
                let shareholder_signer = create_signer(shareholder);
                let amount = coin::balance<SupraCoin>(shareholder);
                let amount_to_extract = (amount * (pool_config.vpool_locking_percentage as u64)) / 100;
                let coin_share = coin::withdraw<SupraCoin>(&shareholder_signer, amount_to_extract);
                simple_map::add(&mut buy_ins, shareholder, coin_share);
                j = j + 1;
            };
            vesting_without_staking::create_vesting_contract(
                &admin,
                buy_ins,
                vesting_schedule,
                pool_config.withdrawal_address,
                VESTING_CONTRACT_SEED
            );
        });
    }

    fun initialize_validator(pool_address: address, validator: &ValidatorConfiguration) {
        let operator = &create_signer(validator.operator_address);

        stake::rotate_consensus_key_genesis(
            operator,
            pool_address,
            validator.consensus_pubkey,
        );
        stake::update_network_and_fullnode_addresses(
            operator,
            pool_address,
            validator.network_addresses,
            validator.full_node_network_addresses,
        );
        stake::join_validator_set_internal(operator, pool_address);
    }

    /// The last step of genesis.
    fun set_genesis_end(supra_framework: &signer) {
        // Destroy the mint capability owned by the framework account. The stake and transaction_fee
        // modules should be the only holders of this capability, which they will use to
        // mint block rewards and storage refunds, respectively.
        supra_coin::destroy_mint_cap(supra_framework);
        stake::on_new_epoch();
        chain_status::set_genesis_end(supra_framework);
    }

    #[verify_only]
    fun initialize_for_verification(
        gas_schedule: vector<u8>,
        chain_id: u8,
        initial_version: u64,
        consensus_config: vector<u8>,
        execution_config: vector<u8>,
        supra_config: vector<u8>,
        evm_config: vector<u8>,
        epoch_interval_microsecs: u64,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_duration_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64,
        supra_framework: &signer,
        // min_voting_threshold: u128,
        // required_proposer_stake: u64,
        voting_duration_secs: u64,
        supra_min_voting_threshold: u64,
        voters: vector<address>,
        accounts: vector<AccountMap>,
        employee_vesting_start: u64,
        employee_vesting_period_duration: u64,
        employees: vector<EmployeeAccountMap>,
        validators: vector<ValidatorConfigurationWithCommission>
    ) {
        initialize(
            gas_schedule,
            chain_id,
            initial_version,
            consensus_config,
            execution_config,
            supra_config,
            epoch_interval_microsecs,
            minimum_stake,
            maximum_stake,
            recurring_lockup_duration_secs,
            allow_validator_set_change,
            rewards_rate,
            rewards_rate_denominator,
            voting_power_increase_limit,
            0,
            evm_config,

        );
        features::change_feature_flags_for_verification(supra_framework, vector[1, 2, 11], vector[]);
        initialize_supra_coin(supra_framework);
        supra_governance::initialize_for_verification(
            supra_framework,
            // min_voting_threshold,
            // required_proposer_stake,
            voting_duration_secs,
            supra_min_voting_threshold,
            voters,
        );
        create_accounts(supra_framework, accounts);
        create_employee_validators(employee_vesting_start, employee_vesting_period_duration, employees);
        create_initialize_validators_with_commission(supra_framework, true, validators);
        set_genesis_end(supra_framework);
    }

    #[test_only]
    const ONE_SUPRA: u64 = 100000000;

    #[test_only]
    public fun setup() {
        initialize(
            x"000000000000000000", // empty gas schedule
            4u8, // TESTING chain ID
            0,
            x"12",
            x"13",
            x"14",
            1,
            0,
            1000 * ONE_SUPRA,
            1,
            true,
            1,
            1,
            30,
            0,
            x"15",
        )
    }

    #[test]
    fun test_setup() {
        setup();
        assert!(account::exists_at(@supra_framework), 1);
        assert!(account::exists_at(@0x2), 1);
        assert!(account::exists_at(@0x3), 1);
        assert!(account::exists_at(@0x4), 1);
        assert!(account::exists_at(@0x5), 1);
        assert!(account::exists_at(@0x6), 1);
        assert!(account::exists_at(@0x7), 1);
        assert!(account::exists_at(@0x8), 1);
        assert!(account::exists_at(@0x9), 1);
        assert!(account::exists_at(@0xa), 1);
    }

    #[test(supra_framework = @0x1)]
    fun test_create_account(supra_framework: &signer) {
        setup();
        initialize_supra_coin(supra_framework);

        let addr = @0x121341; // 01 -> 0a are taken
        let test_signer_before = create_account(supra_framework, addr, 15);
        let test_signer_after = create_account(supra_framework, addr, 500);
        assert!(test_signer_before == test_signer_after, 0);
        assert!(coin::balance<SupraCoin>(addr) == 15, 1);
    }

    #[test(supra_framework = @0x1)]
    fun test_create_accounts(supra_framework: &signer) {
        setup();
        initialize_supra_coin(supra_framework);

        // 01 -> 0a are taken
        let addr0 = @0x121341;
        let addr1 = @0x121345;

        let accounts = vector[
            AccountMap {
                account_address: addr0,
                balance: 12345,
            },
            AccountMap {
                account_address: addr1,
                balance: 67890,
            },
        ];

        create_accounts(supra_framework, accounts);
        assert!(coin::balance<SupraCoin>(addr0) == 12345, 0);
        assert!(coin::balance<SupraCoin>(addr1) == 67890, 1);

        create_account(supra_framework, addr0, 23456);
        assert!(coin::balance<SupraCoin>(addr0) == 12345, 2);
    }

    #[test_only]
    fun generate_multisig_account(owner: &signer, addition_owner: vector<address>): address {
        let owner_addr = aptos_std::signer::address_of(owner);
        let multisig_addr = multisig_account::get_next_multisig_account_address(owner_addr);
        multisig_account::create_with_owners(owner, addition_owner, 2, vector[], vector[], 300);
        multisig_addr
    }

    #[test(supra_framework = @0x1)]

    fun test_create_root_account(supra_framework: &signer) {
        use supra_framework::aggregator_factory;
        use supra_framework::object;
        use supra_framework::primary_fungible_store;
        use supra_framework::fungible_asset::Metadata;
        use std::features;

        let feature = features::get_new_accounts_default_to_fa_supra_store_feature();
        features::change_feature_flags_for_testing(supra_framework, vector[feature], vector[]);
        aggregator_factory::initialize_aggregator_factory_for_test(supra_framework);

        let (burn_cap, mint_cap) = supra_coin::initialize(supra_framework);
        supra_coin::ensure_initialized_with_sup_fa_metadata_for_test();

        let core_resources = account::create_account(@core_resources);
        supra_account::register_supra(&core_resources); // registers SUPRA store

        let sup_metadata = object::address_to_object<Metadata>(@supra_fungible_asset);
        assert!(primary_fungible_store::primary_store_exists(@core_resources, sup_metadata), 2);

        supra_coin::configure_accounts_for_test(supra_framework, &core_resources, mint_cap);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(supra_framework = @0x1)]
    fun test_create_pbo_delegation_pool(supra_framework: &signer) {
        use std::features;
        setup();

        features::change_feature_flags_for_testing(supra_framework, vector[11], vector[]);

        initialize_supra_coin(supra_framework);
        let owner = @0x121341;
        let (_, pk_1) = stake::generate_identity();
        let _pk_1 = ed25519::unvalidated_public_key_to_bytes(&pk_1);
        create_account(supra_framework, owner, 0);
        let validator_config_commission = ValidatorConfigurationWithCommission {
            validator_config: ValidatorConfiguration {
                owner_address: @0x121341,
                operator_address: @0x121342,
                voter_address: @0x121343,
                stake_amount: 0,
                consensus_pubkey: _pk_1,
                network_addresses: x"222222",
                full_node_network_addresses: x"333333",
            },
            commission_percentage: 10,
            join_during_genesis: true,
        };
        let delegation_pool_creation_seed = x"121341";
        let delegator_addresses = vector[@0x121342, @0x121343];
        let initial_balance = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let i = 0;
        let delegation_percentage = 10;
        let delegator_stakes: vector<u64> = vector::empty();
        while (i < vector::length(&delegator_addresses)) {
            create_account(
                supra_framework,
                *vector::borrow(&delegator_addresses, i),
                *vector::borrow(&initial_balance, i)
            );
            vector::push_back(
                &mut delegator_stakes,
                *vector::borrow(&initial_balance, i) * delegation_percentage / 100
            );
            i = i + 1;
        };
        let principle_lockup_time = 100;
        let multisig = generate_multisig_account(&account::create_signer_for_test(owner), vector[@0x12134]);
        let pbo_delegator_config = PboDelegatorConfiguration {
            multisig_admin: multisig,
            unlock_period_duration: 12,
            unlock_schedule_denominator: 10,
            unlock_schedule_numerators: vector[2, 2, 3],
            unlock_startup_time_from_now: principle_lockup_time,
            delegator_config: DelegatorConfiguration {
                owner_address: owner,
                validator: validator_config_commission,
                delegation_pool_creation_seed,
                delegator_addresses,
                delegator_stakes,
            },
        };
        create_pbo_delegation_pool(&pbo_delegator_config, delegation_percentage);
        let pool_address = pbo_delegation_pool::get_owned_pool_address(owner);
        assert!(pbo_delegation_pool::delegation_pool_exists(pool_address), 0);
    }

    #[test(supra_framework = @0x1)]
    fun test_create_pbo_delegation_pools(supra_framework: &signer) {
        use std::features;
        setup();
        features::change_feature_flags_for_testing(supra_framework, vector[11], vector[]);
        initialize_supra_coin(supra_framework);
        let owner1 = @0x121341;
        create_account(supra_framework, owner1, 0);
        let (_, pk_1) = stake::generate_identity();
        let (_, pk_2) = stake::generate_identity();
        let _pk_1 = ed25519::unvalidated_public_key_to_bytes(&pk_1);
        let _pk_2 = ed25519::unvalidated_public_key_to_bytes(&pk_2);
        let validator_config_commission1 = ValidatorConfigurationWithCommission {
            validator_config: ValidatorConfiguration {
                owner_address: owner1,
                operator_address: @0x121342,
                voter_address: @0x121343,
                stake_amount: 100 * ONE_SUPRA,
                consensus_pubkey: _pk_1,
                network_addresses: x"222222",
                full_node_network_addresses: x"333333",
            },
            commission_percentage: 10,
            join_during_genesis: true,
        };
        let delegation_pool_creation_seed1 = x"121341";
        let delegator_address1 = vector[@0x121342, @0x121343];
        let initial_balance1 = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let delegator_stakes1: vector<u64> = vector::empty();
        let delegation_percentage: u64 = 10;
        let i = 0;
        while (i < vector::length(&delegator_address1)) {
            create_account(
                supra_framework,
                *vector::borrow(&delegator_address1, i),
                *vector::borrow(&initial_balance1, i)
            );
            vector::push_back(
                &mut delegator_stakes1,
                *vector::borrow(&initial_balance1, i) * delegation_percentage / 100
            );
            i = i + 1;
        };
        let principle_lockup_time1 = 100;
        let multisig1 = generate_multisig_account(&account::create_signer_for_test(owner1), vector[@0x121342]);
        let pbo_delegator_config1 = PboDelegatorConfiguration {
            multisig_admin: multisig1,
            unlock_period_duration: 12,
            unlock_schedule_denominator: 10,
            unlock_schedule_numerators: vector[2, 2, 3],
            unlock_startup_time_from_now: principle_lockup_time1,
            delegator_config: DelegatorConfiguration {
                owner_address: owner1,
                validator: validator_config_commission1,
                delegation_pool_creation_seed: delegation_pool_creation_seed1,
                delegator_addresses: delegator_address1,
                delegator_stakes: delegator_stakes1,
            },
        };

        let owner2 = @0x121344;
        create_account(supra_framework, owner2, 0);
        let validator_config_commission2 = ValidatorConfigurationWithCommission {
            validator_config: ValidatorConfiguration {
                owner_address: owner2,
                operator_address: @0x121345,
                voter_address: @0x121346,
                stake_amount: 100 * ONE_SUPRA,
                consensus_pubkey: _pk_2,
                network_addresses: x"222222",
                full_node_network_addresses: x"333333",
            },
            commission_percentage: 20,
            join_during_genesis: true,
        };
        let delegation_pool_creation_seed2 = x"121344";
        let delegator_address2 = vector[@0x121345, @0x121346];
        let initial_balance2 = vector[300 * ONE_SUPRA, 400 * ONE_SUPRA];
        let j = 0;
        let delegator_stakes2: vector<u64> = vector::empty();
        while (j < vector::length(&delegator_address2)) {
            let bal = vector::borrow(&initial_balance2, j);
            create_account(supra_framework, *vector::borrow(&delegator_address2, j), *bal);
            vector::push_back(&mut delegator_stakes2, (*bal) * delegation_percentage / 100);
            j = j + 1;
        };
        let principle_lockup_time2 = 200;
        let multisig2 = generate_multisig_account(&account::create_signer_for_test(owner2), vector[@0x121347]);
        let pbo_delegator_config2 = PboDelegatorConfiguration {
            multisig_admin: multisig2,
            unlock_period_duration: 12,
            unlock_schedule_denominator: 10,
            unlock_schedule_numerators: vector[2, 2, 3],
            unlock_startup_time_from_now: principle_lockup_time2,
            delegator_config: DelegatorConfiguration {
                owner_address: owner2,
                validator: validator_config_commission2,
                delegation_pool_creation_seed: delegation_pool_creation_seed2,
                delegator_addresses: delegator_address2,
                delegator_stakes: delegator_stakes2,
            },
        };
        let pbo_delegator_configs = vector[pbo_delegator_config1, pbo_delegator_config2];
        create_pbo_delegation_pools(pbo_delegator_configs, delegation_percentage);
        let pool_address1 = pbo_delegation_pool::get_owned_pool_address(owner1);
        let pool_address2 = pbo_delegation_pool::get_owned_pool_address(owner2);
        assert!(pbo_delegation_pool::delegation_pool_exists(pool_address1), 0);
        assert!(pbo_delegation_pool::delegation_pool_exists(pool_address2), 1);
    }

    #[test (supra_framework= @0x1, owner1= @0x1234, owner2= @0x2345, owner3= @0x3456)]
    fun test_create_multisig_account_with_balance(
        supra_framework: &signer,
        owner1: address,
        owner2: address,
        owner3: address
    )
    {
        setup();
        initialize_supra_coin(supra_framework);
        let additional_owners = vector[owner2, owner3];
        let timeout_duration = 600;
        let num_signatures_required = 2;
        let metadata_keys: vector<String> = vector::empty();
        let metadata_values: vector<vector<u8>> = vector::empty();
        let balance = 10000000000;
        create_account(supra_framework, owner1, 0);
        create_account(supra_framework, owner2, 0);
        create_account(supra_framework, owner3, 0);
        let addr = create_multisig_account_with_balance(supra_framework, owner1, additional_owners,
            num_signatures_required, metadata_keys, metadata_values, timeout_duration, balance);
        //Ensure it is indeed on-chain multisig account with required threshold
        assert!(multisig_account::num_signatures_required(addr) == 2, 1);
        //Ensure the account is seeded with supplied balance
        assert!(coin::balance<SupraCoin>(addr) == balance, 2);
        // Ensure that you can transfer out funds from multisig account
        let multisig_signer = create_signer(addr);
        coin::transfer<SupraCoin>(&multisig_signer, owner1, balance);
        assert!(coin::balance<SupraCoin>(owner1) == balance, 3);
    }

    #[test (supra_framework= @0x1, owner1= @0x1234, owner2= @0x2345, owner3= @0x3456)]
    fun test_create_multisig_account_with_schema(
        supra_framework: &signer,
        owner1: address,
        owner2: address,
        owner3: address
    )
    {
        setup();
        initialize_supra_coin(supra_framework);
        let additional_owners = vector[owner2, owner3];
        let timeout_duration = 600;
        let num_signatures_required = 2;
        let metadata_keys: vector<String> = vector::empty();
        let metadata_values: vector<vector<u8>> = vector::empty();
        let balance = 10000000000;
        let num_accounts = 3;
        create_account(supra_framework, owner1, 0);
        create_account(supra_framework, owner2, 0);
        create_account(supra_framework, owner3, 0);
        let vec_addr = create_multiple_multisig_accounts_with_schema(supra_framework, owner1,
            additional_owners, num_signatures_required, metadata_keys, metadata_values,
            timeout_duration, balance, num_accounts);
        //Ensure they are indeed on-chain multisig account with required threshold
        assert!(vector::all(&vec_addr, |elem| { multisig_account::num_signatures_required(*elem) == 2 }), 1);
        //Ensure the accounts are seeded with supplied balance
        assert!(vector::all(&vec_addr, |elem| { coin::balance<SupraCoin>(*elem) == balance }), 2);
    }


    #[test(supra_framework = @0x1)]
    fun test_create_vesting_without_staking_pools(supra_framework: &signer) {
        // use supra_framework::supra_account::create_account;
        setup();
        initialize_supra_coin(supra_framework);
        timestamp::set_time_has_started_for_testing(supra_framework);
        stake::initialize_for_test(supra_framework);
        let admin_address = @0x121341;
        let vpool_locking_percentage = 10;
        let vesting_numerators = vector[1, 2, 3];
        let vesting_denominator = 6;
        let withdrawal_address = @0x121342;
        let shareholders = vector[@0x121343, @0x121344];
        create_account(supra_framework, admin_address, 0);
        create_account(supra_framework, withdrawal_address, 0);
        vector::for_each_ref(&shareholders, |addr| {
            let addr: address = *addr;
            if (!account::exists_at(addr)) {
                create_account(supra_framework, addr, 100 * ONE_SUPRA);
            };
        });
        let cliff_period_in_seconds = 100;
        let period_duration_in_seconds = 200;
        let pool_config = VestingPoolsMap {
            admin_address: admin_address,
            vpool_locking_percentage: vpool_locking_percentage,
            vesting_numerators: vesting_numerators,
            vesting_denominator: vesting_denominator,
            withdrawal_address: withdrawal_address,
            shareholders: shareholders,
            cliff_period_in_seconds: cliff_period_in_seconds,
            period_duration_in_seconds: period_duration_in_seconds,
        };
        let vesting_pool_map = vector[pool_config];
        create_vesting_without_staking_pools(vesting_pool_map);
        let vesting_contracts = vesting_without_staking::vesting_contracts(admin_address);
        assert!(vector::length(&vesting_contracts) == 1, 0);
    }
}
