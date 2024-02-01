module 0x1::staking_contract {
    struct AddDistributionEvent has drop, store {
        operator: address,
        pool_address: address,
        amount: u64,
    }
    
    struct AddStakeEvent has drop, store {
        operator: address,
        pool_address: address,
        amount: u64,
    }
    
    struct BeneficiaryForOperator has key {
        beneficiary_for_operator: address,
    }
    
    struct CreateStakingContractEvent has drop, store {
        operator: address,
        voter: address,
        pool_address: address,
        principal: u64,
        commission_percentage: u64,
    }
    
    struct DistributeEvent has drop, store {
        operator: address,
        pool_address: address,
        recipient: address,
        amount: u64,
    }
    
    struct RequestCommissionEvent has drop, store {
        operator: address,
        pool_address: address,
        accumulated_rewards: u64,
        commission_amount: u64,
    }
    
    struct ResetLockupEvent has drop, store {
        operator: address,
        pool_address: address,
    }
    
    struct SetBeneficiaryForOperator has drop, store {
        operator: address,
        old_beneficiary: address,
        new_beneficiary: address,
    }
    
    struct StakingContract has store {
        principal: u64,
        pool_address: address,
        owner_cap: 0x1::stake::OwnerCapability,
        commission_percentage: u64,
        distribution_pool: 0x1::pool_u64::Pool,
        signer_cap: 0x1::account::SignerCapability,
    }
    
    struct StakingGroupContainer {
        dummy_field: bool,
    }
    
    struct StakingGroupUpdateCommissionEvent has key {
        update_commission_events: 0x1::event::EventHandle<UpdateCommissionEvent>,
    }
    
    struct Store has key {
        staking_contracts: 0x1::simple_map::SimpleMap<address, StakingContract>,
        create_staking_contract_events: 0x1::event::EventHandle<CreateStakingContractEvent>,
        update_voter_events: 0x1::event::EventHandle<UpdateVoterEvent>,
        reset_lockup_events: 0x1::event::EventHandle<ResetLockupEvent>,
        add_stake_events: 0x1::event::EventHandle<AddStakeEvent>,
        request_commission_events: 0x1::event::EventHandle<RequestCommissionEvent>,
        unlock_stake_events: 0x1::event::EventHandle<UnlockStakeEvent>,
        switch_operator_events: 0x1::event::EventHandle<SwitchOperatorEvent>,
        add_distribution_events: 0x1::event::EventHandle<AddDistributionEvent>,
        distribute_events: 0x1::event::EventHandle<DistributeEvent>,
    }
    
    struct SwitchOperatorEvent has drop, store {
        old_operator: address,
        new_operator: address,
        pool_address: address,
    }
    
    struct UnlockStakeEvent has drop, store {
        operator: address,
        pool_address: address,
        amount: u64,
        commission_paid: u64,
    }
    
    struct UpdateCommissionEvent has drop, store {
        staker: address,
        operator: address,
        old_commission_percentage: u64,
        new_commission_percentage: u64,
    }
    
    struct UpdateVoterEvent has drop, store {
        operator: address,
        pool_address: address,
        old_voter: address,
        new_voter: address,
    }
    
    fun add_distribution(arg0: address, arg1: &mut StakingContract, arg2: address, arg3: u64, arg4: &mut 0x1::event::EventHandle<AddDistributionEvent>) {
        let v0 = &mut arg1.distribution_pool;
        let (_, _, _, v4) = 0x1::stake::get_stake(arg1.pool_address);
        update_distribution_pool(v0, v4, arg0, arg1.commission_percentage);
        0x1::pool_u64::buy_in(v0, arg2, arg3);
        let v5 = AddDistributionEvent{
            operator     : arg0, 
            pool_address : arg1.pool_address, 
            amount       : arg3,
        };
        0x1::event::emit_event<AddDistributionEvent>(arg4, v5);
    }
    
    public entry fun add_stake(arg0: &signer, arg1: address, arg2: u64) acquires Store {
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let v1 = borrow_global_mut<Store>(v0);
        let v2 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v1.staking_contracts, &arg1);
        let v3 = 0x1::coin::withdraw<0x1::aptos_coin::AptosCoin>(arg0, arg2);
        0x1::stake::add_stake_with_cap(&v2.owner_cap, v3);
        v2.principal = v2.principal + arg2;
        let v4 = AddStakeEvent{
            operator     : arg1, 
            pool_address : v2.pool_address, 
            amount       : arg2,
        };
        0x1::event::emit_event<AddStakeEvent>(&mut v1.add_stake_events, v4);
    }
    
    fun assert_staking_contract_exists(arg0: address, arg1: address) acquires Store {
        assert!(exists<Store>(arg0), 0x1::error::not_found(3));
        let v0 = borrow_global_mut<Store>(arg0);
        let v1 = 0x1::simple_map::contains_key<address, StakingContract>(&mut v0.staking_contracts, &arg1);
        assert!(v1, 0x1::error::not_found(4));
    }
    
    public fun beneficiary_for_operator(arg0: address) : address acquires BeneficiaryForOperator {
        if (exists<BeneficiaryForOperator>(arg0)) {
            return borrow_global<BeneficiaryForOperator>(arg0).beneficiary_for_operator
        };
        arg0
    }
    
    public fun commission_percentage(arg0: address, arg1: address) : u64 acquires Store {
        assert_staking_contract_exists(arg0, arg1);
        let v0 = &borrow_global<Store>(arg0).staking_contracts;
        0x1::simple_map::borrow<address, StakingContract>(v0, &arg1).commission_percentage
    }
    
    fun create_resource_account_seed(arg0: address, arg1: address, arg2: vector<u8>) : vector<u8> {
        let v0 = 0x1::bcs::to_bytes<address>(&arg0);
        0x1::vector::append<u8>(&mut v0, 0x1::bcs::to_bytes<address>(&arg1));
        0x1::vector::append<u8>(&mut v0, b"aptos_framework::staking_contract");
        0x1::vector::append<u8>(&mut v0, arg2);
        v0
    }
    
    fun create_stake_pool(arg0: &signer, arg1: address, arg2: address, arg3: vector<u8>) : (signer, 0x1::account::SignerCapability, 0x1::stake::OwnerCapability) {
        let v0 = create_resource_account_seed(0x1::signer::address_of(arg0), arg1, arg3);
        let (v1, v2) = 0x1::account::create_resource_account(arg0, v0);
        let v3 = v1;
        0x1::stake::initialize_stake_owner(&v3, 0, arg1, arg2);
        (v3, v2, 0x1::stake::extract_owner_cap(&v3))
    }
    
    public entry fun create_staking_contract(arg0: &signer, arg1: address, arg2: address, arg3: u64, arg4: u64, arg5: vector<u8>) acquires Store {
        let v0 = 0x1::coin::withdraw<0x1::aptos_coin::AptosCoin>(arg0, arg3);
        create_staking_contract_with_coins(arg0, arg1, arg2, v0, arg4, arg5);
    }
    
    public fun create_staking_contract_with_coins(arg0: &signer, arg1: address, arg2: address, arg3: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>, arg4: u64, arg5: vector<u8>) : address acquires Store {
        assert!(arg4 >= 0 && arg4 <= 100, 0x1::error::invalid_argument(2));
        let v0 = 0x1::staking_config::get();
        let (v1, _) = 0x1::staking_config::get_required_stake(&v0);
        let v3 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&arg3);
        assert!(v3 >= v1, 0x1::error::invalid_argument(1));
        let v4 = 0x1::signer::address_of(arg0);
        if (!exists<Store>(v4)) {
            move_to<Store>(arg0, new_staking_contracts_holder(arg0));
        };
        let v5 = borrow_global_mut<Store>(v4);
        let v6 = &mut v5.staking_contracts;
        assert!(!0x1::simple_map::contains_key<address, StakingContract>(v6, &arg1), 0x1::error::already_exists(6));
        let (v7, v8, v9) = create_stake_pool(arg0, arg1, arg2, arg5);
        let v10 = v9;
        let v11 = v7;
        0x1::stake::add_stake_with_cap(&v10, arg3);
        let v12 = 0x1::signer::address_of(&v11);
        let v13 = 0x1::pool_u64::create(20);
        let v14 = StakingContract{
            principal             : v3, 
            pool_address          : v12, 
            owner_cap             : v10, 
            commission_percentage : arg4, 
            distribution_pool     : v13, 
            signer_cap            : v8,
        };
        0x1::simple_map::add<address, StakingContract>(v6, arg1, v14);
        let v15 = CreateStakingContractEvent{
            operator              : arg1, 
            voter                 : arg2, 
            pool_address          : v12, 
            principal             : v3, 
            commission_percentage : arg4,
        };
        0x1::event::emit_event<CreateStakingContractEvent>(&mut v5.create_staking_contract_events, v15);
        v12
    }
    
    public entry fun distribute(arg0: address, arg1: address) acquires BeneficiaryForOperator, Store {
        assert_staking_contract_exists(arg0, arg1);
        let v0 = borrow_global_mut<Store>(arg0);
        let v1 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v0.staking_contracts, &arg1);
        distribute_internal(arg0, arg1, v1, &mut v0.distribute_events);
    }
    
    fun distribute_internal(arg0: address, arg1: address, arg2: &mut StakingContract, arg3: &mut 0x1::event::EventHandle<DistributeEvent>) acquires BeneficiaryForOperator {
        let v0 = arg2.pool_address;
        let (_, v2, _, v4) = 0x1::stake::get_stake(v0);
        let v5 = 0x1::stake::withdraw_with_cap(&arg2.owner_cap, v2 + v4);
        let v6 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v5);
        if (v6 == 0) {
            0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v5);
            return
        };
        let v7 = &mut arg2.distribution_pool;
        update_distribution_pool(v7, v6, arg1, arg2.commission_percentage);
        while (0x1::pool_u64::shareholders_count(v7) > 0) {
            let v8 = 0x1::pool_u64::shareholders(v7);
            let v9 = *0x1::vector::borrow<address>(&mut v8, 0);
            let v10 = v9;
            let v11 = 0x1::pool_u64::redeem_shares(v7, v9, 0x1::pool_u64::shares(v7, v9));
            if (v9 == arg1) {
                v10 = beneficiary_for_operator(arg1);
            };
            let v12 = 0x1::coin::extract<0x1::aptos_coin::AptosCoin>(&mut v5, v11);
            0x1::aptos_account::deposit_coins<0x1::aptos_coin::AptosCoin>(v10, v12);
            let v13 = DistributeEvent{
                operator     : arg1, 
                pool_address : v0, 
                recipient    : v10, 
                amount       : v11,
            };
            0x1::event::emit_event<DistributeEvent>(arg3, v13);
        };
        if (0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v5) > 0) {
            0x1::aptos_account::deposit_coins<0x1::aptos_coin::AptosCoin>(arg0, v5);
            0x1::pool_u64::update_total_coins(v7, 0);
        } else {
            0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v5);
        };
    }
    
    public fun get_expected_stake_pool_address(arg0: address, arg1: address, arg2: vector<u8>) : address {
        0x1::account::create_resource_address(&arg0, create_resource_account_seed(arg0, arg1, arg2))
    }
    
    fun get_staking_contract_amounts_internal(arg0: &StakingContract) : (u64, u64, u64) {
        let (v0, _, v2, _) = 0x1::stake::get_stake(arg0.pool_address);
        let v4 = v0 + v2;
        let v5 = v4 - arg0.principal;
        (v4, v5, v5 * arg0.commission_percentage / 100)
    }
    
    public fun last_recorded_principal(arg0: address, arg1: address) : u64 acquires Store {
        assert_staking_contract_exists(arg0, arg1);
        let v0 = &borrow_global<Store>(arg0).staking_contracts;
        0x1::simple_map::borrow<address, StakingContract>(v0, &arg1).principal
    }
    
    fun new_staking_contracts_holder(arg0: &signer) : Store {
        let v0 = 0x1::simple_map::create<address, StakingContract>();
        let v1 = 0x1::account::new_event_handle<CreateStakingContractEvent>(arg0);
        let v2 = 0x1::account::new_event_handle<UpdateVoterEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<ResetLockupEvent>(arg0);
        let v4 = 0x1::account::new_event_handle<AddStakeEvent>(arg0);
        let v5 = 0x1::account::new_event_handle<RequestCommissionEvent>(arg0);
        let v6 = 0x1::account::new_event_handle<UnlockStakeEvent>(arg0);
        let v7 = 0x1::account::new_event_handle<SwitchOperatorEvent>(arg0);
        let v8 = 0x1::account::new_event_handle<AddDistributionEvent>(arg0);
        let v9 = 0x1::account::new_event_handle<DistributeEvent>(arg0);
        Store{
            staking_contracts              : v0, 
            create_staking_contract_events : v1, 
            update_voter_events            : v2, 
            reset_lockup_events            : v3, 
            add_stake_events               : v4, 
            request_commission_events      : v5, 
            unlock_stake_events            : v6, 
            switch_operator_events         : v7, 
            add_distribution_events        : v8, 
            distribute_events              : v9,
        }
    }
    
    public fun pending_distribution_counts(arg0: address, arg1: address) : u64 acquires Store {
        assert_staking_contract_exists(arg0, arg1);
        let v0 = borrow_global<Store>(arg0);
        let v1 = &0x1::simple_map::borrow<address, StakingContract>(&v0.staking_contracts, &arg1).distribution_pool;
        0x1::pool_u64::shareholders_count(v1)
    }
    
    public entry fun request_commission(arg0: &signer, arg1: address, arg2: address) acquires BeneficiaryForOperator, Store {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = if (v0 == arg1 || v0 == arg2) {
            true
        } else {
            let v2 = beneficiary_for_operator(arg2);
            v0 == v2
        };
        assert!(v1, 0x1::error::unauthenticated(8));
        assert_staking_contract_exists(arg1, arg2);
        let v3 = borrow_global_mut<Store>(arg1);
        let v4 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v3.staking_contracts, &arg2);
        if (v4.commission_percentage == 0) {
            return
        };
        distribute_internal(arg1, arg2, v4, &mut v3.distribute_events);
        let v5 = &mut v3.request_commission_events;
        request_commission_internal(arg2, v4, &mut v3.add_distribution_events, v5);
    }
    
    fun request_commission_internal(arg0: address, arg1: &mut StakingContract, arg2: &mut 0x1::event::EventHandle<AddDistributionEvent>, arg3: &mut 0x1::event::EventHandle<RequestCommissionEvent>) : u64 {
        let (v0, v1, v2) = get_staking_contract_amounts_internal(arg1);
        arg1.principal = v0 - v2;
        if (v2 == 0) {
            return 0
        };
        add_distribution(arg0, arg1, arg0, v2, arg2);
        0x1::stake::unlock_with_cap(v2, &arg1.owner_cap);
        let v3 = arg1.pool_address;
        let v4 = RequestCommissionEvent{
            operator            : arg0, 
            pool_address        : v3, 
            accumulated_rewards : v1, 
            commission_amount   : v2,
        };
        0x1::event::emit_event<RequestCommissionEvent>(arg3, v4);
        v2
    }
    
    public entry fun reset_lockup(arg0: &signer, arg1: address) acquires Store {
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let v1 = borrow_global_mut<Store>(v0);
        let v2 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v1.staking_contracts, &arg1);
        0x1::stake::increase_lockup_with_cap(&v2.owner_cap);
        let v3 = ResetLockupEvent{
            operator     : arg1, 
            pool_address : v2.pool_address,
        };
        0x1::event::emit_event<ResetLockupEvent>(&mut v1.reset_lockup_events, v3);
    }
    
    public entry fun set_beneficiary_for_operator(arg0: &signer, arg1: address) acquires BeneficiaryForOperator {
        assert!(0x1::features::operator_beneficiary_change_enabled(), 0x1::error::invalid_state(9));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = beneficiary_for_operator(v0);
        if (exists<BeneficiaryForOperator>(v0)) {
            borrow_global_mut<BeneficiaryForOperator>(v0).beneficiary_for_operator = arg1;
        } else {
            let v2 = BeneficiaryForOperator{beneficiary_for_operator: arg1};
            move_to<BeneficiaryForOperator>(arg0, v2);
        };
        let v3 = SetBeneficiaryForOperator{
            operator        : v0, 
            old_beneficiary : v1, 
            new_beneficiary : arg1,
        };
        0x1::event::emit<SetBeneficiaryForOperator>(v3);
    }
    
    public fun stake_pool_address(arg0: address, arg1: address) : address acquires Store {
        assert_staking_contract_exists(arg0, arg1);
        let v0 = &borrow_global<Store>(arg0).staking_contracts;
        0x1::simple_map::borrow<address, StakingContract>(v0, &arg1).pool_address
    }
    
    public fun staking_contract_amounts(arg0: address, arg1: address) : (u64, u64, u64) acquires Store {
        assert_staking_contract_exists(arg0, arg1);
        let v0 = &borrow_global<Store>(arg0).staking_contracts;
        get_staking_contract_amounts_internal(0x1::simple_map::borrow<address, StakingContract>(v0, &arg1))
    }
    
    public fun staking_contract_exists(arg0: address, arg1: address) : bool acquires Store {
        if (!exists<Store>(arg0)) {
            return false
        };
        let v0 = &borrow_global<Store>(arg0).staking_contracts;
        0x1::simple_map::contains_key<address, StakingContract>(v0, &arg1)
    }
    
    public entry fun switch_operator(arg0: &signer, arg1: address, arg2: address, arg3: u64) acquires BeneficiaryForOperator, Store {
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let v1 = borrow_global_mut<Store>(v0);
        let v2 = &mut v1.staking_contracts;
        let v3 = !0x1::simple_map::contains_key<address, StakingContract>(v2, &arg2);
        assert!(v3, 0x1::error::invalid_state(5));
        let (_, v5) = 0x1::simple_map::remove<address, StakingContract>(v2, &arg1);
        let v6 = v5;
        distribute_internal(v0, arg1, &mut v6, &mut v1.distribute_events);
        let v7 = &mut v1.request_commission_events;
        request_commission_internal(arg1, &mut v6, &mut v1.add_distribution_events, v7);
        0x1::stake::set_operator_with_cap(&v6.owner_cap, arg2);
        v6.commission_percentage = arg3;
        0x1::simple_map::add<address, StakingContract>(v2, arg2, v6);
        let v8 = SwitchOperatorEvent{
            old_operator : arg1, 
            new_operator : arg2, 
            pool_address : v6.pool_address,
        };
        0x1::event::emit_event<SwitchOperatorEvent>(&mut v1.switch_operator_events, v8);
    }
    
    public entry fun switch_operator_with_same_commission(arg0: &signer, arg1: address, arg2: address) acquires BeneficiaryForOperator, Store {
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let v1 = commission_percentage(v0, arg1);
        switch_operator(arg0, arg1, arg2, v1);
    }
    
    public entry fun unlock_rewards(arg0: &signer, arg1: address) acquires BeneficiaryForOperator, Store {
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let (_, v2, v3) = staking_contract_amounts(v0, arg1);
        unlock_stake(arg0, arg1, v2 - v3);
    }
    
    public entry fun unlock_stake(arg0: &signer, arg1: address, arg2: u64) acquires BeneficiaryForOperator, Store {
        if (arg2 == 0) {
            return
        };
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let v1 = borrow_global_mut<Store>(v0);
        let v2 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v1.staking_contracts, &arg1);
        distribute_internal(v0, arg1, v2, &mut v1.distribute_events);
        let v3 = &mut v1.add_distribution_events;
        let v4 = request_commission_internal(arg1, v2, v3, &mut v1.request_commission_events);
        let (v5, _, _, _) = 0x1::stake::get_stake(v2.pool_address);
        if (v5 < arg2) {
            arg2 = v5;
        };
        v2.principal = v2.principal - arg2;
        add_distribution(arg1, v2, v0, arg2, &mut v1.add_distribution_events);
        0x1::stake::unlock_with_cap(arg2, &v2.owner_cap);
        let v9 = UnlockStakeEvent{
            operator        : arg1, 
            pool_address    : v2.pool_address, 
            amount          : arg2, 
            commission_paid : v4,
        };
        0x1::event::emit_event<UnlockStakeEvent>(&mut v1.unlock_stake_events, v9);
    }
    
    public entry fun update_commision(arg0: &signer, arg1: address, arg2: u64) acquires BeneficiaryForOperator, StakingGroupUpdateCommissionEvent, Store {
        assert!(arg2 >= 0 && arg2 <= 100, 0x1::error::invalid_argument(2));
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists<Store>(v0), 0x1::error::not_found(3));
        let v1 = borrow_global_mut<Store>(v0);
        let v2 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v1.staking_contracts, &arg1);
        distribute_internal(v0, arg1, v2, &mut v1.distribute_events);
        request_commission_internal(arg1, v2, &mut v1.add_distribution_events, &mut v1.request_commission_events);
        let v3 = v2.commission_percentage;
        v2.commission_percentage = arg2;
        if (!exists<StakingGroupUpdateCommissionEvent>(v0)) {
            let v4 = 0x1::account::new_event_handle<UpdateCommissionEvent>(arg0);
            let v5 = StakingGroupUpdateCommissionEvent{update_commission_events: v4};
            move_to<StakingGroupUpdateCommissionEvent>(arg0, v5);
        };
        let v6 = &mut borrow_global_mut<StakingGroupUpdateCommissionEvent>(v0).update_commission_events;
        let v7 = arg1;
        let v8 = UpdateCommissionEvent{
            staker                    : v0, 
            operator                  : v7, 
            old_commission_percentage : v3, 
            new_commission_percentage : arg2,
        };
        0x1::event::emit_event<UpdateCommissionEvent>(v6, v8);
    }
    
    fun update_distribution_pool(arg0: &mut 0x1::pool_u64::Pool, arg1: u64, arg2: address, arg3: u64) {
        if (0x1::pool_u64::total_coins(arg0) == arg1) {
            return
        };
        let v0 = 0x1::pool_u64::shareholders(arg0);
        let v1 = &v0;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            let v3 = *0x1::vector::borrow<address>(v1, v2);
            if (v3 != arg2) {
                let v4 = 0x1::pool_u64::shares(arg0, v3);
                let v5 = 0x1::pool_u64::shares_to_amount_with_total_coins(arg0, v4, arg1) - 0x1::pool_u64::balance(arg0, v3);
                let v6 = 0x1::pool_u64::amount_to_shares_with_total_coins(arg0, v5 * arg3 / 100, arg1);
                0x1::pool_u64::transfer_shares(arg0, v3, arg2, v6);
            };
            v2 = v2 + 1;
        };
        0x1::pool_u64::update_total_coins(arg0, arg1);
    }
    
    public entry fun update_voter(arg0: &signer, arg1: address, arg2: address) acquires Store {
        let v0 = 0x1::signer::address_of(arg0);
        assert_staking_contract_exists(v0, arg1);
        let v1 = borrow_global_mut<Store>(v0);
        let v2 = 0x1::simple_map::borrow_mut<address, StakingContract>(&mut v1.staking_contracts, &arg1);
        let v3 = v2.pool_address;
        let v4 = 0x1::stake::get_delegated_voter(v3);
        0x1::stake::set_delegated_voter_with_cap(&v2.owner_cap, arg2);
        let v5 = UpdateVoterEvent{
            operator     : arg1, 
            pool_address : v3, 
            old_voter    : v4, 
            new_voter    : arg2,
        };
        0x1::event::emit_event<UpdateVoterEvent>(&mut v1.update_voter_events, v5);
    }
    
    // decompiled from Move bytecode v6
}
