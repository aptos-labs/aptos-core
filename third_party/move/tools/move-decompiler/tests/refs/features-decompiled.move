module 0x1::features {
    struct Features has key {
        features: vector<u8>,
    }
    
    fun contains(arg0: &vector<u8>, arg1: u64) : bool {
        let v0 = arg1 / 8;
        let v1 = v0 < 0x1::vector::length<u8>(arg0);
        v1 && *0x1::vector::borrow<u8>(arg0, v0) & 1 << ((arg1 % 8) as u8) != 0
    }
    
    public fun aggregator_snapshots_enabled() : bool acquires Features {
        is_enabled(30)
    }
    
    public fun aggregator_v2_api_enabled() : bool acquires Features {
        is_enabled(30)
    }
    
    public fun allow_vm_binary_format_v6() : bool acquires Features {
        is_enabled(5)
    }
    
    public fun aptos_stdlib_chain_id_enabled() : bool acquires Features {
        is_enabled(4)
    }
    
    public fun auids_enabled() : bool acquires Features {
        is_enabled(23)
    }
    
    public fun blake2b_256_enabled() : bool acquires Features {
        is_enabled(8)
    }
    
    public fun bls12_381_structures_enabled() : bool acquires Features {
        is_enabled(13)
    }
    
    public fun bn254_structures_enabled() : bool acquires Features {
        is_enabled(43)
    }
    
    public fun bulletproofs_enabled() : bool acquires Features {
        is_enabled(24)
    }
    
    public fun change_feature_flags(arg0: &signer, arg1: vector<u64>, arg2: vector<u64>) acquires Features {
        assert!(0x1::signer::address_of(arg0) == @0x1, 0x1::error::permission_denied(1));
        if (!exists<Features>(@0x1)) {
            let v0 = Features{features: b""};
            move_to<Features>(arg0, v0);
        };
        let v1 = &mut borrow_global_mut<Features>(@0x1).features;
        let v2 = &arg1;
        let v3 = 0;
        while (v3 < 0x1::vector::length<u64>(v2)) {
            set(v1, *0x1::vector::borrow<u64>(v2, v3), true);
            v3 = v3 + 1;
        };
        let v4 = &arg2;
        let v5 = 0;
        while (v5 < 0x1::vector::length<u64>(v4)) {
            set(v1, *0x1::vector::borrow<u64>(v4, v5), false);
            v5 = v5 + 1;
        };
    }
    
    public fun code_dependency_check_enabled() : bool acquires Features {
        is_enabled(1)
    }
    
    public fun collect_and_distribute_gas_fees() : bool acquires Features {
        is_enabled(6)
    }
    
    public fun commission_change_delegation_pool_enabled() : bool acquires Features {
        is_enabled(42)
    }
    
    public fun concurrent_assets_enabled() : bool acquires Features {
        is_enabled(37) && aggregator_v2_api_enabled()
    }
    
    public fun cryptography_algebra_enabled() : bool acquires Features {
        is_enabled(12)
    }
    
    public fun delegation_pool_partial_governance_voting_enabled() : bool acquires Features {
        is_enabled(21)
    }
    
    public fun delegation_pools_enabled() : bool acquires Features {
        is_enabled(11)
    }
    
    public fun fee_payer_enabled() : bool acquires Features {
        is_enabled(22)
    }
    
    public fun get_aggregator_snapshots_feature() : u64 {
        30
    }
    
    public fun get_aggregator_v2_api_feature() : u64 {
        30
    }
    
    public fun get_aptos_stdlib_chain_id_feature() : u64 {
        4
    }
    
    public fun get_auids() : u64 {
        23
    }
    
    public fun get_blake2b_256_feature() : u64 {
        8
    }
    
    public fun get_bls12_381_strutures_feature() : u64 {
        13
    }
    
    public fun get_bn254_strutures_feature() : u64 {
        43
    }
    
    public fun get_bulletproofs_feature() : u64 {
        24
    }
    
    public fun get_collect_and_distribute_gas_fees_feature() : u64 {
        6
    }
    
    public fun get_commission_change_delegation_pool_feature() : u64 {
        42
    }
    
    public fun get_concurrent_assets_feature() : u64 {
        37
    }
    
    public fun get_cryptography_algebra_natives_feature() : u64 {
        12
    }
    
    public fun get_delegation_pool_partial_governance_voting() : u64 {
        21
    }
    
    public fun get_delegation_pools_feature() : u64 {
        11
    }
    
    public fun get_module_event_feature() : u64 {
        26
    }
    
    public fun get_multisig_accounts_feature() : u64 {
        10
    }
    
    public fun get_operator_beneficiary_change_feature() : u64 {
        39
    }
    
    public fun get_partial_governance_voting() : u64 {
        17
    }
    
    public fun get_periodical_reward_rate_decrease_feature() : u64 {
        16
    }
    
    public fun get_resource_groups_feature() : u64 {
        9
    }
    
    public fun get_sha_512_and_ripemd_160_feature() : u64 {
        3
    }
    
    public fun get_signer_native_format_fix_feature() : u64 {
        25
    }
    
    public fun get_sponsored_automatic_account_creation() : u64 {
        34
    }
    
    public fun get_vm_binary_format_v6() : u64 {
        5
    }
    
    public fun get_zkid_feature() : u64 {
        46
    }
    
    public fun get_zkid_zkless_feature() : u64 {
        47
    }
    
    public fun is_enabled(arg0: u64) : bool acquires Features {
        exists<Features>(@0x1) && contains(&borrow_global<Features>(@0x1).features, arg0)
    }
    
    public fun module_event_enabled() : bool acquires Features {
        is_enabled(26)
    }
    
    public fun multi_ed25519_pk_validate_v2_enabled() : bool acquires Features {
        is_enabled(7)
    }
    
    public fun multi_ed25519_pk_validate_v2_feature() : u64 {
        7
    }
    
    public fun multisig_accounts_enabled() : bool acquires Features {
        is_enabled(10)
    }
    
    public fun operator_beneficiary_change_enabled() : bool acquires Features {
        is_enabled(39)
    }
    
    public fun partial_governance_voting_enabled() : bool acquires Features {
        is_enabled(17)
    }
    
    public fun periodical_reward_rate_decrease_enabled() : bool acquires Features {
        is_enabled(16)
    }
    
    public fun resource_groups_enabled() : bool acquires Features {
        is_enabled(9)
    }
    
    fun set(arg0: &mut vector<u8>, arg1: u64, arg2: bool) {
        let v0 = arg1 / 8;
        while (0x1::vector::length<u8>(arg0) <= v0) {
            0x1::vector::push_back<u8>(arg0, 0);
        };
        let v1 = 0x1::vector::borrow_mut<u8>(arg0, v0);
        if (arg2) {
            *v1 = *v1 | 1 << ((arg1 % 8) as u8);
        } else {
            *v1 = *v1 & (255 ^ 1 << ((arg1 % 8) as u8));
        };
    }
    
    public fun sha_512_and_ripemd_160_enabled() : bool acquires Features {
        is_enabled(3)
    }
    
    public fun signer_native_format_fix_enabled() : bool acquires Features {
        is_enabled(25)
    }
    
    public fun sponsored_automatic_account_creation_enabled() : bool acquires Features {
        is_enabled(34)
    }
    
    public fun treat_friend_as_private() : bool acquires Features {
        is_enabled(2)
    }
    
    public fun zkid_feature_enabled() : bool acquires Features {
        is_enabled(46)
    }
    
    public fun zkid_zkless_feature_enabled() : bool acquires Features {
        is_enabled(47)
    }
    
    // decompiled from Move bytecode v6
}
