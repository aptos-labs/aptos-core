// Regression: an opaque callee may create a resource which the caller then
// mutates. Inference must compose the caller's update with the callee's
// post-state instead of reading a potentially absent pre-state resource.
module 0x42::opaque_create_then_update {
    struct Resource has key {
        value: u64,
    }

    native fun create_resource(addr: address);

    spec create_resource {
        pragma opaque;
        pragma inference = none;
        modifies Resource[addr];
        aborts_if false;
        ensures exists<Resource>(addr);
        ensures Resource[addr].value == 0;
    }

    inline fun ensure_resource(addr: address, create: bool) {
        if (create) {
            if (!exists<Resource>(addr)) {
                create_resource(addr);
            };
        } else {
            assert!(exists<Resource>(addr), 1);
        };
    }

    spec ensure_resource {
        pragma opaque;
        pragma inference = none;
        modifies Resource[addr];
        aborts_if !exists<Resource>(addr) && !create;
        ensures exists<Resource>(addr);
        ensures old(exists<Resource>(addr)) ==>
            Resource[addr] == old(Resource[addr]);
        ensures !old(exists<Resource>(addr)) ==>
            Resource[addr].value == 0;
    }

    fun increment(addr: address, create: bool) acquires Resource {
        ensure_resource(addr, create);
        let value = &mut Resource[addr].value;
        assert!((*value as u128) < 18446744073709551615, 2);
        *value = *value + 1;
    }
}
