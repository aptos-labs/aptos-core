module 0x1::aptos_coin {
    struct AptosCoin has key {
        dummy_field: bool,
    }
    
    struct DelegatedMintCapability has store {
        to: address,
    }
    
    struct Delegations has key {
        inner: vector<DelegatedMintCapability>,
    }
    
    struct MintCapStore has key {
        mint_cap: 0x1::coin::MintCapability<AptosCoin>,
    }
    
    public(friend) fun destroy_mint_cap(arg0: &signer) acquires MintCapStore {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let MintCapStore { mint_cap: v0 } = move_from<MintCapStore>(@0x1);
        0x1::coin::destroy_mint_cap<AptosCoin>(v0);
    }
    
    public entry fun mint(arg0: &signer, arg1: address, arg2: u64) acquires MintCapStore {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists<MintCapStore>(v0), 0x1::error::not_found(1));
        let v1 = 0x1::coin::mint<AptosCoin>(arg2, &borrow_global<MintCapStore>(v0).mint_cap);
        0x1::coin::deposit<AptosCoin>(arg1, v1);
    }
    
    public entry fun claim_mint_capability(arg0: &signer) acquires Delegations, MintCapStore {
        let v0 = find_delegation(0x1::signer::address_of(arg0));
        assert!(0x1::option::is_some<u64>(&v0), 3);
        let v1 = &mut borrow_global_mut<Delegations>(@0x3000).inner;
        let DelegatedMintCapability {  } = 0x1::vector::swap_remove<DelegatedMintCapability>(v1, *0x1::option::borrow<u64>(&v0));
        let v2 = MintCapStore{mint_cap: borrow_global<MintCapStore>(@0x3000).mint_cap};
        move_to<MintCapStore>(arg0, v2);
    }
    
    public(friend) fun configure_accounts_for_test(arg0: &signer, arg1: &signer, arg2: 0x1::coin::MintCapability<AptosCoin>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::coin::register<AptosCoin>(arg1);
        let v0 = 0x1::coin::mint<AptosCoin>(18446744073709551615, &arg2);
        0x1::coin::deposit<AptosCoin>(0x1::signer::address_of(arg1), v0);
        let v1 = MintCapStore{mint_cap: arg2};
        move_to<MintCapStore>(arg1, v1);
        let v2 = Delegations{inner: 0x1::vector::empty<DelegatedMintCapability>()};
        move_to<Delegations>(arg1, v2);
    }
    
    public entry fun delegate_mint_capability(arg0: signer, arg1: address) acquires Delegations {
        0x1::system_addresses::assert_core_resource(&arg0);
        let v0 = &mut borrow_global_mut<Delegations>(@0x3000).inner;
        let v1 = v0;
        let v2 = 0;
        while (v2 < 0x1::vector::length<DelegatedMintCapability>(v1)) {
            let v3 = 0x1::vector::borrow<DelegatedMintCapability>(v1, v2).to != arg1;
            assert!(v3, 0x1::error::invalid_argument(2));
            v2 = v2 + 1;
        };
        let v4 = DelegatedMintCapability{to: arg1};
        0x1::vector::push_back<DelegatedMintCapability>(v0, v4);
    }
    
    fun find_delegation(arg0: address) : 0x1::option::Option<u64> acquires Delegations {
        let v0 = &borrow_global<Delegations>(@0x3000).inner;
        let v1 = 0;
        let v2 = 0x1::option::none<u64>();
        while (v1 < 0x1::vector::length<DelegatedMintCapability>(v0)) {
            if (0x1::vector::borrow<DelegatedMintCapability>(v0, v1).to == arg0) {
                v2 = 0x1::option::some<u64>(v1);
                break
            };
            v1 = v1 + 1;
        };
        v2
    }
    
    public fun has_mint_capability(arg0: &signer) : bool {
        exists<MintCapStore>(0x1::signer::address_of(arg0))
    }
    
    public(friend) fun initialize(arg0: &signer) : (0x1::coin::BurnCapability<AptosCoin>, 0x1::coin::MintCapability<AptosCoin>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = 0x1::string::utf8(b"Aptos Coin");
        let v1 = 0x1::string::utf8(b"APT");
        let (v2, v3, v4) = 0x1::coin::initialize_with_parallelizable_supply<AptosCoin>(arg0, v0, v1, 8, true);
        let v5 = MintCapStore{mint_cap: v4};
        move_to<MintCapStore>(arg0, v5);
        0x1::coin::destroy_freeze_cap<AptosCoin>(v3);
        (v2, v4)
    }
    
    // decompiled from Move bytecode v6
}
