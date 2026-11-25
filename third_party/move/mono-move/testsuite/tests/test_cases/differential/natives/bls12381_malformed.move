// bls12381 verifiers on a malformed key or signature (off curve, last byte dropped, or a zero
// byte appended), and signing and PoP generation with a zero secret key. Each sits beside the
// same call on the valid input. SK, PK, the expected signature of MSG under SK, and the PoP
// keys and proof are aptos_std test vectors; AGG_SIG combines aptos_std's three-signer
// aggregate with PK_A's signature on "tail 82".

// RUN: publish
module 0x1::bls12381 {
    struct PublicKeyWithPoP has copy, drop, store {
        bytes: vector<u8>,
    }

    const SK: vector<u8> = x"077c8a56f26259215a4a245373ab6ddf328ac6e00e5ea38d8700efa361bdc58d";
    // Zero is outside the valid scalar range [1, r).
    const ZERO_SK: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000";
    const MSG: vector<u8> = b"Hello Aptos!";
    const PK: vector<u8> = x"94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858";

    const MSG_A: vector<u8> = b"tail 225";
    const PK_A: vector<u8> = x"a3caedb9c2a5d8e922359ef69f9c35b8c819bcb081610343148dc3a2c50255c9caa6090f49f890ca31d853384fc80d00";
    const PK_A_OFF_CURVE: vector<u8> = x"a3caedb9c2a5d8e922359ef69f9c35b8c819bcb081610343148dc3a2c50255c9caa6090f49f890ca31d853384fc80d01";
    const SIG_A: vector<u8> = x"82d7c2d71d44f39a0f20f4183635fb632940120464b52b5f03d899365fa8e51818237d79c35f749ced3e2ceecf3b54e3092e54fd70c58bd36782d6889cf861b778642a8a69a1ab0c83ffba576d064c5f6db0ab5a5776c1831d8cb7662bc9b100";
    const SIG_A_OFF_CURVE: vector<u8> = x"82d7c2d71d44f39a0f20f4183635fb632940120464b52b5f03d899365fa8e51818237d79c35f749ced3e2ceecf3b54e3092e54fd70c58bd36782d6889cf861b778642a8a69a1ab0c83ffba576d064c5f6db0ab5a5776c1831d8cb7662bc9b103";

    const POP_PK_0: vector<u8> = x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7";
    const POP_PK_0_OFF_CURVE: vector<u8> = x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d9";
    const POP_PK_1: vector<u8> = x"8843843c76d167c02842a214c21277bad0bfd83da467cb5cf2d3ee67b2dcc7221b9fafa6d430400164012580e0c34d27";
    const POP_0: vector<u8> = x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364";
    const POP_0_OFF_CURVE: vector<u8> = x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863365";
    const PK_B: vector<u8> = x"a90981ef556f8e9a9f9aaf2ab7765db49c71dd3bcde51e4df7d40b8c48b77c30498a9a767b3dffe2210b84f78ee070e6";
    const POP_B: vector<u8> = x"b9f207f5e63922c93992ff372ea10f7c0dc453868f0acf8ef6e9459c2e7343b7d9a30d7b278a3e27f24516059e52f7bb1035b4b9f390b12dcb9c3df6190396c9d71d97981a59c6e568f520ba1e17c6b3a4bae222e92cfc42e4ba5efeef135e00";

    const AGG_PK_0: vector<u8> = x"b93d6aabb2b83e52f4b8bda43c24ea920bbced87a03ffc80f8f70c814a8b3f5d69fbb4e579ca76ee008d61365747dbc6";
    const AGG_PK_1: vector<u8> = x"b45648ceae3a983bcb816a96db599b5aef3b688c5753fa20ce36ac7a4f2c9ed792ab20af6604e85e42dab746398bb82c";
    const AGG_PK_2: vector<u8> = x"b3e4921277221e01ed71284be5e3045292b26c7f465a6fcdba53ee47edd39ec5160da3b229a73c75671024dcb36de091";
    const AGG_SIG: vector<u8> = x"94ad23c02902ca8efde147d1de95933ca89885e6ca76f2b65b0d77c5a50d98f944ff8f528c6bbb40f6ad33b73ee1f9ec103e6e461f8fda100c4a02d957f434fd6bedae07ff02015916dfccb64a34d19f72eca2759c57c89da2789e8551819100";
    const AGG_SIG_OFF_CURVE: vector<u8> = x"94ad23c02902ca8efde147d1de95933ca89885e6ca76f2b65b0d77c5a50d98f944ff8f528c6bbb40f6ad33b73ee1f9ec103e6e461f8fda100c4a02d957f434fd6bedae07ff02015916dfccb64a34d19f72eca2759c57c89da2789e8551819101";

    native fun verify_normal_signature_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;
    native fun verify_signature_share_internal(
        signature_share: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;
    native fun verify_multisignature_internal(
        multisignature: vector<u8>,
        agg_public_key: vector<u8>,
        message: vector<u8>,
    ): bool;
    native fun verify_proof_of_possession_internal(
        public_key: vector<u8>,
        proof_of_possession: vector<u8>,
    ): bool;
    native fun verify_aggregate_signature_internal(
        aggsig: vector<u8>,
        public_keys: vector<PublicKeyWithPoP>,
        messages: vector<vector<u8>>,
    ): bool;
    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;
    native fun generate_proof_of_possession_internal(sk: vector<u8>): vector<u8>;

    // The valid inputs passed to these end in a zero byte, so a decoder that padded or
    // truncated would recover them.
    fun drop_last(bytes: vector<u8>): vector<u8> {
        bytes.pop_back();
        bytes
    }

    fun append_zero(bytes: vector<u8>): vector<u8> {
        bytes.push_back(0);
        bytes
    }

    // 0: normal, 1: share, 2: multisignature.
    fun verify_with(verifier: u8, sig: vector<u8>, pk: vector<u8>): bool {
        if (verifier == 0) {
            verify_normal_signature_internal(sig, pk, MSG_A)
        } else if (verifier == 1) {
            verify_signature_share_internal(sig, pk, MSG_A)
        } else {
            verify_multisignature_internal(sig, pk, MSG_A)
        }
    }

    fun pk_cases(verifier: u8): (bool, bool, bool, bool) {
        (
            verify_with(verifier, SIG_A, PK_A),
            verify_with(verifier, SIG_A, PK_A_OFF_CURVE),
            verify_with(verifier, SIG_A, drop_last(PK_A)),
            verify_with(verifier, SIG_A, append_zero(PK_A)),
        )
    }

    fun signature_cases(verifier: u8): (bool, bool, bool, bool) {
        (
            verify_with(verifier, SIG_A, PK_A),
            verify_with(verifier, SIG_A_OFF_CURVE, PK_A),
            verify_with(verifier, drop_last(SIG_A), PK_A),
            verify_with(verifier, append_zero(SIG_A), PK_A),
        )
    }

    public fun normal_pk_malformed(): (bool, bool, bool, bool) {
        pk_cases(0)
    }

    public fun share_pk_malformed(): (bool, bool, bool, bool) {
        pk_cases(1)
    }

    public fun multisig_pk_malformed(): (bool, bool, bool, bool) {
        pk_cases(2)
    }

    public fun normal_signature_malformed(): (bool, bool, bool, bool) {
        signature_cases(0)
    }

    public fun share_signature_malformed(): (bool, bool, bool, bool) {
        signature_cases(1)
    }

    public fun multisig_signature_malformed(): (bool, bool, bool, bool) {
        signature_cases(2)
    }

    public fun verify_pop(): (bool, bool, bool, bool) {
        (
            verify_proof_of_possession_internal(POP_PK_0, POP_0),
            // A malformed proof with a good key.
            verify_proof_of_possession_internal(POP_PK_0, POP_0_OFF_CURVE),
            // A valid proof under another valid key: decodes, fails verification.
            verify_proof_of_possession_internal(POP_PK_1, POP_0),
            // A same-length off-curve key with the valid proof.
            verify_proof_of_possession_internal(POP_PK_0_OFF_CURVE, POP_0),
        )
    }

    public fun verify_pop_wrong_length(): (bool, bool, bool) {
        (
            verify_proof_of_possession_internal(PK_B, POP_B),
            verify_proof_of_possession_internal(PK_B, drop_last(POP_B)),
            verify_proof_of_possession_internal(PK_B, append_zero(POP_B)),
        )
    }

    // PK_A is second, paired with "tail 82".
    fun verify_aggregate(aggsig: vector<u8>, second_key: vector<u8>): bool {
        verify_aggregate_signature_internal(
            aggsig,
            vector[
                PublicKeyWithPoP { bytes: AGG_PK_0 },
                PublicKeyWithPoP { bytes: second_key },
                PublicKeyWithPoP { bytes: AGG_PK_1 },
                PublicKeyWithPoP { bytes: AGG_PK_2 },
            ],
            vector[b"Hello, Aptos 1!", b"tail 82", b"Hello, Aptos 2!", b"Hello, Aptos 3!"],
        )
    }

    public fun verify_aggregate_pk_malformed(): (bool, bool, bool, bool) {
        (
            verify_aggregate(AGG_SIG, PK_A),
            verify_aggregate(AGG_SIG, PK_A_OFF_CURVE),
            verify_aggregate(AGG_SIG, drop_last(PK_A)),
            verify_aggregate(AGG_SIG, append_zero(PK_A)),
        )
    }

    public fun verify_aggregate_signature_malformed(): (bool, bool, bool, bool) {
        (
            verify_aggregate(AGG_SIG, PK_A),
            verify_aggregate(AGG_SIG_OFF_CURVE, PK_A),
            verify_aggregate(drop_last(AGG_SIG), PK_A),
            verify_aggregate(append_zero(AGG_SIG), PK_A),
        )
    }

    public fun sign_valid_key(): vector<u8> {
        sign_internal(SK, MSG)
    }

    public fun sign_zero_key(): vector<u8> {
        sign_internal(ZERO_SK, MSG)
    }

    public fun pop_valid_key(): bool {
        verify_proof_of_possession_internal(PK, generate_proof_of_possession_internal(SK))
    }

    public fun pop_zero_key(): vector<u8> {
        generate_proof_of_possession_internal(ZERO_SK)
    }
}

// RUN: execute 0x1::bls12381::normal_pk_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::share_pk_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::multisig_pk_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::normal_signature_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::share_signature_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::multisig_signature_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::verify_pop
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::verify_pop_wrong_length
// CHECK: results: true, false, false

// RUN: execute 0x1::bls12381::verify_aggregate_pk_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::verify_aggregate_signature_malformed
// CHECK: results: true, false, false, false

// RUN: execute 0x1::bls12381::sign_valid_key
// CHECK: results: 0xb01ce4632e94d8c611736e96aa2ad8e0528a02f927a81a92db8047b002a8c71dc2d6bfb94729d0973790c10b6ece446817e4b7543afd7ca9a17c75de301ae835d66231c26a003f11ae26802b98d90869a9e73788c38739f7ac9d52659e1f7cf7

// RUN: execute 0x1::bls12381::sign_zero_key
// CHECK: aborted: code 655361 (BLS12381 sign computation failed) in 0x1::bls12381

// RUN: execute 0x1::bls12381::pop_valid_key
// CHECK: results: true

// RUN: execute 0x1::bls12381::pop_zero_key
// CHECK: aborted: code 655362 (BLS12381 proof-of-possession secret key deserialization failed) in 0x1::bls12381
