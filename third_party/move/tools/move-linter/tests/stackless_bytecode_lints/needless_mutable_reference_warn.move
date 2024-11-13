module 0xc0ffee::m {
    fun consume_mut(x: &mut u64) {
        *x = 10;
    }

    fun consume_immut(_x: &u64) {}

    fun test1_no_warn(x: u64) {
        let y = &mut x;
        *y = 10;
    }

    fun test1_warn(x: u64): u64 {
        let y = &mut x;
        *y
    }

    fun test2_no_warn(x: u64) {
        let y = &mut x;
        consume_mut(y);
    }

    fun test2_warn(x: u64) {
        let y = &mut x;
        consume_immut(y);
    }

    struct S {
        x: u64,
        y: U,
    }

    struct U {
        u: u64,
    }

    fun test3_no_warn(s: &mut S) {
        s.y.u = 42;
    }

    fun test3_warn(s: &mut S): u64 {
        s.y.u
    }

    fun test4_no_warn(s: &mut S): u64 {
        s.y.u = 42;
        s.x
    }

    fun test5_no_warn(s: &mut S) {
        let t = s;
        consume_mut(&mut t.y.u);
    }

    fun test6_no_warn() {
        let x = 1;
        let z = 2;
        let y = &mut x;
        consume_immut(y);
        y = &mut z;
        *y = 5;
    }
}


module 0xc0ffee::n {
    struct R has key, store {
        x: u64
    }

    struct S has key {
        r: R
    }

    public fun test_no_warn_1(addr: address) acquires R {
        let r = borrow_global_mut<R>(addr);
        r.x = 5;
    }

    public fun test_no_warn_2(addr: address): u64 acquires R {
        let r = borrow_global_mut<R>(addr);
        let y = &mut r.x;
        *y = 5;
        *y
    }

    public fun test_no_warn_3(addr: address) acquires S {
        let s = borrow_global_mut<S>(addr);
        let y = &mut s.r;
        y.x = 5;
    }

    fun helper_mut(r: &mut R) {
        r.x = 5;
    }

    fun helper_immut(_r: &R) {}

    public fun test_no_warn_4(addr: address, p: bool) acquires R {
        let r = borrow_global_mut<R>(addr);
        if (p) {
            helper_mut(r);
        } else {
            helper_immut(r);
        }
    }

    public fun test_warn_1(addr: address) acquires R {
        let r = borrow_global_mut<R>(addr);
        helper_immut(r);
    }

    public fun test_warn_2(addr: address): u64 acquires R {
        let r = borrow_global_mut<R>(addr);
        let y = r.x;
        y
    }

    fun test_warn_3(s: &mut S, p: bool, addr: address): u64 acquires S {
        let ref = borrow_global_mut<S>(addr);
        if (p) {
            ref = s;
        };
        ref.r.x
    }

    fun test_no_warn_5(p: bool, a: address, s: &S): u64 acquires S {
        if (p) {
            s = borrow_global<S>(a);
        };
        let y = &s.r;
        y.x
    }
}

module 0xc0ffee::o {
    enum E has drop {
        A(u64),
        B(u64),
    }

    fun test_no_warn_1(e1: E, e2: E) {
        let a = &mut e1;
        let b = &mut a.0;
        *b = 5;
        b = &mut e2.0;
        *b = 1;
    }

    fun test_warn_1(e1: E, e2: E, p: bool): u64 {
        let a;
        if (p) {
            a = &mut e1.0;
        } else {
            a = &mut e2.0;
        };
        *a
    }

    fun test_warn_2(a: &mut E, b: &mut E) {
        let x = 1;
        loop {
            let temp = a;
            a = b;
            b = temp;
            x = x + 1;
            if (x == 10) {
                break;
            }
        }
    }

    public fun test_warn_3(x: &mut u64) {
        let y = x;
        x = y; // Produces a cycle in the `derived_edges`.
    }

    public fun test_no_warn_3(x: &mut u64) {
        let y = x;
        x = y;
        *x = 5;
    }

    #[lint::skip(needless_mutable_reference)]
    fun test_no_warn_4(e1: E, e2: E, p: bool): u64 {
        let a;
        if (p) {
            a = &mut e1.0;
        } else {
            a = &mut e2.0;
        };
        *a
    }

    struct S has drop {
        x: u64,
    }

    fun test_warn_5(s: S): u64 {
        *&mut s.x
    }

    fun test_no_warn_5(s: S): S {
        *&mut s
    }

    struct U has copy, drop {
        x: u64,
    }

    fun test_no_warn_6(u: U): U {
        *&mut u
    }

    // This should produce a warning once `vector::borrow_mut` is added to origins.
    fun test_warn_7(): u64 {
        let x = vector[1, 5, 5];
        let y = &mut x[0];
        *y
    }
}

module 0xc0ffee::more_struct_tests {
    struct S has copy, drop {
        x: u64
    }

    fun consume_mut(s: &mut S) {
        s.x = 10;
    }

    fun consume_immut(s: &S): u64 {
        s.x
    }

    fun test1_no_warn(s: S) {
        let u = &mut s;
        *u = S { x: 10 };
    }

    fun test1_warn(s: S): S {
        let u = &mut s;
        *u
    }

    fun test2_no_warn(s: S) {
        let u = &mut s;
        consume_mut(u);
    }

    fun test2_warn(s: S) {
        let u = &mut s;
        consume_immut(u);
    }
}
