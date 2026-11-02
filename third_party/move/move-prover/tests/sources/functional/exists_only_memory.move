// flag: --verify-only=verified_entry
module 0x42::exists_only_memory {
    struct Probed has key { dummy: u8 }

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
