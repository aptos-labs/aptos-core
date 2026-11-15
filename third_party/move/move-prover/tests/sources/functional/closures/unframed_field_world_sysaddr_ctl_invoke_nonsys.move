// A public function invokes a known function value minting a signer for a hash-derived address and
// returns it: not a system address, and the rule stays on.
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

// Stand-in for the framework's `account` resource addresses.
module 0x1::account {
    use 0x1::create_signer;
    public fun create_resource_address(source: &address, seed: vector<u8>): address {
        if (*source == @0x0 && seed == vector[]) @0x1235 else @0x1234
    }
    public fun resource_signer(seed: vector<u8>): signer {
        let f: |vector<u8>| signer = |s| create_signer::create_signer(create_resource_address(&@0x1, s));
        f(seed)
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

    // The callback holds no signer for @0x1, so it cannot call `sa_gated::set`. Verified.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
