spec aptos_framework::primary_fungible_store {
    spec module {
        // TODO: verification disabled until this module is specified.
        pragma verify = false;
    }

    spec create_primary_store<T: key>(
        owner_addr: address,
        metadata: Object<T>,
    ): Object<FungibleStore> {
        use aptos_framework::object;
        pragma verify = true;
        pragma aborts_if_is_partial;
        let metadata_addr = metadata.inner;
        aborts_if !exists<object::ObjectCore>(metadata_addr);
        aborts_if !exists<Metadata>(metadata_addr);
    }
}
