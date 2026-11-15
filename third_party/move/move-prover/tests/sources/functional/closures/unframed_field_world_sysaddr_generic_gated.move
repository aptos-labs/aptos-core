// A generic resource every write of which, in every instantiation, is at the framework address
// behind a check on the caller's signer stays unchanged where unknown code runs, for a named
// and for an abstract instantiation. Each function states its required verdict.
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
module 0x1::account {
    use 0x1::create_signer;
    struct SignerCapability has drop, store { account: address }
    public fun create_signer_with_capability(capability: &SignerCapability): signer {
        create_signer::create_signer(capability.account)
    }
}
module 0x1::sa_gated_gen {
    use std::signer;
    use 0x1::system_addresses;
    struct G<phantom T> has key { v: u64 }
    public fun init<T>(fw: &signer) {
        system_addresses::assert_aptos_framework(fw);
        move_to(fw, G<T> { v: 0 })
    }
    public fun set<T>(fw: &signer, v: u64) acquires G {
        system_addresses::assert_aptos_framework(fw);
        G<T>[signer::address_of(fw)].v = v;
    }
}
module 0x60::user {
    use 0x1::sa_gated_gen;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // Verifies.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated_gen::G<u64>>(@0x1);
        ensures global<sa_gated_gen::G<u64>>(@0x1) == old(global<sa_gated_gen::G<u64>>(@0x1));
    }

    // Verifies.
    public fun gated_invoke_gen<T>(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke_gen {
        requires exists<sa_gated_gen::G<T>>(@0x1);
        ensures global<sa_gated_gen::G<T>>(@0x1) == old(global<sa_gated_gen::G<T>>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
