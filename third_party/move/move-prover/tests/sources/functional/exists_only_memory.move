// flag: --verify-only=verified_entry
module 0x42::exists_only_memory {
    struct Probed has key { dummy: u8 }

    // A bare type parameter in a memory access must be tracked as generic memory,
    // not passed to the struct-only memory path.
    public fun generic_exists<T: key>(a: address): bool {
        exists<T>(a)
    }

    spec generic_exists {
        ensures result == exists<T>(a);
    }

    // The only reference to `Probed` anywhere.
    #[persistent]
    fun probe_impl(_a: address): bool {
        exists<Probed>(@0x42)
    }

    // Reaches `probe_impl` only through a closure invocation. `Invoke` is not
    // handled by usage analysis, so `Probed` never enters this function's
    // accessed memory -- yet the translator resolves the closure statically and
    // inlines `probe_impl`, emitting `$ResourceExists(Probed_$memory, ..)`.
    public fun verified_entry(a: address): bool {
        let f: |address| bool has copy + drop = probe_impl;
        f(a)
    }
}
