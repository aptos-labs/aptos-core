module 0x1::managed_coin {
    struct Capabilities<phantom T0> has key {
        burn_cap: 0x1::coin::BurnCapability<T0>,
        freeze_cap: 0x1::coin::FreezeCapability<T0>,
        mint_cap: 0x1::coin::MintCapability<T0>,
    }
    
    public entry fun burn<T0>(arg0: &signer, arg1: u64) acquires Capabilities {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists<Capabilities<T0>>(v0), 0x1::error::not_found(1));
        let v1 = &borrow_global<Capabilities<T0>>(v0).burn_cap;
        0x1::coin::burn<T0>(0x1::coin::withdraw<T0>(arg0, arg1), v1);
    }
    
    public entry fun initialize<T0>(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>, arg3: u8, arg4: bool) {
        let (v0, v1, v2) = 0x1::coin::initialize<T0>(arg0, 0x1::string::utf8(arg1), 0x1::string::utf8(arg2), arg3, arg4);
        let v3 = Capabilities<T0>{
            burn_cap   : v0, 
            freeze_cap : v1, 
            mint_cap   : v2,
        };
        move_to<Capabilities<T0>>(arg0, v3);
    }
    
    public entry fun mint<T0>(arg0: &signer, arg1: address, arg2: u64) acquires Capabilities {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists<Capabilities<T0>>(v0), 0x1::error::not_found(1));
        let v1 = 0x1::coin::mint<T0>(arg2, &borrow_global<Capabilities<T0>>(v0).mint_cap);
        0x1::coin::deposit<T0>(arg1, v1);
    }
    
    public entry fun register<T0>(arg0: &signer) {
        0x1::coin::register<T0>(arg0);
    }
    
    // decompiled from Move bytecode v6
}
