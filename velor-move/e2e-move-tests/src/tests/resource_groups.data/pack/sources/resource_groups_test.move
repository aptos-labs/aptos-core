module 0x1::resource_groups_test {
    use std::string::{Self, String};
    use velor_framework::account;
    use std::signer;

    const ENOT_EQUAL: u64 = 17;
    const EINVALID_ARG: u64 = 18;
    const ERESOURCE_DOESNT_EXIST: u64 = 19;

    #[resource_group(scope = global)]
    struct MyGroup {}

    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource1 has key, drop {
        name: String,
        value: u32,
    }

    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource2 has key, drop {
        name: String,
        value: u32,
    }

    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource3 has key, drop {
        name: String,
        value: u32,
    }

    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource4 has key, drop {
        name: String,
        value: u32,
    }

    struct MainResource has key {
        signer_cap: account::SignerCapability
    }

    public entry fun init_signer(main_account: &signer, seed: vector<u8>) {
        let (_resource_account_signer, signer_cap) = account::create_resource_account(main_account, seed);
        let main_resource = MainResource {
            signer_cap,
        };
        move_to<MainResource>(main_account, main_resource);
    }

    public entry fun set_resource(main_account: address, index: u32, name: String, value: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner = account::create_signer_with_capability(&main_resource.signer_cap);
        let owner_address = signer::address_of(&owner);

        if (index == 1) {
            if (exists<MyResource1>(owner_address)) {
                let resource = borrow_global_mut<MyResource1>(owner_address);
                resource.name = name;
                resource.value = value;
            } else {
                let resource = MyResource1 {
                    name,
                    value,
                };
                move_to<MyResource1>(&owner, resource);
            }
        } else if (index == 2) {
            if (exists<MyResource2>(owner_address)) {
                let resource = borrow_global_mut<MyResource2>(owner_address);
                resource.name = name;
                resource.value = value;
            } else {
                let resource = MyResource2 {
                    name,
                    value,
                };
                move_to<MyResource2>(&owner, resource);
            }
        } else if (index == 3) {
            if (exists<MyResource3>(owner_address)) {
                let resource = borrow_global_mut<MyResource3>(owner_address);
                resource.name = name;
                resource.value = value;
            } else {
                let resource = MyResource3 {
                    name,
                    value,
                };
                move_to<MyResource3>(&owner, resource);
            }
        } else if (index == 4) {
            if (exists<MyResource4>(owner_address)) {
                let resource = borrow_global_mut<MyResource4>(owner_address);
                resource.name = name;
                resource.value = value;
            } else {
                let resource = MyResource4 {
                    name,
                    value,
                };
                move_to<MyResource4>(&owner, resource);
            }
        } else {
            assert!(false, EINVALID_ARG);
        }
    }

    public entry fun check(main_account: address, index: u32, name: String, value: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner_address = account::get_signer_capability_address(&main_resource.signer_cap);

        if (index == 1) {
            if (exists<MyResource1>(owner_address)) {
                let resource = borrow_global<MyResource1>(owner_address);
                assert!(resource.name == name, ENOT_EQUAL);
                assert!(resource.value == value, ENOT_EQUAL);
            } else {
                assert!(false, ERESOURCE_DOESNT_EXIST);
            }
        } else if (index == 2) {
            if (exists<MyResource2>(owner_address)) {
                let resource = borrow_global<MyResource2>(owner_address);
                assert!(resource.name == name, ENOT_EQUAL);
                assert!(resource.value == value, ENOT_EQUAL);
            } else {
                assert!(false, ERESOURCE_DOESNT_EXIST);
            }
        } else if (index == 3) {
            if (exists<MyResource3>(owner_address)) {
                let resource = borrow_global<MyResource3>(owner_address);
                assert!(resource.name == name, ENOT_EQUAL);
                assert!(resource.value == value, ENOT_EQUAL);
            } else {
                assert!(false, ERESOURCE_DOESNT_EXIST);
            }
        } else if (index == 4) {
            if (exists<MyResource4>(owner_address)) {
                let resource = borrow_global<MyResource4>(owner_address);
                assert!(resource.name == name, ENOT_EQUAL);
                assert!(resource.value == value, ENOT_EQUAL);
            } else {
                assert!(false, ERESOURCE_DOESNT_EXIST);
            }
        } else {
            assert!(false, EINVALID_ARG);
        }
    }

    public entry fun unset_resource(main_account: address, index: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner_address = account::get_signer_capability_address(&main_resource.signer_cap);
        // TODO: Is this how we unset a resource?
        if (index == 1) {
            if (exists<MyResource1>(owner_address)) {
                move_from<MyResource1>(owner_address);
            }
        } else if (index == 2) {
            if (exists<MyResource2>(owner_address)) {
                move_from<MyResource2>(owner_address);
            }
        } else if (index == 3) {
            if (exists<MyResource3>(owner_address)) {
                move_from<MyResource3>(owner_address);
            }
        } else if (index == 4) {
            if (exists<MyResource4>(owner_address)) {
                move_from<MyResource4>(owner_address);
            }
        } else {
            assert!(false, EINVALID_ARG);
        }
    }

    public entry fun read_or_init(main_account: address, index: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner = account::create_signer_with_capability(&main_resource.signer_cap);
        let owner_address = signer::address_of(&owner);

        if (index == 1) {
            if (exists<MyResource1>(owner_address)) {
                let _resource = borrow_global_mut<MyResource1>(owner_address);
            } else {
                let resource = MyResource1 {
                    name: string::utf8(b"init_name"),
                    value: 5,
                };
                move_to<MyResource1>(&owner, resource);
            }
        } else if (index == 2) {
            if (exists<MyResource2>(owner_address)) {
                let _resource = borrow_global_mut<MyResource2>(owner_address);
            } else {
                let resource = MyResource2 {
                    name: string::utf8(b"init_name"),
                    value: 5,
                };
                move_to<MyResource2>(&owner, resource);
            }
        } else if (index == 3) {
            if (exists<MyResource3>(owner_address)) {
                let _resource = borrow_global_mut<MyResource3>(owner_address);
            } else {
                let resource = MyResource3 {
                    name: string::utf8(b"init_name"),
                    value: 5,
                };
                move_to<MyResource3>(&owner, resource);
            }
        } else if (index == 4) {
            if (exists<MyResource4>(owner_address)) {
                let _resource = borrow_global_mut<MyResource4>(owner_address);
            } else {
                let resource = MyResource4 {
                    name: string::utf8(b"init_name"),
                    value: 5,
                };
                move_to<MyResource4>(&owner, resource);
            }
        } else {
            assert!(false, EINVALID_ARG);
        }
    }

    public entry fun set_3_group_members(main_account: address, index1: u32, index2: u32, index3: u32, name: String, value: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        set_resource(main_account, index1, name, value);
        set_resource(main_account, index2, name, value);
        set_resource(main_account, index3, name, value);
    }

    public entry fun set_resource_and_read(main_account: address, set_index: u32, read_index: u32, name: String, value: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        set_resource(main_account, set_index, name, value);
        read_or_init(main_account, read_index);
    }

    public entry fun set_and_check(main_account: address, set_index: u32, check_index: u32, name1: String, value1: u32, name2: String, value2: u32) acquires MainResource, MyResource1, MyResource2, MyResource3, MyResource4 {
        set_resource(main_account, set_index, name1, value1);
        check(main_account, check_index, name2, value2);
    }
}
