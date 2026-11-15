// flag: --dependency=tests/sources/functional/closures/fun_param_frame_captured_declared_dep_dep.move
// `wrapper` hands a closure whose footprint is not known to an opaque callee declaring a frame
// covering R2. The havoc at that call covers the declared frame.
// Expected: the false claim fails as a post-condition.
module 0x42::captured_declared_dep_client {
    use 0x42::captured_declared_dep::{Self, R2};

    // False: `wrapper` writes R2.
    fun h(x: u64) { captured_declared_dep::wrapper(x) }
    spec h {
        pragma aborts_if_is_partial;
        ensures global<R2>(@0x42) == old(global<R2>(@0x42));
    }
}
