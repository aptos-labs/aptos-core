// A function value read from a field without a `reads_of`/`modifies_of` declaration, or with
// `modifies_of<f> *`, may touch any memory. It may only reach a parameter of an opaque function
// whose frame is `modifies_of<p> *`: an undeclared parameter frame is empty, and the opaque
// function is verified under it. A parameter may be forwarded: every call site of the forwarding
// function is held to the parameter's frame.
module 0x42::ff {
    struct Own has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    enum CbE has drop, copy { V1 { f: |u64| u64 has copy + drop } }
    struct CbW has drop, copy { f: |u64| u64 has copy + drop }
    spec CbW { modifies_of<f> *; }

    public fun apply_p(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }
    spec apply_p { pragma opaque; ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }

    public fun apply_w(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }
    spec apply_w { pragma opaque; modifies_of<f> *; }

    // Rejected: the field declares no frame.
    public fun fwd_field(c: &Cb, x: u64): u64 { apply_p(c.f, x) }

    // Rejected: an enum field.
    public fun fwd_variant(c: &CbE, x: u64): u64 { apply_p(c.f, x) }

    // Rejected: an unpacked field.
    public fun fwd_unpacked(c: Cb, x: u64): u64 {
        let Cb { f } = c;
        apply_p(f, x)
    }

    // Rejected: a `*` field into an empty frame.
    public fun fwd_wild_field(c: &CbW, x: u64): u64 { apply_p(c.f, x) }

    // Accepted: the call sites of a transparent function are held to its parameter's frame too.
    public fun fwd_param(f: |u64| u64 has copy + drop, x: u64): u64 { apply_p(f, x) }

    // Accepted: the same through an opaque function.
    public fun fwd_param_opaque(f: |u64| u64 has copy + drop, x: u64): u64 { apply_p(f, x) }
    spec fwd_param_opaque { pragma opaque; }

    // Accepted: into a `*` parameter.
    public fun fwd_field_w(c: &Cb, x: u64): u64 { apply_w(c.f, x) }

    public fun fwd_param_w(f: |u64| u64 has copy + drop, x: u64): u64 { apply_w(f, x) }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
