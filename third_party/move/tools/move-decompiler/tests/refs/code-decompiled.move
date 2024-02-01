module 0x1::code {
    struct AllowedDep has drop {
        account: address,
        module_name: 0x1::string::String,
    }
    
    struct ModuleMetadata has drop, store {
        name: 0x1::string::String,
        source: vector<u8>,
        source_map: vector<u8>,
        extension: 0x1::option::Option<0x1::copyable_any::Any>,
    }
    
    struct PackageDep has copy, drop, store {
        account: address,
        package_name: 0x1::string::String,
    }
    
    struct PackageMetadata has drop, store {
        name: 0x1::string::String,
        upgrade_policy: UpgradePolicy,
        upgrade_number: u64,
        source_digest: 0x1::string::String,
        manifest: vector<u8>,
        modules: vector<ModuleMetadata>,
        deps: vector<PackageDep>,
        extension: 0x1::option::Option<0x1::copyable_any::Any>,
    }
    
    struct PackageRegistry has drop, store, key {
        packages: vector<PackageMetadata>,
    }
    
    struct UpgradePolicy has copy, drop, store {
        policy: u8,
    }
    
    public fun can_change_upgrade_policy_to(arg0: UpgradePolicy, arg1: UpgradePolicy) : bool {
        arg0.policy <= arg1.policy
    }
    
    fun check_coexistence(arg0: &PackageMetadata, arg1: &vector<0x1::string::String>) {
        let v0 = &arg0.modules;
        let v1 = 0;
        while (v1 < 0x1::vector::length<ModuleMetadata>(v0)) {
            let v2 = 0x1::vector::borrow<ModuleMetadata>(v0, v1);
            let v3 = 0;
            while (v3 < 0x1::vector::length<0x1::string::String>(arg1)) {
                let v4 = &v2.name != 0x1::vector::borrow<0x1::string::String>(arg1, v3);
                assert!(v4, 0x1::error::already_exists(1));
                v3 = v3 + 1;
            };
            v1 = v1 + 1;
        };
    }
    
    fun check_dependencies(arg0: address, arg1: &PackageMetadata) : vector<AllowedDep> acquires PackageRegistry {
        let v0 = 0x1::vector::empty<AllowedDep>();
        let v1 = &arg1.deps;
        let v2 = 0;
        while (v2 < 0x1::vector::length<PackageDep>(v1)) {
            let v3 = 0x1::vector::borrow<PackageDep>(v1, v2);
            assert!(exists<PackageRegistry>(v3.account), 0x1::error::not_found(5));
            if (is_policy_exempted_address(v3.account)) {
                let v4 = AllowedDep{
                    account     : v3.account, 
                    module_name : 0x1::string::utf8(b""),
                };
                0x1::vector::push_back<AllowedDep>(&mut v0, v4);
            } else {
                let v5 = &borrow_global<PackageRegistry>(v3.account).packages;
                let v6 = false;
                let v7 = 0;
                while (v7 < 0x1::vector::length<PackageMetadata>(v5)) {
                    let v8 = 0x1::vector::borrow<PackageMetadata>(v5, v7);
                    let v9 = if (v8.name == v3.package_name) {
                        assert!(v8.upgrade_policy.policy >= arg1.upgrade_policy.policy, 0x1::error::invalid_argument(6));
                        if (v8.upgrade_policy == upgrade_policy_arbitrary()) {
                            assert!(v3.account == arg0, 0x1::error::invalid_argument(7));
                        };
                        let v10 = 0;
                        while (v10 < 0x1::vector::length<ModuleMetadata>(&v8.modules)) {
                            let v11 = 0x1::vector::borrow<ModuleMetadata>(&v8.modules, v10).name;
                            let v12 = AllowedDep{
                                account     : v3.account, 
                                module_name : v11,
                            };
                            0x1::vector::push_back<AllowedDep>(&mut v0, v12);
                            v10 = v10 + 1;
                        };
                        true
                    } else {
                        false
                    };
                    v6 = v9;
                    if (v9) {
                        break
                    };
                    v7 = v7 + 1;
                };
                assert!(v6, 0x1::error::not_found(5));
            };
            v2 = v2 + 1;
        };
        v0
    }
    
    fun check_upgradability(arg0: &PackageMetadata, arg1: &PackageMetadata, arg2: &vector<0x1::string::String>) {
        let v0 = upgrade_policy_immutable();
        assert!(arg0.upgrade_policy.policy < v0.policy, 0x1::error::invalid_argument(2));
        let v1 = can_change_upgrade_policy_to(arg0.upgrade_policy, arg1.upgrade_policy);
        assert!(v1, 0x1::error::invalid_argument(3));
        let v2 = get_module_names(arg0);
        let v3 = &v2;
        let v4 = 0;
        while (v4 < 0x1::vector::length<0x1::string::String>(v3)) {
            let v5 = 0x1::vector::contains<0x1::string::String>(arg2, 0x1::vector::borrow<0x1::string::String>(v3, v4));
            assert!(v5, 4);
            v4 = v4 + 1;
        };
    }
    
    fun get_module_names(arg0: &PackageMetadata) : vector<0x1::string::String> {
        let v0 = 0x1::vector::empty<0x1::string::String>();
        let v1 = &arg0.modules;
        let v2 = 0;
        while (v2 < 0x1::vector::length<ModuleMetadata>(v1)) {
            let v3 = 0x1::vector::borrow<ModuleMetadata>(v1, v2).name;
            0x1::vector::push_back<0x1::string::String>(&mut v0, v3);
            v2 = v2 + 1;
        };
        v0
    }
    
    fun initialize(arg0: &signer, arg1: &signer, arg2: PackageMetadata) acquires PackageRegistry {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = 0x1::signer::address_of(arg1);
        if (!exists<PackageRegistry>(v0)) {
            let v1 = 0x1::vector::empty<PackageMetadata>();
            0x1::vector::push_back<PackageMetadata>(&mut v1, arg2);
            let v2 = PackageRegistry{packages: v1};
            move_to<PackageRegistry>(arg1, v2);
        } else {
            0x1::vector::push_back<PackageMetadata>(&mut borrow_global_mut<PackageRegistry>(v0).packages, arg2);
        };
    }
    
    fun is_policy_exempted_address(arg0: address) : bool {
        arg0 == @0x1 || arg0 == @0x2 || arg0 == @0x3 || arg0 == @0x4 || arg0 == @0x5 || arg0 == @0x6 || arg0 == @0x7 || arg0 == @0x8 || arg0 == @0x9 || arg0 == @0xa
    }
    
    public fun publish_package(arg0: &signer, arg1: PackageMetadata, arg2: vector<vector<u8>>) acquires PackageRegistry {
        let v0 = upgrade_policy_arbitrary();
        assert!(arg1.upgrade_policy.policy > v0.policy, 0x1::error::invalid_argument(8));
        let v1 = 0x1::signer::address_of(arg0);
        if (!exists<PackageRegistry>(v1)) {
            let v2 = PackageRegistry{packages: 0x1::vector::empty<PackageMetadata>()};
            move_to<PackageRegistry>(arg0, v2);
        };
        let v3 = check_dependencies(v1, &arg1);
        let v4 = get_module_names(&arg1);
        let v5 = &borrow_global<PackageRegistry>(v1).packages;
        let v6 = 0x1::vector::length<PackageMetadata>(v5);
        let v7 = v6;
        let v8 = 0;
        let v9 = 0;
        while (v9 < 0x1::vector::length<PackageMetadata>(v5)) {
            let v10 = 0x1::vector::borrow<PackageMetadata>(v5, v9);
            if (v10.name == arg1.name) {
                v8 = v10.upgrade_number + 1;
                check_upgradability(v10, &arg1, &v4);
                v7 = v9;
            } else {
                check_coexistence(v10, &v4);
            };
            v9 = v9 + 1;
        };
        arg1.upgrade_number = v8;
        let v11 = arg1.upgrade_policy;
        if (v7 < v6) {
            let v12 = 0x1::vector::borrow_mut<PackageMetadata>(&mut borrow_global_mut<PackageRegistry>(v1).packages, v7);
            *v12 = arg1;
        } else {
            0x1::vector::push_back<PackageMetadata>(&mut borrow_global_mut<PackageRegistry>(v1).packages, arg1);
        };
        if (0x1::features::code_dependency_check_enabled()) {
            request_publish_with_allowed_deps(v1, v4, v3, arg2, v11.policy);
        } else {
            request_publish(v1, v4, arg2, v11.policy);
        };
    }
    
    public entry fun publish_package_txn(arg0: &signer, arg1: vector<u8>, arg2: vector<vector<u8>>) acquires PackageRegistry {
        publish_package(arg0, 0x1::util::from_bytes<PackageMetadata>(arg1), arg2);
    }
    
    native fun request_publish(arg0: address, arg1: vector<0x1::string::String>, arg2: vector<vector<u8>>, arg3: u8);
    native fun request_publish_with_allowed_deps(arg0: address, arg1: vector<0x1::string::String>, arg2: vector<AllowedDep>, arg3: vector<vector<u8>>, arg4: u8);
    public fun upgrade_policy_arbitrary() : UpgradePolicy {
        UpgradePolicy{policy: 0}
    }
    
    public fun upgrade_policy_compat() : UpgradePolicy {
        UpgradePolicy{policy: 1}
    }
    
    public fun upgrade_policy_immutable() : UpgradePolicy {
        UpgradePolicy{policy: 2}
    }
    
    // decompiled from Move bytecode v6
}
