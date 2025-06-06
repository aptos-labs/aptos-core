module 0x42::m {

    fun exec<T: copy+drop, R>(f: |T|R has copy+drop, x: T): R {
        let r = f(x);
        spec { assert r == f(x); };
        r
    }

    // Function spec block
    fun function_spec_block(x: u64): u64 {
        x + 1
    }
    spec function_spec_block {
        ensures result == exec(|x| x + 1, x);
        ensures result == exec(|x| x + 1, x);
    }

    // Function code spec block
    fun function_code_spec_block(x: u64): u64 {
        spec { assert exec(|y| y > 0, x); }; // This is lifted and leads to followup errors, need to enable in specs
        x + 1
    }

    // Struct spec block
    struct S has key, copy, drop { f: u64 }
    spec S { invariant exec(|x| x > 0, f); }

    // Global invariant
    spec module {
        invariant forall a: address:
            exists<S>(a) ==> exec(|a| get<S>(a).f < 10, a);
    }
    inline fun get<R:key>(a: address): &R { borrow_global<R>(a) }
}
