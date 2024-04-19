module 0xABCD::resource_groups_example {
    use std::error;
    use std::signer;
    use std::string::{Self, String};

    const EINDEX_TOO_LARGE: u64 = 1;
    const EVALUE_TOO_LARGE: u64 = 2;

    #[resource_group(scope = global)]
    struct ExampleGroup {}

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource0 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource1 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource2 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource3 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource4 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource5 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource6 has key {
        value: u64,
        name: String,
    }

    #[resource_group_member(group = 0xABCD::resource_groups_example::ExampleGroup)]
    struct ExampleResource7 has key {
        value: u64,
        name: String,
    }

    public entry fun set(owner: &signer, index: u64, name: String) acquires ExampleResource0, ExampleResource1, ExampleResource2, ExampleResource3, ExampleResource4, ExampleResource5, ExampleResource6, ExampleResource7 {
        let owner_address = signer::address_of(owner);
        assert!(index < 8, error::invalid_argument(EINDEX_TOO_LARGE));
        if (index == 0) {
            if (exists<ExampleResource0>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource0>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource0 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 1) {
            if (exists<ExampleResource1>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource1>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource1 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 2) {
            if (exists<ExampleResource2>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource2>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource2 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 3) {
            if (exists<ExampleResource3>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource3>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource3 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 4) {
            if (exists<ExampleResource4>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource4>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource4 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 5) {
            if (exists<ExampleResource5>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource5>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource5 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 6) {
            if (exists<ExampleResource6>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource6>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource6 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        } else if (index == 7) {
            if (exists<ExampleResource7>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource7>(owner_address);
                resource.name = name;
            } else {
                let resource = ExampleResource7 {
                    value: 0,
                    name,
                };
                move_to(owner, resource);
            }
        };
    }

    public entry fun read_or_init(owner: &signer, index: u64) acquires ExampleResource0, ExampleResource1, ExampleResource2, ExampleResource3, ExampleResource4, ExampleResource5, ExampleResource6, ExampleResource7 {
        let owner_address = signer::address_of(owner);
        assert!(index < 8, error::invalid_argument(EINDEX_TOO_LARGE));
        if (index == 0) {
            if (exists<ExampleResource0>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource0>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource0 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 1) {
            if (exists<ExampleResource1>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource1>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource1 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 2) {
            if (exists<ExampleResource2>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource2>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource2 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 3) {
            if (exists<ExampleResource3>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource3>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource3 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 4) {
            if (exists<ExampleResource4>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource4>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource4 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 5) {
            if (exists<ExampleResource5>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource5>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource5 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 6) {
            if (exists<ExampleResource6>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource6>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource6 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        } else if (index == 7) {
            if (exists<ExampleResource7>(owner_address)) {
                let resource = borrow_global_mut<ExampleResource7>(owner_address);
                assert!(resource.value < 1000000000000, error::invalid_state(EVALUE_TOO_LARGE));
            } else {
                let resource = ExampleResource7 {
                    value: 0,
                    name: string::utf8(b"init_name"),
                };
                move_to(owner, resource);
            }
        };
    }

    public entry fun set_p(_delegated_signer: &signer, owner: &signer, index: u64, name: String) acquires ExampleResource0, ExampleResource1, ExampleResource2, ExampleResource3, ExampleResource4, ExampleResource5, ExampleResource6, ExampleResource7 {
        set(owner, index, name);
    }

    public entry fun set_3(owner: &signer, index1: u64, index2: u64, index3: u64, name: String) acquires ExampleResource0, ExampleResource1, ExampleResource2, ExampleResource3, ExampleResource4, ExampleResource5, ExampleResource6, ExampleResource7 {
        set(owner, index1, name);
        set(owner, index2, name);
        set(owner, index3, name);
    }

    public entry fun set_and_read(owner: &signer, set_index: u64, read_index: u64, name: String) acquires ExampleResource0, ExampleResource1, ExampleResource2, ExampleResource3, ExampleResource4, ExampleResource5, ExampleResource6, ExampleResource7 {
        set(owner, set_index, name);
        read_or_init(owner, read_index);
    }

    public entry fun set_and_read_p(_delegated_signer: &signer, owner: &signer, set_index: u64, read_index: u64, name: String) acquires ExampleResource0, ExampleResource1, ExampleResource2, ExampleResource3, ExampleResource4, ExampleResource5, ExampleResource6, ExampleResource7 {
        set_and_read(owner, set_index, read_index, name);
    }

    #[test(creator = @0x123, owner = @0x456)]
    entry fun test_set(creator: &signer, owner: &signer) acquires Collection {
        set(creator, owner, 0, string::utf8(b"a"));
        set(creator, owner, 0, string::utf8(b"aa"));
        set(creator, owner, 1, string::utf8(b"b"));
        set(creator, owner, 2, string::utf8(b"c"));
        set(creator, owner, 3, string::utf8(b"d"));
        set(creator, owner, 4, string::utf8(b"e"));
        set(creator, owner, 8, string::utf8(b"f"));
        set(creator, owner, 0, string::utf8(b"g"));
        set(creator, owner, 0, string::utf8(b"bdsvs"));
        set(creator, owner, 0, string::utf8(b"bdfewfwe"));
    }
}
