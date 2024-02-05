module 0x1::resource_groups_test {
    use aptos_std::table::{Self, Table};
    use std::vector;
    use std::string::{Self, String};
    use aptos_framework::account;
    use std::signer;

    const USE_RESOURCE_TYPE: u32 = 0;
    const USE_TABLE_TYPE: u32 = 1;
    const USE_RESOURCE_GROUP_TYPE: u32 = 2;

    /// When checking the value of aggregator fails.
    const ENOT_EQUAL: u64 = 17;

    const EINVALID_ARG: u64 = 18;

    const ERESOURCE_DOESNT_EXIST: u64 = 19;
    const ETABLE_DOESNT_EXIST: u64 = 20;
    const ERESOURCE_GROUP_DOESNT_EXIST: u64 = 21;
    const EINDEX_DOESNT_EXIST: u64 = 22;
    const EOPTION_DOESNT_EXIST: u64 = 23;

    #[resource_group(scope = global)]
    struct MyGroup {}

    // TODO: I choose all these resources to have different structure to have a more comprehensive testing.
    // But this also means, I needed to have individual set functions for each resource. Is that okay?
    // Or should I have same structure for all the resources?
    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource1 has key, drop {
        name: String,
        value: u32,
    }

    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource2 has key, drop {
        data: u32,
    }

    #[resource_group_member(group = 0x1::resource_groups_test::MyGroup)]
    struct MyResource3 has key, drop {
        data: vector<u32>,
        padding: vector<u32>,
    }

    // #[resource_group_member(group = 0x1::resource_group_test::MyGroup)]
    // struct MyResource4 has key, drop {
    //     data: Table<u32, u64>,
    // }

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

    public entry fun set_resource1(_delegated_signer: &signer, main_account: address, name: String, value: u32) acquires MainResource, MyResource1 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner = account::create_signer_with_capability(&main_resource.signer_cap);
        let owner_address = signer::address_of(&owner);

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
    }

    public entry fun set_resource2(_delegated_signer: &signer, main_account: address, data: u32) acquires MainResource, MyResource2 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner = account::create_signer_with_capability(&main_resource.signer_cap);
        let owner_address = signer::address_of(&owner);

        if (exists<MyResource2>(owner_address)) {
            let resource = borrow_global_mut<MyResource2>(owner_address);
            resource.data = data;
        } else {
            let resource = MyResource2 {
                data,
            };
            move_to<MyResource2>(&owner, resource);
        }
    }

    public entry fun set_resource3(_delegated_signer: &signer, main_account: address, data: vector<u32>, padding: vector<u32>) acquires MainResource, MyResource3 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner = account::create_signer_with_capability(&main_resource.signer_cap);
        let owner_address = signer::address_of(&owner);

        if (exists<MyResource3>(owner_address)) {
            let resource = borrow_global_mut<MyResource3>(owner_address);
            resource.data = data;
            resource.padding = padding;
        } else {
            let resource = MyResource3 {
                data,
                padding,
            };
            move_to<MyResource3>(&owner, resource);
        }
    }

    // public entry fun set_resource4(_delegated_signer: &signer, main_account: address, key: u32, value: u64) acquires MyResource4 {
    //     let main_resource = borrow_global_mut<MainResource>(main_account);
    //     let owner = account::create_signer_with_capability(&main_resource.signer_cap);
    //     let owner_address = signer::address_of(&owner);

    //     if (exists<MyResource4>(owner_address)) {
    //         let resource = borrow_global_mut<MyResource4>(owner_address);
    //         table::upsert(&mut resource.data, key, value);
    //     } else {
    //         let resource = MyResource4 {
    //             data,
    //         };
    //         move_to<MyResource4>(&owner, resource);
    //     }
    // }

    public entry fun unset_resource(_delegated_signer: &signer,  main_account: address, index: u32) acquires MainResource, MyResource1, MyResource2, MyResource3 {
        let main_resource = borrow_global_mut<MainResource>(main_account);
        let owner_address = account::get_signer_capability_address(&main_resource.signer_cap);
        // TODO: Is this how we unset a resource?
        if (index == 1) {
            move_from<MyResource1>(owner_address);
        } else if (index == 2) {
            move_from<MyResource2>(owner_address);
        } else if (index == 3) {
            move_from<MyResource3>(owner_address);
        // } else if (index == 4) {
        //     move_from<MyResource4>(owner_address);
        } else {
            assert!(false, EINVALID_ARG);
        }
    }

    public entry fun read_or_init(_delegated_signer: &signer, main_account: address, index: u32) acquires MainResource, MyResource1, MyResource2, MyResource3 {
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
                    data: 10,
                };
                move_to<MyResource2>(&owner, resource);
            }
        } else if (index == 3) {
            if (exists<MyResource3>(owner_address)) {
                let _resource = borrow_global_mut<MyResource3>(owner_address);
            } else {
                let resource = MyResource3 {
                    data: vector[1, 2, 3],
                    padding: vector[5, 6],
                };
                move_to<MyResource3>(&owner, resource);
            }
        // } else if (index == 4) {
        //     if (exists<MyResource4>(owner_address)) {
        //         let resource = borrow_global_mut<MyResource4>(owner_address);
        //     } else {
        //         let resource = MyResource4 {
        //             data: table::new(),
        //         };
        //         move_to<MyResource4>(&owner, resource);
        //     }
        } else {
            assert!(false, EINVALID_ARG);
        }
    }

    // TODO: Are set and set_3 functions necessary?

    // public entry fun set(owner: &signer, index: u64, name: String, value: u32) acquires MyResource1, MyResource2, MyResource3, MyResource4 {
    //     if (index == 1) {
    //         set_resource1(owner, name, value as u64);
    //     } else if (index == 2) {
    //         set_resource2(owner, value);
    //     } else if (index == 3) {
    //         set_resource3(owner, vector::from_values([value, 3, 3]), vector::from_values([value, 3, 3]));
    //     } else if (index == 4) {
    //         set_resource4(owner, value, 4);
    //     } else {
    //         assert(false, EINVALID_ARG);
    //     }
    // }

    // public entry fun set_3(owner: &signer, index1: u64, index2: u64, index3: u64, name: String, value: u32) acquires MyResource1, MyResource2, MyResource3, MyResource4 {
    //     set(owner, index1, name, value);
    //     set(owner, index2, name, value);
    //     set(owner, index3, name, value);
    // }

    // public entry fun set_resource1_and_read(_delegated_signer: &signer, owner: &signer, set_index: u64, read_index: u64, name: String, value: u32) acquires MyResource1, MyResource2, MyResource3, MyResource4 {
    //     set_resource1(owner, name, value);
    //     read_or_init(owner, read_index);
    // }

    // public entry fun set_resource2_and_read(_delegated_signer: &signer, owner: &signer, set_index: u64, read_index: u64, value: u32) acquires MyResource1, MyResource2, MyResource3, MyResource4 {
    //     set_resource2(owner, set_index, value);
    //     read_or_init(owner, read_index);
    // }

    // public entry fun set_resource3_and_read(_delegated_signer: &signer, owner: &signer, set_index: u64, read_index: u64, data: vector<u32>, padding: vector<u32>) acquires MyResource1, MyResource2, MyResource3, MyResource4 {
    //     set_resource3(owner, set_index, data, padding);
    //     read_or_init(owner, read_index);
    // }

    // public entry fun set_resource4_and_read(_delegated_signer: &signer, owner: &signer, set_index: u64, read_index: u64, key: u32, value: u64) acquires MyResource1, MyResource2, MyResource3, MyResource4 {
    //     set_resource4(owner, set_index, key, value);
    //     read_or_init(owner, read_index);
    // }
}
