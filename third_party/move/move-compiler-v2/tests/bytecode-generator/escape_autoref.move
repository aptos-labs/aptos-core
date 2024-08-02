module 0x42::m {
    struct Object has copy, drop {
        inner: address
    }

    struct ObjectCore has key {
        owner: address
    }

    fun owner_correct(o: Object): address acquires ObjectCore {
        let addr = o.inner;
        borrow_global<ObjectCore>(addr).owner
    }

    // In a previous bug, a ReadRef was not generated for `o.inner`
    fun owner_read_ref_missing(o: Object): address acquires ObjectCore {
        borrow_global<ObjectCore>(o.inner).owner
    }

    // Ensure that autoref still works
    fun make(): Object { abort 0 }
    fun will_autoref(): address {
        make().inner
    }
}
