// Differential test for the `aptos_hash` natives.
//
// The aptos-framework is not pre-published here, so the natives are re-declared
// inline (same pattern as signer.move).
//
// TODO: re-declaring the natives inline can drift from their real framework
// signatures, a known source of hidden bugs (e.g. renaming a parameter can
// silently enable receiver-style calls). Replace this by publishing the real
// framework, gated behind a natives-only include directive.

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
    public fun sip_empty(): u64 { sip_hash(b"") }
    public fun sip_abc(): u64 { sip_hash(b"abc") }
    public fun sha2_512_empty(): vector<u8> { sha2_512_internal(b"") }
    public fun sha2_512_abc(): vector<u8> { sha2_512_internal(b"abc") }
    public fun sha3_512_empty(): vector<u8> { sha3_512_internal(b"") }
    public fun sha3_512_abc(): vector<u8> { sha3_512_internal(b"abc") }
    public fun ripemd160_empty(): vector<u8> { ripemd160_internal(b"") }
    public fun ripemd160_abc(): vector<u8> { ripemd160_internal(b"abc") }
    public fun blake2b_256_empty(): vector<u8> { blake2b_256_internal(b"") }
    public fun blake2b_256_abc(): vector<u8> { blake2b_256_internal(b"abc") }
}

// RUN: execute 0x1::aptos_hash::keccak_empty
// CHECK: results: 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470

// RUN: execute 0x1::aptos_hash::keccak_abc
// CHECK: results: 0x4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45

// RUN: execute 0x1::aptos_hash::sip_empty
// CHECK: results: 2202906307356721367

// RUN: execute 0x1::aptos_hash::sip_abc
// CHECK: results: 4596069200710135518

// RUN: execute 0x1::aptos_hash::sha2_512_empty
// CHECK: results: 0xcf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e

// RUN: execute 0x1::aptos_hash::sha2_512_abc
// CHECK: results: 0xddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f

// RUN: execute 0x1::aptos_hash::sha3_512_empty
// CHECK: results: 0xa69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26

// RUN: execute 0x1::aptos_hash::sha3_512_abc
// CHECK: results: 0xb751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0

// RUN: execute 0x1::aptos_hash::ripemd160_empty
// CHECK: results: 0x9c1185a5c5e9fc54612808977ee8f548b2258d31

// RUN: execute 0x1::aptos_hash::ripemd160_abc
// CHECK: results: 0x8eb208f7e05d987a9b044a8e98c6b087f15a0bfc

// RUN: execute 0x1::aptos_hash::blake2b_256_empty
// CHECK: results: 0x0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8

// RUN: execute 0x1::aptos_hash::blake2b_256_abc
// CHECK: results: 0xbddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319
