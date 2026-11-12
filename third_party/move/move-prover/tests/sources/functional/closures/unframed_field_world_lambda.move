// A lambda forwarding an unframed field: the contract derived for it applies a behavioral
// predicate to a function value that may touch any memory, over the lambda's own memory. The
// claim below is false (the field's value may depend on what its first invocation wrote).
module 0x42::lam {
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    public fun via_lambda(c: &Cb, x: u64): (u64, u64) {
        let cc = *c;
        let h: |u64| u64 has copy + drop = |y: u64| (cc.f)(y);
        (h(x), h(x))
    }
    spec via_lambda { ensures result_1 == result_2; }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
