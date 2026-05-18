// New grammar: `name in <value>` and `name != <value>` build a fuzz spec.
// Parser acceptance is asserted by the absence of a parse error; with no
// FuzzValueSource registered the planner reports a clear diagnostic.
module 0x1::M {
    #[test(_a != @0x42)]
    public fun ne_single(_a: signer) { }

    #[test(_a != [@0x42, @0x41])]
    public fun ne_list(_a: signer) { }

    #[test(_a in [@0x1, @0x2, @0x3])]
    public fun in_list(_a: signer) { }

    #[test(_a in 1..=10)]
    public fun in_inclusive_range(_a: signer) { }

    #[test(_a in 1..10)]
    public fun in_half_open_range(_a: signer) { }

    #[test(_a in @0x1 | @0x5..=@0x10 | @0x20)]
    public fun in_union(_a: signer) { }

    // Combining `in` and `!=` on the same parameter is allowed; the domain
    // narrows and the exclude set accumulates.
    #[test(_a in [@0x1, @0x2, @0x3], _a != @0x2)]
    public fun in_with_excludes(_a: signer) { }
}
