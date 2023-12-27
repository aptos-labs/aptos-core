module 0x42::m {
    struct Object has copy, drop {
        inner: address
    }

    struct ObjectCore has key {
        owner: address
    }

    fun owner_correct(o: Object): address {
        let addr = o.inner;
        borrow_global<ObjectCore>(addr).owner
    }

    fun owner_read_ref_missing(o: Object): address {
        borrow_global<ObjectCore>(o.inner).owner
    }
}
