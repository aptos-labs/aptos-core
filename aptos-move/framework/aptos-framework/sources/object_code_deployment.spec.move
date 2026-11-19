spec aptos_framework::object_code_deployment {
    /// Publishes the package to a new object named after the publisher's next sequence number.
    spec publish(
        publisher: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        use std::bcs;
        use std::features;
        use aptos_framework::account;
        // Partial: the aborts of publishing the package are not specified.
        pragma aborts_if_is_partial;
        let publisher_address = signer::address_of(publisher);
        let sequence_number = if (exists<account::Account>(publisher_address)) {
            global<account::Account>(publisher_address).sequence_number
        } else {
            0
        };
        let object_address = object::spec_create_object_address(
            publisher_address,
            concat(bcs::to_bytes(OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR), bcs::to_bytes((sequence_number + 1) as u64))
        );
        aborts_if !exists<account::Account>(publisher_address)
            && !features::spec_is_enabled(features::DEFAULT_ACCOUNT_RESOURCE);
        aborts_if sequence_number + 1 > MAX_U64;
        aborts_if exists<object::ObjectCore>(object_address);
        ensures global<object::ObjectCore>(object_address).owner == publisher_address;
        ensures global<ManagingRefs>(object_address).extend_ref.self == object_address;
    }

    /// The signer of the code object, for its owner.
    spec get_code_object_signer(publisher: &signer, code_object: Object<PackageRegistry>): signer {
        aborts_if !exists<object::ObjectCore>(code_object.inner);
        aborts_if global<object::ObjectCore>(code_object.inner).owner != signer::address_of(publisher);
        aborts_if !exists<ManagingRefs>(code_object.inner);
        ensures signer::address_of(result) == global<ManagingRefs>(code_object.inner).extend_ref.self;
    }

    /// Only the owner of a code object created by `publish` may upgrade it.
    spec upgrade(
        publisher: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
        code_object: Object<PackageRegistry>,
    ) {
        // Partial: the aborts of publishing the package are not specified.
        pragma aborts_if_is_partial;
        aborts_if !exists<object::ObjectCore>(code_object.inner);
        aborts_if global<object::ObjectCore>(code_object.inner).owner != signer::address_of(publisher);
        aborts_if !exists<ManagingRefs>(code_object.inner);
    }
}
