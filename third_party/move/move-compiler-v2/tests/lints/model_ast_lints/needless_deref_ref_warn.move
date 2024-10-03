module 0xc0ffee::m {
    struct S has key, drop {
        x: u64,
        y: U
    }

    struct U has copy, store, drop {
        a: u64
    }

    enum E has drop {
        A { x: u64 },
        B { x: u64 },
    }

    fun test1_warn(addr: address) acquires S {
        let r = borrow_global_mut<S>(addr);
        *&mut r.x = 5;
    }

    fun test1_no_warn(addr: address) acquires S {
        let r = borrow_global_mut<S>(addr);
        r.x = 5;
    }

    fun test2_warn(addr: address): U acquires S {
        *&borrow_global<S>(addr).y
    }

    fun test2_no_warn(addr: address): U acquires S {
        borrow_global<S>(addr).y
    }

    fun test3_warn(addr: address): U acquires S {
        *&mut borrow_global_mut<S>(addr).y
    }

    fun test5_no_warn(x: &u64) {
        let y = &mut *x;
        *y = 4;
    }

    fun test_6_no_warn(x: &u64) {
        *&mut *x = 4;
    }

    fun one(): u64 {
        1
    }

    fun test7_no_warn() {
        *&mut (one()) = 5;
    }

    fun make_S(): S {
        S { x: 5, y: U { a: 6 } }
    }

    fun test8_warn() {
        *&mut make_S().x = 5;
    }

    fun test8_no_warn() {
        make_S().x = 5;
    }

    fun test9_warn() {
        *&mut make_S().y.a = 5;
    }

    fun test9_no_warn() {
        make_S().y.a = 5;
    }

    fun mod_S(s: &mut S): &mut S {
        s.x = 48;
        s
    }

    fun test10_warn(): u64 {
        let s = make_S();
        *&mut mod_S(&mut s).x = 6;
        s.x
    }

    fun test10_no_warn(): u64 {
        let s = make_S();
        mod_S(&mut s).x = 6;
        s.x
    }

    fun test11_no_warn() {
        let s = vector[make_S(), make_S()];
        s[0].x = 8;
    }

    fun test12_warn(): S {
        let s = make_S();
        *&s
    }

    fun test12_no_warn(): S {
        let s = make_S();
        s
    }

    fun test13_warn(): S {
        let s = make_S();
        *&mut s
    }

    fun test14_warn(): u64 {
        let s = make_S();
        *&mut s.x
    }

    fun test15_warn(): u64 {
        let s = make_S();
        *& s.y.a
    }

    fun test15_no_warn(): u64 {
        let s = make_S();
        s.y.a
    }

    fun test16_warn(): u64 {
        let e = E::A { x: 5 };
        *& e.x
    }

    fun test16_no_warn(): u64 {
        let e = E::A { x: 5 };
        e.x
    }

    fun test17_warn(): u64 {
        let e = E::A { x: 5 };
        *&mut e.x
    }

    fun test18_warn() {
        let e = E::A { x: 5 };
        *&mut e.x = 6;
    }

    fun test19_warn(x: u64) {
        *&mut x = 42;
    }

    fun test20_warn() {
        let x = 42;
        *&mut x = 5;
    }
}

module 0xc0ffee::n {
    struct Foo has copy, drop {
        x: u64,
        y: bool
    }

    struct Bar has copy, drop {
        foo: Foo
    }

    fun test1_warn() {
        let foo = Foo { x: 3, y: true };
        let bar = Bar { foo };
        let _foo1: Foo = *&bar.foo;
        let _foo2: Foo = bar.foo;
    }

    fun test1_no_warn() {
        let foo = Foo { x: 3, y: true };
        let bar = Bar { foo };
        let _foo1: Foo = bar.foo;
        let _foo2: Foo = bar.foo;
    }

    #[lint::skip(needless_deref_ref)]
    fun test2_no_warn() {
        let foo = Foo { x: 3, y: true };
        let bar = Bar { foo };
        let _foo1: Foo = *&bar.foo;
        let _foo2: Foo = bar.foo;
    }
}
