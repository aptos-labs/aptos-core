// Target of fun_param_frame_dependency_fwd_opaque; used by its dependency _fwd_opaque_dep.
// `fwd` forwards its undeclared parameter to the opaque `opq`. As a target on its own, with no
// dependency using it, every caller of `fwd` is checked and the file verifies.
module 0x42::frame_fwdo_hof {
    struct R has key { v: u64 }
    public fun bump() acquires R { R[@0x42].v = R[@0x42].v + 1 }
    fun opq(g: |u64| has drop) { g(0) }
    spec opq { pragma opaque; pragma aborts_if_is_partial; ensures global<R>(@0x42) == old(global<R>(@0x42)); }
    public fun fwd(g: |u64| has drop) { opq(g) }
}
