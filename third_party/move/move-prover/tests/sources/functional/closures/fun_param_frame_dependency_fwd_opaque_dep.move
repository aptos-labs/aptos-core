// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_fwd_opaque_hof.move
// Dependency of fun_param_frame_dependency_fwd_opaque.
// As a target on its own: ERR at `wrapper`'s call to `fwd` (empty frame of `g`).
module 0x42::frame_fwdo_dep {
    fun writer(_x: u64) { 0x42::frame_fwdo_hof::bump() }
    public fun wrapper() { 0x42::frame_fwdo_hof::fwd(writer) }
}
