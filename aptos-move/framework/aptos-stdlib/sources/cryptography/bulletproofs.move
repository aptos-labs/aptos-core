/// This module implements Bulletproof-based zero-knowledge range proof: i.e., a proof that a value `v` committed in a
/// Pedersen commitment `com` satisfies $v \in [0, 2^{num_bits})$. Currently, this module only supports num_bits \in
/// {8, 16, 32, 64}.
module aptos_std::bulletproofs {
    use std::features;
    use aptos_std::elgamal;
    use aptos_std::pedersen;
    use aptos_std::ristretto255::{Self, RistrettoPoint};

    //
    // Constants
    //

    /// The maximum range supported by the Bulletproofs library is [0, 2^{64}).
    const MAX_RANGE_BITS : u64 = 64;

    //
    // Error codes
    //

    /// Error deserializing one of the arguments.
    const E_DESERIALIZE_RANGE_PROOF: u64 = 1;

    /// The input value given to the prover is too large.
    const E_VALUE_OUTSIDE_RANGE: u64 = 2;

    /// The range proof system only supports proving ranges of type [0, 2^{bits}) where bits \in {8, 16, 32, 64}.
    const E_RANGE_NOT_SUPPORTED: u64 = 3;

    /// The native functions have not been rolled out yet.
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 4;

    //
    // Structs
    //

    /// Represents a zero-knowledge range proof that a value committed inside a Pedersen commitment lies in [0, 2^{MAX_RANGE_BITS}).
    struct RangeProof has copy, drop, store {
        bytes: vector<u8>
    }

    //
    // Public functions
    //

    /// Returns the maximum # of bits the range proof system can verify proofs for.
    public fun get_max_range_bits(): u64 {
        MAX_RANGE_BITS
    }

    /// Deserializes a range proof from a sequence of bytes.
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
    /// commitment key; see `pedersen::new_commitment_for_bulletproof`) satisfies $v \in [0, 2^{num_bits})$. Only works
    /// for `num_bits` \in {8, 16, 32, 64}. Additionally, checks that the prover used dst as a domain-separation 
    /// tag (DST). This prevents proofs computed for one application (a.k.a. a domain) with dst_1 from verifying 
    /// in a different application with dst_2 != dst_1.
    public fun verify_range_proof_pedersen(com: &pedersen::Commitment, proof: &RangeProof, num_bits: u64, dst: vector<u8>): bool {
        assert(features::bulletproofs_enabled(), E_NATIVE_FUN_NOT_AVAILABLE);

        verify_range_proof_internal(
            ristretto255::point_to_bytes(&pedersen::commitment_as_compressed_point(com)),
            &ristretto255::basepoint(), &ristretto255::hash_to_point_base(),
            proof.bytes,
            num_bits,
            dst
        )
    }

    /// Verifies a zero-knowledge range proof that the value `v` encrypted by `ct` = (r * G, v * G + r * pubkey) 
    /// with ElGamal public key `pubkey` = sk * G where G is the ristretto255 basepoint obtained by 
    /// ristretto255::basepoint() satisfies $v \in [0, 2^{num_bits})$. Only works
    /// for `num_bits` \in {8, 16, 32, 64}.
    public fun verify_range_proof_elgamal(ct: &elgamal::Ciphertext, proof: &RangeProof, pubkey: &elgamal::Pubkey, num_bits: u64, dst: vector<u8>): bool {
        assert!(features::bulletproofs_enabled(), E_NATIVE_FUN_NOT_AVAILABLE);

        verify_range_proof_internal(
            ristretto255::point_to_bytes(&elgamal::get_value_component_compressed(ct)),
            &ristretto255::basepoint(), &elgamal::get_point_from_pubkey(pubkey),
            proof.bytes,
            num_bits,
            dst
        )
    }

    /// Verifies a zero-knowledge range proof that the value `v` committed in `com` (as v * val_base + r * rand_base,
    /// for some randomness `r`) satisfies $v \in [0, 2^{num_bits})$. Only works for `num_bits` \in {8, 16, 32, 64}.
    public fun verify_range_proof(
        com: &RistrettoPoint,
        val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
        proof: &RangeProof, num_bits: u64, dst: vector<u8>): bool
    {
        assert!(features::bulletproofs_enabled(), E_NATIVE_FUN_NOT_AVAILABLE);

        verify_range_proof_internal(
            ristretto255::point_to_bytes(&ristretto255::point_compress(com)),
            val_base, rand_base,
            proof.bytes, num_bits, dst
        )
    }

