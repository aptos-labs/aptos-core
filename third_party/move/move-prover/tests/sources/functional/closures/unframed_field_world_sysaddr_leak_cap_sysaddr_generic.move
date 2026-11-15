// As `leak_cap_sysaddr_to_invoke`, through a generic helper instantiated with the capability: the
// rule is off.
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
module 0x1::account {
    use 0x1::create_signer;
    friend 0x1::sa_leak;
    struct SignerCapability has drop, store { account: address }
    public fun create_signer_with_capability(capability: &SignerCapability): signer {
        create_signer::create_signer(capability.account)
    }
    // Stand-in for create_framework_reserved_account's capability half.
    public(friend) fun reserved_cap(addr: address): SignerCapability { SignerCapability { account: addr } }
}

module 0x1::sa_leak {
    use 0x1::account::{Self, SignerCapability};
    // A capability for @0x1 (constant) handed to a received function value.
    fun pass<T>(g: |T|, x: T) { g(x) }
    public fun leak(g: |SignerCapability|) { pass<SignerCapability>(g, account::reserved_cap(@0x1)) }
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

    // The callback may obtain a signer for @0x1 through `sa_leak::leak`. Rejected.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
