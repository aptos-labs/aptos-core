spec aptos_framework::object_code_deployment {
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
