// Two disjoint dead regions in the same function — should produce two
// separate warnings, not one merged span.
#[lint::skip(needless_return)]
module 0xc0ffee::m {
    public fun test(p: bool): u64 {
        if (p) {
            return 1;
            let x = 1;     // dead region 1
            x + 1
        } else {
            abort 0;
            let y = 2;     // dead region 2
            y + 2
        }
    }
}
