module 0x8675309::M {
    struct S { f: u64 }

    fun foo(_s: &S, _u: u64) {}
    fun t0(s: &mut S) {
        let x = 0;
        let z = 1;
        let f = { x = x + 1; &mut ({x = x + 1; z; s}).f };
        if (z == 1) {
            x = x + 1;
            foo(freeze(s), { *({x = x + 1; f}) = 0; 1 })
        } else {
            x = x + 1
        };
        assert!(x == 4, 0);
    }

    fun bar(_s: &mut u64, _u: u64) {}
    fun t1(s: &mut S) {
        bar(&mut s.f, { s.f = 0; 1 })
    }
}
