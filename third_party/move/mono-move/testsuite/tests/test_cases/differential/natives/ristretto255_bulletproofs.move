// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);

    // The Ristretto basepoint: `val_base` of the default Pedersen commitment key.
    public fun basepoint(): RistrettoPoint {
        let (handle, _) = point_decompress_internal(
            x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76"
        );
        RistrettoPoint { handle }
    }

    // `hash_to_point(basepoint)`: `rand_base` of the default commitment key.
    public fun hash_base(): RistrettoPoint {
        let (handle, _) = point_decompress_internal(
            x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134"
        );
        RistrettoPoint { handle }
    }
}

module 0x1::ristretto255_bulletproofs {
    use 0x1::ristretto255::{Self, RistrettoPoint};

    const A_DST: vector<u8> = b"AptosBulletproofs";
    // 5020644638028926087u64, as a 32-byte scalar.
    const A_VALUE: vector<u8> = x"870c2fa1b2e9ac45000000000000000000000000000000000000000000000000";
    // 3123139139123912123u64, as a 32-byte scalar.
    const B_VALUE: vector<u8> = x"bb9d99fb7f9e572b000000000000000000000000000000000000000000000000";
    const A_BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const B_BLINDER: vector<u8> = x"ce224fe5e1111a394fc254ee503aa2406706ef606efac6e2d0332711c7a7bc06";
    // Pedersen commitment to A_VALUE under A_BLINDER, and to B_VALUE under B_BLINDER.
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";
    const B_COMM: vector<u8> = x"748c244d880a1de3970a3d01670a04db6b74b9741bfec8732e512312384a6515";
    // 64-bit range proof for A_COMM under A_DST.
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";
    // 64-bit batch range proof for [A_COMM, B_COMM] under A_DST.
    const AB_BATCH_RANGE_PROOF_PEDERSEN: vector<u8> = x"103086c56ead10712514d2807c5605cb5f3a090566196549b5f03bedd7c1f450b4619bca9b00f87b2e039e844c24f9f2512901eea7f8f322f218f58c37186e1bd40ae74942f69b18f6806a536b2ab0793ab8e646eafc6e31d5219545dfcbb21334230c4e063e682d1f37fdfe7258d1735af1ba4764ca182803ef4566ddd386143550b83b8d686514988ee05bb7b4180f3b296a0a9711976365b678b537e2190c49cecded1d209ecec733e5cb85d5427f1f2ef1a44ebac41fdbf822692bd68b012515065faab0611aaabe87c1facbe68e648f2e2a0de6e5e81490dfa178546d0e1ec7a7c7ee6eb1e72f0e62b6a81abf23d4e4f946e5c5b28ca287d7ee30c72667ec1203ea9314a4ef182e3ed8a49700cb2452c3765fd29611e2abb5d8aa1970387452cd473383707a0b8e2eb46ba6826654e03ba5f73b56a0ae30012dc723576e76b280339600decef76eda350232ee9e53b373d745b958a19c8b4e7133f4b846727dab188441bb7d2484a73a9a83c1c94e7bea0ea0253418d3d5a751e63f940106e597772d169a01d93b495d10c08725c5d8cdef24306a164a2e1fa1b19eb0217239bbc661e0f1ead2bf3ecc3f178b6b49c61aa2c45f4832ba9ebc2744b79b413081e824b0978cab1934d29760f77751450e409da17941ff693b7dbc0b45d0659aeca05e1e92572fcd4c4d5846e7963e25cce6d54fc4a963da031747695a8e2000469e22e682e1b3f141891121d189504db63b4ab40e0d4c59f0b945b8188b79f0eb4916723a757bcfc787863ff28c5555c8ad93df81bba7b2ff9c164e180331a8b24cff4a9de0d2a8b71f73d24521781f0ced1a064698af138c00160c87eb7ffca5ab1d9a1bec5144c648c5f51a6093dbe8ed88a2fcaab4d5412c60ebb25827d8cab48787f705c5781e2ecd82939d3b3f864c21701fcecbc57b196db7c055273e86ac654a24016abd8ba7c6e87610a0e1b70ff57378992b2d5d45c963829b0aa9323b0dde3f02382e583cb3733c187b46903ed629820ec8043a8c18df42dc0a";

    native fun verify_range_proof_internal(
        com: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    native fun verify_batch_range_proof_internal(
        comms: vector<vector<u8>>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

    native fun prove_range_internal(
        val: vector<u8>,
        r: vector<u8>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint): (vector<u8>, vector<u8>);

    native fun prove_batch_range_internal(
        vals: vector<vector<u8>>,
        rs: vector<vector<u8>>,
        num_bits: u64,
        dst: vector<u8>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint): (vector<u8>, vector<vector<u8>>);

    fun verify(comm: vector<u8>, proof: vector<u8>, num_bits: u64, dst: vector<u8>): bool {
        verify_range_proof_internal(
            comm, &ristretto255::basepoint(), &ristretto255::hash_base(), proof, num_bits, dst
        )
    }

    fun verify_batch(
        comms: vector<vector<u8>>, proof: vector<u8>, num_bits: u64, dst: vector<u8>
    ): bool {
        verify_batch_range_proof_internal(
            comms, &ristretto255::basepoint(), &ristretto255::hash_base(), proof, num_bits, dst
        )
    }

    public fun verify_single_valid(): bool {
        verify(A_COMM, A_RANGE_PROOF_PEDERSEN, 64, A_DST)
    }

    // The `vector<vector<u8>>` argument path.
    public fun verify_batch_valid(): bool {
        verify_batch(vector[A_COMM, B_COMM], AB_BATCH_RANGE_PROOF_PEDERSEN, 64, A_DST)
    }

    // A single flipped proof byte fails verification without aborting: the
    // proof still deserializes.
    public fun verify_single_tampered_proof(): bool {
        let proof = A_RANGE_PROOF_PEDERSEN;
        *&mut proof[0] = 0xd9u8;
        verify(A_COMM, proof, 64, A_DST)
    }

    public fun verify_single_wrong_dst(): bool {
        verify(A_COMM, A_RANGE_PROOF_PEDERSEN, 64, b"WrongDST")
    }

    public fun verify_batch_wrong_dst(): bool {
        verify_batch(vector[A_COMM, B_COMM], AB_BATCH_RANGE_PROOF_PEDERSEN, 64, b"WrongDST")
    }

    // Swapping the commitments breaks the batch proof.
    public fun verify_batch_swapped_comms(): bool {
        verify_batch(vector[B_COMM, A_COMM], AB_BATCH_RANGE_PROOF_PEDERSEN, 64, A_DST)
    }

    // Aborts with NFE_RANGE_NOT_SUPPORTED (0x01_0003).
    public fun verify_unsupported_bits(): bool {
        verify(A_COMM, A_RANGE_PROOF_PEDERSEN, 10, A_DST)
    }

    // Aborts with NFE_BATCH_SIZE_NOT_SUPPORTED (0x01_0004). The bit-width check
    // runs first, so this also pins the abort ordering.
    public fun verify_batch_unsupported_size(): bool {
        verify_batch(vector[A_COMM, B_COMM, A_COMM], AB_BATCH_RANGE_PROOF_PEDERSEN, 64, A_DST)
    }

    // Aborts with NFE_DESERIALIZE_RANGE_PROOF (0x01_0001).
    public fun verify_garbage_proof(): bool {
        verify(A_COMM, b"not a range proof", 64, A_DST)
    }

    public fun verify_batch_garbage_proof(): bool {
        verify_batch(vector[A_COMM, B_COMM], vector[], 64, A_DST)
    }

    // The prover's commitment is deterministic even though its proof is not.
    public fun prove_single_commitment(): vector<u8> {
        let (_proof, comm) = prove_range_internal(
            A_VALUE, A_BLINDER, 64, A_DST, &ristretto255::basepoint(), &ristretto255::hash_base()
        );
        comm
    }

    public fun prove_verify_single_roundtrip(): bool {
        let (proof, comm) = prove_range_internal(
            A_VALUE, A_BLINDER, 64, A_DST, &ristretto255::basepoint(), &ristretto255::hash_base()
        );
        verify(comm, proof, 64, A_DST) && !verify(comm, proof, 32, A_DST)
    }

    fun prove_batch(num_bits: u64): (vector<u8>, vector<vector<u8>>) {
        prove_batch_range_internal(
            vector[A_VALUE, B_VALUE],
            vector[A_BLINDER, B_BLINDER],
            num_bits,
            A_DST,
            &ristretto255::basepoint(),
            &ristretto255::hash_base(),
        )
    }

    // The `vector<vector<u8>>` construction path: the returned commitments are
    // deterministic, so their contents are checked directly.
    public fun prove_batch_commitments(): (u64, vector<u8>, vector<u8>) {
        let (_proof, comms) = prove_batch(64);
        (comms.length(), comms[0], comms[1])
    }

    public fun prove_verify_batch_roundtrip(): bool {
        let (proof, comms) = prove_batch(64);
        verify_batch(comms, proof, 64, A_DST)
    }

    // Allocate after the prover returns, so the collector has to relocate the
    // constructed outer vector along with each inner byte vector.
    public fun prove_batch_survives_gc(rounds: u64): bool {
        let (proof, comms) = prove_batch(64);
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
        comms[0] == A_COMM && comms[1] == B_COMM && verify_batch(comms, proof, 64, A_DST)
    }
}

// RUN: execute 0x1::ristretto255_bulletproofs::verify_single_valid
// CHECK: results: true

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch_valid
// CHECK: results: true

// RUN: execute 0x1::ristretto255_bulletproofs::verify_single_tampered_proof
// CHECK: results: false

// RUN: execute 0x1::ristretto255_bulletproofs::verify_single_wrong_dst
// CHECK: results: false

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch_wrong_dst
// CHECK: results: false

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch_swapped_comms
// CHECK: results: false

// RUN: execute 0x1::ristretto255_bulletproofs::verify_unsupported_bits
// CHECK-SUBSTR: aborted: code 65539

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch_unsupported_size
// CHECK-SUBSTR: aborted: code 65540

// RUN: execute 0x1::ristretto255_bulletproofs::verify_garbage_proof
// CHECK-SUBSTR: aborted: code 65537

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch_garbage_proof
// CHECK-SUBSTR: aborted: code 65537

// RUN: execute 0x1::ristretto255_bulletproofs::prove_single_commitment
// CHECK: results: 0x0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14

// RUN: execute 0x1::ristretto255_bulletproofs::prove_verify_single_roundtrip
// CHECK: results: true

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_commitments
// CHECK: results: 2, 0x0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14, 0x748c244d880a1de3970a3d01670a04db6b74b9741bfec8732e512312384a6515

// RUN: execute 0x1::ristretto255_bulletproofs::prove_verify_batch_roundtrip
// CHECK: results: true

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_survives_gc --args 0 --heap-size 16384
// CHECK: results: true
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch_survives_gc --args 40000 --heap-size 16384
// CHECK: results: true
// CHECK-GC-COUNT: 31
