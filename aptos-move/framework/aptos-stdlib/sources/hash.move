/// Cryptographic hashes:
/// - Keccak-256: see https://keccak.team/keccak.html
///
/// In addition, SHA2-256 and SHA3-256 are available in `std::hash`. Note that SHA3-256 is a variant of Keccak: it is
/// NOT the same as Keccak-256.
///
/// Non-cryptograhic hashes:
/// - SipHash: an add-rotate-xor (ARX) based family of pseudorandom functions created by Jean-Philippe Aumasson and Daniel J. Bernstein in 2012
module aptos_std::aptos_hash {
    use std::bcs;

    native public fun sip_hash(bytes: vector<u8>): u64;

    public fun sip_hash_from_value<MoveValue>(v: &MoveValue): u64 {
        let bytes = bcs::to_bytes(v);

        sip_hash(bytes)
    }

    //
    // Native functions
    //

    native public fun keccak256(bytes: vector<u8>): vector<u8>;

    //
    // Testing
    //

    #[test]
    fun keccak256_test() {
        let inputs = vector[
            b"testing",
            b"",
        ];

        let outputs = vector[
            x"5f16f4c7f149ac4f9510d9cf8cf384038ad348b3bcdc01915f95de12df9d1b02",
            x"c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
        ];

        let i = 0;
        while (i < std::vector::length(&inputs)) {
            let input = *std::vector::borrow(&inputs, i);
            let hash_expected = *std::vector::borrow(&outputs, i);
            let hash = keccak256(input);

            assert!(hash_expected == hash, 1);

            i = i + 1;
        };
    }
}
