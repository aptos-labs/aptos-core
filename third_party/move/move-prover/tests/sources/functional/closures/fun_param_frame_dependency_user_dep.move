// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_hof.move
// Dependency of fun_param_frame_dependency_user, calling a module that test verifies.
// As a target on its own: ERR at `wrapper`'s call to `apply` (empty frame of `f`).
module 0x42::frame_dep_user {
    use 0x42::frame_hof;

    struct R has key { v: u64 }

    native fun native_writer(x: u64);
    spec native_writer {
        pragma opaque;
        modifies global<R>(@0x42);
    }

    public fun wrapper(x: u64): u64 { frame_hof::apply(native_writer); x }
}
