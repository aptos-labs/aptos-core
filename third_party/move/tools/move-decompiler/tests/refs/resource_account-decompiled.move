module 0x1::resource_account {
    struct Container has key {
        store: 0x1::simple_map::SimpleMap<address, 0x1::account::SignerCapability>,
    }
    
    public entry fun create_resource_account(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>) acquires Container {
        let (v0, v1) = 0x1::account::create_resource_account(arg0, arg1);
        rotate_account_authentication_key_and_store_capability(arg0, v0, v1, arg2);
    }
    
    public entry fun create_resource_account_and_fund(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>, arg3: u64) acquires Container {
        let (v0, v1) = 0x1::account::create_resource_account(arg0, arg1);
        let v2 = v0;
        0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v2);
        0x1::coin::transfer<0x1::aptos_coin::AptosCoin>(arg0, 0x1::signer::address_of(&v2), arg3);
        rotate_account_authentication_key_and_store_capability(arg0, v2, v1, arg2);
    }
    
    public entry fun create_resource_account_and_publish_package(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>, arg3: vector<vector<u8>>) acquires Container {
        let (v0, v1) = 0x1::account::create_resource_account(arg0, arg1);
        let v2 = v0;
        0x1::code::publish_package_txn(&v2, arg2, arg3);
        rotate_account_authentication_key_and_store_capability(arg0, v2, v1, x"0000000000000000000000000000000000000000000000000000000000000000");
    }
    
    public fun retrieve_resource_account_cap(arg0: &signer, arg1: address) : 0x1::account::SignerCapability acquires Container {
        assert!(exists<Container>(arg1), 0x1::error::not_found(1));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = borrow_global_mut<Container>(arg1);
        let v2 = 0x1::simple_map::contains_key<address, 0x1::account::SignerCapability>(&v1.store, &v0);
        assert!(v2, 0x1::error::invalid_argument(2));
        let (_, v4) = 0x1::simple_map::remove<address, 0x1::account::SignerCapability>(&mut v1.store, &v0);
        if (0x1::simple_map::length<address, 0x1::account::SignerCapability>(&v1.store) == 0) {
            let Container { store: v5 } = move_from<Container>(arg1);
            0x1::simple_map::destroy_empty<address, 0x1::account::SignerCapability>(v5);
        };
        0x1::account::rotate_authentication_key_internal(arg0, x"0000000000000000000000000000000000000000000000000000000000000000");
        v4
    }
    
    fun rotate_account_authentication_key_and_store_capability(arg0: &signer, arg1: signer, arg2: 0x1::account::SignerCapability, arg3: vector<u8>) acquires Container {
        let v0 = 0x1::signer::address_of(arg0);
        if (!exists<Container>(v0)) {
            let v1 = Container{store: 0x1::simple_map::create<address, 0x1::account::SignerCapability>()};
            move_to<Container>(arg0, v1);
        };
        let v2 = 0x1::signer::address_of(&arg1);
        let v3 = &mut borrow_global_mut<Container>(v0).store;
        0x1::simple_map::add<address, 0x1::account::SignerCapability>(v3, v2, arg2);
        let v4 = if (0x1::vector::is_empty<u8>(&arg3)) {
            0x1::account::get_authentication_key(v0)
        } else {
            arg3
        };
        0x1::account::rotate_authentication_key_internal(&arg1, v4);
    }
    
    // decompiled from Move bytecode v6
}
