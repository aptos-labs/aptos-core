//# publish
module 0xc0ffee::m {
    // Every discriminator shape that the match transforms rewrite must evaluate its
    // sub-expressions exactly once, left to right, no matter how many arms and guards
    // are tested. `next` returns the number of calls made so far, so the values seen
    // by the arms reveal the evaluation order and the final counter the count.
    enum Color has drop { Red, Blue }

    fun next(c: &mut u64): u64 { let v = *c; *c = v + 1; v }
    fun color(c: &mut u64): Color { if (next(c) == 0) Color::Red else Color::Blue }
    fun pair(c: &mut u64): (Color, u64) { let col = color(c); (col, next(c)) }

    // Primitive scalar: a call.
    public fun scalar(): u64 {
        let c = 0;
        let r = match (next(&mut c)) {
            0 if (c > 100) => 1,
            0 if (c > 50) => 2,
            0 => 3,
            _ => 4,
        };
        assert!(c == 1);
        r
    }

    // Primitive tuple: two calls.
    public fun prim_tuple(): u64 {
        let c = 0;
        let r = match ((next(&mut c), next(&mut c))) {
            (0, 1) if (c > 100) => 1,
            (1, 0) => 2,
            (0, 1) => 3,
            _ => 4,
        };
        assert!(c == 2);
        r
    }

    // Mixed tuple: an enum call and a primitive call.
    public fun mixed_tuple(): u64 {
        let c = 0;
        let r = match ((color(&mut c), next(&mut c))) {
            (Color::Red, 1) if (c > 100) => 1,
            (Color::Blue, 0) => 2,
            (Color::Red, x) if (x == 1) => 3,
            _ => 4,
        };
        assert!(c == 2);
        r
    }

    // Mixed tuple from a call returning a tuple.
    public fun tuple_call(): u64 {
        let c = 0;
        let r = match (pair(&mut c)) {
            (Color::Red, 1) if (c > 100) => 1,
            (Color::Red, x) if (x == 1) => 3,
            _ => 4,
        };
        assert!(c == 2);
        r
    }
}

//# run 0xc0ffee::m::scalar

//# run 0xc0ffee::m::prim_tuple

//# run 0xc0ffee::m::mixed_tuple

//# run 0xc0ffee::m::tuple_call
