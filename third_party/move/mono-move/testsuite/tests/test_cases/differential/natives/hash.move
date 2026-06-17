// Differential test for the `std::hash` natives (sha2_256, sha3_256).
//
// `std::hash` is part of the Move stdlib, which is pre-published into both VMs,
// so it can be called directly. Inputs are built as byte-string literals since
// the test harness only passes integer/bool/address arguments. Expected values
// are the standard NIST test vectors; `// CHECK` asserts both VMs agree.

// RUN: publish
module 0x1::main {
    use std::hash;

    public fun sha2_empty(): vector<u8> { hash::sha2_256(b"") }
    public fun sha2_abc(): vector<u8> { hash::sha2_256(b"abc") }
    public fun sha3_empty(): vector<u8> { hash::sha3_256(b"") }
    public fun sha3_abc(): vector<u8> { hash::sha3_256(b"abc") }
}

// RUN: execute 0x1::main::sha2_empty
// CHECK: results: 0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

// RUN: execute 0x1::main::sha2_abc
// CHECK: results: 0xba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

// RUN: execute 0x1::main::sha3_empty
// CHECK: results: 0xa7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a

// RUN: execute 0x1::main::sha3_abc
// CHECK: results: 0x3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532
