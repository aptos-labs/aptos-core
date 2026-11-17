// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_fwd_opaque_dep.move
// flag: tests/sources/functional/closures/fun_param_frame_dependency_fwd_opaque_hof.move
// exclude_for: lean
// A dependency calls the transparent target `fwd` with a writer, unchecked, and `fwd` forwards
// its undeclared parameter to an opaque callee: the parameter is not bounded by its empty frame.
// Expected: ERR at `fwd`'s call to `opq` (the false claim in `h` is not verified).
module 0x42::frame_fwdo_client {
    public fun h() { 0x42::frame_fwdo_dep::wrapper() }
    spec h { pragma aborts_if_is_partial; ensures global<0x42::frame_fwdo_hof::R>(@0x42) == old(global<0x42::frame_fwdo_hof::R>(@0x42)); }
}
