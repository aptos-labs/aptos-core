module TestFunFormat {

    struct SomeOtherStruct has drop {
        some_field: u64,
    }

    public fun multi_arg(p1: u64, p2: u64): /* test comment locate before return type */ u64 {
        p1 + p2
    }

    public fun multi_arg(p1: u64, p2: u64): u64 /* test comment locate after return type */ {
        p1 + p2
    }

    struct SomeStruct has key, drop, store {
        some_field: u64,
    }

    fun acq(addr: address): u64 /* test comment locate before acquires */
        acquires SomeStruct {
        let val = borrow_global<SomeStruct>(addr);

        val.some_field
    }

    fun acq22(addr: address): u64
        acquires SomeStruct /* test comment locate after acquires */ {
        let val = borrow_global<SomeStruct>(addr);
        val.some_field
    }

    fun acq33(addr: address): u64
        acquires /* test comment locate between acquires */ SomeStruct {
        let val = borrow_global<SomeStruct>(addr);
        val.some_field
    }
}