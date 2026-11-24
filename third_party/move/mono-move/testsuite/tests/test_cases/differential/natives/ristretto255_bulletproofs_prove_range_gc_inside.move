// A collection forced inside the range prover, between its 672-byte proof and 32-byte
// commitment allocations; `prove_fill` shows the heap is sized so the commitment is the
// allocation that collects. The second `prove_verified` run adds two more collections before
// the proof is verified.

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
    const A_BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";

    native fun prove_range_internal(
        val: vector<u8>,
        blinder: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &ristretto255::RistrettoPoint,
        rand_base: &ristretto255::RistrettoPoint): (vector<u8>, vector<u8>);

    native fun verify_range_proof_internal(
        com: vector<u8>,
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

    public fun prove_only(rounds: u64): (u64, u64, u8, u8) {
        let val = A_VALUE;
        let blinder = A_BLINDER;
        let dst = A_DST;
        let val_base = ristretto255::basepoint();
        let rand_base = ristretto255::hash_base();
        churn(rounds);
        let (proof, comm) = prove_range_internal(val, blinder, 64, dst, &val_base, &rand_base);
        (proof.length(), comm.length(), comm[0], comm[31])
    }

    public fun prove_verified(rounds: u64, more: u64): (u64, u64, u8, u8, bool) {
        let val = A_VALUE;
        let blinder = A_BLINDER;
        let dst = A_DST;
        let val_base = ristretto255::basepoint();
        let rand_base = ristretto255::hash_base();
        churn(rounds);
        let (proof, comm) = prove_range_internal(val, blinder, 64, dst, &val_base, &rand_base);
        let proof_len = proof.length();
        let comm_len = comm.length();
        let first = comm[0];
        let last = comm[31];
        churn(more);
        let ok = verify_range_proof_internal(comm, &val_base, &rand_base, proof, 64, A_DST);
        (proof_len, comm_len, first, last, ok)
    }

    public fun prove_fill(rounds: u64): u64 {
        let val = A_VALUE;
        let blinder = A_BLINDER;
        let dst = A_DST;
        let val_base = ristretto255::basepoint();
        let rand_base = ristretto255::hash_base();
        churn(rounds);
        let first = A_RANGE_PROOF_PEDERSEN;
        let _ = val_base;
        let _ = rand_base;
        val.length() + blinder.length() + dst.length() + first.length()
    }
}

// RUN: execute 0x1::ristretto255_bulletproofs::prove_fill --args 2652 --heap-size 32768
// CHECK: results: 753
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::ristretto255_bulletproofs::prove_only --args 2652 --heap-size 32768
// CHECK: results: 672, 32, 10, 20
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::ristretto255_bulletproofs::prove_verified --args 2652, 0 --heap-size 32768
// CHECK: results: 672, 32, 10, 20, true
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::ristretto255_bulletproofs::prove_verified --args 2652, 6400 --heap-size 32768
// CHECK: results: 672, 32, 10, 20, true
// CHECK-GC-COUNT: 3
