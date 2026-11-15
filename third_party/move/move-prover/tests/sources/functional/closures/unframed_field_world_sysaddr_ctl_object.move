// Control: a public function returns the signer of an object it creates, whose address is
// hash-derived: the rule stays on. `0x1::object` and `0x1::transaction_context` are stand-ins for the
// framework modules.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
    public fun assert_core_resource(s: &signer) { assert!(signer::address_of(s) == @0xA550C18, 2) }
}

module 0x1::create_signer {
    friend 0x1::object;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}

module 0x1::transaction_context {
    public native fun generate_auid_address(): address;
    spec generate_auid_address { pragma opaque; }
}

module 0x1::object {
    use 0x1::create_signer;
    use 0x1::transaction_context;
    struct ConstructorRef has drop { self: address }
    public fun create_object(): ConstructorRef {
        ConstructorRef { self: transaction_context::generate_auid_address() }
    }
    public fun generate_signer(self: &ConstructorRef): signer { create_signer::create_signer(self.self) }
}

module 0x1::sa_leak {
    use 0x1::object;
    public fun fresh(): signer { object::generate_signer(&object::create_object()) }
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

    // The callback holds no signer for @0x1. Verifies.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
