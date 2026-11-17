// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_dep.move
// `wrapper_capture` captures a native writer for `lam`'s `g` in a closure handed to `apply`.
// Nothing checks that capture in a dependency, so neither `lam` nor `apply` has a bounded
// invoke there and the closure over `wrapper_capture` has no known footprint.
// Expected: ERR at `h`'s call to `call_g` (the false claim is not verified).
module 0x42::frame_dep_capture_client {
    use 0x42::frame_dep::{Self, R};

    // Leaves the frame of `g` to its call sites.
    fun call_g(g: |u64| u64 has copy + drop, x: u64): u64 { g(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // False: the closure writes R.
    public fun h(): u64 { call_g(frame_dep::wrapper_capture, 1) }
    spec h {
        pragma aborts_if_is_partial;
        ensures global<R>(@0x42).v == old(global<R>(@0x42).v);
    }
}
