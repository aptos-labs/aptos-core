// flag: --dependency=tests/sources/functional/closures/fun_param_frame_dependency_forward_dep.move
// flag: tests/sources/functional/closures/fun_param_frame_dependency_forward_hof.move
// exclude_for: lean
// As fun_param_frame_dependency_forward, through `fwd_cap`, which captures its parameter. ERR at
// `h`'s call to `call_g`, and at `fwd_cap`'s `apply(|x| g(x))` (a dependency calls `fwd_cap`).
module 0x42::frame_dep_fwd_cap_client {
    use 0x42::frame_dep_fwd::{Self, R};

    // Leaves the frame of `g` to its call sites.
    fun call_g(g: |u64| u64 has copy + drop, x: u64): u64 { g(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // False: the closure writes R.
    public fun h(): u64 { call_g(frame_dep_fwd::wrapper_cap, 1) }
    spec h {
        pragma aborts_if_is_partial;
        ensures global<R>(@0x42).v == old(global<R>(@0x42).v);
    }
}
