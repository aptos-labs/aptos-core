module 0x1::primary_fungible_store {
    struct DeriveRefPod has key {
        metadata_derive_ref: 0x1::object::DeriveRef,
    }
    
    public fun balance<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : u64 {
        if (primary_store_exists<T0>(arg0, arg1)) {
            0x1::fungible_asset::balance<0x1::fungible_asset::FungibleStore>(primary_store<T0>(arg0, arg1))
        } else {
            0
        }
    }
    
    public fun deposit(arg0: address, arg1: 0x1::fungible_asset::FungibleAsset) acquires DeriveRefPod {
        let v0 = 0x1::fungible_asset::asset_metadata(&arg1);
        let v1 = ensure_primary_store_exists<0x1::fungible_asset::Metadata>(arg0, v0);
        0x1::fungible_asset::deposit<0x1::fungible_asset::FungibleStore>(v1, arg1);
    }
    
    public fun deposit_with_ref(arg0: &0x1::fungible_asset::TransferRef, arg1: address, arg2: 0x1::fungible_asset::FungibleAsset) acquires DeriveRefPod {
        let v0 = 0x1::fungible_asset::transfer_ref_metadata(arg0);
        let v1 = ensure_primary_store_exists<0x1::fungible_asset::Metadata>(arg1, v0);
        0x1::fungible_asset::deposit_with_ref<0x1::fungible_asset::FungibleStore>(arg0, v1, arg2);
    }
    
    public fun is_frozen<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : bool {
        let v0 = primary_store_exists<T0>(arg0, arg1);
        v0 && 0x1::fungible_asset::is_frozen<0x1::fungible_asset::FungibleStore>(primary_store<T0>(arg0, arg1))
    }
    
    public fun set_frozen_flag(arg0: &0x1::fungible_asset::TransferRef, arg1: address, arg2: bool) acquires DeriveRefPod {
        let v0 = 0x1::fungible_asset::transfer_ref_metadata(arg0);
        let v1 = ensure_primary_store_exists<0x1::fungible_asset::Metadata>(arg1, v0);
        0x1::fungible_asset::set_frozen_flag<0x1::fungible_asset::FungibleStore>(arg0, v1, arg2);
    }
    
    public entry fun transfer<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: address, arg3: u64) acquires DeriveRefPod {
        let v0 = ensure_primary_store_exists<T0>(0x1::signer::address_of(arg0), arg1);
        may_be_unburn(arg0, v0);
        let v1 = ensure_primary_store_exists<T0>(arg2, arg1);
        0x1::fungible_asset::transfer<0x1::fungible_asset::FungibleStore>(arg0, v0, v1, arg3);
    }
    
    public fun transfer_with_ref(arg0: &0x1::fungible_asset::TransferRef, arg1: address, arg2: address, arg3: u64) acquires DeriveRefPod {
        let v0 = primary_store<0x1::fungible_asset::Metadata>(arg1, 0x1::fungible_asset::transfer_ref_metadata(arg0));
        let v1 = 0x1::fungible_asset::transfer_ref_metadata(arg0);
        let v2 = ensure_primary_store_exists<0x1::fungible_asset::Metadata>(arg2, v1);
        0x1::fungible_asset::transfer_with_ref<0x1::fungible_asset::FungibleStore>(arg0, v0, v2, arg3);
    }
    
    public fun withdraw<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: u64) : 0x1::fungible_asset::FungibleAsset {
        let v0 = primary_store<T0>(0x1::signer::address_of(arg0), arg1);
        may_be_unburn(arg0, v0);
        0x1::fungible_asset::withdraw<0x1::fungible_asset::FungibleStore>(arg0, v0, arg2)
    }
    
    public fun withdraw_with_ref(arg0: &0x1::fungible_asset::TransferRef, arg1: address, arg2: u64) : 0x1::fungible_asset::FungibleAsset {
        let v0 = primary_store<0x1::fungible_asset::Metadata>(arg1, 0x1::fungible_asset::transfer_ref_metadata(arg0));
        0x1::fungible_asset::withdraw_with_ref<0x1::fungible_asset::FungibleStore>(arg0, v0, arg2)
    }
    
    public fun burn(arg0: &0x1::fungible_asset::BurnRef, arg1: address, arg2: u64) {
        let v0 = primary_store<0x1::fungible_asset::Metadata>(arg1, 0x1::fungible_asset::burn_ref_metadata(arg0));
        0x1::fungible_asset::burn_from<0x1::fungible_asset::FungibleStore>(arg0, v0, arg2);
    }
    
    public fun create_primary_store<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : 0x1::object::Object<0x1::fungible_asset::FungibleStore> acquires DeriveRefPod {
        let v0 = 0x1::object::object_address<T0>(&arg1);
        0x1::object::address_to_object<0x1::fungible_asset::Metadata>(v0);
        let v1 = 0x1::object::create_user_derived_object(arg0, &borrow_global<DeriveRefPod>(v0).metadata_derive_ref);
        let v2 = &v1;
        let v3 = 0x1::object::generate_transfer_ref(v2);
        0x1::object::disable_ungated_transfer(&v3);
        0x1::fungible_asset::create_store<T0>(v2, arg1)
    }
    
    public fun create_primary_store_enabled_fungible_asset(arg0: &0x1::object::ConstructorRef, arg1: 0x1::option::Option<u128>, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u8, arg5: 0x1::string::String, arg6: 0x1::string::String) {
        0x1::fungible_asset::add_fungibility(arg0, arg1, arg2, arg3, arg4, arg5, arg6);
        let v0 = 0x1::object::generate_signer(arg0);
        let v1 = DeriveRefPod{metadata_derive_ref: 0x1::object::generate_derive_ref(arg0)};
        move_to<DeriveRefPod>(&v0, v1);
    }
    
    public fun ensure_primary_store_exists<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : 0x1::object::Object<0x1::fungible_asset::FungibleStore> acquires DeriveRefPod {
        if (!primary_store_exists<T0>(arg0, arg1)) {
            create_primary_store<T0>(arg0, arg1)
        } else {
            primary_store<T0>(arg0, arg1)
        }
    }
    
    fun may_be_unburn(arg0: &signer, arg1: 0x1::object::Object<0x1::fungible_asset::FungibleStore>) {
        if (0x1::object::is_burnt<0x1::fungible_asset::FungibleStore>(arg1)) {
            0x1::object::unburn<0x1::fungible_asset::FungibleStore>(arg0, arg1);
        };
    }
    
    public fun mint(arg0: &0x1::fungible_asset::MintRef, arg1: address, arg2: u64) acquires DeriveRefPod {
        let v0 = 0x1::fungible_asset::mint_ref_metadata(arg0);
        let v1 = ensure_primary_store_exists<0x1::fungible_asset::Metadata>(arg1, v0);
        0x1::fungible_asset::mint_to<0x1::fungible_asset::FungibleStore>(arg0, v1, arg2);
    }
    
    public fun primary_store<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : 0x1::object::Object<0x1::fungible_asset::FungibleStore> {
        let v0 = primary_store_address<T0>(arg0, arg1);
        0x1::object::address_to_object<0x1::fungible_asset::FungibleStore>(v0)
    }
    
    public fun primary_store_address<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : address {
        0x1::object::create_user_derived_object_address(arg0, 0x1::object::object_address<T0>(&arg1))
    }
    
    public fun primary_store_exists<T0: key>(arg0: address, arg1: 0x1::object::Object<T0>) : bool {
        0x1::fungible_asset::store_exists(primary_store_address<T0>(arg0, arg1))
    }
    
    // decompiled from Move bytecode v6
}