    #[test_only]
    /// Computes a range proof for the Pedersen commitment to 'val' with randomness 'r', under the default Bulletproofs
    /// commitment key; see `pedersen::new_commitment_for_bulletproof`. Returns the said commitment too.
    ///  Only works for `num_bits` \in {8, 16, 32, 64}.
    public fun prove_range_pedersen(val: &Scalar, r: &Scalar, num_bits: u64, dst: vector<u8>): (RangeProof, pedersen::Commitment) {
        let (bytes, compressed_comm) = prove_range_internal(scalar_to_bytes(val), scalar_to_bytes(r), num_bits, dst, &ristretto255::basepoint(), &ristretto255::hash_to_point_base());
        let point = ristretto255::new_compressed_point_from_bytes(compressed_comm);
        let point = &std::option::extract(&mut point);

        (
            RangeProof { bytes },
            pedersen::new_commitment_from_compressed(point)
        )
    }


    #[test_only]
    /// Computes a range proof for the ElGamal encryption of `val` with randomness `r`, under the default ristretto255  /// basepoint and provided ElGamal public key `pubkey`. Returnsboth the range proof and the encryption. 
    /// ///  Only works for `num_bits` \in {8, 16, 32, 64}.
    public fun prove_range_elgamal(val: &Scalar, r: &Scalar, pubkey: &elgamal::Pubkey, num_bits: u64, dst: vector<u8>): (RangeProof, elgamal::Ciphertext) {
        let compressed_pubkey_point = elgamal::get_compressed_point_from_pubkey(pubkey);
        let (bytes, compressed_comm) = prove_range_internal(scalar_to_bytes(val), scalar_to_bytes(r), num_bits, dst, &ristretto255::basepoint(), &ristretto255::point_decompress(&compressed_pubkey_point));
        let left = ristretto255::new_point_from_bytes(compressed_comm);
        let left = std::option::extract<RistrettoPoint>(&mut left);
        let right = ristretto255::basepoint_mul(r);
        (
            RangeProof { bytes },
            elgamal::new_ciphertext_from_points(left, right)
        )
    }

    //
    // Native functions
    //

