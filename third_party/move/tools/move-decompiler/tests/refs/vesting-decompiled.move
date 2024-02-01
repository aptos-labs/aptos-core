module 0x1::vesting {
    struct AdminStore has key {
        vesting_contracts: vector<address>,
        nonce: u64,
        create_events: 0x1::event::EventHandle<CreateVestingContractEvent>,
    }
    
    struct AdminWithdrawEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        amount: u64,
    }
    
    struct CreateVestingContractEvent has drop, store {
        operator: address,
        voter: address,
        grant_amount: u64,
        withdrawal_address: address,
        vesting_contract_address: address,
        staking_pool_address: address,
        commission_percentage: u64,
    }
    
    struct DistributeEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        amount: u64,
    }
    
    struct ResetLockupEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        staking_pool_address: address,
        new_lockup_expiration_secs: u64,
    }
    
    struct SetBeneficiaryEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        shareholder: address,
        old_beneficiary: address,
        new_beneficiary: address,
    }
    
    struct StakingInfo has store {
        pool_address: address,
        operator: address,
        voter: address,
        commission_percentage: u64,
    }
    
    struct TerminateEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
    }
    
    struct UnlockRewardsEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        staking_pool_address: address,
        amount: u64,
    }
    
    struct UpdateOperatorEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        staking_pool_address: address,
        old_operator: address,
        new_operator: address,
        commission_percentage: u64,
    }
    
    struct UpdateVoterEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        staking_pool_address: address,
        old_voter: address,
        new_voter: address,
    }
    
    struct VestEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        staking_pool_address: address,
        period_vested: u64,
        amount: u64,
    }
    
    struct VestingAccountManagement has key {
        roles: 0x1::simple_map::SimpleMap<0x1::string::String, address>,
    }
    
    struct VestingContract has key {
        state: u64,
        admin: address,
        grant_pool: 0x1::pool_u64::Pool,
        beneficiaries: 0x1::simple_map::SimpleMap<address, address>,
        vesting_schedule: VestingSchedule,
        withdrawal_address: address,
        staking: StakingInfo,
        remaining_grant: u64,
        signer_cap: 0x1::account::SignerCapability,
        update_operator_events: 0x1::event::EventHandle<UpdateOperatorEvent>,
        update_voter_events: 0x1::event::EventHandle<UpdateVoterEvent>,
        reset_lockup_events: 0x1::event::EventHandle<ResetLockupEvent>,
        set_beneficiary_events: 0x1::event::EventHandle<SetBeneficiaryEvent>,
        unlock_rewards_events: 0x1::event::EventHandle<UnlockRewardsEvent>,
        vest_events: 0x1::event::EventHandle<VestEvent>,
        distribute_events: 0x1::event::EventHandle<DistributeEvent>,
        terminate_events: 0x1::event::EventHandle<TerminateEvent>,
        admin_withdraw_events: 0x1::event::EventHandle<AdminWithdrawEvent>,
    }
    
    struct VestingSchedule has copy, drop, store {
        schedule: vector<0x1::fixed_point32::FixedPoint32>,
        start_timestamp_secs: u64,
        period_duration: u64,
        last_vested_period: u64,
    }
    
    public fun shareholders(arg0: address) : vector<address> acquires VestingContract {
        assert_active_vesting_contract(arg0);
        0x1::pool_u64::shareholders(&borrow_global<VestingContract>(arg0).grant_pool)
    }
    
    public entry fun distribute(arg0: address) acquires VestingContract {
        assert_active_vesting_contract(arg0);
        let v0 = borrow_global_mut<VestingContract>(arg0);
        let v1 = withdraw_stake(v0, arg0);
        let v2 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v1);
        if (v2 == 0) {
            0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v1);
            return
        };
        let v3 = &v0.grant_pool;
        let v4 = 0x1::pool_u64::shareholders(v3);
        let v5 = &v4;
        let v6 = 0;
        while (v6 < 0x1::vector::length<address>(v5)) {
            let v7 = *0x1::vector::borrow<address>(v5, v6);
            let v8 = 0x1::pool_u64::shares_to_amount_with_total_coins(v3, 0x1::pool_u64::shares(v3, v7), v2);
            let v9 = 0x1::coin::extract<0x1::aptos_coin::AptosCoin>(&mut v1, v8);
            0x1::aptos_account::deposit_coins<0x1::aptos_coin::AptosCoin>(get_beneficiary(v0, v7), v9);
            v6 = v6 + 1;
        };
        if (0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v1) > 0) {
            0x1::aptos_account::deposit_coins<0x1::aptos_coin::AptosCoin>(v0.withdrawal_address, v1);
        } else {
            0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v1);
        };
        let v10 = DistributeEvent{
            admin                    : v0.admin, 
            vesting_contract_address : arg0, 
            amount                   : v2,
        };
        0x1::event::emit_event<DistributeEvent>(&mut v0.distribute_events, v10);
    }
    
    public entry fun reset_lockup(arg0: &signer, arg1: address) acquires VestingContract {
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        let v1 = get_vesting_account_signer_internal(v0);
        0x1::staking_contract::reset_lockup(&v1, v0.staking.operator);
        let v2 = v0.admin;
        let v3 = v0.staking.pool_address;
        let v4 = 0x1::stake::get_lockup_secs(v0.staking.pool_address);
        let v5 = ResetLockupEvent{
            admin                      : v2, 
            vesting_contract_address   : arg1, 
            staking_pool_address       : v3, 
            new_lockup_expiration_secs : v4,
        };
        0x1::event::emit_event<ResetLockupEvent>(&mut v0.reset_lockup_events, v5);
    }
    
    public entry fun set_beneficiary_for_operator(arg0: &signer, arg1: address) {
        0x1::staking_contract::set_beneficiary_for_operator(arg0, arg1);
    }
    
    fun unlock_stake(arg0: &VestingContract, arg1: u64) {
        let v0 = get_vesting_account_signer_internal(arg0);
        0x1::staking_contract::unlock_stake(&v0, arg0.staking.operator, arg1);
    }
    
    public entry fun update_voter(arg0: &signer, arg1: address, arg2: address) acquires VestingContract {
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        let v1 = get_vesting_account_signer_internal(v0);
        let v2 = v0.staking.voter;
        0x1::staking_contract::update_voter(&v1, v0.staking.operator, arg2);
        v0.staking.voter = arg2;
        let v3 = v0.admin;
        let v4 = v0.staking.pool_address;
        let v5 = UpdateVoterEvent{
            admin                    : v3, 
            vesting_contract_address : arg1, 
            staking_pool_address     : v4, 
            old_voter                : v2, 
            new_voter                : arg2,
        };
        0x1::event::emit_event<UpdateVoterEvent>(&mut v0.update_voter_events, v5);
    }
    
    public fun accumulated_rewards(arg0: address, arg1: address) : u64 acquires VestingContract {
        assert_active_vesting_contract(arg0);
        let v0 = total_accumulated_rewards(arg0);
        let v1 = shareholder(arg0, arg1);
        let v2 = borrow_global<VestingContract>(arg0);
        let v3 = 0x1::pool_u64::shares(&v2.grant_pool, v1);
        0x1::pool_u64::shares_to_amount_with_total_coins(&v2.grant_pool, v3, v0)
    }
    
    public entry fun admin_withdraw(arg0: &signer, arg1: address) acquires VestingContract {
        assert!(borrow_global<VestingContract>(arg1).state == 2, 0x1::error::invalid_state(9));
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        let v1 = withdraw_stake(v0, arg1);
        let v2 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v1);
        if (v2 == 0) {
            0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v1);
            return
        };
        0x1::aptos_account::deposit_coins<0x1::aptos_coin::AptosCoin>(v0.withdrawal_address, v1);
        let v3 = AdminWithdrawEvent{
            admin                    : v0.admin, 
            vesting_contract_address : arg1, 
            amount                   : v2,
        };
        0x1::event::emit_event<AdminWithdrawEvent>(&mut v0.admin_withdraw_events, v3);
    }
    
    fun assert_active_vesting_contract(arg0: address) acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        assert!(borrow_global<VestingContract>(arg0).state == 1, 0x1::error::invalid_state(8));
    }
    
    fun assert_vesting_contract_exists(arg0: address) {
        assert!(exists<VestingContract>(arg0), 0x1::error::not_found(10));
    }
    
    public fun beneficiary(arg0: address, arg1: address) : address acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        get_beneficiary(borrow_global<VestingContract>(arg0), arg1)
    }
    
    public fun create_vesting_contract(arg0: &signer, arg1: &vector<address>, arg2: 0x1::simple_map::SimpleMap<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>, arg3: VestingSchedule, arg4: address, arg5: address, arg6: address, arg7: u64, arg8: vector<u8>) : address acquires AdminStore {
        assert!(!0x1::system_addresses::is_reserved_address(arg4), 0x1::error::invalid_argument(1));
        0x1::aptos_account::assert_account_is_registered_for_apt(arg4);
        assert!(0x1::vector::length<address>(arg1) > 0, 0x1::error::invalid_argument(4));
        assert!(0x1::simple_map::length<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(&arg2) == 0x1::vector::length<address>(arg1), 0x1::error::invalid_argument(5));
        let v0 = 0x1::coin::zero<0x1::aptos_coin::AptosCoin>();
        let v1 = 0;
        let v2 = 0x1::pool_u64::create(30);
        let v3 = 0;
        while (v3 < 0x1::vector::length<address>(arg1)) {
            let v4 = *0x1::vector::borrow<address>(arg1, v3);
            let (_, v6) = 0x1::simple_map::remove<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(&mut arg2, &v4);
            let v7 = v6;
            let v8 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v7);
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v0, v7);
            0x1::pool_u64::buy_in(&mut v2, v4, v8);
            v1 = v1 + v8;
            v3 = v3 + 1;
        };
        assert!(v1 > 0, 0x1::error::invalid_argument(12));
        let v9 = 0x1::signer::address_of(arg0);
        if (!exists<AdminStore>(v9)) {
            let v10 = AdminStore{
                vesting_contracts : 0x1::vector::empty<address>(), 
                nonce             : 0, 
                create_events     : 0x1::account::new_event_handle<CreateVestingContractEvent>(arg0),
            };
            move_to<AdminStore>(arg0, v10);
        };
        let (v11, v12) = create_vesting_contract_account(arg0, arg8);
        let v13 = v11;
        let v14 = 0x1::staking_contract::create_staking_contract_with_coins(&v13, arg5, arg6, v0, arg7, arg8);
        let v15 = 0x1::signer::address_of(&v13);
        let v16 = borrow_global_mut<AdminStore>(v9);
        0x1::vector::push_back<address>(&mut v16.vesting_contracts, v15);
        let v17 = CreateVestingContractEvent{
            operator                 : arg5, 
            voter                    : arg6, 
            grant_amount             : v1, 
            withdrawal_address       : arg4, 
            vesting_contract_address : v15, 
            staking_pool_address     : v14, 
            commission_percentage    : arg7,
        };
        0x1::event::emit_event<CreateVestingContractEvent>(&mut v16.create_events, v17);
        let v18 = 0x1::simple_map::create<address, address>();
        let v19 = StakingInfo{
            pool_address          : v14, 
            operator              : arg5, 
            voter                 : arg6, 
            commission_percentage : arg7,
        };
        let v20 = 0x1::account::new_event_handle<UpdateOperatorEvent>(&v13);
        let v21 = 0x1::account::new_event_handle<UpdateVoterEvent>(&v13);
        let v22 = 0x1::account::new_event_handle<ResetLockupEvent>(&v13);
        let v23 = 0x1::account::new_event_handle<SetBeneficiaryEvent>(&v13);
        let v24 = 0x1::account::new_event_handle<UnlockRewardsEvent>(&v13);
        let v25 = 0x1::account::new_event_handle<VestEvent>(&v13);
        let v26 = 0x1::account::new_event_handle<DistributeEvent>(&v13);
        let v27 = 0x1::account::new_event_handle<TerminateEvent>(&v13);
        let v28 = 0x1::account::new_event_handle<AdminWithdrawEvent>(&v13);
        let v29 = VestingContract{
            state                  : 1, 
            admin                  : v9, 
            grant_pool             : v2, 
            beneficiaries          : v18, 
            vesting_schedule       : arg3, 
            withdrawal_address     : arg4, 
            staking                : v19, 
            remaining_grant        : v1, 
            signer_cap             : v12, 
            update_operator_events : v20, 
            update_voter_events    : v21, 
            reset_lockup_events    : v22, 
            set_beneficiary_events : v23, 
            unlock_rewards_events  : v24, 
            vest_events            : v25, 
            distribute_events      : v26, 
            terminate_events       : v27, 
            admin_withdraw_events  : v28,
        };
        move_to<VestingContract>(&v13, v29);
        0x1::simple_map::destroy_empty<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(arg2);
        v15
    }
    
    fun create_vesting_contract_account(arg0: &signer, arg1: vector<u8>) : (signer, 0x1::account::SignerCapability) acquires AdminStore {
        let v0 = borrow_global_mut<AdminStore>(0x1::signer::address_of(arg0));
        let v1 = 0x1::signer::address_of(arg0);
        let v2 = 0x1::bcs::to_bytes<address>(&v1);
        0x1::vector::append<u8>(&mut v2, 0x1::bcs::to_bytes<u64>(&v0.nonce));
        v0.nonce = v0.nonce + 1;
        0x1::vector::append<u8>(&mut v2, b"aptos_framework::vesting");
        0x1::vector::append<u8>(&mut v2, arg1);
        let (v3, v4) = 0x1::account::create_resource_account(arg0, v2);
        let v5 = v3;
        0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v5);
        (v5, v4)
    }
    
    public fun create_vesting_schedule(arg0: vector<0x1::fixed_point32::FixedPoint32>, arg1: u64, arg2: u64) : VestingSchedule {
        let v0 = 0x1::vector::length<0x1::fixed_point32::FixedPoint32>(&arg0) > 0;
        assert!(v0, 0x1::error::invalid_argument(2));
        assert!(arg2 > 0, 0x1::error::invalid_argument(3));
        assert!(arg1 >= 0x1::timestamp::now_seconds(), 0x1::error::invalid_argument(6));
        let v1 = arg0;
        VestingSchedule{
            schedule             : v1, 
            start_timestamp_secs : arg1, 
            period_duration      : arg2, 
            last_vested_period   : 0,
        }
    }
    
    public entry fun distribute_many(arg0: vector<address>) acquires VestingContract {
        assert!(0x1::vector::length<address>(&arg0) != 0, 0x1::error::invalid_argument(16));
        let v0 = &arg0;
        let v1 = 0;
        while (v1 < 0x1::vector::length<address>(v0)) {
            distribute(*0x1::vector::borrow<address>(v0, v1));
            v1 = v1 + 1;
        };
    }
    
    fun get_beneficiary(arg0: &VestingContract, arg1: address) : address {
        if (0x1::simple_map::contains_key<address, address>(&arg0.beneficiaries, &arg1)) {
            *0x1::simple_map::borrow<address, address>(&arg0.beneficiaries, &arg1)
        } else {
            arg1
        }
    }
    
    public fun get_role_holder(arg0: address, arg1: 0x1::string::String) : address acquires VestingAccountManagement {
        assert!(exists<VestingAccountManagement>(arg0), 0x1::error::not_found(13));
        let v0 = &borrow_global<VestingAccountManagement>(arg0).roles;
        let v1 = 0x1::simple_map::contains_key<0x1::string::String, address>(v0, &arg1);
        assert!(v1, 0x1::error::not_found(14));
        *0x1::simple_map::borrow<0x1::string::String, address>(v0, &arg1)
    }
    
    public fun get_vesting_account_signer(arg0: &signer, arg1: address) : signer acquires VestingContract {
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        get_vesting_account_signer_internal(v0)
    }
    
    fun get_vesting_account_signer_internal(arg0: &VestingContract) : signer {
        0x1::account::create_signer_with_capability(&arg0.signer_cap)
    }
    
    public fun operator(arg0: address) : address acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).staking.operator
    }
    
    public fun operator_commission_percentage(arg0: address) : u64 acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).staking.commission_percentage
    }
    
    public fun period_duration_secs(arg0: address) : u64 acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).vesting_schedule.period_duration
    }
    
    public fun remaining_grant(arg0: address) : u64 acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).remaining_grant
    }
    
    public entry fun reset_beneficiary(arg0: &signer, arg1: address, arg2: address) acquires VestingAccountManagement, VestingContract {
        let v0 = borrow_global_mut<VestingContract>(arg1);
        let v1 = 0x1::signer::address_of(arg0);
        let v2 = if (v1 == v0.admin) {
            true
        } else {
            let v3 = get_role_holder(arg1, 0x1::string::utf8(b"ROLE_BENEFICIARY_RESETTER"));
            v1 == v3
        };
        assert!(v2, 0x1::error::permission_denied(15));
        let v4 = &mut v0.beneficiaries;
        if (0x1::simple_map::contains_key<address, address>(v4, &arg2)) {
            let (_, _) = 0x1::simple_map::remove<address, address>(v4, &arg2);
        };
    }
    
    public entry fun set_beneficiary(arg0: &signer, arg1: address, arg2: address, arg3: address) acquires VestingContract {
        0x1::aptos_account::assert_account_is_registered_for_apt(arg3);
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        let v1 = get_beneficiary(v0, arg2);
        let v2 = &mut v0.beneficiaries;
        if (0x1::simple_map::contains_key<address, address>(v2, &arg2)) {
            *0x1::simple_map::borrow_mut<address, address>(v2, &arg2) = arg3;
        } else {
            0x1::simple_map::add<address, address>(v2, arg2, arg3);
        };
        let v3 = v0.admin;
        let v4 = arg2;
        let v5 = SetBeneficiaryEvent{
            admin                    : v3, 
            vesting_contract_address : arg1, 
            shareholder              : v4, 
            old_beneficiary          : v1, 
            new_beneficiary          : arg3,
        };
        0x1::event::emit_event<SetBeneficiaryEvent>(&mut v0.set_beneficiary_events, v5);
    }
    
    public entry fun set_beneficiary_resetter(arg0: &signer, arg1: address, arg2: address) acquires VestingAccountManagement, VestingContract {
        set_management_role(arg0, arg1, 0x1::string::utf8(b"ROLE_BENEFICIARY_RESETTER"), arg2);
    }
    
    public entry fun set_management_role(arg0: &signer, arg1: address, arg2: 0x1::string::String, arg3: address) acquires VestingAccountManagement, VestingContract {
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        if (!exists<VestingAccountManagement>(arg1)) {
            let v1 = get_vesting_account_signer_internal(v0);
            let v2 = VestingAccountManagement{roles: 0x1::simple_map::create<0x1::string::String, address>()};
            move_to<VestingAccountManagement>(&v1, v2);
        };
        let v3 = &mut borrow_global_mut<VestingAccountManagement>(arg1).roles;
        if (0x1::simple_map::contains_key<0x1::string::String, address>(v3, &arg2)) {
            *0x1::simple_map::borrow_mut<0x1::string::String, address>(v3, &arg2) = arg3;
        } else {
            0x1::simple_map::add<0x1::string::String, address>(v3, arg2, arg3);
        };
    }
    
    public fun shareholder(arg0: address, arg1: address) : address acquires VestingContract {
        assert_active_vesting_contract(arg0);
        let v0 = shareholders(arg0);
        let v1 = &v0;
        if (0x1::vector::contains<address>(v1, &arg1)) {
            return arg1
        };
        let v2 = @0x0;
        let v3 = 0;
        while (v3 < 0x1::vector::length<address>(v1)) {
            let v4 = 0x1::vector::borrow<address>(v1, v3);
            let v5 = if (arg1 == get_beneficiary(borrow_global<VestingContract>(arg0), *v4)) {
                v2 = *v4;
                true
            } else {
                false
            };
            if (v5) {
                break
            };
            v3 = v3 + 1;
        };
        v2
    }
    
    public fun stake_pool_address(arg0: address) : address acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).staking.pool_address
    }
    
    public entry fun terminate_vesting_contract(arg0: &signer, arg1: address) acquires VestingContract {
        assert_active_vesting_contract(arg1);
        distribute(arg1);
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        let (v1, _, v3, _) = 0x1::stake::get_stake(v0.staking.pool_address);
        assert!(v3 == 0, 0x1::error::invalid_state(11));
        v0.state = 2;
        v0.remaining_grant = 0;
        unlock_stake(v0, v1);
        let v5 = TerminateEvent{
            admin                    : v0.admin, 
            vesting_contract_address : arg1,
        };
        0x1::event::emit_event<TerminateEvent>(&mut v0.terminate_events, v5);
    }
    
    public fun total_accumulated_rewards(arg0: address) : u64 acquires VestingContract {
        assert_active_vesting_contract(arg0);
        let v0 = borrow_global<VestingContract>(arg0);
        let (v1, _, v3) = 0x1::staking_contract::staking_contract_amounts(arg0, v0.staking.operator);
        v1 - v0.remaining_grant - v3
    }
    
    public entry fun unlock_rewards(arg0: address) acquires VestingContract {
        let v0 = total_accumulated_rewards(arg0);
        unlock_stake(borrow_global<VestingContract>(arg0), v0);
    }
    
    public entry fun unlock_rewards_many(arg0: vector<address>) acquires VestingContract {
        assert!(0x1::vector::length<address>(&arg0) != 0, 0x1::error::invalid_argument(16));
        let v0 = &arg0;
        let v1 = 0;
        while (v1 < 0x1::vector::length<address>(v0)) {
            unlock_rewards(*0x1::vector::borrow<address>(v0, v1));
            v1 = v1 + 1;
        };
    }
    
    public entry fun update_commission_percentage(arg0: &signer, arg1: address, arg2: u64) acquires VestingContract {
        let v0 = operator(arg1);
        let v1 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v1);
        let v2 = get_vesting_account_signer_internal(v1);
        0x1::staking_contract::update_commision(&v2, v0, arg2);
        v1.staking.commission_percentage = arg2;
    }
    
    public entry fun update_operator(arg0: &signer, arg1: address, arg2: address, arg3: u64) acquires VestingContract {
        let v0 = borrow_global_mut<VestingContract>(arg1);
        verify_admin(arg0, v0);
        let v1 = get_vesting_account_signer_internal(v0);
        let v2 = v0.staking.operator;
        0x1::staking_contract::switch_operator(&v1, v2, arg2, arg3);
        v0.staking.operator = arg2;
        v0.staking.commission_percentage = arg3;
        let v3 = v0.admin;
        let v4 = v0.staking.pool_address;
        let v5 = UpdateOperatorEvent{
            admin                    : v3, 
            vesting_contract_address : arg1, 
            staking_pool_address     : v4, 
            old_operator             : v2, 
            new_operator             : arg2, 
            commission_percentage    : arg3,
        };
        0x1::event::emit_event<UpdateOperatorEvent>(&mut v0.update_operator_events, v5);
    }
    
    public entry fun update_operator_with_same_commission(arg0: &signer, arg1: address, arg2: address) acquires VestingContract {
        let v0 = operator_commission_percentage(arg1);
        update_operator(arg0, arg1, arg2, v0);
    }
    
    fun verify_admin(arg0: &signer, arg1: &VestingContract) {
        assert!(0x1::signer::address_of(arg0) == arg1.admin, 0x1::error::unauthenticated(7));
    }
    
    public entry fun vest(arg0: address) acquires VestingContract {
        unlock_rewards(arg0);
        let v0 = borrow_global_mut<VestingContract>(arg0);
        if (v0.vesting_schedule.start_timestamp_secs > 0x1::timestamp::now_seconds()) {
            return
        };
        let v1 = &mut v0.vesting_schedule;
        let v2 = v1.last_vested_period + 1;
        if ((0x1::timestamp::now_seconds() - v1.start_timestamp_secs) / v1.period_duration < v2) {
            return
        };
        let v3 = &v1.schedule;
        let v4 = v2 - 1;
        let v5 = if (v4 < 0x1::vector::length<0x1::fixed_point32::FixedPoint32>(v3)) {
            *0x1::vector::borrow<0x1::fixed_point32::FixedPoint32>(v3, v4)
        } else {
            *0x1::vector::borrow<0x1::fixed_point32::FixedPoint32>(v3, 0x1::vector::length<0x1::fixed_point32::FixedPoint32>(v3) - 1)
        };
        let v6 = 0x1::fixed_point32::multiply_u64(0x1::pool_u64::total_coins(&v0.grant_pool), v5);
        let v7 = 0x1::math64::min(v6, v0.remaining_grant);
        v0.remaining_grant = v0.remaining_grant - v7;
        v1.last_vested_period = v2;
        unlock_stake(v0, v7);
        let v8 = v0.admin;
        let v9 = v0.staking.pool_address;
        let v10 = VestEvent{
            admin                    : v8, 
            vesting_contract_address : arg0, 
            staking_pool_address     : v9, 
            period_vested            : v2, 
            amount                   : v7,
        };
        0x1::event::emit_event<VestEvent>(&mut v0.vest_events, v10);
    }
    
    public entry fun vest_many(arg0: vector<address>) acquires VestingContract {
        assert!(0x1::vector::length<address>(&arg0) != 0, 0x1::error::invalid_argument(16));
        let v0 = &arg0;
        let v1 = 0;
        while (v1 < 0x1::vector::length<address>(v0)) {
            vest(*0x1::vector::borrow<address>(v0, v1));
            v1 = v1 + 1;
        };
    }
    
    public fun vesting_contracts(arg0: address) : vector<address> acquires AdminStore {
        if (!exists<AdminStore>(arg0)) {
            0x1::vector::empty<address>()
        } else {
            borrow_global<AdminStore>(arg0).vesting_contracts
        }
    }
    
    public fun vesting_schedule(arg0: address) : VestingSchedule acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).vesting_schedule
    }
    
    public fun vesting_start_secs(arg0: address) : u64 acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).vesting_schedule.start_timestamp_secs
    }
    
    public fun voter(arg0: address) : address acquires VestingContract {
        assert_vesting_contract_exists(arg0);
        borrow_global<VestingContract>(arg0).staking.voter
    }
    
    fun withdraw_stake(arg0: &VestingContract, arg1: address) : 0x1::coin::Coin<0x1::aptos_coin::AptosCoin> {
        0x1::staking_contract::distribute(arg1, arg0.staking.operator);
        let v0 = 0x1::coin::balance<0x1::aptos_coin::AptosCoin>(arg1);
        let v1 = get_vesting_account_signer_internal(arg0);
        0x1::coin::withdraw<0x1::aptos_coin::AptosCoin>(&v1, v0)
    }
    
    // decompiled from Move bytecode v6
}
