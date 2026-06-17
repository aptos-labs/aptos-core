// Differential test for the `aptos_hash` natives.
//
// The aptos-framework is not pre-published into the test environment, so the
// natives are re-declared inline (same pattern as signer.move). The feature-
// gated hashes use their true `*_internal` native entry points. Inputs are
// byte-string literals; `// CHECK` asserts both VMs agree.

// RUN: publish
module 0x1::aptos_hash {
    public native fun keccak256(bytes: vector<u8>): vector<u8>;
    public native fun sip_hash(bytes: vector<u8>): u64;
    native fun sha2_512_internal(bytes: vector<u8>): vector<u8>;
    native fun sha3_512_internal(bytes: vector<u8>): vector<u8>;
    native fun ripemd160_internal(bytes: vector<u8>): vector<u8>;
    native fun blake2b_256_internal(bytes: vector<u8>): vector<u8>;

    public fun keccak_empty(): vector<u8> { keccak256(b"") }
    public fun keccak_abc(): vector<u8> { keccak256(b"abc") }
    public fun sip(): u64 { sip_hash(b"abc") }
    public fun sha2_512_abc(): vector<u8> { sha2_512_internal(b"abc") }
    public fun sha3_512_abc(): vector<u8> { sha3_512_internal(b"abc") }
    public fun ripemd160_abc(): vector<u8> { ripemd160_internal(b"abc") }
    public fun blake2b_256_abc(): vector<u8> { blake2b_256_internal(b"abc") }
}

// RUN: execute 0x1::aptos_hash::keccak_empty
// CHECK: results: 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470

// RUN: execute 0x1::aptos_hash::keccak_abc
// CHECK: results: 0x4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45

// RUN: execute 0x1::aptos_hash::sip
// CHECK: results: 4596069200710135518

// RUN: execute 0x1::aptos_hash::sha2_512_abc
// CHECK: results: 0xddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f

// RUN: execute 0x1::aptos_hash::sha3_512_abc
// CHECK: results: 0xb751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0

// RUN: execute 0x1::aptos_hash::ripemd160_abc
// CHECK: results: 0x8eb208f7e05d987a9b044a8e98c6b087f15a0bfc

// RUN: execute 0x1::aptos_hash::blake2b_256_abc
// CHECK: results: 0xbddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319
