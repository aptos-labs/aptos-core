module tournament::object_refs {
    use std::option::{Self, Option};
    use std::string::String;
    use std::type_info::type_of;
    use aptos_framework::object::{Self, ConstructorRef, DeleteRef, ExtendRef, LinearTransferRef, TransferRef};

    use aptos_token_objects::property_map;
    use aptos_token_objects::token::{Self, BurnRef, Token};

    friend tournament::tournament_manager;
    friend tournament::token_manager;
    friend tournament::matchmaker;
    friend tournament::room;
    friend tournament::round;

    #[test_only]
    friend tournament::rps_unit_tests;
    #[test_only]
    friend tournament::main_unit_test;
    #[test_only]
    friend tournament::test_utils;

    /// That object doesn't have valid Refs (capabilities) on it.
    const EOBJECT_HAS_NO_REFS: u64 = 0;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Refs has key {
        extend_ref: ExtendRef,
        transfer_ref: TransferRef,
        delete_ref: DeleteRef,  // for basic objects
        burn_ref: Option<BurnRef>,
        property_mutator_ref: Option<property_map::MutatorRef>,
    }

    public(friend) fun get_signer(obj_addr: address): signer acquires Refs {
        assert!(exists<Refs>(obj_addr), EOBJECT_HAS_NO_REFS);
        let refs = borrow_global<Refs>(obj_addr);
        object::generate_signer_for_extending(&refs.extend_ref)
    }

    public(friend) fun get_linear_transfer_ref(obj_addr: address): LinearTransferRef acquires Refs {
        assert!(exists<Refs>(obj_addr), EOBJECT_HAS_NO_REFS);
        let refs = borrow_global<Refs>(obj_addr);
        object::generate_linear_transfer_ref(&refs.transfer_ref)
    }

    public(friend) fun property_map_add_typed<T: drop>(obj_addr: address, key: String, value: T) acquires Refs {
        assert!(exists<Refs>(obj_addr), EOBJECT_HAS_NO_REFS);
        let refs = borrow_global<Refs>(obj_addr);
        let ref = option::borrow(&refs.property_mutator_ref);
        property_map::add_typed(ref, key, value);
    }

    public(friend) fun property_map_update_typed<T: drop>(obj_addr: address, key: String, value: T) acquires Refs {
        assert!(exists<Refs>(obj_addr), EOBJECT_HAS_NO_REFS);
        let refs = borrow_global<Refs>(obj_addr);
        let ref = option::borrow(&refs.property_mutator_ref);
        property_map::update_typed(ref, &key, value);
    }

    public(friend) fun create_refs<T: key>(constructor_ref: &ConstructorRef): (signer, address) {
        let extend_ref = object::generate_extend_ref(constructor_ref);
        let transfer_ref = object::generate_transfer_ref(constructor_ref);
        let delete_ref = object::generate_delete_ref(constructor_ref);
        let obj_signer = object::generate_signer(constructor_ref);
        let (burn_ref, property_mutator_ref) = if (type_of<T>() == type_of<Token>()) {
            let burn_ref = token::generate_burn_ref(constructor_ref);
            let properties = property_map::prepare_input(vector[], vector[], vector[]);
            property_map::init(constructor_ref, properties);
            let property_mutator_ref = property_map::generate_mutator_ref(constructor_ref);
            (option::some<BurnRef>(burn_ref), option::some<property_map::MutatorRef>(property_mutator_ref))
        } else {
            (option::none<BurnRef>(), option::none<property_map::MutatorRef>()) // if it's not a token we don't store a burn ref, don't care what type it is
        };
        object::disable_ungated_transfer(&transfer_ref);
        move_to(
            &obj_signer,
            Refs {
                extend_ref,
                transfer_ref,
                delete_ref,
                burn_ref,
                property_mutator_ref,
            },
        );
        (obj_signer, object::address_from_constructor_ref(constructor_ref))
    }

    public(friend) fun destroy_object(obj_addr: address) acquires Refs {
        let Refs {
            extend_ref: _,
            transfer_ref: _,
            delete_ref,
            burn_ref: _,
            property_mutator_ref: _,
        } = move_from<Refs>(obj_addr);
        object::delete(delete_ref);
    }

    public(friend) fun destroy_for_token(tournament_token_addr: address): (BurnRef, property_map::MutatorRef) acquires Refs {
        let Refs {
            extend_ref: _,
            transfer_ref: _,
            delete_ref: _,
            burn_ref,
            property_mutator_ref,
        } = move_from<Refs>(tournament_token_addr);

        (option::extract(&mut burn_ref), option::extract(&mut property_mutator_ref))
    }
}

