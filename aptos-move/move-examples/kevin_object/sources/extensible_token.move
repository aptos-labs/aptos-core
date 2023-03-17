module token_objects::extensible_token {
    use std::string::{String, utf8};
    use std::signer;
    use aptos_framework::object::{Self, ExtendRef, TransferRef};
    use token_objects::token;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct ExtensibleToken has key {
        desc: String,
        extend_ref: ExtendRef,
        transfer_ref: TransferRef,
    }

    public entry fun create_extensible_token(creator: &signer, name: String) {
        let constructor_ref = token::create_token(
            creator,
            utf8(b"Extensible Token"),
            utf8(b"This token is extensible"),
            token::create_mutability_config(true, true, true),
            name,
            utf8(b"Extensible Token"),
        );
        let extend_ref = object::generate_extend_ref(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);

        let object_signer = &object::generate_signer(&constructor_ref);
        move_to(object_signer, ExtensibleToken {
            desc: name,
            extend_ref,
            transfer_ref,
        })
    }

    entry fun set_desc(owner: &signer, creator: address, name: String, new_desc: String) acquires ExtensibleToken {
        let object_address = get_extensible_token_address(&creator, &name);
        assert!(object::is_owner(object::address_to_object<ExtensibleToken>(object_address), signer::address_of(owner)), 0);
        let extensible_token = borrow_global_mut<ExtensibleToken>(object_address);
        extensible_token.desc = new_desc;
    }

    public fun owner_get_object_signer(owner: &signer, creator: address, name: String): signer acquires ExtensibleToken {
        let object_address = get_extensible_token_address(&creator, &name);
        assert!(object::is_owner(object::address_to_object<ExtensibleToken>(object_address), signer::address_of(owner)), 0);
        let extensible_token = borrow_global<ExtensibleToken>(object_address);
        let extend_ref = &extensible_token.extend_ref;
        object::generate_signer_for_extending(extend_ref)
    }

    public fun get_extensible_token_address(creator: &address, name: &String): address {
        token::create_token_address(creator, &utf8(b"Extensible Token"), name)
    }

    #[test(creator = @0x123, new_owner = @0x124)]
    entry fun test_create_and_transfer(creator: &signer, new_owner: &signer) {
        let name = utf8(b"Kevin");
        create_extensible_token(creator, name);
        let creator_address = signer::address_of(creator);
        let new_owner_address = signer::address_of(new_owner);
        let object_address = get_extensible_token_address(&creator_address, &name);
        let object_ref = object::address_to_object<ExtensibleToken>(object_address);
        object::transfer(creator, object_ref, new_owner_address);
    }
}
