// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_forward_hof.move
// Dependency of fun_param_frame_dependency_forward and _forward_capture.
// As a target on its own: ERR at `wrapper`'s call to `fwd` and `wrapper_cap`'s call to
// `fwd_cap` (empty frame of `g`).
module 0x42::frame_dep_fwd {
    use 0x42::frame_fwd_hof;

    struct R has key { v: u64 }

    native fun native_writer(x: u64);
    spec native_writer {
        pragma opaque;
        modifies global<R>(@0x42);
    }

    public fun wrapper(x: u64): u64 { frame_fwd_hof::fwd(native_writer); x }

    public fun wrapper_cap(x: u64): u64 { frame_fwd_hof::fwd_cap(native_writer); x }
}
