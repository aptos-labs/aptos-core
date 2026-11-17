// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_mask_dep.move
// flag: tests/sources/functional/closures/fun_param_frame_dependency_mask_hof.move
// exclude_for: lean
// A dependency calls the target `forward` with a native writer, unchecked, so the parameter
// `forward` captures is not bounded by its empty frame and the closure over `apply` has no
// known footprint. Expected: ERR at `forward`'s call to `deferred` (the false claim in
// `test` is not verified).
module 0x42::frame_mask_client {
    use 0x42::frame_mask_dep;
    public fun test() { frame_mask_dep::wrapper(); }
    spec test {
        pragma aborts_if_is_partial;
        ensures global<frame_mask_dep::R>(@0x42) == old(global<frame_mask_dep::R>(@0x42));
    }
}
