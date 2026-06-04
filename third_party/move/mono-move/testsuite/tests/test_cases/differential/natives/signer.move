// Differential test for the signer natives.

// `create_signer` is declared here for two reasons:
// 1. To avoid pulling in the Aptos Framework, which is a bit more heavy weight for now
// 2. The version defined in the Aptos Framework is public(friend) so we can't use it anyway

// RUN: publish
module 0x1::create_signer {
    public native fun create_signer(addr: address): signer;
}
module 0x1::permissioned_signer {
    public native fun is_permissioned_signer_impl(s: &signer): bool;
}
module 0x1::main {
    public fun roundtrip(addr: address): address {
        let s = 0x1::create_signer::create_signer(addr);
        *std::signer::borrow_address(&s)
    }

    // A signer made by `create_signer` is a master (non-permissioned) signer,
    // so this is always false.
    public fun is_permissioned(addr: address): bool {
        let s = 0x1::create_signer::create_signer(addr);
        0x1::permissioned_signer::is_permissioned_signer_impl(&s)
    }

    public fun eq(a: address, b: address): bool {
        let s = 0x1::create_signer::create_signer(a);
        let t = 0x1::create_signer::create_signer(b);
        s == t
    }

    public fun neq(a: address, b: address): bool {
        let s = 0x1::create_signer::create_signer(a);
        let t = 0x1::create_signer::create_signer(b);
        s != t
    }

    public fun sel(a: address, b: address): u64 {
        let s = 0x1::create_signer::create_signer(a);
        let t = 0x1::create_signer::create_signer(b);
        if (&s == &t) { 10 } else { 20 }
    }
}

// RUN: execute 0x1::main::roundtrip --args 0xcafe
// CHECK: results: 0xcafe

// RUN: execute 0x1::main::roundtrip --args 0x0
// CHECK: results: 0x0

// RUN: execute 0x1::main::roundtrip --args 0x123456789abcdef
// CHECK: results: 0x123456789abcdef

// RUN: execute 0x1::main::is_permissioned --args 0xcafe
// CHECK: results: false

// RUN: execute 0x1::main::eq --args 0xcafe, 0xcafe
// CHECK: results: true

// RUN: execute 0x1::main::eq --args 0xcafe, 0x1
// CHECK: results: false

// RUN: execute 0x1::main::neq --args 0xcafe, 0x1
// CHECK: results: true

// RUN: execute 0x1::main::neq --args 0x7, 0x7
// CHECK: results: false

// RUN: execute 0x1::main::sel --args 0x9, 0x9
// CHECK: results: 10

// RUN: execute 0x1::main::sel --args 0x9, 0xa
// CHECK: results: 20
