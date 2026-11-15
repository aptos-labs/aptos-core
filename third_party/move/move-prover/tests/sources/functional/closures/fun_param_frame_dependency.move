// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_dep.move
// `wrapper` passes a native writer to the dependency's transparent `apply`, which declares
// no frame for `f`. Nothing checks that call, so `apply`'s invoke stays unbounded and the
// closure over `wrapper` has no known footprint.
// Expected: ERR at `h`'s call to `call_g` (the false claim is not verified).
module 0x42::frame_dep_client {
    use 0x42::frame_dep::{Self, R};

    // Leaves the frame of `g` to its call sites.
    fun call_g(g: |u64| u64 has copy + drop, x: u64): u64 { g(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // False: the closure writes R.
    public fun h(): u64 { call_g(frame_dep::wrapper, 1) }
    spec h {
        pragma aborts_if_is_partial;
        ensures global<R>(@0x42).v == old(global<R>(@0x42).v);
    }
}
