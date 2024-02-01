module 0x1::genesis {
    struct AccountMap has drop {
        account_address: address,
        balance: u64,
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
        proof_of_possession: vector<u8>,
        network_addresses: vector<u8>,
        full_node_network_addresses: vector<u8>,
    }
    
    struct ValidatorConfigurationWithCommission has copy, drop {
        validator_config: ValidatorConfiguration,
        commission_percentage: u64,
        join_during_genesis: bool,
    }
    
    fun create_account(arg0: &signer, arg1: address, arg2: u64) : signer {
        if (0x1::account::exists_at(arg1)) {
            0x1::create_signer::create_signer(arg1)
        } else {
            let v1 = 0x1::account::create_account(arg1);
            0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v1);
            0x1::aptos_coin::mint(arg0, arg1, arg2);
            v1
        }
    }
    
    fun initialize(arg0: vector<u8>, arg1: u8, arg2: u64, arg3: vector<u8>, arg4: vector<u8>, arg5: u64, arg6: u64, arg7: u64, arg8: u64, arg9: bool, arg10: u64, arg11: u64, arg12: u64) {
        let (v0, v1) = 0x1::account::create_framework_reserved_account(@0x1);
        let v2 = v0;
        0x1::account::initialize(&v2);
        let v3 = &v2;
        0x1::transaction_validation::initialize(v3, b"script_prologue", b"module_prologue", b"multi_agent_script_prologue", b"epilogue");
        0x1::aptos_governance::store_signer_cap(&v2, @0x1, v1);
        let v4 = vector[@0x2, @0x3, @0x4, @0x5, @0x6, @0x7, @0x8, @0x9, @0xa];
        while (!0x1::vector::is_empty<address>(&v4)) {
            let v5 = 0x1::vector::pop_back<address>(&mut v4);
            let (_, v7) = 0x1::account::create_framework_reserved_account(v5);
            0x1::aptos_governance::store_signer_cap(&v2, v5, v7);
        };
        0x1::consensus_config::initialize(&v2, arg3);
        0x1::execution_config::set(&v2, arg4);
        0x1::version::initialize(&v2, arg2);
        0x1::stake::initialize(&v2);
        0x1::staking_config::initialize(&v2, arg6, arg7, arg8, arg9, arg10, arg11, arg12);
        0x1::storage_gas::initialize(&v2);
        0x1::gas_schedule::initialize(&v2, arg0);
        0x1::aggregator_factory::initialize_aggregator_factory(&v2);
        0x1::coin::initialize_supply_config(&v2);
        0x1::chain_id::initialize(&v2, arg1);
        0x1::reconfiguration::initialize(&v2);
        0x1::block::initialize(&v2, arg5);
        0x1::state_storage::initialize(&v2);
        0x1::timestamp::set_time_has_started(&v2);
        0x1::jwks::initialize(&v2);
    }
    
    fun set_genesis_end(arg0: &signer) {
        0x1::chain_status::set_genesis_end(arg0);
    }
    
    fun create_accounts(arg0: &signer, arg1: vector<AccountMap>) {
        let v0 = 0x1::vector::empty<address>();
        let v1 = &arg1;
        let v2 = 0;
        while (v2 < 0x1::vector::length<AccountMap>(v1)) {
            let v3 = 0x1::vector::borrow<AccountMap>(v1, v2);
            assert!(!0x1::vector::contains<address>(&v0, &v3.account_address), 0x1::error::already_exists(1));
            0x1::vector::push_back<address>(&mut v0, v3.account_address);
            create_account(arg0, v3.account_address, v3.balance);
            v2 = v2 + 1;
        };
    }
    
    fun create_employee_validators(arg0: u64, arg1: u64, arg2: vector<EmployeeAccountMap>) {
        let v0 = 0x1::vector::empty<address>();
        let v1 = &arg2;
        let v2 = 0;
        while (v2 < 0x1::vector::length<EmployeeAccountMap>(v1)) {
            let v3 = 0x1::vector::borrow<EmployeeAccountMap>(v1, v2);
            let v4 = 0;
            let v5 = 0x1::simple_map::create<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>();
            while (v4 < 0x1::vector::length<address>(&v3.accounts)) {
                let v6 = 0x1::vector::borrow<address>(&v3.accounts, v4);
                assert!(!0x1::vector::contains<address>(&v0, v6), 0x1::error::already_exists(1));
                0x1::vector::push_back<address>(&mut v0, *v6);
                let v7 = 0x1::create_signer::create_signer(*v6);
                let v8 = 0x1::coin::balance<0x1::aptos_coin::AptosCoin>(*v6);
                let v9 = 0x1::coin::withdraw<0x1::aptos_coin::AptosCoin>(&v7, v8);
                0x1::simple_map::add<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(&mut v5, *v6, v9);
                v4 = v4 + 1;
            };
            let v10 = 0;
            let v11 = 0x1::vector::empty<0x1::fixed_point32::FixedPoint32>();
            while (v10 < 0x1::vector::length<u64>(&v3.vesting_schedule_numerator)) {
                let v12 = &v3.vesting_schedule_numerator;
                let v13 = v3.vesting_schedule_denominator;
                let v14 = 0x1::fixed_point32::create_from_rational(*0x1::vector::borrow<u64>(v12, v10), v13);
                0x1::vector::push_back<0x1::fixed_point32::FixedPoint32>(&mut v11, v14);
                v10 = v10 + 1;
            };
            let v15 = 0x1::vesting::create_vesting_schedule(v11, arg0, arg1);
            let v16 = v3.validator.validator_config.owner_address;
            let v17 = 0x1::create_signer::create_signer(v16);
            let v18 = &v17;
            let v19 = v3.validator.validator_config.operator_address;
            let v20 = v3.validator.validator_config.voter_address;
            let v21 = v3.validator.commission_percentage;
            let v22 = 0x1::vesting::create_vesting_contract(v18, &v3.accounts, v5, v15, v16, v19, v20, v21, b"");
            if (v3.beneficiary_resetter != @0x0) {
                0x1::vesting::set_beneficiary_resetter(v18, v22, v3.beneficiary_resetter);
            };
            let v23 = &v3.validator.validator_config;
            assert!(0x1::account::exists_at(v23.owner_address), 0x1::error::not_found(2));
            assert!(0x1::account::exists_at(v23.operator_address), 0x1::error::not_found(2));
            assert!(0x1::account::exists_at(v23.voter_address), 0x1::error::not_found(2));
            if (v3.validator.join_during_genesis) {
                initialize_validator(0x1::vesting::stake_pool_address(v22), v23);
            };
            v2 = v2 + 1;
        };
    }
    
    fun create_initialize_validator(arg0: &signer, arg1: &ValidatorConfigurationWithCommission, arg2: bool) {
        let v0 = &arg1.validator_config;
        let v1 = create_account(arg0, v0.owner_address, v0.stake_amount);
        create_account(arg0, v0.operator_address, 0);
        create_account(arg0, v0.voter_address, 0);
        let v2 = if (arg2) {
            let v3 = v0.operator_address;
            let v4 = v0.voter_address;
            let v5 = arg1.commission_percentage;
            0x1::staking_contract::create_staking_contract(&v1, v3, v4, v0.stake_amount, v5, b"");
            0x1::staking_contract::stake_pool_address(v0.owner_address, v0.operator_address)
        } else {
            0x1::stake::initialize_stake_owner(&v1, v0.stake_amount, v0.operator_address, v0.voter_address);
            v0.owner_address
        };
        if (arg1.join_during_genesis) {
            initialize_validator(v2, v0);
        };
    }
    
    fun create_initialize_validators(arg0: &signer, arg1: vector<ValidatorConfiguration>) {
        let v0 = 0x1::vector::empty<ValidatorConfigurationWithCommission>();
        let v1 = arg1;
        let v2 = 0x1::vector::length<ValidatorConfiguration>(&v1);
        while (v2 > 0) {
            let v3 = 0x1::vector::pop_back<ValidatorConfiguration>(&mut v1);
            let v4 = ValidatorConfigurationWithCommission{
                validator_config      : v3, 
                commission_percentage : 0, 
                join_during_genesis   : true,
            };
            0x1::vector::push_back<ValidatorConfigurationWithCommission>(&mut v0, v4);
            v2 = v2 - 1;
        };
        0x1::vector::destroy_empty<ValidatorConfiguration>(v1);
        create_initialize_validators_with_commission(arg0, false, v0);
    }
    
    fun create_initialize_validators_with_commission(arg0: &signer, arg1: bool, arg2: vector<ValidatorConfigurationWithCommission>) {
        let v0 = &arg2;
        let v1 = 0;
        while (v1 < 0x1::vector::length<ValidatorConfigurationWithCommission>(v0)) {
            let v2 = 0x1::vector::borrow<ValidatorConfigurationWithCommission>(v0, v1);
            create_initialize_validator(arg0, v2, arg1);
            v1 = v1 + 1;
        };
        0x1::aptos_coin::destroy_mint_cap(arg0);
        0x1::stake::on_new_epoch();
    }
    
    fun initialize_aptos_coin(arg0: &signer) {
        let (v0, v1) = 0x1::aptos_coin::initialize(arg0);
        0x1::stake::store_aptos_coin_mint_cap(arg0, v1);
        0x1::transaction_fee::store_aptos_coin_burn_cap(arg0, v0);
        0x1::transaction_fee::store_aptos_coin_mint_cap(arg0, v1);
    }
    
    fun initialize_core_resources_and_aptos_coin(arg0: &signer, arg1: vector<u8>) {
        let (v0, v1) = 0x1::aptos_coin::initialize(arg0);
        0x1::stake::store_aptos_coin_mint_cap(arg0, v1);
        0x1::transaction_fee::store_aptos_coin_burn_cap(arg0, v0);
        0x1::transaction_fee::store_aptos_coin_mint_cap(arg0, v1);
        let v2 = 0x1::account::create_account(@0x3000);
        0x1::account::rotate_authentication_key_internal(&v2, arg1);
        0x1::aptos_coin::configure_accounts_for_test(arg0, &v2, v1);
    }
    
    fun initialize_validator(arg0: address, arg1: &ValidatorConfiguration) {
        let v0 = 0x1::create_signer::create_signer(arg1.operator_address);
        let v1 = &v0;
        0x1::stake::rotate_consensus_key(v1, arg0, arg1.consensus_pubkey, arg1.proof_of_possession);
        let v2 = arg1.full_node_network_addresses;
        0x1::stake::update_network_and_fullnode_addresses(v1, arg0, arg1.network_addresses, v2);
        0x1::stake::join_validator_set_internal(v1, arg0);
    }
    
    // decompiled from Move bytecode v6
}
