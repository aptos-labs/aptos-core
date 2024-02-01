module 0x1::transaction_fee {
    struct AptosCoinCapabilities has key {
        burn_cap: 0x1::coin::BurnCapability<0x1::aptos_coin::AptosCoin>,
    }
    
    struct AptosCoinMintCapability has key {
        mint_cap: 0x1::coin::MintCapability<0x1::aptos_coin::AptosCoin>,
    }
    
    struct CollectedFeesPerBlock has key {
        amount: 0x1::coin::AggregatableCoin<0x1::aptos_coin::AptosCoin>,
        proposer: 0x1::option::Option<address>,
        burn_percentage: u8,
    }
    
    struct FeeStatement has drop, store {
        total_charge_gas_units: u64,
        execution_gas_units: u64,
        io_gas_units: u64,
        storage_fee_octas: u64,
        storage_fee_refund_octas: u64,
    }
    
    fun burn_coin_fraction(arg0: &mut 0x1::coin::Coin<0x1::aptos_coin::AptosCoin>, arg1: u8) acquires AptosCoinCapabilities {
        assert!(arg1 <= 100, 0x1::error::out_of_range(3));
        let v0 = (arg1 as u64) * 0x1::coin::value<0x1::aptos_coin::AptosCoin>(arg0) / 100;
        if (v0 > 0) {
            let v1 = 0x1::coin::extract<0x1::aptos_coin::AptosCoin>(arg0, v0);
            let v2 = &borrow_global<AptosCoinCapabilities>(@0x1).burn_cap;
            0x1::coin::burn<0x1::aptos_coin::AptosCoin>(v1, v2);
        };
    }
    
    public(friend) fun burn_fee(arg0: address, arg1: u64) acquires AptosCoinCapabilities {
        let v0 = &borrow_global<AptosCoinCapabilities>(@0x1).burn_cap;
        0x1::coin::burn_from<0x1::aptos_coin::AptosCoin>(arg0, arg1, v0);
    }
    
    public(friend) fun collect_fee(arg0: address, arg1: u64) acquires CollectedFeesPerBlock {
        let v0 = &mut borrow_global_mut<CollectedFeesPerBlock>(@0x1).amount;
        0x1::coin::collect_into_aggregatable_coin<0x1::aptos_coin::AptosCoin>(arg0, arg1, v0);
    }
    
    fun emit_fee_statement(arg0: FeeStatement) {
        0x1::event::emit<FeeStatement>(arg0);
    }
    
    public fun initialize_fee_collection_and_distribution(arg0: &signer, arg1: u8) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!exists<CollectedFeesPerBlock>(@0x1), 0x1::error::already_exists(1));
        assert!(arg1 <= 100, 0x1::error::out_of_range(3));
        0x1::stake::initialize_validator_fees(arg0);
        let v0 = 0x1::coin::initialize_aggregatable_coin<0x1::aptos_coin::AptosCoin>(arg0);
        let v1 = CollectedFeesPerBlock{
            amount          : v0, 
            proposer        : 0x1::option::none<address>(), 
            burn_percentage : arg1,
        };
        move_to<CollectedFeesPerBlock>(arg0, v1);
    }
    
    public fun initialize_storage_refund(arg0: &signer) {
        abort 0x1::error::not_implemented(4)
    }
    
    fun is_fees_collection_enabled() : bool {
        exists<CollectedFeesPerBlock>(@0x1)
    }
    
    public(friend) fun mint_and_refund(arg0: address, arg1: u64) acquires AptosCoinMintCapability {
        let v0 = borrow_global<AptosCoinMintCapability>(@0x1);
        let v1 = 0x1::coin::mint<0x1::aptos_coin::AptosCoin>(arg1, &v0.mint_cap);
        0x1::coin::force_deposit<0x1::aptos_coin::AptosCoin>(arg0, v1);
    }
    
    public(friend) fun process_collected_fees() acquires AptosCoinCapabilities, CollectedFeesPerBlock {
        if (!is_fees_collection_enabled()) {
            return
        };
        let v0 = borrow_global_mut<CollectedFeesPerBlock>(@0x1);
        if (0x1::coin::is_aggregatable_coin_zero<0x1::aptos_coin::AptosCoin>(&v0.amount)) {
            if (0x1::option::is_some<address>(&v0.proposer)) {
                0x1::option::extract<address>(&mut v0.proposer);
            };
            return
        };
        let v1 = 0x1::coin::drain_aggregatable_coin<0x1::aptos_coin::AptosCoin>(&mut v0.amount);
        if (0x1::option::is_some<address>(&v0.proposer)) {
            let v2 = 0x1::option::extract<address>(&mut v0.proposer);
            if (v2 == @0x3001) {
                burn_coin_fraction(&mut v1, 100);
                0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v1);
                return
            };
            burn_coin_fraction(&mut v1, v0.burn_percentage);
            0x1::stake::add_transaction_fee(v2, v1);
            return
        };
        burn_coin_fraction(&mut v1, 100);
        0x1::coin::destroy_zero<0x1::aptos_coin::AptosCoin>(v1);
    }
    
    public(friend) fun register_proposer_for_fee_collection(arg0: address) acquires CollectedFeesPerBlock {
        if (is_fees_collection_enabled()) {
            let v0 = &mut borrow_global_mut<CollectedFeesPerBlock>(@0x1).proposer;
            0x1::option::swap_or_fill<address>(v0, arg0);
        };
    }
    
    public(friend) fun store_aptos_coin_burn_cap(arg0: &signer, arg1: 0x1::coin::BurnCapability<0x1::aptos_coin::AptosCoin>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = AptosCoinCapabilities{burn_cap: arg1};
        move_to<AptosCoinCapabilities>(arg0, v0);
    }
    
    public(friend) fun store_aptos_coin_mint_cap(arg0: &signer, arg1: 0x1::coin::MintCapability<0x1::aptos_coin::AptosCoin>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = AptosCoinMintCapability{mint_cap: arg1};
        move_to<AptosCoinMintCapability>(arg0, v0);
    }
    
    public fun upgrade_burn_percentage(arg0: &signer, arg1: u8) acquires AptosCoinCapabilities, CollectedFeesPerBlock {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg1 <= 100, 0x1::error::out_of_range(3));
        process_collected_fees();
        if (is_fees_collection_enabled()) {
            borrow_global_mut<CollectedFeesPerBlock>(@0x1).burn_percentage = arg1;
        };
    }
    
    // decompiled from Move bytecode v6
}
