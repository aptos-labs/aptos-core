module 0x42::assign {

    struct S {
        f: u64,
        g: T
    }

    struct T {
        h: u64
    }

    fun assign_int(x: &mut u64) {
       *x = 42;
    }

    fun assign_struct(s: &mut S) {
        *s = S { f: 42, g: T { h: 42 } };
    }

    fun assign_pattern(s: S, f: u64, h: u64): u64 {
        S { f, g: T { h } } = s;
        f + h
    }

    fun assign_field(s: &mut S, f: u64) {
        s.f = f;
    }
}
