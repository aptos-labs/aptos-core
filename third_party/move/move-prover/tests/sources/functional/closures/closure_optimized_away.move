module 0x42::test {

    fun add(x: u64, y: u64): u64 {
        x + y
    }
    fun test_closure_1(x: u64): u64 {
        (add)(x, 1)
    }
    spec test_closure_1 {
        ensures result == x + 1;
    }

    fun test_closure_2(x: u64): u64 {
        (|y| y + x)(1)
    }
    spec test_closure_2 {
        ensures result == x + 2; // This does not verify
    }

    fun test_closure_3(x: u64): u64 {
        (|y| y + x)(1)
    }
    spec test_closure_3 {
        ensures result == (add)(1 as u64, x);
    }

    fun and(a: bool, b: bool): bool {
        a && b
    }

    fun test_closure_4(x: bool): bool {
        and(x, false)
    }
    spec test_closure_4 {
        ensures and(x, false) == (|y| y && x)(false);
    }

    struct S<T: copy + drop> has copy, drop {
        f: T
    }

    fun test_closure_5(x: S<bool>): bool {
        and(x.f, false)
    }
    spec test_closure_5 {
        ensures and(x.f, false) == (|s: S<bool>| s.f && x.f )(S { f: false });
    }

}
