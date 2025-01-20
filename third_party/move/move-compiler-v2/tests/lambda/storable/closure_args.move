module 0x42::mod1 {
    struct S has drop {
        x: u64
    }

    struct Scopy has copy, drop {
        x: u64
    }

    struct Sstore has store, drop {
        x: u64
    }

    struct Sboth has store, copy, drop {
        x: u64
    }

    public fun base_fun(a: S, b: u64) : u64 {
        a.x * b
    }

    public fun base_fun2(a: u64, b: S) : u64 {
        a * b.x
    }

    public fun copy_fun(a: Scopy, b: u64) : u64 {
        a.x * b
    }

    public fun copy_fun2(a: u64, b: Scopy) : u64 {
        a * b.x
    }

    public fun store_fun(a: Sstore, b: u64) : u64 {
        a.x * b
    }

    public fun store_fun2(a: u64, b: Sstore) : u64 {
        a * b.x
    }

    public fun both_fun(a: Sboth, b: u64) : u64 {
        a.x * b
    }

    public fun both_fun2(a: u64, b: Sboth) : u64 {
        a * b.x
    }

    // just drop
    public fun use_function_base(key: u64, x: u64): u64 {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x)
            } else if (key == 1) {
                move |x| base_fun2(x, a)
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x)
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy)
            } else if (key == 4) {
                move |x| store_fun(a_store, x)
            } else if (key == 5) {
                move |x| store_fun2(x, a_store)
            } else if (key == 6) {
                move |x| both_fun(a_both, x)
            } else if (key == 7) {
                move |x| both_fun2(x, a_both)
            } else {
                move |x| x * 2
            };
        f(x)
    }

    public fun return_function_base(key: u64, x: u64): |u64|u64 {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x)
            } else if (key == 1) {
                move |x| base_fun2(x, a)
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x)
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy)
            } else if (key == 4) {
                move |x| store_fun(a_store, x)
            } else if (key == 5) {
                move |x| store_fun2(x, a_store)
            } else if (key == 6) {
                move |x| both_fun(a_both, x)
            } else if (key == 7) {
                move |x| both_fun2(x, a_both)
            } else {
                move |x| x * 2 with copy
            };
        f
    }


    // copy
    public fun use_function_copy(key: u64, x: u64): u64 {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x) with copy
            } else if (key == 1) {
                move |x| base_fun2(x, a) with copy
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x) with copy
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy) with copy
            } else if (key == 4) {
                move |x| store_fun(a_store, x) with copy
            } else if (key == 5) {
                move |x| store_fun2(x, a_store) with copy
            } else if (key == 6) {
                move |x| both_fun(a_both, x) with copy
            } else if (key == 7) {
                move |x| both_fun2(x, a_both) with copy
            } else {
                move |x| x * 2 with copy
            };
        f(x)
    }

    public fun return_function_copy(key: u64, x: u64): |u64|u64 with copy {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x) with copy
            } else if (key == 1) {
                move |x| base_fun2(x, a) with copy
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x) with copy
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy) with copy
            } else if (key == 4) {
                move |x| store_fun(a_store, x) with copy
            } else if (key == 5) {
                move |x| store_fun2(x, a_store) with copy
            } else if (key == 6) {
                move |x| both_fun(a_both, x) with copy
            } else if (key == 7) {
                move |x| both_fun2(x, a_both) with copy
            } else {
                move |x| x * 2 with copy
            };
        f
    }

    // store
    public fun use_function_store(key: u64, x: u64): u64 {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x) with store
            } else if (key == 1) {
                move |x| base_fun2(x, a) with store
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x) with store
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy) with store
            } else if (key == 4) {
                move |x| store_fun(a_store, x) with store
            } else if (key == 5) {
                move |x| store_fun2(x, a_store) with store
            } else if (key == 6) {
                move |x| both_fun(a_both, x) with store
            } else if (key == 7) {
                move |x| both_fun2(x, a_both) with store
            } else {
                move |x| x * 2 with store
            };
        f(x)
    }

    public fun return_function_store(key: u64, x: u64): |u64|u64 with store {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x) with store
            } else if (key == 1) {
                move |x| base_fun2(x, a) with store
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x) with store
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy) with store
            } else if (key == 4) {
                move |x| store_fun(a_store, x) with store
            } else if (key == 5) {
                move |x| store_fun2(x, a_store) with store
            } else if (key == 6) {
                move |x| both_fun(a_both, x) with store
            } else if (key == 7) {
                move |x| both_fun2(x, a_both) with store
            } else {
                move |x| x * 2 with store
            };
        f
    }


    // both = store+copy
    public fun use_function_both(key: u64, x: u64): u64 {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x) with store+copy
            } else if (key == 1) {
                move |x| base_fun2(x, a) with store+copy
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x) with store+copy
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy) with store+copy
            } else if (key == 4) {
                move |x| store_fun(a_store, x) with store+copy
            } else if (key == 5) {
                move |x| store_fun2(x, a_store) with store+copy
            } else if (key == 6) {
                move |x| both_fun(a_both, x) with store+copy
            } else if (key == 7) {
                move |x| both_fun2(x, a_both) with store+copy
            } else {
                move |x| x * 2 with store+copy
            };
        f(x)
    }

    public fun return_function_both(key: u64, x: u64): |u64|u64 with store+copy {
        let a = S { x: 2 };
        let a_copy = Scopy { x: 2 };
        let a_store = Sstore { x: 2 };
        let a_both = Sboth { x: 2 };
        let f =
            if (key == 0) {
                let x = 3;
                move |x| base_fun(a, x) with store+copy
            } else if (key == 1) {
                move |x| base_fun2(x, a) with store+copy
            } else if (key == 2) {
                move |x| copy_fun(a_copy, x) with store+copy
            } else if (key == 3) {
                move |x| copy_fun2(x, a_copy) with store+copy
            } else if (key == 4) {
                move |x| store_fun(a_store, x) with store+copy
            } else if (key == 5) {
                move |x| store_fun2(x, a_store) with store+copy
            } else if (key == 6) {
                move |x| both_fun(a_both, x) with store+copy
            } else if (key == 7) {
                move |x| both_fun2(x, a_both) with store+copy
            } else {
                move |x| x * 2 with store+copy
            };
        f
    }
}
