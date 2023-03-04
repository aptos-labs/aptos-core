module token_v2::refs {
    use aptos_framework::object::{ExtendRef, TransferRef, ConstructorRef, generate_extend_ref, address_from_constructor_ref, generate_transfer_ref, generate_delete_ref, Object, address_to_object, address_from_extend_ref, address_from_transfer_ref, address_from_delete_ref, DeleteRef};
    use std::option::{Option, is_some};
    use std::option;
    use token_v2::common::assert_flags_length;
    use std::vector;
    use std::error;
    #[test_only]
    use token_v2::common::create_test_token;

    /// Error about ExtendRef.
    const EEXTEND_REF: u64 = 1;
    /// Error about TransferRef.
    const ETRANSFER_REF: u64 = 2;
    /// Error about DeleteRef.
    const EDELETE_REF: u64 = 3;

    // ================================================================================================================
    // Refs - a collection of ExtendRef, TransferRef and DeleteRef.
    // ================================================================================================================
    struct Refs has drop, store {
        object_address: address,
        extend: Option<ExtendRef>,
        transfer: Option<TransferRef>,
        delete: Option<DeleteRef>,
    }

    public fun new_refs_from_constructor_ref(
        constructor_ref: &ConstructorRef,
        enabled_refs: vector<bool>
    ): Refs {
        assert_flags_length(&enabled_refs);
        let enable_extend = *vector::borrow(&enabled_refs, 0);
        let enable_transfer = *vector::borrow(&enabled_refs, 1);
        let enable_delete = *vector::borrow(&enabled_refs, 2);
        Refs {
            object_address: address_from_constructor_ref(constructor_ref),
            extend: if (enable_extend) { option::some(generate_extend_ref(constructor_ref)) } else { option::none() },
            transfer: if (enable_transfer) { option::some(generate_transfer_ref(constructor_ref)) } else {
                option::none()
            },
            delete: if (enable_delete) { option::some(generate_delete_ref(constructor_ref)) } else { option::none() },
        }
    }

    public fun address_of_refs(refs: &Refs): address {
        refs.object_address
    }

    public fun generate_object_from_refs<T: key>(refs: &Refs): Object<T> {
        address_to_object<T>(refs.object_address)
    }

    public fun put_extend_to_refs(refs: &mut Refs, ref: ExtendRef) {
        assert!(option::is_none(&refs.extend), error::already_exists(EEXTEND_REF));
        assert!(address_from_extend_ref(&ref) == refs.object_address, error::invalid_argument(EEXTEND_REF));
        option::fill(&mut refs.extend, ref);
    }

    public fun put_transfer_to_refs(refs: &mut Refs, ref: TransferRef) {
        assert!(option::is_none(&refs.transfer), error::already_exists(ETRANSFER_REF));
        assert!(address_from_transfer_ref(&ref) == refs.object_address, error::invalid_argument(ETRANSFER_REF));
        option::fill(&mut refs.transfer, ref);
    }

    public fun put_delete_to_refs(refs: &mut Refs, ref: DeleteRef) {
        assert!(option::is_none(&refs.delete), error::already_exists(EDELETE_REF));
        assert!(address_from_delete_ref(&ref) == refs.object_address, error::invalid_argument(EDELETE_REF));
        option::fill(&mut refs.delete, ref);
    }

    public fun refs_contain_extend(refs: &Refs): bool {
        is_some(&refs.extend)
    }

    public fun refs_contain_transfer(refs: &Refs): bool {
        is_some(&refs.transfer)
    }

    public fun refs_contain_delete(refs: &Refs): bool {
        is_some(&refs.delete)
    }

    public fun borrow_extend_from_refs(refs: &Refs): &ExtendRef {
        assert!(is_some(&refs.extend), error::not_found(EEXTEND_REF));
        option::borrow(&refs.extend)
    }

    public fun borrow_transfer_from_refs(refs: &Refs): &TransferRef {
        assert!(is_some(&refs.transfer), error::not_found(ETRANSFER_REF));
        option::borrow(&refs.transfer)
    }

    public fun borrow_delete_from_refs(refs: &Refs): &DeleteRef {
        assert!(is_some(&refs.delete), error::not_found(EDELETE_REF));
        option::borrow(&refs.delete)
    }

    public fun get_extend_from_refs(refs: &mut Refs): ExtendRef {
        assert!(is_some(&refs.extend), error::not_found(EEXTEND_REF));
        option::extract(&mut refs.extend)
    }

    public fun get_transfer_from_refs(refs: &mut Refs): TransferRef {
        assert!(is_some(&refs.transfer), error::not_found(ETRANSFER_REF));
        option::extract(&mut refs.transfer)
    }

    public fun get_delete_from_refs(refs: &mut Refs): DeleteRef {
        assert!(is_some(&refs.delete), error::not_found(EDELETE_REF));
        option::extract(&mut refs.delete)
    }

    #[test(creator = @0xcafe)]
    fun test_refs(creator: &signer) {
        let (creator_ref, _token_obj) = create_test_token(creator);
        let refs = new_refs_from_constructor_ref(&creator_ref, vector[true, true, true]);
        assert!(refs_contain_extend(&refs), 0);
        assert!(refs_contain_transfer(&refs), 1);
        assert!(refs_contain_delete(&refs), 2);

        let extend_ref = get_extend_from_refs(&mut refs);
        let transfer_ref = get_transfer_from_refs(&mut refs);
        let delete_ref = get_delete_from_refs(&mut refs);

        assert!(!refs_contain_extend(&refs), 3);
        assert!(!refs_contain_transfer(&refs), 4);
        assert!(!refs_contain_delete(&refs), 5);

        put_extend_to_refs(&mut refs, extend_ref);
        put_transfer_to_refs(&mut refs, transfer_ref);
        put_delete_to_refs(&mut refs, delete_ref);

        assert!(refs_contain_extend(&refs), 6);
        assert!(refs_contain_transfer(&refs), 7);
        assert!(refs_contain_delete(&refs), 8);

        assert!(address_from_extend_ref(borrow_extend_from_refs(&refs)) == address_of_refs(&refs), 9);
        assert!(address_from_transfer_ref(borrow_transfer_from_refs(&refs)) == address_of_refs(&refs), 10);
        assert!(address_from_delete_ref(borrow_delete_from_refs(&refs)) == address_of_refs(&refs), 11);
    }
}
