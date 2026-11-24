// Range proofs at 8, 16 and 32 bits (fixed and fresh proofs) and batches of 1 (64 bits), 4
// (32 bits), and 8 and 16 (16 bits). Fresh proofs are random, so the checks are the proof
// length, the commitments, and verification at the proof's width, at another width, and for
// batches with the commitments reversed.

// RUN: publish
module 0x1::ristretto255 {
    struct RistrettoPoint has drop {
        handle: u64
    }

    native fun point_decompress_internal(maybe_non_canonical_bytes: vector<u8>): (u64, bool);

    native fun scalar_from_u64_internal(num: u64): vector<u8>;

    native fun basepoint_mul_internal(scalar: vector<u8>): u64;

    native fun point_mul_internal(point: &RistrettoPoint, scalar: vector<u8>, in_place: bool): u64;

    native fun point_add_internal(
        left: &RistrettoPoint, right: &RistrettoPoint, in_place: bool
    ): u64;

    native fun point_compress_internal(point: &RistrettoPoint): vector<u8>;

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

    public fun scalar(value: u64): vector<u8> {
        scalar_from_u64_internal(value)
    }

    // The Pedersen commitment value * B + blinder * H, compressed.
    public fun commitment(value: u64, blinder: vector<u8>): vector<u8> {
        let value_part = RistrettoPoint { handle: basepoint_mul_internal(scalar(value)) };
        let blinder_part = RistrettoPoint {
            handle: point_mul_internal(&hash_base(), blinder, false)
        };
        let sum = RistrettoPoint { handle: point_add_internal(&value_part, &blinder_part, false) };
        point_compress_internal(&sum)
    }
}

