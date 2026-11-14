// Target of fun_param_frame_dependency_mask; used by its dependency _mask_dep.
// `forward` captures its own parameter into a closure over the opaque `apply` and hands the
// closure to `deferred`, which leaves the frame to the call site. As a target on its own, with
// no dependency using it, the capture bounds the closure and the file verifies.
module 0x42::frame_mask_hof {
    public fun apply(g: |u64| has copy + drop, x: u64) { g(x) }
    spec apply { pragma opaque; pragma aborts_if_is_partial; }

    fun deferred(f: |u64| has copy + drop, x: u64) { f(x) }
    spec deferred { pragma opaque; pragma verify = false; aborts_if false; }

    public fun forward(g: |u64| has copy + drop) {
        deferred(|x| apply(g, x), 0);
    }
}
