module 0x1::stake {
    struct AddStakeEvent has drop, store {
        pool_address: address,
        amount_added: u64,
    }
    
    struct AllowedValidators has key {
        accounts: vector<address>,
    }
    
    struct AptosCoinCapabilities has key {
        mint_cap: 0x1::coin::MintCapability<0x1::aptos_coin::AptosCoin>,
    }
    
    struct DistributeRewardsEvent has drop, store {
        pool_address: address,
        rewards_amount: u64,
    }
    
    struct IncreaseLockupEvent has drop, store {
        pool_address: address,
        old_locked_until_secs: u64,
        new_locked_until_secs: u64,
    }
    
    struct IndividualValidatorPerformance has drop, store {
        successful_proposals: u64,
        failed_proposals: u64,
    }
    
    struct JoinValidatorSetEvent has drop, store {
        pool_address: address,
    }
    
    struct LeaveValidatorSetEvent has drop, store {
        pool_address: address,
    }
    
    struct OwnerCapability has store, key {
        pool_address: address,
    }
    
    struct ReactivateStakeEvent has drop, store {
        pool_address: address,
        amount: u64,
    }
    
    struct RegisterValidatorCandidateEvent has drop, store {
        pool_address: address,
    }
    
    struct RotateConsensusKeyEvent has drop, store {
        pool_address: address,
        old_consensus_pubkey: vector<u8>,
        new_consensus_pubkey: vector<u8>,
    }
    
    struct SetOperatorEvent has drop, store {
        pool_address: address,
        old_operator: address,
        new_operator: address,
    }
    
    struct StakePool has key {
        active: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>,
        inactive: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>,
        pending_active: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>,
        pending_inactive: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>,
        locked_until_secs: u64,
        operator_address: address,
        delegated_voter: address,
        initialize_validator_events: 0x1::event::EventHandle<RegisterValidatorCandidateEvent>,
        set_operator_events: 0x1::event::EventHandle<SetOperatorEvent>,
        add_stake_events: 0x1::event::EventHandle<AddStakeEvent>,
        reactivate_stake_events: 0x1::event::EventHandle<ReactivateStakeEvent>,
        rotate_consensus_key_events: 0x1::event::EventHandle<RotateConsensusKeyEvent>,
        update_network_and_fullnode_addresses_events: 0x1::event::EventHandle<UpdateNetworkAndFullnodeAddressesEvent>,
        increase_lockup_events: 0x1::event::EventHandle<IncreaseLockupEvent>,
        join_validator_set_events: 0x1::event::EventHandle<JoinValidatorSetEvent>,
        distribute_rewards_events: 0x1::event::EventHandle<DistributeRewardsEvent>,
        unlock_stake_events: 0x1::event::EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: 0x1::event::EventHandle<WithdrawStakeEvent>,
        leave_validator_set_events: 0x1::event::EventHandle<LeaveValidatorSetEvent>,
    }
    
    struct UnlockStakeEvent has drop, store {
        pool_address: address,
        amount_unlocked: u64,
    }
    
    struct UpdateNetworkAndFullnodeAddressesEvent has drop, store {
        pool_address: address,
        old_network_addresses: vector<u8>,
        new_network_addresses: vector<u8>,
        old_fullnode_addresses: vector<u8>,
        new_fullnode_addresses: vector<u8>,
    }
    
    struct ValidatorConfig has copy, drop, store, key {
        consensus_pubkey: vector<u8>,
        network_addresses: vector<u8>,
        fullnode_addresses: vector<u8>,
        validator_index: u64,
    }
    
    struct ValidatorFees has key {
        fees_table: 0x1::table::Table<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>,
    }
    
    struct ValidatorInfo has copy, drop, store {
        addr: address,
        voting_power: u64,
        config: ValidatorConfig,
    }
    
    struct ValidatorPerformance has key {
        validators: vector<IndividualValidatorPerformance>,
    }
    
    struct ValidatorSet has key {
        consensus_scheme: u8,
        active_validators: vector<ValidatorInfo>,
        pending_inactive: vector<ValidatorInfo>,
        pending_active: vector<ValidatorInfo>,
        total_voting_power: u128,
        total_joining_power: u128,
    }
    
    struct WithdrawStakeEvent has drop, store {
        pool_address: address,
        amount_withdrawn: u64,
    }
    
    public entry fun withdraw(arg0: &signer, arg1: u64) acquires OwnerCapability, StakePool, ValidatorSet {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        let v1 = withdraw_with_cap(borrow_global<OwnerCapability>(v0), arg1);
        0x1::coin::deposit<0x1::aptos_coin::AptosCoin>(v0, v1);
    }
    
    public entry fun add_stake(arg0: &signer, arg1: u64) acquires OwnerCapability, StakePool, ValidatorSet {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        let v1 = 0x1::coin::withdraw<0x1::aptos_coin::AptosCoin>(arg0, arg1);
        add_stake_with_cap(borrow_global<OwnerCapability>(v0), v1);
    }
    
    public fun add_stake_with_cap(arg0: &OwnerCapability, arg1: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>) acquires StakePool, ValidatorSet {
        let v0 = arg0.pool_address;
        assert_stake_pool_exists(v0);
        let v1 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&arg1);
        if (v1 == 0) {
            0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(arg1);
            return
        };
        let v2 = borrow_global_mut<ValidatorSet>(@0x1);
        let v3 = find_validator(&v2.active_validators, v0);
        let v4 = if (0x1::option::is_some<u64>(&v3)) {
            true
        } else {
            let v5 = find_validator(&v2.pending_active, v0);
            0x1::option::is_some<u64>(&v5)
        };
        if (v4) {
            update_voting_power_increase(v1);
        };
        let v6 = borrow_global_mut<StakePool>(v0);
        if (is_current_epoch_validator(v0)) {
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v6.pending_active, arg1);
        } else {
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v6.active, arg1);
        };
        let v7 = 0x1::staking_config::get();
        let (_, v9) = 0x1::staking_config::get_required_stake(&v7);
        assert!(get_next_epoch_voting_power(v6) <= v9, 0x1::error::invalid_argument(7));
        let v10 = AddStakeEvent{
            pool_address : v0, 
            amount_added : v1,
        };
        0x1::event::emit_event<AddStakeEvent>(&mut v6.add_stake_events, v10);
    }
    
    public(friend) fun add_transaction_fee(arg0: address, arg1: 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>) acquires ValidatorFees {
        let v0 = &mut borrow_global_mut<ValidatorFees>(@0x1).fees_table;
        if (0x1::table::contains<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(v0, arg0)) {
            let v1 = 0x1::table::borrow_mut<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(v0, arg0);
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(v1, arg1);
        } else {
            0x1::table::add<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(v0, arg0, arg1);
        };
    }
    
    fun append<T0>(arg0: &mut vector<T0>, arg1: &mut vector<T0>) {
        while (!0x1::vector::is_empty<T0>(arg1)) {
            0x1::vector::push_back<T0>(arg0, 0x1::vector::pop_back<T0>(arg1));
        };
    }
    
    fun assert_owner_cap_exists(arg0: address) {
        assert!(exists<OwnerCapability>(arg0), 0x1::error::not_found(15));
    }
    
    fun assert_stake_pool_exists(arg0: address) {
        assert!(stake_pool_exists(arg0), 0x1::error::invalid_argument(14));
    }
    
    fun calculate_rewards_amount(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) : u64 {
        let v0 = (arg4 as u128) * (arg2 as u128);
        if (v0 > 0) {
            ((arg0 as u128) * (arg3 as u128) * (arg1 as u128) / v0) as u64
        } else {
            0
        }
    }
    
    public fun configure_allowed_validators(arg0: &signer, arg1: vector<address>) acquires AllowedValidators {
        let v0 = 0x1::signer::address_of(arg0);
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (!exists<AllowedValidators>(v0)) {
            let v1 = AllowedValidators{accounts: arg1};
            move_to<AllowedValidators>(arg0, v1);
        } else {
            borrow_global_mut<AllowedValidators>(v0).accounts = arg1;
        };
    }
    
    public fun deposit_owner_cap(arg0: &signer, arg1: OwnerCapability) {
        assert!(!exists<OwnerCapability>(0x1::signer::address_of(arg0)), 0x1::error::not_found(16));
        move_to<OwnerCapability>(arg0, arg1);
    }
    
    public fun destroy_owner_cap(arg0: OwnerCapability) {
        let OwnerCapability {  } = arg0;
    }
    
    fun distribute_rewards(arg0: &mut 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>, arg1: u64, arg2: u64, arg3: u64, arg4: u64) : u64 acquires AptosCoinCapabilities {
        let v0 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(arg0);
        let v1 = if (v0 > 0) {
            calculate_rewards_amount(v0, arg1, arg2, arg3, arg4)
        } else {
            0
        };
        if (v1 > 0) {
            let v2 = borrow_global<AptosCoinCapabilities>(@0x1);
            let v3 = 0x1::coin::mint<0x1::aptos_coin::AptosCoin>(v1, &v2.mint_cap);
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(arg0, v3);
        };
        v1
    }
    
    public fun extract_owner_cap(arg0: &signer) : OwnerCapability acquires OwnerCapability {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        move_from<OwnerCapability>(v0)
    }
    
    fun find_validator(arg0: &vector<ValidatorInfo>, arg1: address) : 0x1::option::Option<u64> {
        let v0 = 0;
        while (v0 < 0x1::vector::length<ValidatorInfo>(arg0)) {
            if (0x1::vector::borrow<ValidatorInfo>(arg0, v0).addr == arg1) {
                return 0x1::option::some<u64>(v0)
            };
            v0 = v0 + 1;
        };
        0x1::option::none<u64>()
    }
    
    fun generate_validator_info(arg0: address, arg1: &StakePool, arg2: ValidatorConfig) : ValidatorInfo {
        ValidatorInfo{
            addr         : arg0, 
            voting_power : get_next_epoch_voting_power(arg1), 
            config       : arg2,
        }
    }
    
    public fun get_current_epoch_proposal_counts(arg0: u64) : (u64, u64) acquires ValidatorPerformance {
        let v0 = &borrow_global<ValidatorPerformance>(@0x1).validators;
        let v1 = 0x1::vector::borrow<IndividualValidatorPerformance>(v0, arg0);
        (v1.successful_proposals, v1.failed_proposals)
    }
    
    public fun get_current_epoch_voting_power(arg0: address) : u64 acquires StakePool, ValidatorSet {
        assert_stake_pool_exists(arg0);
        let v0 = get_validator_state(arg0);
        if (v0 == 2 || v0 == 3) {
            0x1::coin::value<0x1::aptos_coin::AptosCoin>(&borrow_global<StakePool>(arg0).active) + 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&borrow_global<StakePool>(arg0).pending_inactive)
        } else {
            0
        }
    }
    
    public fun get_delegated_voter(arg0: address) : address acquires StakePool {
        assert_stake_pool_exists(arg0);
        borrow_global<StakePool>(arg0).delegated_voter
    }
    
    public fun get_lockup_secs(arg0: address) : u64 acquires StakePool {
        assert_stake_pool_exists(arg0);
        borrow_global<StakePool>(arg0).locked_until_secs
    }
    
    fun get_next_epoch_voting_power(arg0: &StakePool) : u64 {
        let v0 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&arg0.pending_active);
        let v1 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&arg0.pending_inactive);
        v0 + 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&arg0.active) + v1
    }
    
    public fun get_operator(arg0: address) : address acquires StakePool {
        assert_stake_pool_exists(arg0);
        borrow_global<StakePool>(arg0).operator_address
    }
    
    public fun get_owned_pool_address(arg0: &OwnerCapability) : address {
        arg0.pool_address
    }
    
    public fun get_remaining_lockup_secs(arg0: address) : u64 acquires StakePool {
        assert_stake_pool_exists(arg0);
        let v0 = borrow_global<StakePool>(arg0).locked_until_secs;
        if (v0 <= 0x1::timestamp::now_seconds()) {
            0
        } else {
            v0 - 0x1::timestamp::now_seconds()
        }
    }
    
    public fun get_stake(arg0: address) : (u64, u64, u64, u64) acquires StakePool {
        assert_stake_pool_exists(arg0);
        let v0 = borrow_global<StakePool>(arg0);
        let v1 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v0.inactive);
        let v2 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v0.pending_active);
        let v3 = 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v0.pending_inactive);
        (0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v0.active), v1, v2, v3)
    }
    
    public fun get_validator_config(arg0: address) : (vector<u8>, vector<u8>, vector<u8>) acquires ValidatorConfig {
        assert_stake_pool_exists(arg0);
        let v0 = borrow_global<ValidatorConfig>(arg0);
        (v0.consensus_pubkey, v0.network_addresses, v0.fullnode_addresses)
    }
    
    public fun get_validator_index(arg0: address) : u64 acquires ValidatorConfig {
        assert_stake_pool_exists(arg0);
        borrow_global<ValidatorConfig>(arg0).validator_index
    }
    
    public fun get_validator_state(arg0: address) : u64 acquires ValidatorSet {
        let v0 = borrow_global<ValidatorSet>(@0x1);
        let v1 = find_validator(&v0.pending_active, arg0);
        if (0x1::option::is_some<u64>(&v1)) {
            1
        } else {
            let v3 = find_validator(&v0.active_validators, arg0);
            let v4 = if (0x1::option::is_some<u64>(&v3)) {
                2
            } else {
                let v5 = find_validator(&v0.pending_inactive, arg0);
                let v6 = if (0x1::option::is_some<u64>(&v5)) {
                    3
                } else {
                    4
                };
                v6
            };
            v4
        }
    }
    
    public entry fun increase_lockup(arg0: &signer) acquires OwnerCapability, StakePool {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        increase_lockup_with_cap(borrow_global<OwnerCapability>(v0));
    }
    
    public fun increase_lockup_with_cap(arg0: &OwnerCapability) acquires StakePool {
        let v0 = arg0.pool_address;
        assert_stake_pool_exists(v0);
        let v1 = 0x1::staking_config::get();
        let v2 = borrow_global_mut<StakePool>(v0);
        let v3 = v2.locked_until_secs;
        let v4 = 0x1::timestamp::now_seconds() + 0x1::staking_config::get_recurring_lockup_duration(&v1);
        assert!(v3 < v4, 0x1::error::invalid_argument(18));
        v2.locked_until_secs = v4;
        let v5 = IncreaseLockupEvent{
            pool_address          : v0, 
            old_locked_until_secs : v3, 
            new_locked_until_secs : v4,
        };
        0x1::event::emit_event<IncreaseLockupEvent>(&mut v2.increase_lockup_events, v5);
    }
    
    public(friend) fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = 0x1::vector::empty<ValidatorInfo>();
        let v1 = 0x1::vector::empty<ValidatorInfo>();
        let v2 = 0x1::vector::empty<ValidatorInfo>();
        let v3 = ValidatorSet{
            consensus_scheme    : 0, 
            active_validators   : v0, 
            pending_inactive    : v2, 
            pending_active      : v1, 
            total_voting_power  : 0, 
            total_joining_power : 0,
        };
        move_to<ValidatorSet>(arg0, v3);
        let v4 = ValidatorPerformance{validators: 0x1::vector::empty<IndividualValidatorPerformance>()};
        move_to<ValidatorPerformance>(arg0, v4);
    }
    
    fun initialize_owner(arg0: &signer) acquires AllowedValidators {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(is_allowed(v0), 0x1::error::not_found(17));
        assert!(!stake_pool_exists(v0), 0x1::error::already_exists(8));
        let v1 = 0x1::coin::zero<0x1::aptos_coin::AptosCoin>();
        let v2 = 0x1::coin::zero<0x1::aptos_coin::AptosCoin>();
        let v3 = 0x1::coin::zero<0x1::aptos_coin::AptosCoin>();
        let v4 = 0x1::coin::zero<0x1::aptos_coin::AptosCoin>();
        let v5 = 0x1::account::new_event_handle<RegisterValidatorCandidateEvent>(arg0);
        let v6 = 0x1::account::new_event_handle<SetOperatorEvent>(arg0);
        let v7 = 0x1::account::new_event_handle<AddStakeEvent>(arg0);
        let v8 = 0x1::account::new_event_handle<ReactivateStakeEvent>(arg0);
        let v9 = 0x1::account::new_event_handle<RotateConsensusKeyEvent>(arg0);
        let v10 = 0x1::account::new_event_handle<UpdateNetworkAndFullnodeAddressesEvent>(arg0);
        let v11 = 0x1::account::new_event_handle<IncreaseLockupEvent>(arg0);
        let v12 = 0x1::account::new_event_handle<JoinValidatorSetEvent>(arg0);
        let v13 = 0x1::account::new_event_handle<DistributeRewardsEvent>(arg0);
        let v14 = 0x1::account::new_event_handle<UnlockStakeEvent>(arg0);
        let v15 = 0x1::account::new_event_handle<WithdrawStakeEvent>(arg0);
        let v16 = 0x1::account::new_event_handle<LeaveValidatorSetEvent>(arg0);
        let v17 = StakePool{
            active                                       : v1, 
            inactive                                     : v4, 
            pending_active                               : v2, 
            pending_inactive                             : v3, 
            locked_until_secs                            : 0, 
            operator_address                             : v0, 
            delegated_voter                              : v0, 
            initialize_validator_events                  : v5, 
            set_operator_events                          : v6, 
            add_stake_events                             : v7, 
            reactivate_stake_events                      : v8, 
            rotate_consensus_key_events                  : v9, 
            update_network_and_fullnode_addresses_events : v10, 
            increase_lockup_events                       : v11, 
            join_validator_set_events                    : v12, 
            distribute_rewards_events                    : v13, 
            unlock_stake_events                          : v14, 
            withdraw_stake_events                        : v15, 
            leave_validator_set_events                   : v16,
        };
        move_to<StakePool>(arg0, v17);
        let v18 = OwnerCapability{pool_address: v0};
        move_to<OwnerCapability>(arg0, v18);
    }
    
    public entry fun initialize_stake_owner(arg0: &signer, arg1: u64, arg2: address, arg3: address) acquires AllowedValidators, OwnerCapability, StakePool, ValidatorSet {
        initialize_owner(arg0);
        let v0 = 0x1::vector::empty<u8>();
        let v1 = 0x1::vector::empty<u8>();
        let v2 = 0x1::vector::empty<u8>();
        let v3 = ValidatorConfig{
            consensus_pubkey   : v0, 
            network_addresses  : v1, 
            fullnode_addresses : v2, 
            validator_index    : 0,
        };
        move_to<ValidatorConfig>(arg0, v3);
        if (arg1 > 0) {
            add_stake(arg0, arg1);
        };
        let v4 = 0x1::signer::address_of(arg0);
        if (v4 != arg2) {
            set_operator(arg0, arg2);
        };
        if (v4 != arg3) {
            set_delegated_voter(arg0, arg3);
        };
    }
    
    public entry fun initialize_validator(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<u8>) acquires AllowedValidators {
        let v0 = 0x1::bls12381::proof_of_possession_from_bytes(arg2);
        let v1 = 0x1::bls12381::public_key_from_bytes_with_pop(arg1, &v0);
        assert!(0x1::option::is_some<0x1::bls12381::PublicKeyWithPoP>(&mut v1), 0x1::error::invalid_argument(11));
        initialize_owner(arg0);
        let v2 = ValidatorConfig{
            consensus_pubkey   : arg1, 
            network_addresses  : arg3, 
            fullnode_addresses : arg4, 
            validator_index    : 0,
        };
        move_to<ValidatorConfig>(arg0, v2);
    }
    
    public(friend) fun initialize_validator_fees(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!exists<ValidatorFees>(@0x1), 0x1::error::already_exists(19));
        let v0 = ValidatorFees{fees_table: 0x1::table::new<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>()};
        move_to<ValidatorFees>(arg0, v0);
    }
    
    fun is_allowed(arg0: address) : bool acquires AllowedValidators {
        let v0 = exists<AllowedValidators>(@0x1);
        !v0 || 0x1::vector::contains<address>(&borrow_global<AllowedValidators>(@0x1).accounts, &arg0)
    }
    
    public fun is_current_epoch_validator(arg0: address) : bool acquires ValidatorSet {
        assert_stake_pool_exists(arg0);
        let v0 = get_validator_state(arg0);
        v0 == 2 || v0 == 3
    }
    
    public entry fun join_validator_set(arg0: &signer, arg1: address) acquires StakePool, ValidatorConfig, ValidatorSet {
        let v0 = 0x1::staking_config::get();
        assert!(0x1::staking_config::get_allow_validator_set_change(&v0), 0x1::error::invalid_argument(10));
        join_validator_set_internal(arg0, arg1);
    }
    
    public(friend) fun join_validator_set_internal(arg0: &signer, arg1: address) acquires StakePool, ValidatorConfig, ValidatorSet {
        assert_stake_pool_exists(arg1);
        let v0 = borrow_global_mut<StakePool>(arg1);
        assert!(0x1::signer::address_of(arg0) == v0.operator_address, 0x1::error::unauthenticated(9));
        let v1 = get_validator_state(arg1);
        assert!(v1 == 4, 0x1::error::invalid_state(4));
        let v2 = 0x1::staking_config::get();
        let (v3, v4) = 0x1::staking_config::get_required_stake(&v2);
        let v5 = get_next_epoch_voting_power(v0);
        assert!(v5 >= v3, 0x1::error::invalid_argument(2));
        assert!(v5 <= v4, 0x1::error::invalid_argument(3));
        update_voting_power_increase(v5);
        let v6 = borrow_global_mut<ValidatorConfig>(arg1);
        assert!(!0x1::vector::is_empty<u8>(&v6.consensus_pubkey), 0x1::error::invalid_argument(11));
        let v7 = borrow_global_mut<ValidatorSet>(@0x1);
        let v8 = generate_validator_info(arg1, v0, *v6);
        0x1::vector::push_back<ValidatorInfo>(&mut v7.pending_active, v8);
        let v9 = 0x1::vector::length<ValidatorInfo>(&v7.active_validators);
        let v10 = v9 + 0x1::vector::length<ValidatorInfo>(&v7.pending_active) <= 65536;
        assert!(v10, 0x1::error::invalid_argument(12));
        let v11 = JoinValidatorSetEvent{pool_address: arg1};
        0x1::event::emit_event<JoinValidatorSetEvent>(&mut v0.join_validator_set_events, v11);
    }
    
    public entry fun leave_validator_set(arg0: &signer, arg1: address) acquires StakePool, ValidatorSet {
        let v0 = 0x1::staking_config::get();
        assert!(0x1::staking_config::get_allow_validator_set_change(&v0), 0x1::error::invalid_argument(10));
        assert_stake_pool_exists(arg1);
        let v1 = borrow_global_mut<StakePool>(arg1);
        assert!(0x1::signer::address_of(arg0) == v1.operator_address, 0x1::error::unauthenticated(9));
        let v2 = borrow_global_mut<ValidatorSet>(@0x1);
        let v3 = find_validator(&v2.pending_active, arg1);
        if (0x1::option::is_some<u64>(&v3)) {
            0x1::vector::swap_remove<ValidatorInfo>(&mut v2.pending_active, 0x1::option::extract<u64>(&mut v3));
            let v4 = get_next_epoch_voting_power(v1) as u128;
            if (v2.total_joining_power > v4) {
                v2.total_joining_power = v2.total_joining_power - v4;
            } else {
                v2.total_joining_power = 0;
            };
        } else {
            let v5 = find_validator(&v2.active_validators, arg1);
            assert!(0x1::option::is_some<u64>(&v5), 0x1::error::invalid_state(5));
            let v6 = &mut v5;
            let v7 = 0x1::vector::swap_remove<ValidatorInfo>(&mut v2.active_validators, 0x1::option::extract<u64>(v6));
            assert!(0x1::vector::length<ValidatorInfo>(&v2.active_validators) > 0, 0x1::error::invalid_state(6));
            0x1::vector::push_back<ValidatorInfo>(&mut v2.pending_inactive, v7);
            let v8 = LeaveValidatorSetEvent{pool_address: arg1};
            0x1::event::emit_event<LeaveValidatorSetEvent>(&mut v1.leave_validator_set_events, v8);
        };
    }
    
    public(friend) fun on_new_epoch() acquires AptosCoinCapabilities, StakePool, ValidatorConfig, ValidatorFees, ValidatorPerformance, ValidatorSet {
        let v0 = borrow_global_mut<ValidatorSet>(@0x1);
        let v1 = 0x1::staking_config::get();
        let v2 = borrow_global_mut<ValidatorPerformance>(@0x1);
        let v3 = &v0.active_validators;
        let v4 = 0;
        while (v4 < 0x1::vector::length<ValidatorInfo>(v3)) {
            update_stake_pool(v2, 0x1::vector::borrow<ValidatorInfo>(v3, v4).addr, &v1);
            v4 = v4 + 1;
        };
        let v5 = &v0.pending_inactive;
        let v6 = 0;
        while (v6 < 0x1::vector::length<ValidatorInfo>(v5)) {
            update_stake_pool(v2, 0x1::vector::borrow<ValidatorInfo>(v5, v6).addr, &v1);
            v6 = v6 + 1;
        };
        append<ValidatorInfo>(&mut v0.active_validators, &mut v0.pending_active);
        v0.pending_inactive = 0x1::vector::empty<ValidatorInfo>();
        let v7 = 0x1::vector::empty<ValidatorInfo>();
        let (v8, _) = 0x1::staking_config::get_required_stake(&v1);
        let v10 = 0;
        let v11 = 0;
        while (v11 < 0x1::vector::length<ValidatorInfo>(&v0.active_validators)) {
            let v12 = 0x1::vector::borrow_mut<ValidatorInfo>(&mut v0.active_validators, v11).addr;
            let v13 = *borrow_global_mut<ValidatorConfig>(v12);
            let v14 = generate_validator_info(v12, borrow_global_mut<StakePool>(v12), v13);
            if (v14.voting_power >= v8) {
                v10 = v10 + (v14.voting_power as u128);
                0x1::vector::push_back<ValidatorInfo>(&mut v7, v14);
            };
            v11 = v11 + 1;
        };
        v0.active_validators = v7;
        v0.total_voting_power = v10;
        v0.total_joining_power = 0;
        v2.validators = 0x1::vector::empty<IndividualValidatorPerformance>();
        let v15 = 0;
        while (v15 < 0x1::vector::length<ValidatorInfo>(&v0.active_validators)) {
            let v16 = 0x1::vector::borrow_mut<ValidatorInfo>(&mut v0.active_validators, v15);
            v16.config.validator_index = v15;
            borrow_global_mut<ValidatorConfig>(v16.addr).validator_index = v15;
            let v17 = IndividualValidatorPerformance{
                successful_proposals : 0, 
                failed_proposals     : 0,
            };
            0x1::vector::push_back<IndividualValidatorPerformance>(&mut v2.validators, v17);
            let v18 = borrow_global_mut<StakePool>(v16.addr);
            if (v18.locked_until_secs <= 0x1::timestamp::now_seconds()) {
                let v19 = 0x1::timestamp::now_seconds() + 0x1::staking_config::get_recurring_lockup_duration(&v1);
                v18.locked_until_secs = v19;
            };
            v15 = v15 + 1;
        };
        if (0x1::features::periodical_reward_rate_decrease_enabled()) {
            0x1::staking_config::calculate_and_save_latest_epoch_rewards_rate();
        };
    }
    
    public entry fun reactivate_stake(arg0: &signer, arg1: u64) acquires OwnerCapability, StakePool {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        reactivate_stake_with_cap(borrow_global<OwnerCapability>(v0), arg1);
    }
    
    public fun reactivate_stake_with_cap(arg0: &OwnerCapability, arg1: u64) acquires StakePool {
        let v0 = arg0.pool_address;
        assert_stake_pool_exists(v0);
        let v1 = borrow_global_mut<StakePool>(v0);
        let v2 = 0x1::math64::min(arg1, 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v1.pending_inactive));
        let v3 = 0x1::coin::extract<0x1::aptos_coin::AptosCoin>(&mut v1.pending_inactive, v2);
        0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v1.active, v3);
        let v4 = ReactivateStakeEvent{
            pool_address : v0, 
            amount       : v2,
        };
        0x1::event::emit_event<ReactivateStakeEvent>(&mut v1.reactivate_stake_events, v4);
    }
    
    public fun remove_validators(arg0: &signer, arg1: &vector<address>) acquires ValidatorSet {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = borrow_global_mut<ValidatorSet>(@0x1);
        let v1 = &mut v0.active_validators;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(arg1)) {
            let v3 = find_validator(v1, *0x1::vector::borrow<address>(arg1, v2));
            if (0x1::option::is_some<u64>(&v3)) {
                let v4 = 0x1::vector::swap_remove<ValidatorInfo>(v1, *0x1::option::borrow<u64>(&v3));
                0x1::vector::push_back<ValidatorInfo>(&mut v0.pending_inactive, v4);
            };
            v2 = v2 + 1;
        };
    }
    
    public entry fun rotate_consensus_key(arg0: &signer, arg1: address, arg2: vector<u8>, arg3: vector<u8>) acquires StakePool, ValidatorConfig {
        assert_stake_pool_exists(arg1);
        let v0 = borrow_global_mut<StakePool>(arg1);
        assert!(0x1::signer::address_of(arg0) == v0.operator_address, 0x1::error::unauthenticated(9));
        assert!(exists<ValidatorConfig>(arg1), 0x1::error::not_found(1));
        let v1 = borrow_global_mut<ValidatorConfig>(arg1);
        let v2 = v1.consensus_pubkey;
        let v3 = 0x1::bls12381::proof_of_possession_from_bytes(arg3);
        let v4 = 0x1::bls12381::public_key_from_bytes_with_pop(arg2, &v3);
        let v5 = 0x1::option::is_some<0x1::bls12381::PublicKeyWithPoP>(&mut v4);
        assert!(v5, 0x1::error::invalid_argument(11));
        v1.consensus_pubkey = arg2;
        let v6 = RotateConsensusKeyEvent{
            pool_address         : arg1, 
            old_consensus_pubkey : v2, 
            new_consensus_pubkey : arg2,
        };
        0x1::event::emit_event<RotateConsensusKeyEvent>(&mut v0.rotate_consensus_key_events, v6);
    }
    
    public entry fun set_delegated_voter(arg0: &signer, arg1: address) acquires OwnerCapability, StakePool {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        set_delegated_voter_with_cap(borrow_global<OwnerCapability>(v0), arg1);
    }
    
    public fun set_delegated_voter_with_cap(arg0: &OwnerCapability, arg1: address) acquires StakePool {
        let v0 = arg0.pool_address;
        assert_stake_pool_exists(v0);
        borrow_global_mut<StakePool>(v0).delegated_voter = arg1;
    }
    
    public entry fun set_operator(arg0: &signer, arg1: address) acquires OwnerCapability, StakePool {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        set_operator_with_cap(borrow_global<OwnerCapability>(v0), arg1);
    }
    
    public fun set_operator_with_cap(arg0: &OwnerCapability, arg1: address) acquires StakePool {
        let v0 = arg0.pool_address;
        assert_stake_pool_exists(v0);
        let v1 = borrow_global_mut<StakePool>(v0);
        v1.operator_address = arg1;
        let v2 = SetOperatorEvent{
            pool_address : v0, 
            old_operator : v1.operator_address, 
            new_operator : arg1,
        };
        0x1::event::emit_event<SetOperatorEvent>(&mut v1.set_operator_events, v2);
    }
    
    public fun stake_pool_exists(arg0: address) : bool {
        exists<StakePool>(arg0)
    }
    
    public(friend) fun store_aptos_coin_mint_cap(arg0: &signer, arg1: 0x1::coin::MintCapability<0x1::aptos_coin::AptosCoin>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = AptosCoinCapabilities{mint_cap: arg1};
        move_to<AptosCoinCapabilities>(arg0, v0);
    }
    
    public entry fun unlock(arg0: &signer, arg1: u64) acquires OwnerCapability, StakePool {
        let v0 = 0x1::signer::address_of(arg0);
        assert_owner_cap_exists(v0);
        unlock_with_cap(arg1, borrow_global<OwnerCapability>(v0));
    }
    
    public fun unlock_with_cap(arg0: u64, arg1: &OwnerCapability) acquires StakePool {
        if (arg0 == 0) {
            return
        };
        let v0 = arg1.pool_address;
        assert_stake_pool_exists(v0);
        let v1 = borrow_global_mut<StakePool>(v0);
        let v2 = 0x1::math64::min(arg0, 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v1.active));
        let v3 = 0x1::coin::extract<0x1::aptos_coin::AptosCoin>(&mut v1.active, v2);
        0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v1.pending_inactive, v3);
        let v4 = UnlockStakeEvent{
            pool_address    : v0, 
            amount_unlocked : v2,
        };
        0x1::event::emit_event<UnlockStakeEvent>(&mut v1.unlock_stake_events, v4);
    }
    
    public entry fun update_network_and_fullnode_addresses(arg0: &signer, arg1: address, arg2: vector<u8>, arg3: vector<u8>) acquires StakePool, ValidatorConfig {
        assert_stake_pool_exists(arg1);
        let v0 = borrow_global_mut<StakePool>(arg1);
        assert!(0x1::signer::address_of(arg0) == v0.operator_address, 0x1::error::unauthenticated(9));
        assert!(exists<ValidatorConfig>(arg1), 0x1::error::not_found(1));
        let v1 = borrow_global_mut<ValidatorConfig>(arg1);
        let v2 = v1.network_addresses;
        v1.network_addresses = arg2;
        let v3 = v1.fullnode_addresses;
        v1.fullnode_addresses = arg3;
        let v4 = UpdateNetworkAndFullnodeAddressesEvent{
            pool_address           : arg1, 
            old_network_addresses  : v2, 
            new_network_addresses  : arg2, 
            old_fullnode_addresses : v3, 
            new_fullnode_addresses : arg3,
        };
        0x1::event::emit_event<UpdateNetworkAndFullnodeAddressesEvent>(&mut v0.update_network_and_fullnode_addresses_events, v4);
    }
    
    public(friend) fun update_performance_statistics(arg0: 0x1::option::Option<u64>, arg1: vector<u64>) acquires ValidatorPerformance {
        let v0 = borrow_global_mut<ValidatorPerformance>(@0x1);
        let v1 = 0x1::vector::length<IndividualValidatorPerformance>(&v0.validators);
        if (0x1::option::is_some<u64>(&arg0)) {
            let v2 = 0x1::option::extract<u64>(&mut arg0);
            if (v2 < v1) {
                let v3 = 0x1::vector::borrow_mut<IndividualValidatorPerformance>(&mut v0.validators, v2);
                v3.successful_proposals = v3.successful_proposals + 1;
            };
        };
        let v4 = 0;
        while (v4 < 0x1::vector::length<u64>(&arg1)) {
            let v5 = *0x1::vector::borrow<u64>(&arg1, v4);
            if (v5 < v1) {
                let v6 = 0x1::vector::borrow_mut<IndividualValidatorPerformance>(&mut v0.validators, v5);
                v6.failed_proposals = v6.failed_proposals + 1;
            };
            v4 = v4 + 1;
        };
    }
    
    fun update_stake_pool(arg0: &ValidatorPerformance, arg1: address, arg2: &0x1::staking_config::StakingConfig) acquires AptosCoinCapabilities, StakePool, ValidatorConfig, ValidatorFees {
        let v0 = borrow_global_mut<StakePool>(arg1);
        let v1 = borrow_global<ValidatorConfig>(arg1).validator_index;
        let v2 = 0x1::vector::borrow<IndividualValidatorPerformance>(&arg0.validators, v1);
        let v3 = v2.successful_proposals;
        let v4 = v2.successful_proposals + v2.failed_proposals;
        let (v5, v6) = 0x1::staking_config::get_reward_rate(arg2);
        let v7 = distribute_rewards(&mut v0.active, v3, v4, v5, v6);
        let v8 = distribute_rewards(&mut v0.pending_inactive, v3, v4, v5, v6);
        let v9 = 0x1::coin::extract_all<0x1::aptos_coin::AptosCoin>(&mut v0.pending_active);
        0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v0.active, v9);
        if (0x1::features::collect_and_distribute_gas_fees()) {
            let v10 = &mut borrow_global_mut<ValidatorFees>(@0x1).fees_table;
            if (0x1::table::contains<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(v10, arg1)) {
                let v11 = 0x1::table::remove<address, 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>>(v10, arg1);
                0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v0.active, v11);
            };
        };
        if (0x1::timestamp::now_seconds() >= v0.locked_until_secs) {
            let v12 = 0x1::coin::extract_all<0x1::aptos_coin::AptosCoin>(&mut v0.pending_inactive);
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v0.inactive, v12);
        };
        let v13 = DistributeRewardsEvent{
            pool_address   : arg1, 
            rewards_amount : v7 + v8,
        };
        0x1::event::emit_event<DistributeRewardsEvent>(&mut v0.distribute_rewards_events, v13);
    }
    
    fun update_voting_power_increase(arg0: u64) acquires ValidatorSet {
        let v0 = borrow_global_mut<ValidatorSet>(@0x1);
        let v1 = 0x1::staking_config::get();
        v0.total_joining_power = v0.total_joining_power + (arg0 as u128);
        if (v0.total_voting_power > 0) {
            let v2 = v0.total_voting_power * (0x1::staking_config::get_voting_power_increase_limit(&v1) as u128) / 100;
            assert!(v0.total_joining_power <= v2, 0x1::error::invalid_argument(13));
        };
    }
    
    public fun withdraw_with_cap(arg0: &OwnerCapability, arg1: u64) : 0x1::coin::Coin<0x1::aptos_coin::AptosCoin> acquires StakePool, ValidatorSet {
        let v0 = arg0.pool_address;
        assert_stake_pool_exists(v0);
        let v1 = borrow_global_mut<StakePool>(v0);
        let v2 = get_validator_state(v0);
        if (v2 == 4 && 0x1::timestamp::now_seconds() >= v1.locked_until_secs) {
            let v3 = 0x1::coin::extract_all<0x1::aptos_coin::AptosCoin>(&mut v1.pending_inactive);
            0x1::coin::merge<0x1::aptos_coin::AptosCoin>(&mut v1.inactive, v3);
        };
        let v4 = 0x1::math64::min(arg1, 0x1::coin::value<0x1::aptos_coin::AptosCoin>(&v1.inactive));
        if (v4 == 0) {
            return 0x1::coin::zero<0x1::aptos_coin::AptosCoin>()
        };
        let v5 = WithdrawStakeEvent{
            pool_address     : v0, 
            amount_withdrawn : v4,
        };
        0x1::event::emit_event<WithdrawStakeEvent>(&mut v1.withdraw_stake_events, v5);
        0x1::coin::extract<0x1::aptos_coin::AptosCoin>(&mut v1.inactive, v4)
    }
    
    // decompiled from Move bytecode v6
}
