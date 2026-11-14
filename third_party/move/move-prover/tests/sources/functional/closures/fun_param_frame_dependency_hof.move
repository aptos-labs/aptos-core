// A target module of fun_param_frame_dependency_user whose transparent HOF a dependency calls.
module 0x42::frame_hof {
    // Transparent; no frame declared for `f`.
    public fun apply(f: |u64| has drop) { f(0) }
}
