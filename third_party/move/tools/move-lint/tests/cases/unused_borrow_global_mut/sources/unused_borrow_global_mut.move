module NamedAddr::counter {

    struct Counter has key { i: u64, z: u64 }


    public fun scope_nested_test(addr: address) acquires Counter {
        let c_ref = borrow_global_mut<Counter>(addr);
        c_ref.i = 3;
        let _d_ref = borrow_global_mut<Counter>(addr);


    }

    public inline fun scope_nested_test1(addr: address) acquires Counter {
        let c_ref = borrow_global_mut<Counter>(addr);
        c_ref.i = 3;
        let _d_ref = borrow_global_mut<Counter>(addr);


    }


}