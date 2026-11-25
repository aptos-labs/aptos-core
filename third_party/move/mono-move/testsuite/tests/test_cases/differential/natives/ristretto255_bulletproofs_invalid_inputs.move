// Forged inputs to the bulletproofs natives, each beside a control with valid bases and
// 32-byte scalars: a base point handle no point has (77 as `val_base`, 78 as `rand_base`) in
// each native, and a value or blinder that is not 32 bytes in the provers. For a forged base,
// V1 aborts with the native's invalid-point code; for a wrong-length scalar, it raises
// INTERNAL_TYPE_ERROR (substring check). MonoMove raises an invariant violation for both.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    fun decompress(bytes: vector<u8>): RistrettoPoint {
        let (handle, _) = point_decompress_internal(bytes);
        RistrettoPoint { handle }
    }

    // `forge` 0: the basepoint and its hash point; 1: a forged `val_base`;
    // 2: a forged `rand_base`.
    public fun bases(forge: u64): (RistrettoPoint, RistrettoPoint) {
        let val_base = if (forge == 1) RistrettoPoint { handle: 77 } else decompress(
            x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76"
        );
        let rand_base = if (forge == 2) RistrettoPoint { handle: 78 } else decompress(
            x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134"
        );
        (val_base, rand_base)
    }

    public fun scalar(value: u64): vector<u8> {
        scalar_from_u64_internal(value)
    }
}

module 0x1::ristretto255_bulletproofs {
    use 0x1::ristretto255::{Self, RistrettoPoint};

    const A_DST: vector<u8> = b"AptosBulletproofs";
    const A_VALUE: vector<u8> = x"870c2fa1b2e9ac45000000000000000000000000000000000000000000000000";
    const A_BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";
    const B_COMM: vector<u8> = x"748c244d880a1de3970a3d01670a04db6b74b9741bfec8732e512312384a6515";
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";
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
        blinder: vector<u8>,
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

    // `bytes` truncated or zero-padded to `len` bytes.
    fun resize(bytes: vector<u8>, len: u64): vector<u8> {
        while (bytes.length() > len) {
            bytes.pop_back();
        };
        while (bytes.length() < len) {
            bytes.push_back(0);
        };
        bytes
    }

    public fun verify(forge: u64): bool {
        let (val_base, rand_base) = ristretto255::bases(forge);
        verify_range_proof_internal(
            A_COMM, &val_base, &rand_base, A_RANGE_PROOF_PEDERSEN, 64, A_DST
        )
    }

    public fun verify_batch(forge: u64): bool {
        let (val_base, rand_base) = ristretto255::bases(forge);
        verify_batch_range_proof_internal(
            vector[A_COMM, B_COMM], &val_base, &rand_base, AB_BATCH_RANGE_PROOF_PEDERSEN, 64, A_DST
        )
    }

    // Returns the proof and commitment lengths.
    public fun prove(forge: u64, val_len: u64, r_len: u64): (u64, u64) {
        let (val_base, rand_base) = ristretto255::bases(forge);
        let (proof, comm) = prove_range_internal(
            resize(A_VALUE, val_len), resize(A_BLINDER, r_len), 64, A_DST, &val_base, &rand_base
        );
        (proof.length(), comm.length())
    }

    // The wrong length goes to the second value and the first blinder.
    // Returns the proof length and commitment count.
    public fun prove_batch(forge: u64, val_len: u64, r_len: u64): (u64, u64) {
        let (val_base, rand_base) = ristretto255::bases(forge);
        let vals = vector[ristretto255::scalar(1), resize(ristretto255::scalar(2), val_len)];
        let rs = vector[resize(A_BLINDER, r_len), ristretto255::scalar(3)];
        let (proof, comms) = prove_batch_range_internal(
            vals, rs, 64, A_DST, &val_base, &rand_base
        );
        (proof.length(), comms.length())
    }
}

// RUN: execute 0x1::ristretto255_bulletproofs::verify --args 0
// CHECK: results: true

// RUN: execute 0x1::ristretto255_bulletproofs::verify --args 1
// CHECK-V1: aborted: code 655362 (Invalid point for verify val base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 77

// RUN: execute 0x1::ristretto255_bulletproofs::verify --args 2
// CHECK-V1: aborted: code 655361 (Invalid point for verify rand base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 78

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch --args 0
// CHECK: results: true

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch --args 1
// CHECK-V1: aborted: code 655364 (Invalid point for verify batch val base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 77

// RUN: execute 0x1::ristretto255_bulletproofs::verify_batch --args 2
// CHECK-V1: aborted: code 655363 (Invalid point for verify batch rand base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 78

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 0, 32, 32
// CHECK: results: 672, 32

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 1, 32, 32
// CHECK-V1: aborted: code 655366 (Invalid point for prove val base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 77

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 2, 32, 32
// CHECK-V1: aborted: code 655365 (Invalid point for prove rand base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 78

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 0, 31, 32
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255_bulletproofs") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255_bulletproofs::prove --args 0, 32, 33
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255_bulletproofs") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 0, 32, 32
// CHECK: results: 736, 2

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 1, 32, 32
// CHECK-V1: aborted: code 655369 (Invalid point for prove batch val base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 77

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 2, 32, 32
// CHECK-V1: aborted: code 655368 (Invalid point for prove batch rand base) in 0x1::ristretto255_bulletproofs
// CHECK-V2: error: native function invariant violation: invalid ristretto255 point handle: 78

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 0, 33, 32
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255_bulletproofs") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes

// RUN: execute 0x1::ristretto255_bulletproofs::prove_batch --args 0, 32, 31
// CHECK-V1-SUBSTR: error: VMError with status INTERNAL_TYPE_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("ristretto255_bulletproofs") }
// CHECK-V2: error: native function invariant violation: ristretto255 scalar must be 32 bytes
