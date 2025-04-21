module 0xc0ffee::m {
    struct S {
        f: u64,
        g: u64,
    }

    fun bar(_r: &mut S): (u64, u64) {
        (1, 1)
    }

    fun f2(_r1: address, _r2: &mut S, _r3: address, _r4: u64, _r5: &mut S) {}

    fun f3(_r1: u64, _r2: &u64) {}

    fun f1(a: &mut S, b: u64, c: address): (&mut S, address, u64) {
        (a, c, b)
    }

    fun foo(a: address, ref1: &mut S, ref2: &mut S): u64 {
        let (r1, b, c) = f1(ref1, 1, a);
        f2(b, r1, b, c, ref2);
        f3(c, &r1.g);
        c
    }

}
