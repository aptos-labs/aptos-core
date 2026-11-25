// bls12381 subgroup checks on points that decode but lie outside the prime-order subgroup:
// ORDER_11 (a G1 point of order 11), PK_PLUS_ORDER_11 (PK plus that point) and
// SIG_PLUS_TORSION (SIG plus a G2 point outside the subgroup). PK, SIG and MSG are from
// aptos_std's bls12381 normal-signature test. Aggregating a single point returns its
// encoding, showing each point decodes. The signature share verifier, which does not check
// the key's subgroup, accepts SIG under PK_PLUS_ORDER_11; the normal verifier rejects it.

// RUN: publish
module 0x1::bls12381 {
    struct PublicKeyWithPoP has copy, drop, store {
        bytes: vector<u8>,
    }

    struct Signature has copy, drop, store {
        bytes: vector<u8>,
    }

    const MSG: vector<u8> = b"Hello Aptos!";
    const PK: vector<u8> = x"94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858";
    const SIG: vector<u8> = x"b01ce4632e94d8c611736e96aa2ad8e0528a02f927a81a92db8047b002a8c71dc2d6bfb94729d0973790c10b6ece446817e4b7543afd7ca9a17c75de301ae835d66231c26a003f11ae26802b98d90869a9e73788c38739f7ac9d52659e1f7cf7";
    const ORDER_11: vector<u8> = x"b9b3e2c8c6bbf59d3c326b531fc1e639d29200c28624ac604f251a12908c9b7f735318617f625954cc71cdf03229b1ef";
    const PK_PLUS_ORDER_11: vector<u8> = x"82d43fadb12cce4c1d300124f15282cc6395417c895b59963a551633461e010dd990b2bb18fca89ecbd14871528e3b14";
    const SIG_PLUS_TORSION: vector<u8> = x"8535ff82fa57a7b0f9cdd6a24d96e6b6bc5d3e9270da9fbafadb4786111a68da6dfe6224498c590335029281e4298f771915c57883d4a6612488764f33d1838bc3f5cbacf73daeedfe842a84ab7d7a7bfd3367d6c06fc512be2117e5d5024dce";

    native fun validate_pubkey_internal(public_key: vector<u8>): bool;
    native fun signature_subgroup_check_internal(signature: vector<u8>): bool;
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
    native fun aggregate_pubkeys_internal(
        public_keys: vector<PublicKeyWithPoP>,
    ): (vector<u8>, bool);
    native fun aggregate_signatures_internal(signatures: vector<Signature>): (vector<u8>, bool);

    public fun validate_pubkey(): (bool, bool, bool) {
        (
            validate_pubkey_internal(PK),
            validate_pubkey_internal(ORDER_11),
            validate_pubkey_internal(PK_PLUS_ORDER_11),
        )
    }

    public fun decode_order_11(): (vector<u8>, bool) {
        aggregate_pubkeys_internal(vector[PublicKeyWithPoP { bytes: ORDER_11 }])
    }

    public fun decode_pk_plus_order_11(): (vector<u8>, bool) {
        aggregate_pubkeys_internal(vector[PublicKeyWithPoP { bytes: PK_PLUS_ORDER_11 }])
    }

    public fun signature_subgroup(): (bool, bool) {
        (
            signature_subgroup_check_internal(SIG),
            signature_subgroup_check_internal(SIG_PLUS_TORSION),
        )
    }

    public fun decode_sig_plus_torsion(): (vector<u8>, bool) {
        aggregate_signatures_internal(vector[Signature { bytes: SIG_PLUS_TORSION }])
    }

    public fun verify_shifted_key(): (bool, bool, bool, bool) {
        (
            verify_normal_signature_internal(SIG, PK, MSG),
            verify_signature_share_internal(SIG, PK, MSG),
            verify_signature_share_internal(SIG, PK_PLUS_ORDER_11, MSG),
            verify_normal_signature_internal(SIG, PK_PLUS_ORDER_11, MSG),
        )
    }
}

// RUN: execute 0x1::bls12381::validate_pubkey
// CHECK: results: true, false, false

// RUN: execute 0x1::bls12381::decode_order_11
// CHECK: results: 0xb9b3e2c8c6bbf59d3c326b531fc1e639d29200c28624ac604f251a12908c9b7f735318617f625954cc71cdf03229b1ef, true

// RUN: execute 0x1::bls12381::decode_pk_plus_order_11
// CHECK: results: 0x82d43fadb12cce4c1d300124f15282cc6395417c895b59963a551633461e010dd990b2bb18fca89ecbd14871528e3b14, true

// RUN: execute 0x1::bls12381::signature_subgroup
// CHECK: results: true, false

// RUN: execute 0x1::bls12381::decode_sig_plus_torsion
// CHECK: results: 0x8535ff82fa57a7b0f9cdd6a24d96e6b6bc5d3e9270da9fbafadb4786111a68da6dfe6224498c590335029281e4298f771915c57883d4a6612488764f33d1838bc3f5cbacf73daeedfe842a84ab7d7a7bfd3367d6c06fc512be2117e5d5024dce, true

// RUN: execute 0x1::bls12381::verify_shifted_key
// CHECK: results: true, true, true, false
