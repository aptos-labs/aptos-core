// hash_to domain separation tags at the 255-byte limit of RFC 9380 section 5.3.3, on the G1
// and G2 suites: a DST over 255 bytes is replaced by SHA-256("H2C-OVERSIZE-DST-" || DST), so a
// 256-byte DST hashes like its digest and a 255-byte one does not. An empty DST is accepted.

// RUN: publish
module 0x1::bls12381_algebra {
    struct G1 {}
    struct G2 {}
    struct FormatG1Uncompr {}
    struct FormatG2Uncompr {}
    struct HashG1XmdSha256SswuRo {}
    struct HashG2XmdSha256SswuRo {}
}

module 0x1::crypto_algebra {
    use std::hash::sha2_256;
    use 0x1::bls12381_algebra as bls;

    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun hash_to_internal<S, H>(dst: &vector<u8>, bytes: &vector<u8>): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;

    fun bits(checks: vector<bool>): vector<u64> {
        checks.map(|check| if (check) 1 else 0)
    }

    fun dst_of_length(length: u64): vector<u8> {
        let dst = vector[];
        while (dst.length() < length) dst.push_back(0x61);
        dst
    }

    fun oversize_digest(dst: vector<u8>): vector<u8> {
        let input = b"H2C-OVERSIZE-DST-";
        input.append(dst);
        sha2_256(input)
    }

    // hash_to(dst, msg) == hash_to(oversize_digest(dst), msg).
    fun matches_digest<Group, Suite>(length: u64): bool {
        let dst = dst_of_length(length);
        eq_internal<Group>(
            hash_to_internal<Group, Suite>(&dst, &b"abc"),
            hash_to_internal<Group, Suite>(&oversize_digest(dst), &b"abc")
        )
    }

    public fun oversize_dst(): vector<u64> {
        bits(vector[
            matches_digest<bls::G1, bls::HashG1XmdSha256SswuRo>(256),
            matches_digest<bls::G1, bls::HashG1XmdSha256SswuRo>(255),
            matches_digest<bls::G2, bls::HashG2XmdSha256SswuRo>(256),
            matches_digest<bls::G2, bls::HashG2XmdSha256SswuRo>(255),
        ])
    }

    public fun empty_dst(): (vector<u8>, vector<u8>) {
        (
            serialize_internal<bls::G1, bls::FormatG1Uncompr>(
                hash_to_internal<bls::G1, bls::HashG1XmdSha256SswuRo>(&b"", &b"abc")
            ),
            serialize_internal<bls::G2, bls::FormatG2Uncompr>(
                hash_to_internal<bls::G2, bls::HashG2XmdSha256SswuRo>(&b"", &b"abc")
            ),
        )
    }
}

// RUN: execute 0x1::crypto_algebra::oversize_dst
// CHECK: results: [1, 0, 1, 0]

// RUN: execute 0x1::crypto_algebra::empty_dst
// CHECK: results: 0x1460d20bee1576e29c7dacb4449dc291eb9842fa897217b6d3b1c877ac684c887b76a8780fa3303d65613d0934a81c5105180b4855b159d0b4046e85b452b9453219eedc576c69d2c5bf88f17719e3af5e431e55948681867cb3f76f424d967c, 0x16ff21f33d81085a6482cd718fd5dcec7d430638dfa25f88e5d81a08dddc9672459ad2ce3a5b3c88b3b2c143a14aa0ad0c16dec11b3b0a038e70bc8eca1032de20c40ac2b7e203dc605494a1469748aea817f8324a82cbb30c2f70cf03b1734b0411acc832e237151ac8ad2c27f07ca3edfb5776b31f8eddc8c9386b96b9fb3f417e912eb40bfa9c67ca254c65b2e2010b5b505320d618e3cab1e60f9094df1804f769941401ddc7b82538c8362494d05329de823310b0311b09129d7880bd01
