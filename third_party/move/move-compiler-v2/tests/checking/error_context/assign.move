module 0x42::m {

    struct S {
        x: u64,
        y: u8
    }

    struct R {
        z: u16,
        s: S
    }

    fun assign_1(r: R, s: S) {
        let x; let y; let z; let s;
        (S{x, y}, R{z, s}) = (r, s);
    }

    fun assign_2() {
        let x; let y;
        (x, y) = (1, 2, 3);
    }

    fun assign_3(s: &mut S, x: &mut u64) {
        s.x = true;
        *x = 1u8;
    }

    fun assign_4(s1: S, s2: S) {
        let r = &mut s1;
        r = &s2;
    }

}
