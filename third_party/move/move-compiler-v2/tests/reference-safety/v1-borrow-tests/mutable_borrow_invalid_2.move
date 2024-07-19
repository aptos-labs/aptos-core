// Extracted out of mutable_borrow_invalid to verify in an isolated setting
module 0x8675309::M {
    struct S { f: u64, g: u64, h: u64 }

    fun t1(root: &mut S, cond: bool) {
        let x = if (cond) &mut root.f else &mut root.g;

        // INVALID
        root.f = 1;
        *x;
    }
}