    native fun verify_range_proof_internal(
        com: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    #[test_only]
    /// Returns a tuple consisting of (1) a range proof for 'val' committed with randomness 'r' under the default Bulletproofs
    /// commitment key and (2) the commitment itself.
    native fun prove_range_internal(
        val: vector<u8>,
        r: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint): (vector<u8>, vector<u8>);

    //
    // Testing
    //

    #[test_only]
    use aptos_std::ristretto255::{Scalar, scalar_to_bytes, point_equals};

    #[test_only]
    const A_DST: vector<u8> = b"AptosBulletproofs";
    #[test_only]
    const A_VALUE: vector<u8> = x"870c2fa1b2e9ac45000000000000000000000000000000000000000000000000";  // i.e., 5020644638028926087u64
    #[test_only]
    const A_BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    // Pedersen commitment to A_VALUE with randomness A_BLINDER
    #[test_only]
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";
    // ElGamal ciphertext of A_VALUE with randomness A_BLINDER with public key generated hashing SOME_RANDOMNESS
    // to a scalar with new_scalar_from_sha2_512 and multiplying the ristretto255 basepoint by it
    #[test_only]
    const A_CT: vector<u8> = x"eaed21ed285dcfba525c8ace42a48b9430957a1aefd77c46bc459f9184a117562e6c7ce69c88faf9e1c194106dce2d00384a271e45e0bd2992698224cf07f871";
    // Range proof for A_COMM using domain-separation tag in A_DST, and MAX_RANGE_BITS
    #[test_only]
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";
    // Range proof for A_CT using A_DST and MAX_RANGE_BITS 
    #[test_only]
    const A_RANGE_PROOF_ELGAMAL: vector<u8> = x"2650f490bfd06802e9a701ab29f096f7db0b612248ad7f7702ca9b138f276870006928c167f5644cc52476c00a938e2e6debb8baa315307d7eac1d0ebe6da370fc7d58a75e06ac86a6ffc26946071f6aec682197ef419b01795bd61c9273747bb241c705169b7877d04dba8c3215d3752bb3d893159bf4ed47fb57231f45cb3f978a595c358b09ba464f5a1e2474d1bb69f32d03d9a41db3e1e4f413db931508e0010a88ca327ea95bb1c43237d00512d2ab24d2d3c4a2881a99a95f25fbaa02dfd9e048d0277c8e799851ced515a7ea1af8ca2bdbb4eacebf87fd56c3f0b3035c7fe02bf84df85f102b4a79e30a91e2fb1d5a0e2f42219f20878bc072bcb72d6068a5a1ba37dae7f39a98e14fb881977d50b7ad6065a741c347f537de2bb0159a386008d0d292291908c9331852936c5debc7adcb79579d792dcccd514dd664a89c782f2ff79c62c4f49d9d76566399b35c5a6be68ed5e489e8328c4584ae281a8f7d2c32d65c1a89132734609f37439908fe96c813bd8779501188eac1147f56b52d5a9e19f616a56509e46b2bedb2f947f99fee11d28de87510330c99c83672c91f8cf3a34748ef242ad6976d291482ceb4505c1f773c7c99dca067688b56f24ee5350d7d200be80ae2e6ed1c4fa69419023a517a33c4342bbcad3181cf476090218519b5a38b70306967fa7a0a11e347461df8805ac162228e4ae4b73700625244f279b82ea61dffbc69a1278a2cfd0b1f7d4a0ab7e57898dbf746edcb295468e12ff081b9dc819464700226965833b8a2c26171b1ee6bcda7894e0f0c75d6255f76312dd9ba4da07ff88ca40f02207ad4369eb785940ef1a73963245f22117f6d968ccab298cfcbd1fce4fc365fb54d13856923ef91e6996802ce3c5b0a4bd8c70d50c1b8e7bf52c0947328eefe806faaba0ec49caa5e9a0e57572b0e0c";
    const SOME_RANDOMNESS: vector<u8> = x"a7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010003, location = Self)]
    fun test_unsupported_ranges_pedersen(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let comm = ristretto255::new_point_from_bytes(A_COMM);
        let comm = std::option::extract(&mut comm);
        let comm = pedersen::new_commitment_from_point(comm);

        assert!(verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), 10, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_prover_pedersen(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let v = ristretto255::new_scalar_from_u64(59);
        let r = ristretto255::new_scalar_from_bytes(A_BLINDER);
        let r = std::option::extract(&mut r);
        let num_bits = 8;

        let (proof, comm) = prove_range_pedersen(&v, &r, num_bits, A_DST);

        assert!(verify_range_proof_pedersen(&comm, &proof, 64, A_DST) == false, 1);
        assert!(verify_range_proof_pedersen(&comm, &proof, 32, A_DST) == false, 1);
        assert!(verify_range_proof_pedersen(&comm, &proof, 16, A_DST) == false, 1);
        assert!(verify_range_proof_pedersen(&comm, &proof, num_bits, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_prover_elgamal(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let v = ristretto255::new_scalar_from_u64(59);
        let r = ristretto255::new_scalar_from_bytes(A_BLINDER);
        let r = std::option::extract(&mut r);
        let privkey = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS);
        let pubkey = elgamal::get_pubkey_from_scalar(&privkey);
        let num_bits = 8;

        let (proof, ct) = prove_range_elgamal(&v, &r, &pubkey, num_bits, A_DST);

        assert!(verify_range_proof_elgamal(&ct, &proof, &pubkey, 64, A_DST) == false, 1);
        assert!(verify_range_proof_elgamal(&ct, &proof, &pubkey, 32, A_DST) == false, 1);
        assert!(verify_range_proof_elgamal(&ct, &proof, &pubkey, 16, A_DST) == false, 1);
        assert!(verify_range_proof_elgamal(&ct, &proof, &pubkey, num_bits, A_DST), 1);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010001, location = Self)]
    fun test_empty_range_proof_pedersen(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let proof = &range_proof_from_bytes(vector[ ]);
        let num_bits = 64;
        let com = pedersen::new_commitment_for_bulletproof(
            &ristretto255::scalar_one(),
            &ristretto255::new_scalar_from_sha2_512(b"hello random world")
        );

        // This will fail with error::invalid_argument(E_DESERIALIZE_RANGE_PROOF)
        verify_range_proof_pedersen(&com, proof, num_bits, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010001, location = Self)]
    fun test_empty_range_proof_elgamal(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let proof = &range_proof_from_bytes(vector[ ]);
        let privkey = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS);
        let pubkey = elgamal::get_pubkey_from_scalar(&privkey);
        let num_bits = 64;
        let ct = elgamal::new_ciphertext_with_basepoint(
            &ristretto255::scalar_one(),
            &ristretto255::new_scalar_from_sha2_512(b"hello random world"),
            &pubkey
        );

        // This will fail with error::invalid_argument(E_DESERIALIZE_RANGE_PROOF)
        verify_range_proof_elgamal(&ct, proof, &pubkey, num_bits, A_DST);
    }

    #[test(fx = @std)]
    fun test_valid_range_proof_verifies_against_point_pedersen(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let comm = ristretto255::new_point_from_bytes(A_COMM);
        let comm = std::option::extract(&mut comm);
        let comm = pedersen::new_commitment_from_point(comm);

        assert!(verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_valid_range_proof_verifies_against_point_elgamal(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let ct = elgamal::new_ciphertext_from_bytes(A_CT);
        let ct = std::option::extract(&mut ct);

        let privkey = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS);
        let pubkey = elgamal::get_pubkey_from_scalar(&privkey); 

        assert!(verify_range_proof_elgamal(
            &ct,
            &range_proof_from_bytes(A_RANGE_PROOF_ELGAMAL), &pubkey, MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_valid_range_proof_verifies_against_comm_pedersen(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let value = ristretto255::new_scalar_from_bytes(A_VALUE);
        let value = std::option::extract(&mut value);

        let blinder = ristretto255::new_scalar_from_bytes(A_BLINDER);
        let blinder = std::option::extract(&mut blinder);

        let comm = pedersen::new_commitment_for_bulletproof(&value, &blinder);

        let expected_comm = std::option::extract(&mut ristretto255::new_point_from_bytes(A_COMM));
        assert!(point_equals(pedersen::commitment_as_point(&comm), &expected_comm), 1);

        assert!(verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_valid_range_proof_verifies_against_ct_elgamal(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let value = ristretto255::new_scalar_from_bytes(A_VALUE);
        let value = std::option::extract(&mut value);

        let blinder = ristretto255::new_scalar_from_bytes(A_BLINDER);
        let blinder = std::option::extract(&mut blinder);

        let privkey = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS);
        let pubkey = elgamal::get_pubkey_from_scalar(&privkey);

        let ct = elgamal::new_ciphertext_with_basepoint(&value, &blinder, &pubkey);

        let expected_ct = std::option::extract(&mut elgamal::new_ciphertext_from_bytes(A_CT));
        assert!(elgamal::ciphertext_equals(&ct, &expected_ct), 1);

        assert!(verify_range_proof_elgamal(
            &ct,
            &range_proof_from_bytes(A_RANGE_PROOF_ELGAMAL), &pubkey, MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_invalid_range_proof_fails_verification_pedersen(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let comm = ristretto255::new_point_from_bytes(A_COMM);
        let comm = std::option::extract(&mut comm);
        let comm = pedersen::new_commitment_from_point(comm);

        // Take a valid proof...
        let range_proof_invalid = A_RANGE_PROOF_PEDERSEN;

        // ...and modify a byte in the middle of the proof
        let pos = std::vector::length(&range_proof_invalid) / 2;
        let byte = std::vector::borrow_mut(&mut range_proof_invalid, pos);
        *byte = *byte + 1;

        assert!(verify_range_proof_pedersen(
            &comm,
            &range_proof_from_bytes(range_proof_invalid), MAX_RANGE_BITS, A_DST) == false, 1);
    }

    #[test(fx = @std)]
    fun test_invalid_range_proof_fails_verification_elgamal(fx: signer) {
        features::change_feature_flags(&fx, vector[ features::get_bulletproofs_feature() ], vector[]);

        let ct = elgamal::new_ciphertext_from_bytes(A_CT);
        let ct = std::option::extract(&mut ct);

        let privkey = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS);
        let pubkey = elgamal::get_pubkey_from_scalar(&privkey); 

        // Take a valid proof...
        let range_proof_invalid = A_RANGE_PROOF_ELGAMAL;

        // ...and modify a byte in the middle of the proof
        let pos = std::vector::length(&range_proof_invalid) / 2;
        let byte = std::vector::borrow_mut(&mut range_proof_invalid, pos);
        *byte = *byte + 1;

        assert!(verify_range_proof_elgamal(
            &ct,
            &range_proof_from_bytes(range_proof_invalid), &pubkey, MAX_RANGE_BITS, A_DST) == false, 1);
    }
}
