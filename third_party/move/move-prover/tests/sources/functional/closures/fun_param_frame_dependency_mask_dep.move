// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_mask_hof.move
// Dependency of fun_param_frame_dependency_mask.
// As a target on its own: ERR at `wrapper`'s call to `forward` (empty frame of `g`).
module 0x42::frame_mask_dep {
    struct R has key { v: u64 }
    native fun writer(x: u64);
    spec writer { pragma opaque; modifies global<R>(@0x42); aborts_if false; }
    public fun wrapper() { 0x42::frame_mask_hof::forward(writer); }
}
