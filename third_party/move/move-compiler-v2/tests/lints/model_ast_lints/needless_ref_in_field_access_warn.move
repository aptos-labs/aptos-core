module 0xc0ffee::m {
    struct S has key, drop {
        x: u64,
        y: U,
    }

    struct U has copy, store, drop {
        a: u64
    }

    public fun make_S(): S {
        S {
            x: 5,
            y: U { a: 6 }
        }
    }

    public fun test1_warn(s: S): u64 {
        (&s).x
    }

    public fun test1_no_warn(s: S): u64 {
        s.x
    }

    public fun test2_warn(s: S): u64 {
        (&(&s).y).a + (&((&s).y)).a
    }

    public fun test2_no_warn(s: S): u64 {
        s.y.a + s.y.a
    }

    public fun test3_warn(s: S): u64 {
        (&s).y.a + (&s.y).a + (&(&s).y).a + (&(s.y)).a
    }

    public fun test3_no_warn(s: S): u64 {
        s.y.a + s.y.a
    }

    public fun test4_warn(): u64 {
        (&make_S()).y.a
    }

    public fun test4_no_warn(): u64 {
        make_S().y.a
    }

    public fun test5_warn(s: S): u64 {
        (&mut make_S()).y.a + (&mut (&mut s).y).a
    }

    public fun test5_no_warn(s: S): u64 {
        make_S().y.a + s.y.a
    }

    public fun test6_warn(s: S) {
        (&mut s).x = 5;
    }

    public fun test6_no_warn(s: S) {
        s.x = 5;
    }

    public fun test7_warn(s: S) {
        (&mut (&mut s).y).a = 5;
        (&mut (s.y)).a = 6;
    }

    public fun test7_no_warn(s: S) {
        s.y.a = 5;
        s.y.a = 6;
    }

    public fun test8_warn() {
        (&mut make_S().y).a = 5;
    }

    public fun test8_no_warn() {
        make_S().y.a = 5;
    }

    enum E has drop {
        A(u64),
        B(u64),
    }

    public fun test_9_warn(e: E): u64 {
        (&e).0 + (&mut e).0
    }

    public fun test_9_no_warn(e: E): u64 {
        e.0 + e.0
    }

    public fun test_10_warn(e: E) {
        (&mut e).0 = 50;
    }

    public fun test_10_no_warn(e: E) {
        e.0 = 50;
    }
}


module 0xc0ffee::no_warn_1 {
    struct S has key, drop {
        x: u64,
    }

    #[lint::skip(needless_ref_in_field_access)]
    public fun test1_warn(s: S): u64 {
        (&s).x
    }
}

#[lint::skip(needless_ref_in_field_access)]
module 0xc0ffee::no_warn_2 {
    struct S has key, drop {
        x: u64,
    }

    public fun test1_warn(s: S): u64 {
        (&s).x + (&mut s).x
    }
}
