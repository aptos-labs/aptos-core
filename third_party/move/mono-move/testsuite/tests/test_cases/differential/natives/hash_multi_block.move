// Hash natives over inputs longer than one block: `len` bytes, each equal to its index mod
// 256, at one length crossing one block boundary and one crossing several (64-byte blocks for
// sha2_256 and ripemd160, 128 for sha2_512 and blake2b_256, 136 and 72 for keccak256 and
// sha3_512, 8-byte words for sip_hash). Expected digests come from Python hashlib (keccak256
// and sip_hash from reference implementations).

// RUN: publish
module 0x1::aptos_hash {
    native fun keccak256(bytes: vector<u8>): vector<u8>;
    native fun sip_hash(bytes: vector<u8>): u64;
    native fun sha2_512_internal(bytes: vector<u8>): vector<u8>;
    native fun sha3_512_internal(bytes: vector<u8>): vector<u8>;
    native fun ripemd160_internal(bytes: vector<u8>): vector<u8>;
    native fun blake2b_256_internal(bytes: vector<u8>): vector<u8>;

    fun pattern(len: u64): vector<u8> {
        let bytes = vector[];
        let pos = 0;
        while (pos < len) {
            bytes.push_back((pos % 256) as u8);
            pos += 1;
        };
        bytes
    }

    public fun sha2_256_of(len: u64): vector<u8> { std::hash::sha2_256(pattern(len)) }
    public fun ripemd160_of(len: u64): vector<u8> { ripemd160_internal(pattern(len)) }
    public fun sha2_512_of(len: u64): vector<u8> { sha2_512_internal(pattern(len)) }
    public fun blake2b_256_of(len: u64): vector<u8> { blake2b_256_internal(pattern(len)) }
    public fun keccak256_of(len: u64): vector<u8> { keccak256(pattern(len)) }
    public fun sha3_512_of(len: u64): vector<u8> { sha3_512_internal(pattern(len)) }
    public fun sip_hash_of(len: u64): u64 { sip_hash(pattern(len)) }
}

// RUN: execute 0x1::aptos_hash::sha2_256_of --args 65
// CHECK: results: 0x4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781

// RUN: execute 0x1::aptos_hash::sha2_256_of --args 200
// CHECK: results: 0x1901da1c9f699b48f6b2636e65cbf73abf99d0441ef67f5c540a42f7051dec6f

// RUN: execute 0x1::aptos_hash::ripemd160_of --args 65
// CHECK: results: 0x109949b95341eeea7365e8ac4d0d3883d98f709a

// RUN: execute 0x1::aptos_hash::ripemd160_of --args 200
// CHECK: results: 0xc315823ea8fe07a2dd18de4e545255afe3af0738

// RUN: execute 0x1::aptos_hash::sha2_512_of --args 129
// CHECK: results: 0x1d9da57fbbdab09afb3506ab2d223d06109d65c1c8ad197f50138f714bc4c3f2fe5787922639c680acad1c651f955990425954ce2cba0c5cc83f2667d878eb0f

// RUN: execute 0x1::aptos_hash::sha2_512_of --args 500
// CHECK: results: 0xf8d1c917da2e66be92022d6a49ecee0f0d897c6d97927adc654c73d735dca420dd808be06794f514d5a88fa2e24c0c0f0817c95ad448371b4cd6a8cb6aa656b3

// RUN: execute 0x1::aptos_hash::blake2b_256_of --args 129
// CHECK: results: 0xf7f3c46ba2564ff4c4c162da1f5b605f9f1c4aa6a20652a9f9a337c1a2f5b9c9

// RUN: execute 0x1::aptos_hash::blake2b_256_of --args 500
// CHECK: results: 0xd91f34aa1d9c881a9f0c1bfc6cbc875069c7824fd47eccbc8aefdd053798d497

// RUN: execute 0x1::aptos_hash::keccak256_of --args 137
// CHECK: results: 0xac73d4fae68b8453f764007c1a20ce95994187861f0c3227a3a8e99a73a3b1db

// RUN: execute 0x1::aptos_hash::keccak256_of --args 500
// CHECK: results: 0xcbfabf79afab5860388c0abad0004bfbf11a8be32b02427078883c926c25745b

// RUN: execute 0x1::aptos_hash::sha3_512_of --args 73
// CHECK: results: 0x921d9b7b2b0f3066a1646dbb058c979cb3925dec0f8c269faaa7f9648e73465ae55ec527257d5d5e1cfdbf5d6799bea1004b6186f5108c74e3b92fe924166558

// RUN: execute 0x1::aptos_hash::sha3_512_of --args 300
// CHECK: results: 0xfa288fe9f54b8301e3012051fb1b275fd3f278a281ef149bb878fd322a647d3f51dc24908905550ed4883870c94f8d297f0690f8661b14d8222e9a46eebcbdf6

// RUN: execute 0x1::aptos_hash::sip_hash_of --args 9
// CHECK: results: 6993662196368935714

// RUN: execute 0x1::aptos_hash::sip_hash_of --args 100
// CHECK: results: 10694629389119761995
