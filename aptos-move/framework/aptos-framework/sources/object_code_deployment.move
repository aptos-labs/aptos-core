module aptos_framework::object_code_deployment {
    use std::bcs;
    use std::features;
    use std::signer;
    use std::string;
    use aptos_framework::account;
    use aptos_framework::code;
    use aptos_framework::code::PackageRegistry;
    use aptos_framework::object;
    use aptos_framework::object::{ExtendRef, Object};

    /// Object code deployment not supported.
    const EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED: u64 = 0x1;
    /// Not the owner of the `PublisherRef`
    const ENOT_OWNER: u64 = 0x2;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct PublisherRef has key {
        extend_ref: ExtendRef,
    }

    /// Create a new object to host the code, and `PublisherRef` if the code is upgradeable,
    /// Send `PublisherRef` to object signer.
    public entry fun publish(
        publisher: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        assert!(features::is_object_code_deployment_enabled(), EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED);

        let object_seed = object_seed(signer::address_of(publisher));
        let constructor_ref = &object::create_named_object(publisher, object_seed);
        let module_signer = &object::generate_signer(constructor_ref);
        code::publish_package_txn(module_signer, metadata_serialized, code);

        if (code::is_package_upgradeable(metadata_serialized)) {
            move_to(module_signer, PublisherRef {
                extend_ref: object::generate_extend_ref(constructor_ref)
            });
        };
    }

    inline fun object_seed(publisher: address): vector<u8> {
        let object_seed = &mut string::utf8(b"aptos_framework::object_code_deployment");
        let sequence_number = account::get_sequence_number(publisher) + 1;
        string::append(object_seed, string::utf8(bcs::to_bytes(&sequence_number)));
        *string::bytes(object_seed)
    }

    /// Upgrade the code in an existing code object.
    /// Requires the publisher to be the owner of the `PublisherRef` object.
    public entry fun upgrade(
        publisher: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
        publisher_ref: Object<PublisherRef>,
    ) acquires PublisherRef {
        assert!(features::is_object_code_deployment_enabled(), EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED);
        assert!(object::is_owner(publisher_ref, signer::address_of(publisher)), ENOT_OWNER);

        let extend_ref = &borrow_global<PublisherRef>(object::object_address(&publisher_ref)).extend_ref;
        let code_signer = &object::generate_signer_for_extending(extend_ref);
        code::publish_package_txn(code_signer, metadata_serialized, code);
    }

    /// Make an existing upgradable package immutable.
    /// Requires the `publisher` to be the owner of the `package_registry` object.
    public entry fun freeze_package_registry(publisher: &signer, package_registry: Object<PackageRegistry>) {
        code::freeze_package_registry(publisher, package_registry);
    }
}
