// A system address is packed into a generic struct and taken out again by a generic function,
// as a type parameter; a public function hands out a signer minted for it: the rule is off.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
    public fun assert_core_resource(s: &signer) { assert!(signer::address_of(s) == @0xA550C18, 2) }
}

module 0x1::create_signer {
    friend 0x1::sa_leak;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}

module 0x1::sa_leak {
    use 0x1::create_signer;
    struct Holder<T> has drop { x: T }
    fun sys(): Holder<address> { Holder { x: @0x1 } }
    fun get<T: copy>(h: &Holder<T>): T { h.x }
    public fun leak(): signer { create_signer::create_signer(get(&sys())) }
}

module 0x1::sa_gated {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public fun set(fw: &signer, v: u64) acquires R {
        system_addresses::assert_aptos_framework(fw);
        R[signer::address_of(fw)].v = v;
    }
}

module 0x60::user {
    use 0x1::sa_gated;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // The callback may obtain a signer for @0x1 and call `sa_gated::set`. Rejected.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }

    // Control: whatever the callback writes, nothing removes the resource. Verifies.
    public fun gated_exists(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_exists {
        requires exists<sa_gated::R>(@0x1);
        ensures exists<sa_gated::R>(@0x1);
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
