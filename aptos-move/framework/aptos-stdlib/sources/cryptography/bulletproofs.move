/// This module implements a zero-knowledge range proof for a value `v` committed in a Pedersen commitment `com`
/// satisfying $v \in [0, 2^{num_bits})$.
module aptos_std::bulletproofs {
    use aptos_std::pedersen;
    use aptos_std::ristretto255::{Self, RistrettoPoint, CompressedRistretto, point_to_bytes};

    /// The maximum range supported by the Bulletproofs library is [0, 2^{64}).
    const MAX_RANGE_BITS : u64 = 64;

    //
    // Error codes
    //

    /// Error deserializing one of the arguments.
    const E_DESERIALIZE_BULLETPROOF : u64 = 1;

    /// The range proof system does not support proving ranges outside [0, 2^{64}).
    const E_RANGE_TOO_LARGE : u64 = 2;

    //
    // Public functions
    //

    /// Verifies a zero-knowledge range proof that the value `v` committed in `com` (under the default Bulletproofs
    /// commitment key, via `new_commitment_for_bulletproof`) satisfies $v \in [0, 2^{num_bits})$.
    public fun verify_comm_default_ck(com: &pedersen::Commitment, proof: vector<u8>, num_bits: u64, dst: vector<u8>): bool {
        verify_point_default_ck(
            &ristretto255::point_compress(pedersen::commitment_as_point(com)),
            proof, num_bits, dst
        )
    }

    /// Same as `verify_single_comm_default_ck`, but takes a CompressedRistretto point directly.
    public fun verify_point_default_ck(com: &CompressedRistretto, proof: vector<u8>, num_bits: u64, dst: vector<u8>): bool {
        assert!(num_bits <= MAX_RANGE_BITS, std::error::invalid_argument(E_RANGE_TOO_LARGE));
        verify_single_default_ck_internal(point_to_bytes(com), proof, num_bits, dst)
    }

    /// Verifies a zero-knowledge range proof that the value `v` committed in `com` (as v * val_base + r * rand_base,
    /// for some randomness `r`) satisfies $v \in [0, 2^{num_bits})$.
    public fun verify_comm(
        com: &pedersen::Commitment,
        val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
        proof: vector<u8>, num_bits: u64, dst: vector<u8>): bool
    {
        verify_point(
            &ristretto255::point_compress(pedersen::commitment_as_point(com)),
            val_base, rand_base,
            proof, num_bits, dst
        )
    }

    /// Same as `verify_comm`, but takes a CompressedRistretto point directly.
    public fun verify_point(
        com: &CompressedRistretto,
        val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
        proof: vector<u8>, num_bits: u64, dst: vector<u8>): bool
    {
        assert!(num_bits <= MAX_RANGE_BITS, std::error::invalid_argument(E_RANGE_TOO_LARGE));
        verify_single_internal(
            point_to_bytes(com),
            val_base, rand_base,
            proof, num_bits, dst
        )
    }

    //
    // Native functions
    //

    native fun verify_single_default_ck_internal(
        com: vector<u8>,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    native fun verify_single_internal(
        com: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    //
    // Testing
    //

    const A_DST: vector<u8> = b"AptosBulletproofs";
    const A_RANGE_PROOF_VALID: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";

    #[test]
    #[expected_failure(abort_code = 0x010001)]
    fun test_empty_proof() {
        let proof = vector[ ];
        let num_bits = 64;
        let com = pedersen::new_commitment_for_bulletproof(
            &ristretto255::scalar_one(),
            &ristretto255::new_scalar_from_sha2_512(b"hello random world")
        );

        // This will fail with E_DESERIALIZE_BULLETPROOF
        verify_comm_default_ck(&com, proof, num_bits, A_DST);
    }

    #[test]
    fun test_valid_bulletproof_verifies_against_point() {
        let comm = ristretto255::new_compressed_point_from_bytes(A_COMM);
        let comm = std::option::extract(&mut comm);

        assert!(verify_point_default_ck(&comm, A_RANGE_PROOF_VALID, MAX_RANGE_BITS, A_DST), 1);
    }

    #[test]
    fun test_valid_bulletproof_verifies_against_comm() {
        let value = ristretto255::new_scalar_from_bytes(x"870c2fa1b2e9ac45000000000000000000000000000000000000000000000000"); // i.e., 5020644638028926087u64
        let value = std::option::extract(&mut value);

        let blinder = ristretto255::new_scalar_from_bytes(x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c");
        let blinder = std::option::extract(&mut blinder);

        let comm = pedersen::new_commitment_for_bulletproof(&value, &blinder);

        assert!(verify_comm_default_ck(&comm, A_RANGE_PROOF_VALID, MAX_RANGE_BITS, A_DST), 1);
    }


    #[test]
    fun test_invalid_bulletproof_fails_verification() {
        let comm = ristretto255::new_compressed_point_from_bytes(A_COMM);
        let comm = std::option::extract(&mut comm);

        // Take a valid proof...
        let range_proof_invalid = A_RANGE_PROOF_VALID;

        // ...and modify a byte in the middle of the proof
        let pos = std::vector::length(&range_proof_invalid) / 2;
        let byte = std::vector::borrow_mut(&mut range_proof_invalid, pos);
        *byte = *byte + 1;

        assert!(verify_point_default_ck(&comm, range_proof_invalid, MAX_RANGE_BITS, A_DST) == false, 1);
    }
}
