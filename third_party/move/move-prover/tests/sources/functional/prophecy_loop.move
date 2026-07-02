// also_include_for: prophecy
module 0x42::prophecy_loop {
    struct S has drop { f: u64, g: u64 }

    // A borrow created and resolved within the loop body (not live across the
    // back-edge). The other field stays invariant.
    fun loop_body(): u64 {
        let s = S { f: 0, g: 5 };
        let i = 0;
        while (i < 3) {
            let r = &mut s.f;
            *r = i;
            i = i + 1;
        } spec {
            invariant i <= 3;
            invariant s.g == 5;
        };
        s.g
    }
    spec loop_body {
        ensures result == 5;
    }
}
