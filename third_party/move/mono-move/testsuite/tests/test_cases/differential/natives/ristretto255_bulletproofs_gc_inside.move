// A collection forced inside the batch range prover, after its 736-byte proof allocation and
// during the commitment allocations; `prove_batch_fill` shows the heap is sized so the proof
// fits and a later allocation collects. The second `prove_batch_verified` run adds two more
// collections before the proof is verified, which reads every byte of it and the commitments.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);

    public fun basepoint(): RistrettoPoint {
        let (handle, _) = point_decompress_internal(
            x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76"
        );
        RistrettoPoint { handle }
    }

    public fun hash_base(): RistrettoPoint {
        let (handle, _) = point_decompress_internal(
            x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134"
        );
        RistrettoPoint { handle }
    }
}
module 0x1::ristretto255_bulletproofs {
    use 0x1::ristretto255;

    const A_DST: vector<u8> = b"AptosBulletproofs";
    const A_VALUE: vector<u8> = x"870c2fa1b2e9ac45000000000000000000000000000000000000000000000000";
    const B_VALUE: vector<u8> = x"bb9d99fb7f9e572b000000000000000000000000000000000000000000000000";
    const A_BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const B_BLINDER: vector<u8> = x"ce224fe5e1111a394fc254ee503aa2406706ef606efac6e2d0332711c7a7bc06";
    const AB_BATCH_RANGE_PROOF_PEDERSEN: vector<u8> = x"103086c56ead10712514d2807c5605cb5f3a090566196549b5f03bedd7c1f450b4619bca9b00f87b2e039e844c24f9f2512901eea7f8f322f218f58c37186e1bd40ae74942f69b18f6806a536b2ab0793ab8e646eafc6e31d5219545dfcbb21334230c4e063e682d1f37fdfe7258d1735af1ba4764ca182803ef4566ddd386143550b83b8d686514988ee05bb7b4180f3b296a0a9711976365b678b537e2190c49cecded1d209ecec733e5cb85d5427f1f2ef1a44ebac41fdbf822692bd68b012515065faab0611aaabe87c1facbe68e648f2e2a0de6e5e81490dfa178546d0e1ec7a7c7ee6eb1e72f0e62b6a81abf23d4e4f946e5c5b28ca287d7ee30c72667ec1203ea9314a4ef182e3ed8a49700cb2452c3765fd29611e2abb5d8aa1970387452cd473383707a0b8e2eb46ba6826654e03ba5f73b56a0ae30012dc723576e76b280339600decef76eda350232ee9e53b373d745b958a19c8b4e7133f4b846727dab188441bb7d2484a73a9a83c1c94e7bea0ea0253418d3d5a751e63f940106e597772d169a01d93b495d10c08725c5d8cdef24306a164a2e1fa1b19eb0217239bbc661e0f1ead2bf3ecc3f178b6b49c61aa2c45f4832ba9ebc2744b79b413081e824b0978cab1934d29760f77751450e409da17941ff693b7dbc0b45d0659aeca05e1e92572fcd4c4d5846e7963e25cce6d54fc4a963da031747695a8e2000469e22e682e1b3f141891121d189504db63b4ab40e0d4c59f0b945b8188b79f0eb4916723a757bcfc787863ff28c5555c8ad93df81bba7b2ff9c164e180331a8b24cff4a9de0d2a8b71f73d24521781f0ced1a064698af138c00160c87eb7ffca5ab1d9a1bec5144c648c5f51a6093dbe8ed88a2fcaab4d5412c60ebb25827d8cab48787f705c5781e2ecd82939d3b3f864c21701fcecbc57b196db7c055273e86ac654a24016abd8ba7c6e87610a0e1b70ff57378992b2d5d45c963829b0aa9323b0dde3f02382e583cb3733c187b46903ed629820ec8043a8c18df42dc0a";

    native fun prove_batch_range_internal(
        vals: vector<vector<u8>>,
        rs: vector<vector<u8>>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &ristretto255::RistrettoPoint,
        rand_base: &ristretto255::RistrettoPoint): (vector<u8>, vector<vector<u8>>);

    native fun verify_batch_range_proof_internal(
        comms: vector<vector<u8>>,
        val_base: &ristretto255::RistrettoPoint,
        rand_base: &ristretto255::RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    // Allocates the proof, then two commitments and their outer vector.
    public fun prove_batch_only(rounds: u64): (u64, u64, u8, u8) {
        let vals = vector[A_VALUE, B_VALUE];
        let rs = vector[A_BLINDER, B_BLINDER];
        let dst = A_DST;
        let val_base = ristretto255::basepoint();
        let rand_base = ristretto255::hash_base();
        churn(rounds);
        let (proof, comms) = prove_batch_range_internal(vals, rs, 64, dst, &val_base, &rand_base);
        (proof.length(), comms.length(), comms[0][0], comms[1][31])
    }

    public fun prove_batch_verified(rounds: u64, more: u64): (u64, u64, u8, u8, bool) {
        let vals = vector[A_VALUE, B_VALUE];
        let rs = vector[A_BLINDER, B_BLINDER];
        let dst = A_DST;
        let val_base = ristretto255::basepoint();
        let rand_base = ristretto255::hash_base();
        churn(rounds);
        let (proof, comms) = prove_batch_range_internal(vals, rs, 64, dst, &val_base, &rand_base);
        let proof_len = proof.length();
        let comms_len = comms.length();
        let first = comms[0][0];
        let last = comms[1][31];
        churn(more);
        let ok = verify_batch_range_proof_internal(comms, &val_base, &rand_base, proof, 64, A_DST);
        (proof_len, comms_len, first, last, ok)
    }

    public fun prove_batch_fill(rounds: u64): u64 {
        let vals = vector[A_VALUE, B_VALUE];
        let rs = vector[A_BLINDER, B_BLINDER];
        let dst = A_DST;
        let val_base = ristretto255::basepoint();
        let rand_base = ristretto255::hash_base();
        churn(rounds);
        let first = AB_BATCH_RANGE_PROOF_PEDERSEN;
        let _ = val_base;
        let _ = rand_base;
        vals.length() + rs.length() + dst.length() + first.length()
    }
}

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_fill --args 2632 --heap-size 32768
// CHECK: results: 757
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_only --args 2632 --heap-size 32768
// CHECK: results: 736, 2, 10, 21
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_verified --args 2632, 0 --heap-size 32768
// CHECK: results: 736, 2, 10, 21, true
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_verified --args 2632, 6600 --heap-size 32768
// CHECK: results: 736, 2, 10, 21, true
// CHECK-GC-COUNT: 3
