// Every write of `R` is at a signer two helpers deep mint for @0x1, behind a check: the rule
// keeps `R`.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
}

module 0x1::create_signer {
    friend 0x1::sa_gated;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}

module 0x1::sa_gated {
    use std::signer;
    use 0x1::system_addresses;
    use 0x1::create_signer;
    struct R has key { v: u64 }
    fun mk0(): signer { create_signer::create_signer(@0x1) }
    fun mk(): signer { mk0() }
    fun write() acquires R { let s = mk(); R[signer::address_of(&s)].v = 7; }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public fun set(fw: &signer) acquires R { system_addresses::assert_aptos_framework(fw); write() }
}

module 0x60::user {
    use 0x1::sa_gated;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // The callback holds no signer for @0x1. Verifies.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
