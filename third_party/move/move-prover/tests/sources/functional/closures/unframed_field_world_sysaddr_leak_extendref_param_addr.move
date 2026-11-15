// An object-style extend ref is built for an address the caller names: the rule is off.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
}
module 0x1::create_signer {
    friend 0x1::object;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}
module 0x1::object {
    use 0x1::create_signer;
    struct ExtendRef has drop, store { self: address }
    public fun generate_signer_for_extending(self: &ExtendRef): signer { create_signer::create_signer(self.self) }
    fun mk(a: address): ExtendRef { ExtendRef { self: a } }
    public fun leak(a: address): ExtendRef { mk(a) }
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
    // Rejected if a system signer may leak; verifies only if the rule stays on.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