module 0x1::ristretto255_bulletproofs {
    use 0x1::ristretto255::{Self, RistrettoPoint};

    const DST: vector<u8> = b"AptosBulletproofs";
    const BLINDER: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    // Proof and commitment for 200 at 8 bits under BLINDER.
    const PROOF_8: vector<u8> = x"dc07b1dd173e48d23b80b7432bc2275551d0a2d2d9b9ac959cb15360fe2d20019c3ad1bc95b498b4370e9a339b74129f2b4c73f6e2db0d05f1a737d9b8368f7f24f990993b818ef0b9a251000bfb5f2939113c49ac810bb441be4f63c78abb3388f1e544d61e3e3108ae4800345f10a9d44d58c5fe34c2c64a1379d6f3ccd011c9690c1ed5e4f87ea1576f874bf376347fb45c4fa9d38ca65bd014843b8559075ed99b583e3e2539a28b7c1d0d3032ed67927165a64217e031d72b8b2d349f02cac8e4b9295cbc6b4e55d051ac04b751833ec92348315a7c65cc8e4d480943091627342f0df603e7644f88fde5f84cfc473c4c6b64e0a4eb218e5c468782c6510e7e97f13ecd08997f28b371f3055c663a325ddadf5a503d8e073df0f60b8349e8dcc7fcf003a1016264887646574b6ef23f7eb04578262a11d5e17116b0413f64a7d13875a339be53435cd694a9cebb04d07d2ae15c73c53e5941b046fd7c14d07f76f244c41b2b29da578c8ad3ef7b00f56338eb49c4b7ee1a5f2528bf7b54e0a8d49ffc8388b2c0e498454243597369b1a90fcf057d39685c48abba7ea036faf34355228eda40cf38a7f97417aa18fc32bd46dd27a2e9db33e1a681ce6801dd12038bafc767766013a57694369831c1c1d4310379750117b8477539d3390d";
    const COMM_8: vector<u8> = x"32b8a003ed55454670138ad4f04c716798224e8971f7c65f36d8041e42f85f26";
    // Proof and commitment for 60000 at 16 bits under BLINDER.
    const PROOF_16: vector<u8> = x"ec9cc71cd5af0dc935ccdadf08cd2a67f2369e1d9ad2ba9e10658211f38d0a198e426df33464fc54205f2d66a910c74372fbdf09c87ec3b27a283920fd0d5339f468acd4c91a0f8b4ccb9274fa852d0fd60bf4ea30382671eb9f66d7bbc3a122229a062ddbdb614cd96106653cfdfb7944c76dbb207c19d78763da199dc10b5344b36a6e03f64f981df6df50986da0b85ef2b7b5629320557601fda1c6c1fa0dfb03635168d53d10d5500486f5cd716e84967c2e5e18eb8499095996d808b9063cf42639f29612663bd271527ae8fe1ee4ea358eac78dd7190cf4ece9c723b05001f383f62ec5ad380a12be51de73ec150bb7f1fe87adb12ba8e6ff6cde67a269e8677dd4d0c772ff465eb45001843c39ac91ad8baddcefe8264838201228733dc4804549e318c00615f186c60c721bd8d1beda1e01554565bff38acbee31b6802ee46b29cfc1e84bab36f6a969b92b94919ed3fc80c3cb63686916eac680365dc554d7161ae804073ba99d7bdf96e6fb7eb1f5d2ce7dd272c225a8bbfee2f7c8c79ee92a7bb09cbad5d5fd24a998dbe1a448b1e4a0a1a0ae397d3b741fe9252a4840ca47339f0709dd22a76f533ada40b21328ddde3ac3a56e41dba1531a66f4a0286ff5d3d5b63ea894c779aa3072f744d9b06ea859a4ec61b46c049d8be79a8bc215ca0a855a63a3e2dca13add9557071b8727462a9b5c409d0b00ad1b106bf17ee544ecc2204d6e225b1f8959c43d60a0e67fd8c46cd072a7696e9cc1302";
    const COMM_16: vector<u8> = x"2c801e20c589b81a2a50845b3736ea6a46a153d8cdb91643b433ef2584337e24";
    // Proof and commitment for 4000000000 at 32 bits under BLINDER.
    const PROOF_32: vector<u8> = x"3080794a7ef999c47e16575432bb82423b1010717317381467d3e7e8b6ff1a28c22cd489204dc86aae2f991c0c546c77db539fcb58ee962a3a73385c9b4c70644ef0df3823e4f6ef8d953bb2a865d937393292d031e0746efc194dcea66fdc62e0f604f49e99c85d8a9f89c5c4722df7923672cca85f2fecf93febfc4a6a43362e724b7abd64ea68c08986714303fde8e7ea6773e6debf7df54f5c584e617b050b98eadbc1f78e16284b1c8c34dedaafe1e803b59ddd7e33275ef2059a406b0d5b6c2c722b997b1518c208613892529fa7aeb7a2ea61d2bb54641e3c81d90e06c0c2caa014e1769f3a0e1788e79242683838fd2626aca4cf0118a9d7e7a1e009d053f3ade5c8995fa8be733f319dd8441ef1ef9ff5051be5d208d5e3d4d0c6406a668a8337ef654fc327d9f86c1fa5816658135749f7fdb72cd93342857f726e3cd1b769c9c0c3ff0462c8d44956f5286ae629ebe6f8cb3c1e171c0e85fcd027004856e9fb9e95392a99c65dd2b39d06865d1fb8eabbb34c0ed1cc675e278e114aa3a5e25c7b0cc5599d4fd8a6d57c1bbd76dfee5dbc49e7023c0786ad40a05e021279d01d79dc950000c289b31a04f8c5ce5fdb42b150ce5005e0aef88b9236c6ac74d7be5ff26245e4f1e16db38f556a61806d83b9c0a16068c494e322823e00c5fe11b1c32bff6e06275d6aeb93317b0176af59ae3c7d8962a869e731c42686beb4764afe7d1b18a990ea79f88e0dab117d015e1a996b96d6e622d124c762cfbec64c8921bbd5b066441f4d414d9508cd966cac2390d8004221a090d12f0adc07b94668cd66ecd9bfa8e19121de01579d60fa6769cf8eb296f81b23d3b804";
    const COMM_32: vector<u8> = x"d0006014adc10cdb1049e3e294491dae082ff89697f3c5e23fb6f24bda92337f";

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

    fun verify(comm: vector<u8>, proof: vector<u8>, num_bits: u64): bool {
        verify_range_proof_internal(
            comm, &ristretto255::basepoint(), &ristretto255::hash_base(), proof, num_bits, DST
        )
    }

    fun verify_batch(comms: vector<vector<u8>>, proof: vector<u8>, num_bits: u64): bool {
        verify_batch_range_proof_internal(
            comms, &ristretto255::basepoint(), &ristretto255::hash_base(), proof, num_bits, DST
        )
    }

    fun pinned_comm(num_bits: u64): vector<u8> {
        if (num_bits == 8) COMM_8 else if (num_bits == 16) COMM_16 else COMM_32
    }

    fun pinned_proof(num_bits: u64): vector<u8> {
        if (num_bits == 8) PROOF_8 else if (num_bits == 16) PROOF_16 else PROOF_32
    }

    // A fixed proof, verified at its own width and at another.
    public fun pinned(num_bits: u64, other_bits: u64): (bool, bool) {
        let comm = pinned_comm(num_bits);
        let proof = pinned_proof(num_bits);
        (verify(comm, proof, num_bits), verify(comm, proof, other_bits))
    }

    // A fresh proof: its length (32 * (2 * log2(num_bits) + 9) bytes), whether its commitment
    // equals the fixed one (commitments are deterministic), and verification at both widths.
    public fun single(value: u64, num_bits: u64, other_bits: u64): (u64, bool, bool, bool) {
        let (proof, comm) = prove_range_internal(
            ristretto255::scalar(value), BLINDER, num_bits, DST,
            &ristretto255::basepoint(), &ristretto255::hash_base()
        );
        (
            proof.length(),
            comm == pinned_comm(num_bits),
            verify(comm, proof, num_bits),
            verify(comm, proof, other_bits)
        )
    }

    fun blinder(index: u64): vector<u8> {
        ristretto255::scalar(1000003 * (index + 1))
    }

    // Values index + 1 under distinct blinders. Returns the proof length (as for `single`, with
    // `size * num_bits` bits), the commitment count, whether each commitment equals
    // value * B + blinder * H at its position, and verification at `num_bits`, at `other_bits`,
    // and with the commitments reversed (which changes nothing for a batch of one).
    public fun batch(size: u64, num_bits: u64, other_bits: u64): (u64, u64, bool, bool, bool, bool) {
        let vals = vector[];
        let rs = vector[];
        let index = 0;
        while (index < size) {
            vals.push_back(ristretto255::scalar(index + 1));
            rs.push_back(blinder(index));
            index = index + 1;
        };
        let (proof, comms) = prove_batch_range_internal(
            vals, rs, num_bits, DST, &ristretto255::basepoint(), &ristretto255::hash_base()
        );
        let in_order = comms.length() == size;
        index = 0;
        while (in_order && index < size) {
            in_order = comms[index] == ristretto255::commitment(index + 1, blinder(index));
            index = index + 1;
        };
        let reversed = comms;
        reversed.reverse();
        (
            proof.length(),
            comms.length(),
            in_order,
            verify_batch(comms, proof, num_bits),
            verify_batch(comms, proof, other_bits),
            verify_batch(reversed, proof, num_bits)
        )
    }
}

