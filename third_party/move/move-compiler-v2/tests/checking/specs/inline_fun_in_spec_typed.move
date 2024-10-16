module 0x42::m {

    inline fun exec<T, R>(f: |T|R, x: T): R {
        let r = f(x);
        spec { assert r == f(x); };
        r
    }

    // Function spec block
    fun function_spec_block(x: u64): u64 {
        x + 1
    }
    spec function_spec_block {
        ensures result == exec(|x: u64| x + 1, x);
    }

    // Function code spec block
    fun function_code_spec_block(x: u64): u64 {
        spec { assert exec(|y: u64| y > 0, x); };
        x + 1
    }

    // Struct spec block
    struct S has key { f: u64 }
    spec S { invariant exec(|x: u64| x > 0, f); }

    // Global invariant
    spec module {
        invariant forall a: address:
            exists<S>(a) ==> exec(|a: address| get<S>(a).f < 10, a);
    }
    inline fun get<R:key>(a: address): &R { borrow_global<R>(a) }
}
