module 0x42::signed {
    // Every signed width appears in a signature, so `translate_type` is
    // exercised for all six.
    fun widths(a: i8, b: i16, c: i32, d: i64, e: i128, f: i256): i256 {
        let _ = a;
        let _ = b;
        let _ = c;
        let _ = d;
        let _ = e;
        f
    }

    // Width-annotated arithmetic on a signed type: every operation should
    // carry a signed `IntType`.
    fun arith(x: i64, y: i64): i64 {
        let sum = x + y;
        let diff = sum - y;
        let prod = diff * y;
        let quot = prod / y;
        quot % y
    }

    // Unary negation has no `neg` operation in the format; the producer
    // normalizes it to `0 - x`.
    fun negate(x: i64): i64 {
        -x
    }

    // Negation of a literal, and a negative literal, which differ in where
    // the constant is introduced.
    fun literals(): i64 {
        let a = -5i64;
        let b = -a;
        a + b
    }

    // Casts across signedness in both directions.
    fun casts(x: i64, y: u64): (i32, u32, i128) {
        ((x as i32), (y as u32), (x as i128))
    }

    // Comparison and equality on signed operands; `gt` normalizes to `lt`
    // with swapped operands, `neq` to `eq` + `not`.
    fun compare(x: i64, y: i64): bool {
        if (x > y) { x != y } else { x <= y }
    }
}
