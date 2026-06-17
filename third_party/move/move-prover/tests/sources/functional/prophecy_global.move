// also_include_for: prophecy
module 0x42::prophecy_global {
    struct R has key, drop { v: u64, w: u64 }

    fun set(a: address) acquires R {
        let r = &mut R[a].v;
        *r = 42;
    }
    spec set {
        aborts_if !exists<R>(a);
        ensures global<R>(a).v == 42;
        ensures global<R>(a).w == old(global<R>(a).w);
    }
}
