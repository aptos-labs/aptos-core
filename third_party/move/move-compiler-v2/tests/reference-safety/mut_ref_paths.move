module 0x42::m {
    struct S { x: u64, y: u64 }

    fun simple_rejoin(c: bool, s: &mut S): &mut u64 {
        // Result reference rejoins directly
        if (c) f(s) else f(s)
    }

    fun f(s: &mut S): &mut u64 {
        &mut s.x
    }

    fun nested_rejoin(c: bool, s: &mut S): &mut u64 {
        let r = if (c) g(s) else g(s);
        if (!c) &mut r.x else &mut r.y

    }

    fun g(s: &mut S): &mut S {
        s
    }
}
