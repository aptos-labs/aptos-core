module supra_std::bls12381_bulletproofs {
    use std::error;
    use std::features;
    use aptos_std::bls12381_algebra::{G1, FormatG1Compr};
    use aptos_std::crypto_algebra::{Element, serialize, one};
    use supra_std::bls12381_pedersen;
    #[test_only]
    use std::option;
    #[test_only]
    use aptos_std::bls12381_algebra::{Fr, FormatFrLsb};
    #[test_only]
    use aptos_std::crypto_algebra::{deserialize, eq};
    #[test_only]
    use supra_std::bls12381_scalar::bls12381_hash_to_scalar;

    //
    // Constants
    //

    /// The maximum range supported by the Bulletproofs library is $[0, 2^{64})$.
    const MAX_RANGE_BITS : u64 = 32;

    //
    // Error codes
    //

    /// There was an error deserializing the range proof.
    const E_DESERIALIZE_RANGE_PROOF: u64 = 1;

    /// The committed value given to the prover is too large.
    const E_VALUE_OUTSIDE_RANGE: u64 = 2;

    /// The range proof system only supports proving ranges of type $[0, 2^b)$ where $b \in \{8, 16, 32, 64\}$.
    const E_RANGE_NOT_SUPPORTED: u64 = 3;

    /// The native functions have not been rolled out yet.
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 4;

    //
    // Structs
    //

    /// Represents a zero-knowledge range proof that a value committed inside a Pedersen commitment lies in
    /// `[0, 2^{MAX_RANGE_BITS})`.
    struct RangeProof has copy, drop, store {
        bytes: vector<u8>
    }

    //
    // Public functions
    //

    /// Returns the maximum # of bits that the range proof system can verify proofs for.
    public fun get_max_range_bits(): u64 {
        MAX_RANGE_BITS
    }

    /// Deserializes a range proof from a sequence of bytes. The serialization format is the same as the format in
    /// the zkcrypto's `bulletproofs` library (https://docs.rs/bulletproofs/4.0.0/bulletproofs/struct.RangeProof.html#method.from_bytes).
    public fun range_proof_from_bytes(bytes: vector<u8>): RangeProof {
        RangeProof {
            bytes
        }
    }

    /// Returns the byte-representation of a range proof.
    public fun range_proof_to_bytes(proof: &RangeProof): vector<u8> {
        proof.bytes
    }

    /// Verifies a zero-knowledge range proof that the value `v` committed in `com` (under the default Bulletproofs
    /// commitment key; see `pedersen::new_commitment_for_bulletproof`) satisfies $v \in [0, 2^b)$. Only works
    /// for $b \in \{8, 16, 32, 64\}$. Additionally, checks that the prover used `dst` as the domain-separation
    /// tag (DST).
    ///
    /// WARNING: The DST check is VERY important for security as it prevents proofs computed for one application
    /// (a.k.a., a _domain_) with `dst_1` from verifying in a different application with `dst_2 != dst_1`.
    public fun verify_range_proof_pedersen(com: &bls12381_pedersen::Commitment, proof: &RangeProof, num_bits: u64, dst: vector<u8>): bool {
        assert!(features::supra_private_poll_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));

        verify_range_proof_internal(
            serialize<G1, FormatG1Compr>(bls12381_pedersen::commitment_as_point(com)),
            serialize<G1, FormatG1Compr>(&one<G1>()),
            serialize<G1, FormatG1Compr>(&bls12381_pedersen::randomness_base_for_bulletproof()),
            proof.bytes,
            num_bits,
            dst
        )
    }

    /// Verifies a zero-knowledge range proof that the value `v` committed in `com` (as v * val_base + r * rand_base,
    /// for some randomness `r`) satisfies `v` in `[0, 2^num_bits)`. Only works for `num_bits` in `{8, 16, 32, 64}`.
    public fun verify_range_proof(
        com: &Element<G1>,
        val_base: &Element<G1>, rand_base: &Element<G1>,
        proof: &RangeProof, num_bits: u64, dst: vector<u8>): bool {
        assert!(features::supra_private_poll_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));

        verify_range_proof_internal(
            serialize<G1, FormatG1Compr>(com),
            serialize<G1, FormatG1Compr>(val_base),
            serialize<G1, FormatG1Compr>(rand_base),
            proof.bytes, num_bits, dst
        )
    }

    //
    // Native functions
    //

    /// Aborts with `error::invalid_argument(E_DESERIALIZE_RANGE_PROOF)` if `proof` is not a valid serialization of a
    /// range proof.
    /// Aborts with `error::invalid_argument(E_RANGE_NOT_SUPPORTED)` if an unsupported `num_bits` is provided.
    native fun verify_range_proof_internal(
        com: vector<u8>,
        val_base: vector<u8>,
        rand_base: vector<u8>,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    //
    // Testing
    //

    #[test_only]
    const A_DST: vector<u8> = x"63727970746f2d626c7331323338312d72616e67652d70726f6f66";
    #[test_only]
    const A_VALUE: vector<u8> = x"c3b1024700000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const A_BLINDER: vector<u8> = x"6d60852d2d767d1df485e93275c4b91fef86a9d98d4a7f1e850177848601441d";
    // Pedersen commitment to A_VALUE with randomness A_BLINDER
    #[test_only]
    const A_COMM: vector<u8> = x"877f5509762ebeaa68e5bf0bbda7df86f4951dae9474aa9ab3b305115b4302b6656536d17557a2e7462fc2405ba98972";
    // Range proof for A_COMM using domain-separation tag in A_DST, and MAX_RANGE_BITS
    #[test_only]
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"aa76b5a2ea1bffb8f313ebf555040301086f2e77b3b358c994190ba8bf4a38da42f8990559f94bd33c36969c0109dfbf98ef172f88ede0e6c7249d9338b0e8c25ea0ed64112c5f6f71baf8ac7757293d10c11d4300bebae305ba7f45c6db3158843556de10893d5d1666199fd50d53d9d2edeabde9b7cffb91fe4642cb3a7200c511037bdb5205575ec2ae477035439fb508ee2174c2bdce6f204a2fd8b4fc46ba41b94d2a4af283f0e2cfd54301bf5ce44024c34bcf4c6100509d161d4bf56b467e30b2daf356640921d4a483a98104cae587eac88b8c667b0120e8307721556a97a75db6f60fe64170688689190f8cec4a67f6138145237c7d5d950668593a3f389c7d649dc85221aa2de574116dc8d5c845265853cae1d64be5ca08034e1fb6368e5463ef0611bb0b56d06e0abedd016c09c918080fb8264d56b80f75e727415ebfca9e6b55f3ab6c1ee901a331eb91efef2b48004891a2473acf887e34330eb0400ff4022ca0ebb6b7c57a4e4f9524285b9dcc2e16f5e575cab68f021b1c8b7f2f2c599df9d29e4f21e72f85f23574c0ee5ff266267bdd3b207003e1ea09a53e6225c1409b0cee9ef1c7291be170b20c7ec4e151d820e288d041d32185bb7245b24d4ae42f72b86cad7f9a0f1021b7dd7274d65a1916fa99b2deb17e0712947f67f7cbe657bed740f5a5b6b8b7cb978e09cb9fbeb2f7268bd635d809068b6814d2877a57f7ce08eb737bfeb3bb7281cddb9c05d02e0e1139d23af7bdbd74687a94cbbe1c702a7bf08279ac9324f3306174a666e0630480e5c2e462daba6c943a6cc14961e96186cc53e1ebf6ec8ce831dbd6113dfc25f9877d3ce9d7a832359968d96f63074768694bd12a80b6fcad9004ead5303dc4d1efae2e9a1d5375b5c02ad6151419f2c51f3d62b1c47869b8f0ba7e41b7ef452c359af727410fb1aea510d379ca5031d4c2784e241da7a5077f96c62a12ce039c13ea2c85fece58695b6f6a4cbef19bacc710e529af263fa7970daf887d31c7cc389181711777dbe9c199c6812e171dab9b06127fa6df16a40285b25e1c876f5183e6efc93e23c13ae7a767e8ca5226b89e6b5e0bd7d8a9ab5b4d0fd7a3374ce58266ef1beae455a98454f73e3816586843bea84314985afbcf71dabe9b0e4397fd8fbedd973c66";

    #[test(fx = @supra_framework)]
    #[expected_failure(abort_code = 0x010003, location = Self)]
    fun test_unsupported_ranges(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[ features::get_supra_private_poll_feature() ], vector[]);

        let comm = deserialize<G1, FormatG1Compr>(&A_COMM);
        let comm = std::option::extract(&mut comm);
        let comm = bls12381_pedersen::commitment_from_point(comm);

        assert!(verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), 10, A_DST), 1);
    }

    #[test(fx = @supra_framework)]
    #[expected_failure(abort_code = 0x010001, location = Self)]
    fun test_empty_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[ features::get_supra_private_poll_feature() ], vector[]);

        let proof = &range_proof_from_bytes(vector[ ]);
        let num_bits = 64;
        let r =  bls12381_hash_to_scalar(vector[], b"hello random world");
        let r = option::extract(&mut r);

        let com = bls12381_pedersen::new_commitment_for_bulletproof(
            &one<Fr>(),
            &r
        );

        // This will fail with error::invalid_argument(E_DESERIALIZE_RANGE_PROOF)
        verify_range_proof_pedersen(&com, proof, num_bits, A_DST);
    }

    #[test(fx = @supra_framework)]
    fun test_valid_range_proof_verifies_against_comm(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[ features::get_supra_private_poll_feature() ], vector[]);

        let value = deserialize<Fr, FormatFrLsb>(&A_VALUE);
        let value = std::option::extract(&mut value);

        let blinder = deserialize<Fr, FormatFrLsb>(&A_BLINDER);
        let blinder = std::option::extract(&mut blinder);

        let comm = bls12381_pedersen::new_commitment_for_bulletproof(&value, &blinder);

        let expected_comm = std::option::extract(&mut deserialize<G1, FormatG1Compr>(&A_COMM));
        assert!(eq(bls12381_pedersen::commitment_as_point(&comm), &expected_comm), 1);

        assert!(verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @supra_framework)]
    #[expected_failure(abort_code = 0x010001, location = Self)]
    fun test_invalid_range_proof_fails_verification(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[ features::get_supra_private_poll_feature() ], vector[]);

        let comm = deserialize<G1, FormatG1Compr>(&A_COMM);
        let comm = std::option::extract(&mut comm);
        let comm = bls12381_pedersen::commitment_from_point(comm);

        // Take a valid proof...
        let range_proof_invalid = A_RANGE_PROOF_PEDERSEN;

        // ...and modify a byte in the middle of the proof
        let pos = std::vector::length(&range_proof_invalid) / 2;
        let byte = std::vector::borrow_mut(&mut range_proof_invalid, pos);
        *byte = *byte + 1;

        verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(range_proof_invalid), MAX_RANGE_BITS, A_DST);
    }
}
