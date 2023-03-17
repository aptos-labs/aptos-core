module token_objects::kevin_extension {
    use std::string::{String, utf8};
    use std::signer;
    use token_objects::extensible_token;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct KevinExtension has key {
        value: u64,
    }

    entry fun add_kevin_extension(owner: &signer, creator: address, name: String) {
        let object_signer = &extensible_token::owner_get_object_signer(owner, creator, name);
        move_to(object_signer, KevinExtension { value: 0 })
    }

    entry fun increment_value(owner: &signer, creator: address, name: String) acquires KevinExtension {
        let object_signer = &extensible_token::owner_get_object_signer(owner, creator, name);
        let extension = borrow_global_mut<KevinExtension>(signer::address_of(object_signer));
        extension.value = extension.value + 1;
    }

    #[view]
    public fun get_value(creator: address, name: String): u64 acquires KevinExtension {
        let object_addr = extensible_token::get_extensible_token_address(&creator, &name);
        let extension = borrow_global<KevinExtension>(object_addr);
        extension.value
    }

    #[test(creator = @0x123)]
    entry fun test_increment_value(creator: &signer) acquires KevinExtension {
        let name = utf8(b"Kevin");
        extensible_token::create_extensible_token(creator, name);
        let creator_address = signer::address_of(creator);
        add_kevin_extension(creator, creator_address, name);
        assert!(get_value(creator_address, name) == 0, 0);
        increment_value(creator, creator_address, name);
        assert!(get_value(creator_address, name) == 1, 1);
    }
}
