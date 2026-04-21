module aptos_std::ristretto255_bulletproofs {
    use std::error;
    use std::features;
    use aptos_std::ristretto255_pedersen as pedersen;
    use aptos_std::ristretto255::{Self, RistrettoPoint};

    const MAX_RANGE_BITS: u64 = 64;

    const E_DESERIALIZE_RANGE_PROOF: u64 = 1;
    const E_VALUE_OUTSIDE_RANGE: u64 = 2;
    const E_RANGE_NOT_SUPPORTED: u64 = 3;
    const E_BATCH_SIZE_NOT_SUPPORTED: u64 = 4;
    const E_VECTOR_LENGTHS_MISMATCH: u64 = 5;
    const E_DST_TOO_LONG: u64 = 6;
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 7;

    struct RangeProof has copy, drop, store {
        bytes: vector<u8>
    }

    public fun get_max_range_bits(): u64 {
        MAX_RANGE_BITS
    }

    public fun range_proof_from_bytes(bytes: vector<u8>): RangeProof {
        RangeProof { bytes }
    }

    public fun range_proof_to_bytes(proof: &RangeProof): vector<u8> {
        proof.bytes
    }

    public fun verify_range_proof_pedersen(
        com: &pedersen::Commitment,
        proof: &RangeProof,
        num_bits: u64,
        dst: vector<u8>,
    ): bool {
        verify_range_proof(
            pedersen::commitment_as_point(com),
            &ristretto255::basepoint(),
            &ristretto255::hash_to_point_base(),
            proof,
            num_bits,
            dst,
        )
    }

    public fun verify_range_proof(
        com: &RistrettoPoint,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: &RangeProof,
        num_bits: u64,
        dst: vector<u8>,
    ): bool {
        assert!(features::bulletproofs_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));
        assert!(dst.length() <= 256, error::invalid_argument(E_DST_TOO_LONG));

        verify_range_proof_internal(
            ristretto255::point_to_bytes(&ristretto255::point_compress(com)),
            val_base,
            rand_base,
            proof.bytes,
            num_bits,
            dst,
        )
    }

    public fun verify_batch_range_proof_pedersen(
        comms: &vector<pedersen::Commitment>,
        proof: &RangeProof,
        num_bits: u64,
        dst: vector<u8>,
    ): bool {
        let points = comms.map_ref(|c| {
            ristretto255::point_clone(pedersen::commitment_as_point(c))
        });
        verify_batch_range_proof(
            &points,
            &ristretto255::basepoint(),
            &ristretto255::hash_to_point_base(),
            proof,
            num_bits,
            dst,
        )
    }

    public fun verify_batch_range_proof(
        comms: &vector<RistrettoPoint>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: &RangeProof,
        num_bits: u64,
        dst: vector<u8>,
    ): bool {
        assert!(features::bulletproofs_batch_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));
        assert!(dst.length() <= 256, error::invalid_argument(E_DST_TOO_LONG));

        let serialized = comms.map_ref(|pt| {
            ristretto255::point_to_bytes(&ristretto255::point_compress(pt))
        });

        verify_batch_range_proof_internal(
            serialized,
            val_base,
            rand_base,
            proof.bytes,
            num_bits,
            dst,
        )
    }

    #[test_only]
    public fun prove_range_pedersen(
        val: &Scalar,
        r: &Scalar,
        num_bits: u64,
        dst: vector<u8>,
    ): (RangeProof, pedersen::Commitment) {
        prove_range(
            val,
            r,
            &ristretto255::basepoint(),
            &ristretto255::hash_to_point_base(),
            num_bits,
            dst,
        )
    }

    #[test_only]
    public fun prove_range(
        val: &Scalar,
        r: &Scalar,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        num_bits: u64,
        dst: vector<u8>,
    ): (RangeProof, pedersen::Commitment) {
        let (proof_bytes, compressed_comm) = prove_range_internal(
            scalar_to_bytes(val),
            scalar_to_bytes(r),
            num_bits,
            dst,
            val_base,
            rand_base,
        );
        let point = ristretto255::new_compressed_point_from_bytes(compressed_comm);
        let point = &point.extract();

        (
            RangeProof { bytes: proof_bytes },
            pedersen::commitment_from_compressed(point),
        )
    }

    #[test_only]
    public fun prove_batch_range_pedersen(
        vals: &vector<Scalar>,
        rs: &vector<Scalar>,
        num_bits: u64,
        dst: vector<u8>,
    ): (RangeProof, vector<pedersen::Commitment>) {
        prove_batch_range(
            vals,
            rs,
            &ristretto255::basepoint(),
            &ristretto255::hash_to_point_base(),
            num_bits,
            dst,
        )
    }

    #[test_only]
    public fun prove_batch_range(
        vals: &vector<Scalar>,
        rs: &vector<Scalar>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        num_bits: u64,
        dst: vector<u8>,
    ): (RangeProof, vector<pedersen::Commitment>) {
        let val_bytes = vals.map_ref(|v| scalar_to_bytes(v));
        let r_bytes = rs.map_ref(|r| scalar_to_bytes(r));

        let (proof_bytes, raw_comms) = prove_batch_range_internal(
            val_bytes,
            r_bytes,
            num_bits,
            dst,
            val_base,
            rand_base,
        );

        let commitments = raw_comms.map(|raw| {
            let c = pedersen::new_commitment_from_bytes(raw);
            c.extract()
        });

        (RangeProof { bytes: proof_bytes }, commitments)
    }

    native fun verify_range_proof_internal(
        com: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
    ): bool;

    native fun verify_batch_range_proof_internal(
        comms: vector<vector<u8>>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
    ): bool;

    #[test_only]
    native fun prove_range_internal(
        val: vector<u8>,
        r: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
    ): (vector<u8>, vector<u8>);

    #[test_only]
    native fun prove_batch_range_internal(
        vals: vector<vector<u8>>,
        rs: vector<vector<u8>>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
    ): (vector<u8>, vector<vector<u8>>);

    #[test_only]
    use aptos_std::ristretto255::{Scalar, scalar_to_bytes};
    #[test_only]
    use aptos_std::ristretto255_pedersen::commitment_equals;

    #[test_only]
    const A_DST: vector<u8> = b"AptosBulletproofs";
    #[test_only]
    const A_VALUE: vector<u8> = x"870c2fa1b2e9ac45000000000000000000000000000000000000000000000000";  // 5020644638028926087u64
    #[test_only]
    const B_VALUE: vector<u8> = x"bb9d99fb7f9e572b000000000000000000000000000000000000000000000000";  // 3123139139123912123u64
    #[test_only]
    const A_BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    #[test_only]
    const B_BLINDER: vector<u8> = x"ce224fe5e1111a394fc254ee503aa2406706ef606efac6e2d0332711c7a7bc06";
    #[test_only]
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";
    #[test_only]
    const B_COMM: vector<u8> = x"748c244d880a1de3970a3d01670a04db6b74b9741bfec8732e512312384a6515";
    #[test_only]
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";
    #[test_only]
    const AB_BATCH_RANGE_PROOF_PEDERSEN: vector<u8> = x"103086c56ead10712514d2807c5605cb5f3a090566196549b5f03bedd7c1f450b4619bca9b00f87b2e039e844c24f9f2512901eea7f8f322f218f58c37186e1bd40ae74942f69b18f6806a536b2ab0793ab8e646eafc6e31d5219545dfcbb21334230c4e063e682d1f37fdfe7258d1735af1ba4764ca182803ef4566ddd386143550b83b8d686514988ee05bb7b4180f3b296a0a9711976365b678b537e2190c49cecded1d209ecec733e5cb85d5427f1f2ef1a44ebac41fdbf822692bd68b012515065faab0611aaabe87c1facbe68e648f2e2a0de6e5e81490dfa178546d0e1ec7a7c7ee6eb1e72f0e62b6a81abf23d4e4f946e5c5b28ca287d7ee30c72667ec1203ea9314a4ef182e3ed8a49700cb2452c3765fd29611e2abb5d8aa1970387452cd473383707a0b8e2eb46ba6826654e03ba5f73b56a0ae30012dc723576e76b280339600decef76eda350232ee9e53b373d745b958a19c8b4e7133f4b846727dab188441bb7d2484a73a9a83c1c94e7bea0ea0253418d3d5a751e63f940106e597772d169a01d93b495d10c08725c5d8cdef24306a164a2e1fa1b19eb0217239bbc661e0f1ead2bf3ecc3f178b6b49c61aa2c45f4832ba9ebc2744b79b413081e824b0978cab1934d29760f77751450e409da17941ff693b7dbc0b45d0659aeca05e1e92572fcd4c4d5846e7963e25cce6d54fc4a963da031747695a8e2000469e22e682e1b3f141891121d189504db63b4ab40e0d4c59f0b945b8188b79f0eb4916723a757bcfc787863ff28c5555c8ad93df81bba7b2ff9c164e180331a8b24cff4a9de0d2a8b71f73d24521781f0ced1a064698af138c00160c87eb7ffca5ab1d9a1bec5144c648c5f51a6093dbe8ed88a2fcaab4d5412c60ebb25827d8cab48787f705c5781e2ecd82939d3b3f864c21701fcecbc57b196db7c055273e86ac654a24016abd8ba7c6e87610a0e1b70ff57378992b2d5d45c963829b0aa9323b0dde3f02382e583cb3733c187b46903ed629820ec8043a8c18df42dc0a";

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010003, location = Self)]
    fun test_unsupported_ranges(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_feature()], vector[]);

        let comm = pedersen::new_commitment_from_bytes(A_COMM).extract();
        verify_range_proof_pedersen(&comm, &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), 10, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010003, location = Self)]
    fun test_unsupported_ranges_batch(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let ca = pedersen::new_commitment_from_bytes(A_COMM).extract();
        let cb = pedersen::new_commitment_from_bytes(B_COMM).extract();
        verify_batch_range_proof_pedersen(&vector[ca, cb], &range_proof_from_bytes(AB_BATCH_RANGE_PROOF_PEDERSEN), 10, A_DST);
    }

    #[test(fx = @std)]
    fun test_prover(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_feature()], vector[]);

        let v = ristretto255::new_scalar_from_u64(59);
        let r = ristretto255::new_scalar_from_bytes(A_BLINDER).extract();
        let bits = 8;

        let (proof, comm) = prove_range_pedersen(&v, &r, bits, A_DST);

        assert!(!verify_range_proof_pedersen(&comm, &proof, 64, A_DST), 1);
        assert!(!verify_range_proof_pedersen(&comm, &proof, 32, A_DST), 1);
        assert!(!verify_range_proof_pedersen(&comm, &proof, 16, A_DST), 1);
        assert!(verify_range_proof_pedersen(&comm, &proof, bits, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_batch_prover(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let vals = vector[
            ristretto255::new_scalar_from_u64(59),
            ristretto255::new_scalar_from_u64(60),
        ];
        let blindings = vector[
            ristretto255::new_scalar_from_bytes(A_BLINDER).extract(),
            ristretto255::new_scalar_from_bytes(B_BLINDER).extract(),
        ];
        let bits = 8;

        let (proof, comms) = prove_batch_range_pedersen(&vals, &blindings, bits, A_DST);

        assert!(!verify_batch_range_proof_pedersen(&comms, &proof, 64, A_DST), 1);
        assert!(!verify_batch_range_proof_pedersen(&comms, &proof, 32, A_DST), 1);
        assert!(!verify_batch_range_proof_pedersen(&comms, &proof, 16, A_DST), 1);
        assert!(verify_batch_range_proof_pedersen(&comms, &proof, bits, A_DST), 1);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x030007, location = Self)]
    fun test_bulletproof_feature_disabled(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[], vector[features::get_bulletproofs_feature()]);

        let v = ristretto255::new_scalar_from_u64(59);
        let r = ristretto255::new_scalar_from_bytes(A_BLINDER).extract();
        let (proof, comm) = prove_range_pedersen(&v, &r, 8, A_DST);
        verify_range_proof_pedersen(&comm, &proof, 8, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x030007, location = Self)]
    fun test_bulletproof_batch_feature_disabled(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[], vector[features::get_bulletproofs_batch_feature()]);

        let vals = vector[
            ristretto255::new_scalar_from_u64(59),
            ristretto255::new_scalar_from_u64(60),
        ];
        let blindings = vector[
            ristretto255::new_scalar_from_bytes(A_BLINDER).extract(),
            ristretto255::new_scalar_from_bytes(B_BLINDER).extract(),
        ];
        let (proof, comms) = prove_batch_range_pedersen(&vals, &blindings, 8, A_DST);
        verify_batch_range_proof_pedersen(&comms, &proof, 8, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010001, location = Self)]
    fun test_empty_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_feature()], vector[]);

        let com = pedersen::new_commitment_for_bulletproof(
            &ristretto255::scalar_one(),
            &ristretto255::new_scalar_from_sha2_512(b"hello random world"),
        );
        verify_range_proof_pedersen(&com, &range_proof_from_bytes(vector[]), 64, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010001, location = Self)]
    fun test_empty_batch_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let com = pedersen::new_commitment_for_bulletproof(
            &ristretto255::scalar_one(),
            &ristretto255::new_scalar_from_sha2_512(b"hello random world"),
        );
        verify_batch_range_proof_pedersen(&vector[com], &range_proof_from_bytes(vector[]), 64, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010002, location = Self)]
    fun test_value_outside_range_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_feature()], vector[]);

        let va = ristretto255::new_scalar_from_bytes(A_VALUE).extract();
        let vb = ristretto255::new_scalar_from_u128(1 << 65);
        let ba = ristretto255::new_scalar_from_bytes(A_BLINDER).extract();
        let bb = ristretto255::new_scalar_from_bytes(B_BLINDER).extract();

        prove_batch_range_pedersen(&vector[va, vb], &vector[ba, bb], 64, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010002, location = Self)]
    fun test_value_outside_range_batch_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let v = ristretto255::new_scalar_from_u128(1 << 65);
        let b = ristretto255::new_scalar_from_bytes(A_BLINDER).extract();
        prove_range_pedersen(&v, &b, 64, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010004, location = Self)]
    fun test_invalid_batch_size_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let vals = vector[
            ristretto255::new_scalar_from_bytes(A_VALUE).extract(),
            ristretto255::new_scalar_from_bytes(B_VALUE).extract(),
            ristretto255::new_scalar_from_u32(1),
        ];
        let blindings = vector[
            ristretto255::new_scalar_from_bytes(A_BLINDER).extract(),
            ristretto255::new_scalar_from_bytes(B_BLINDER).extract(),
            ristretto255::new_scalar_from_u32(1),
        ];
        prove_batch_range_pedersen(&vals, &blindings, 64, A_DST);
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x010005, location = Self)]
    fun test_invalid_args_batch_range_proof(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let vals = vector[
            ristretto255::new_scalar_from_bytes(A_VALUE).extract(),
            ristretto255::new_scalar_from_bytes(B_VALUE).extract(),
        ];
        let blindings = vector[ristretto255::new_scalar_from_bytes(A_BLINDER).extract()];
        prove_batch_range_pedersen(&vals, &blindings, 64, A_DST);
    }

    #[test(fx = @std)]
    fun test_valid_range_proof_verifies_against_comm(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let val = ristretto255::new_scalar_from_bytes(A_VALUE).extract();
        let blinder = ristretto255::new_scalar_from_bytes(A_BLINDER).extract();
        let comm = pedersen::new_commitment_for_bulletproof(&val, &blinder);

        assert!(commitment_equals(&comm, &pedersen::new_commitment_from_bytes(A_COMM).extract()), 1);
        assert!(verify_range_proof_pedersen(&comm, &range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN), MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_valid_batch_range_proof_verifies_against_comm(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let va = ristretto255::new_scalar_from_bytes(A_VALUE).extract();
        let vb = ristretto255::new_scalar_from_bytes(B_VALUE).extract();
        let ba = ristretto255::new_scalar_from_bytes(A_BLINDER).extract();
        let bb = ristretto255::new_scalar_from_bytes(B_BLINDER).extract();

        let comms = vector[va, vb].zip_map(
            vector[ba, bb],
            |v, b| pedersen::new_commitment_for_bulletproof(&v, &b),
        );

        assert!(commitment_equals(comms.borrow(0), &pedersen::new_commitment_from_bytes(A_COMM).extract()), 1);
        assert!(commitment_equals(comms.borrow(1), &pedersen::new_commitment_from_bytes(B_COMM).extract()), 1);
        assert!(verify_batch_range_proof_pedersen(&comms, &range_proof_from_bytes(AB_BATCH_RANGE_PROOF_PEDERSEN), MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_invalid_range_proof_fails_verification(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_feature()], vector[]);

        let comm = pedersen::new_commitment_from_bytes(A_COMM).extract();
        let bad_proof = A_RANGE_PROOF_PEDERSEN;
        let mid = bad_proof.length() / 2;
        let byte = bad_proof.borrow_mut(mid);
        *byte = *byte + 1;

        assert!(!verify_range_proof_pedersen(&comm, &range_proof_from_bytes(bad_proof), MAX_RANGE_BITS, A_DST), 1);
    }

    #[test(fx = @std)]
    fun test_invalid_batch_range_proof_fails_verification(fx: signer) {
        features::change_feature_flags_for_testing(&fx, vector[features::get_bulletproofs_batch_feature()], vector[]);

        let ca = pedersen::new_commitment_from_bytes(A_COMM).extract();
        let cb = pedersen::new_commitment_from_bytes(B_COMM).extract();
        let bad_proof = AB_BATCH_RANGE_PROOF_PEDERSEN;
        let mid = bad_proof.length() / 2;
        bad_proof[mid] = bad_proof[mid] + 1;

        assert!(!verify_batch_range_proof_pedersen(&vector[ca, cb], &range_proof_from_bytes(bad_proof), MAX_RANGE_BITS, A_DST), 1);
    }
}
