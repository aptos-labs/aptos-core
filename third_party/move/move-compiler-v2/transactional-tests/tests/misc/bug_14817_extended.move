//# publish
module 0xc0ffee::m {
    struct P has copy, drop {
        q: Q,
    }

    struct Q has copy, drop {
        r: u64,
    }

    fun new_p(): P {
        P { q: Q { r: 0 } }
    }

    fun test_01(p: P): P {
        p.q.r = 1;
        p
    }

    fun test_02(p: P): u64 {
        p.q.r + 10
    }

    fun test_03(p: &mut P) {
        p.q.r = 12;
    }

    fun test_04(p: P): P {
        *&mut p.q.r = 14;
        p
    }

    fun test_05(p: P): P {
        *&mut (p.q.r) = 15;
        p
    }

    fun test_06(p: P): P {
        let pq = &mut p.q;
        pq.r = 16;
        p
    }

    fun test_07(p: P): P {
        // The code below modifies a temporary value.
        *&mut {p.q.r} = 17;
        p
    }


    fun test_08(v: u64): u64 {
        *&mut {v} = 18;
        v
    }

    fun test_09(p: u64): bool {
        &mut p == &mut (1+2)
    }

    inline fun derive_01(p: &mut P): &mut u64 {
        &mut p.q.r
    }

    fun test_10(p: P): P {
        *derive_01(&mut p) = 20;
        p
    }

    inline fun derive_02(p: P): &mut u64 {
        &mut p.q.r
    }

    fun test_11(p: P): P {
        *derive_02(p) = 21;
        p
    }

    fun test_12(p: P): P {
        *&mut {*&mut p.q.r = 5; p.q.r} = 22;
        p
    }

    fun test_13(p: P): P {
        *(&mut (*(&p.q.r))) = 23;
        p
    }

    fun test_14(p: P): P {
        *&mut {p.q = Q { r: 24 }; p.q.r} = 12;
        p
    }

    public fun main() {
        let p01 = new_p();
        let p01_result = test_01(p01);
        assert!(p01_result.q.r == 1, 0);

        let p02 = new_p();
        let p02_result = test_02(p02);
        assert!(p02_result == 10, 0);

        let p03 = new_p();
        test_03(&mut p03);
        assert!(p03.q.r == 12, 0);

        let p04 = new_p();
        let p04_result = test_04(p04);
        assert!(p04_result.q.r == 14, 0);

        let p05 = new_p();
        let p05_result = test_05(p05);
        assert!(p05_result.q.r == 15, 0);

        let p06 = new_p();
        let p06_result = test_06(p06);
        assert!(p06_result.q.r == 16, 0);

        let p07 = new_p();
        let p07_result = test_07(p07);
        assert!(p07_result.q.r == 0, 0);

        assert!(test_08(1) == 1, 0);

        assert!(test_09(3), 0);

        assert!(test_10(new_p()).q.r == 20, 0);

        let p11 = new_p();
        let p11_result = test_11(p11);
        assert!(p11_result.q.r == 0, 0);

        let p12 = new_p();
        let p12_result = test_12(p12);
        assert!(p12_result.q.r == 5, 0);

        let p13 = new_p();
        let p13_result = test_13(p13);
        assert!(p13_result.q.r == 0, 0);

        let p14 = new_p();
        let p14_result = test_14(p14);
        assert!(p14_result.q.r == 24, 0);
    }
}

//# run 0xc0ffee::m::main

//# publish
module 0xCAFE::m1 {
    struct Struct0 has copy, drop {
        x: bool,
    }

    fun f(s: Struct0) {
        *(&mut (*(&s.x))) = true;
    }

    public fun main() {
        let s = Struct0 { x: true };
        f(s);
    }
}

//# run 0xCAFE::m1::main

//# publish
module 0xCAFE::m2 {
    struct S has copy, drop {
        x: bool,
    }

    fun f(s: S) {
        *({
            *(&mut (true)) = s.x;
            &mut (0u8)
        }) = 123u8;
    }

    public fun main() {
        let s = S { x: true };
        f(s);
    }
}

//# run 0xCAFE::m2::main

//# publish
module 0xc0ffee::m3 {
    struct S has copy, drop {
        f: ||u8
    }

    fun test(s: S) {
        *&mut {(s.f)()} = 1;
    }

    public fun main() {
        let s = S { f: || 42 };
        test(s);
    }
}

//# run 0xc0ffee::m3::main
