// Differential test for the signer natives.

// `create_signer` is declared here to avoid pulling in the Aptos Framework,
// which is a bit more heavy weight for now.

// RUN: publish
module 0x1::create_signer {
    public native fun create_signer(addr: address): signer;
}
module 0x1::main {
    public fun roundtrip(addr: address): address {
        let s = 0x1::create_signer::create_signer(addr);
        *std::signer::borrow_address(&s)
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
