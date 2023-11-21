module tournament::object_refs {
    use std::option::{Self, Option};
    use aptos_framework::object::{Self, ConstructorRef, DeleteRef, ExtendRef, LinearTransferRef, TransferRef};

    friend tournament::tournament_manager;
    friend tournament::token_manager;
    friend tournament::matchmaker;
    friend tournament::room;
    friend tournament::round;
    friend tournament::rewards;

    #[test_only]
    friend tournament::rps_unit_tests;
    #[test_only]
    friend tournament::roulette_unit_tests;
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
        delete_ref: Option<DeleteRef>,
        // for basic objects
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

    public(friend) fun create_refs<T: key>(constructor_ref: &ConstructorRef): (signer, address) {
        let extend_ref = object::generate_extend_ref(constructor_ref);
        let transfer_ref = object::generate_transfer_ref(constructor_ref);
        let delete_ref = if (object::can_generate_delete_ref(constructor_ref)) {
            option::some(object::generate_delete_ref(constructor_ref))
        } else {
            option::none()
        };
        let obj_signer = object::generate_signer(constructor_ref);

        object::disable_ungated_transfer(&transfer_ref);
        move_to(
            &obj_signer,
            Refs {
                extend_ref,
                transfer_ref,
                delete_ref,
            },
        );
        (obj_signer, object::address_from_constructor_ref(constructor_ref))
    }

    public(friend) fun destroy_object(obj_addr: address) acquires Refs {
        let Refs {
            extend_ref: _,
            transfer_ref: _,
            delete_ref,
        } = move_from<Refs>(obj_addr);
        if (option::is_some(&delete_ref)) {
            object::delete(option::extract(&mut delete_ref));
        }
    }
}
