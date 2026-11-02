// flag: --trace
// flag: --verify-only=verified_entry
// Companion to exists_only_memory.move, covering the same resource-registration
// gap under auto-trace, which reaches memory through TraceGlobalMem rather than
// through the resolved closure body. Verified as a real detector: with the
// Exists arm removed from mono_analysis both files fail with
// `undeclared identifier: ..Probed_$memory`.

module 0x42::exists_only_memory_traced {
    struct Probed has key { dummy: u8 }

    #[persistent]
    fun probe_impl(_a: address): bool {
        exists<Probed>(@0x42)
    }

    public fun verified_entry(a: address): bool {
        let f: |address| bool has copy + drop = probe_impl;
        f(a)
    }
}