// RUN: execute 0x1::ristretto255_bulletproofs::pinned --args 8, 16
// CHECK: results: true, false

// RUN: execute 0x1::ristretto255_bulletproofs::pinned --args 16, 8
// CHECK: results: true, false

// RUN: execute 0x1::ristretto255_bulletproofs::pinned --args 32, 64
// CHECK: results: true, false

// RUN: execute 0x1::ristretto255_bulletproofs::single --args 200, 8, 16
// CHECK: results: 480, true, true, false

// RUN: execute 0x1::ristretto255_bulletproofs::single --args 60000, 16, 8
// CHECK: results: 544, true, true, false

// RUN: execute 0x1::ristretto255_bulletproofs::single --args 4000000000, 32, 64
// CHECK: results: 608, true, true, false

// RUN: execute 0x1::ristretto255_bulletproofs::batch --args 1, 64, 32
// CHECK: results: 672, 1, true, true, false, true

// RUN: execute 0x1::ristretto255_bulletproofs::batch --args 4, 32, 64
// CHECK: results: 736, 4, true, true, false, false

// RUN: execute 0x1::ristretto255_bulletproofs::batch --args 8, 16, 8
// CHECK: results: 736, 8, true, true, false, false

// RUN: execute 0x1::ristretto255_bulletproofs::batch --args 16, 16, 8
// CHECK: results: 800, 16, true, true, false, false
