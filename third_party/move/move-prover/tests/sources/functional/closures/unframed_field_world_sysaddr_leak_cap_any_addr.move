// A false claim: the system-address rule must be off here, so `gated_invoke` is rejected.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
    public fun assert_core_resource(s: &signer) { assert!(signer::address_of(s) == @0xA550C18, 2) }
}

module 0x1::create_signer {
    friend 0x1::account;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}

// Stand-in for the framework's `account` capability.
module 0x1::any {
    struct Any has drop, store { data: vector<u8> }
    public native fun pack<T: drop + store>(x: T): Any;
    spec pack { pragma opaque; }
    public native fun unpack<T>(self: Any): T;
    spec unpack { pragma opaque; }
}
module 0x1::account {
    use 0x1::create_signer;
    use 0x1::any::{Self, Any};
    struct SignerCapability has drop, store { account: address }
    public fun create_signer_with_capability(capability: &SignerCapability): signer {
        create_signer::create_signer(capability.account)
    }
    public fun create_resource_address(source: &address, seed: vector<u8>): address {
        if (*source == @0x0 && seed == vector[]) @0x1235 else @0x1234
    }
    // A capability for whatever address the caller packed: an address is not sensitive, so
    // unknown code may pack a system one.
    public fun from_any(a: Any): SignerCapability {
        SignerCapability { account: any::unpack<address>(a) }
    }
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

    // The callback holds no signer for @0x1, so it cannot call `sa_gated::set`; but the rule is off. Rejected.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
