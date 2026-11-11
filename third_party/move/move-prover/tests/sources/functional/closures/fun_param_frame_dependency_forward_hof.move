// Target modules of fun_param_frame_dependency_forward and _forward_capture: `apply`'s only
// users are target functions, but those forward or capture their own parameter for callers
// in a dependency.
module 0x42::frame_fwd_hof {
    // Transparent; no frame declared for `f`.
    public fun apply(f: |u64| has drop) { f(0) }

    // Forwards its parameter.
    public fun fwd(g: |u64| has drop) { apply(g) }

    // Captures its parameter in a closure handed to `apply`.
    public fun fwd_cap(g: |u64| has copy + drop) { apply(|x| g(x)) }
}
