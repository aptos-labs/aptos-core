module 0xc0ffee::m {
    fun get_vector_1(): vector<u8> {
        vector[1, 2, 3]
    }

    fun get_vector_2(): vector<u8> {
        vector[4, 5, 6]
    }

    struct S has copy, drop {
        a: vector<u8>,
        b: vector<u8>,
        c: vector<u8>,
        d: vector<u8>
    }

    public fun test1_warn_no_ref(): (bool, bool) {
        let a = get_vector_1();
        let b = get_vector_2();
        let p = a == b;
        let q = a != b;
        (p, q)
    }

    public fun test2_no_warn_no_ref(): bool {
        let a = get_vector_1();
        let b = get_vector_2();
        a == b
    }

    public fun test3_warn_no_ref(a: S, b: S): S {
        if (a == b) a else b
    }

    fun consume(_a: S, _b: S) {}

    public fun test4_warn_no_ref(a: S, b: S) {
        if (a == b) {
            consume(a, b)
        } else {
            consume(b, a)
        }
    }

    public fun test5_warn_no_ref(a: S, b: S) {
        assert!(a == b);
        consume(a, b);
    }

    public fun test1_no_warn_ref(): (bool, bool) {
        let a = get_vector_1();
        let b = get_vector_2();
        let p = &a == &b;
        let q = &a != &b;
        (p, q)
    }

    public fun test2_no_warn_ref(): bool {
        let a = get_vector_1();
        let b = get_vector_2();
        &a == &b
    }

    public fun test3_no_warn_ref(a: S, b: S): S {
        if (&a == &b) a else b
    }

    public fun test4_no_warn_ref(a: S, b: S) {
        if (&a == &b) {
            consume(a, b)
        } else {
            consume(b, a)
        }
    }

    public fun test5_no_warn_ref(a: S, b: S) {
        assert!(&a == &b);
        consume(a, b);
    }
}

#[lint::skip(avoid_copy_on_identity_comparison)]
module 0xc0ffee::no_warn1 {
    struct S has copy, drop {
        a: vector<u8>,
        b: vector<u8>,
        c: vector<u8>,
        d: vector<u8>
    }

    fun consume(_a: S, _b: S) {}

    public fun test4_warn_no_ref(a: S, b: S) {
        if (a == b) {
            consume(a, b)
        } else {
            consume(b, a)
        }
    }
}

module 0xc0ffee::no_warn2 {
    struct S has copy, drop {
        a: vector<u8>,
        b: vector<u8>,
        c: vector<u8>,
        d: vector<u8>
    }

    fun consume(_a: S, _b: S) {}

    #[lint::skip(avoid_copy_on_identity_comparison)]
    public fun test4_warn_no_ref(a: S, b: S) {
        if (a == b) {
            consume(a, b)
        } else {
            consume(b, a)
        }
    }
}
