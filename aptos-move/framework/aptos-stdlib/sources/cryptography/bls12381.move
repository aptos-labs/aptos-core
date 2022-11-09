/// Contains functions for:
///
///     The minimum-pubkey-size variant of [Boneh-Lynn-Shacham (BLS) signatures](https://en.wikipedia.org/wiki/BLS_digital_signature),
///     where public keys are BLS12-381 elliptic-curve points in $\mathbb{G}_1$ and signatures are in $\mathbb{G}_2$,
///     as per the [IETF BLS draft standard](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature#section-2.1).

module aptos_std::bls12381 {
    use std::option::{Self, Option};
    #[test_only]
    use std::error::invalid_argument;

    /// The signature size, in bytes
    const SIGNATURE_SIZE: u64 = 96;

    /// The public key size, in bytes
    const PUBLIC_KEY_NUM_BYTES: u64 = 48;

    /// The caller was supposed to input one or more public keys.
    const EZERO_PUBKEYS: u64 = 1;

    /// One of the given inputs has the wrong size.s
    const EWRONG_SIZE: u64 = 2;

    /// The number of signers does not match the number of messages to be signed.
    const ESIGNER_COUNT_NOT_MATCH_MESSAGE_COUNT: u64 = 3;

    // TODO: Performance would increase if structs in this module are implemented natively via handles (similar to Table and
    // RistrettoPoint). This will avoid unnecessary (de)serialization. We would need to allow storage of these structs too.

    #[test_only]
    struct SecretKey has copy, drop {
        bytes: vector<u8>,
    }

    /// A *validated* public key that:
    ///   (1) is a point in the prime-order subgroup of the BLS12-381 elliptic curve, and
    ///   (2) is not the identity point
    ///
    /// This struct can be used to verify a normal (non-aggregated) signature.
    ///
    /// This struct can be combined with a ProofOfPossession struct in order to create a PublicKeyWithPop struct, which
    /// can be used to verify a multisignature.
    struct PublicKey has copy, drop, store {
        bytes: vector<u8>
    }

    /// A proof-of-possession (PoP).
    /// Given such a struct and a PublicKey struct, one can construct a PublicKeyWithPoP (see below).
    struct ProofOfPossession has copy, drop, store {
        bytes: vector<u8>
    }

    /// A *validated* public key that had a successfully-verified proof-of-possession (PoP).
    ///
    /// A vector of these structs can be either:
    ///   (1) used to verify an aggregate signature
    ///   (2) aggregated with other PublicKeyWithPoP structs into an AggrPublicKeysWithPoP, which in turn can be used
    ///       to verify a multisignature
    struct PublicKeyWithPoP has copy, drop, store {
        bytes: vector<u8>
    }

    /// An aggregation of public keys with verified PoPs, which can be used to verify multisignatures.
    struct AggrPublicKeysWithPoP has copy, drop, store {
        bytes: vector<u8>
    }

    /// A BLS signature. This can be either a:
    ///   (1) normal (non-aggregated) signature
    ///   (2) signature share (for a multisignature or aggregate signature)
    struct Signature has copy, drop, store {
        bytes: vector<u8>
    }

    /// An aggregation of BLS signatures. This can be either a:
    ///   (4) aggregated signature (i.e., an aggregation of signatures s_i, each on a message m_i)
    ///   (3) multisignature (i.e., an aggregation of signatures s_i, each on the same message m)
    ///
    /// We distinguish between a Signature type and a AggrOrMultiSignature type to prevent developers from interchangeably
    /// calling `verify_multisignature` and `verify_signature_share` to verify both multisignatures and signature shares,
    /// which could create problems down the line.
    struct AggrOrMultiSignature has copy, drop, store {
        bytes: vector<u8>
    }

    /// Creates a new public key from a sequence of bytes.
    public fun public_key_from_bytes(bytes: vector<u8>): Option<PublicKey> {
        if (validate_pubkey_internal(bytes)) {
            option::some(PublicKey {
                bytes
            })
        } else {
            option::none<PublicKey>()
        }
    }

    /// Serializes a public key into 48 bytes.
    public fun public_key_to_bytes(pk: &PublicKey): vector<u8> {
        pk.bytes
    }

    /// Creates a new proof-of-possession (PoP) which can be later used to create a PublicKeyWithPoP struct,
    public fun proof_of_possession_from_bytes(bytes: vector<u8>): ProofOfPossession {
        ProofOfPossession {
            bytes
        }
    }

    /// Serializes the signature into 96 bytes.
    public fun proof_of_possession_to_bytes(pop: &ProofOfPossession): vector<u8> {
        pop.bytes
    }

    /// Creates a PoP'd public key from a normal public key and a corresponding proof-of-possession.
    public fun public_key_from_bytes_with_pop(pk_bytes: vector<u8>, pop: &ProofOfPossession): Option<PublicKeyWithPoP> {
        if (verify_proof_of_possession_internal(pk_bytes, pop.bytes)) {
            option::some(PublicKeyWithPoP {
                bytes: pk_bytes
            })
        } else {
            option::none<PublicKeyWithPoP>()
        }
    }

    /// Creates a normal public key from a PoP'd public key.
    public fun public_key_with_pop_to_normal(pkpop: &PublicKeyWithPoP): PublicKey {
        PublicKey {
            bytes: pkpop.bytes
        }
    }

    /// Serializes a PoP'd public key into 48 bytes.
    public fun public_key_with_pop_to_bytes(pk: &PublicKeyWithPoP): vector<u8> {
        pk.bytes
    }

    /// Creates a new signature from a sequence of bytes. Does not check the signature for prime-order subgroup
    /// membership since that is done implicitly during verification.
    public fun signature_from_bytes(bytes: vector<u8>): Signature {
        Signature {
            bytes
        }
    }

    /// Serializes the signature into 96 bytes.
    public fun signature_to_bytes(sig: &Signature): vector<u8> {
        sig.bytes
    }

    /// Checks that the group element that defines a signature is in the prime-order subgroup.
    /// This check is implicitly performed when verifying any signature via this module, but we expose this functionality
    /// in case it might be useful for applications to easily dismiss invalid signatures early on.
    public fun signature_subgroup_check(signature: &Signature): bool {
        signature_subgroup_check_internal(signature.bytes)
    }

    /// Given a vector of public keys with verified PoPs, combines them into an *aggregated* public key which can be used
    /// to verify multisignatures using `verify_multisignature` and aggregate signatures using `verify_aggregate_signature`.
    /// Aborts if no public keys are given as input.
    public fun aggregate_pubkeys(public_keys: vector<PublicKeyWithPoP>): AggrPublicKeysWithPoP {
        let (bytes, success) = aggregate_pubkeys_internal(public_keys);
        assert!(success, std::error::invalid_argument(EZERO_PUBKEYS));

        AggrPublicKeysWithPoP {
            bytes
        }
    }

    /// Serializes an aggregate public key into 48 bytes.
    public fun aggregate_pubkey_to_bytes(apk: &AggrPublicKeysWithPoP): vector<u8> {
        apk.bytes
    }

    /// Aggregates the input signatures into an aggregate-or-multi-signature structure, which can be later verified via
    /// `verify_aggregate_signature` or `verify_multisignature`. Returns `None` if zero signatures are given as input
    /// or if some of the signatures are not valid group elements.
    public fun aggregate_signatures(signatures: vector<Signature>): Option<AggrOrMultiSignature> {
        let (bytes, success) = aggregate_signatures_internal(signatures);
        if (success) {
            option::some(
                AggrOrMultiSignature {
                    bytes
                }
            )
        } else {
            option::none<AggrOrMultiSignature>()
        }
    }

    /// Serializes an aggregate-or-multi-signature into 96 bytes.
    public fun aggr_or_multi_signature_to_bytes(sig: &AggrOrMultiSignature): vector<u8> {
        sig.bytes
    }

    /// Deserializes an aggregate-or-multi-signature from 96 bytes.
    public fun aggr_or_multi_signature_from_bytes(bytes: vector<u8>): AggrOrMultiSignature {
        assert!(std::vector::length(&bytes) == SIGNATURE_SIZE, std::error::invalid_argument(EWRONG_SIZE));

        AggrOrMultiSignature {
            bytes
        }
    }


    /// Checks that the group element that defines an aggregate-or-multi-signature is in the prime-order subgroup.
    public fun aggr_or_multi_signature_subgroup_check(signature: &AggrOrMultiSignature): bool {
        signature_subgroup_check_internal(signature.bytes)
    }

    /// Verifies an aggregate signature, an aggregation of many signatures `s_i`, each on a different message `m_i`.
    public fun verify_aggregate_signature(
        aggr_sig: &AggrOrMultiSignature,
        public_keys: vector<PublicKeyWithPoP>,
        messages: vector<vector<u8>>,
    ): bool {
        verify_aggregate_signature_internal(aggr_sig.bytes, public_keys, messages)
    }

    /// Verifies a multisignature: an aggregation of many signatures, each on the same message `m`.
    public fun verify_multisignature(
        multisig: &AggrOrMultiSignature,
        aggr_public_key: &AggrPublicKeysWithPoP,
        message: vector<u8>
    ): bool {
        verify_multisignature_internal(multisig.bytes, aggr_public_key.bytes, message)
    }

    /// Verifies a normal, non-aggregated signature.
    public fun verify_normal_signature(
        signature: &Signature,
        public_key: &PublicKey,
        message: vector<u8>
    ): bool {
        verify_normal_signature_internal(signature.bytes, public_key.bytes, message)
    }

    /// Verifies a signature share in the multisignature share or an aggregate signature share.
    public fun verify_signature_share(
        signature_share: &Signature,
        public_key: &PublicKeyWithPoP,
        message: vector<u8>
    ): bool {
        verify_signature_share_internal(signature_share.bytes, public_key.bytes, message)
    }

    #[test_only]
    /// Generates a BLS key-pair: a secret key with its corresponding public key.
    public fun generate_keys(): (SecretKey, PublicKeyWithPoP) {
        let (sk_bytes, pk_bytes) = generate_keys_internal();
        let sk = SecretKey {
            bytes: sk_bytes
        };
        let pkpop = PublicKeyWithPoP {
            bytes: pk_bytes
        };
        (sk, pkpop)
    }

    #[test_only]
    /// Generates a BLS signature for a message with a signing key.
    public fun sign_arbitrary_bytes(signing_key: &SecretKey, message: vector<u8>): Signature {
        Signature {
            bytes: sign_internal(signing_key.bytes, message)
        }
    }

    #[test_only]
    /// Generates a multi-signature for a message with multiple signing keys.
    public fun multi_sign_arbitrary_bytes(signing_keys: &vector<SecretKey>, message: vector<u8>): AggrOrMultiSignature {
        let n = std::vector::length(signing_keys);
        let sigs = vector[];
        let i: u64 = 0;
        while (i < n) {
            let sig = sign_arbitrary_bytes(std::vector::borrow(signing_keys, i), message);
            std::vector::push_back(&mut sigs, sig);
            i = i + 1;
        };
        let multisig = aggregate_signatures(sigs);
        option::extract(&mut multisig)
    }

    #[test_only]
    /// Generates an aggregated signature over all messages in messages, where signing_keys[i] signs messages[i].
    public fun aggr_sign_arbitrary_bytes(signing_keys: &vector<SecretKey>, messages: &vector<vector<u8>>): AggrOrMultiSignature {
        let signing_key_count = std::vector::length(signing_keys);
        let message_count = std::vector::length(messages);
        assert!(signing_key_count == message_count, invalid_argument(ESIGNER_COUNT_NOT_MATCH_MESSAGE_COUNT));
        let sigs = vector[];
        let i: u64 = 0;
        while (i < signing_key_count) {
            let sig = sign_arbitrary_bytes(std::vector::borrow(signing_keys, i), *std::vector::borrow(messages, i));
            std::vector::push_back(&mut sigs, sig);
            i = i + 1;
        };
        let multisig = aggregate_signatures(sigs);
        option::extract(&mut multisig)
    }

    //
    // Native functions
    //

    /// CRYPTOGRAPHY WARNING: This function assumes that the caller verified all public keys have a valid
    /// proof-of-possesion (PoP) using `verify_proof_of_possession`.
    ///
    /// Given a vector of serialized public keys, combines them into an aggregated public key, returning `(bytes, true)`,
    /// where `bytes` store the serialized public key.
    /// Aborts if no public keys are given as input.
    native fun aggregate_pubkeys_internal(public_keys: vector<PublicKeyWithPoP>): (vector<u8>, bool);


    /// CRYPTOGRAPHY WARNING: This function can be safely called without verifying that the input signatures are elements
    /// of the prime-order subgroup of the BLS12-381 curve.
    ///
    /// Given a vector of serialized signatures, combines them into an aggregate signature, returning `(bytes, true)`,
    /// where `bytes` store the serialized signature.
    /// Does not check the input signatures nor the final aggregated signatures for prime-order subgroup membership.
    /// Returns `(_, false)` if no signatures are given as input.
    /// Does not abort.
    native fun aggregate_signatures_internal(signatures: vector<Signature>): (vector<u8>, bool);

    /// Return `true` if the bytes in `public_key` are a valid BLS12-381 public key:
    ///  (1) it is NOT the identity point, and
    ///  (2) it is a BLS12-381 elliptic curve point, and
    ///  (3) it is a prime-order point
    /// Return `false` otherwise.
    /// Does not abort.
    native fun validate_pubkey_internal(public_key: vector<u8>): bool;

    /// Return `true` if the elliptic curve point serialized in `signature`:
    ///  (1) is NOT the identity point, and
    ///  (2) is a BLS12-381 elliptic curve point, and
    ///  (3) is a prime-order point
    /// Return `false` otherwise.
    /// Does not abort.
    native fun signature_subgroup_check_internal(signature: vector<u8>): bool;

    /// CRYPTOGRAPHY WARNING: First, this function assumes all public keys have a valid proof-of-possesion (PoP).
    /// This prevents both small-subgroup attacks and rogue-key attacks. Second, this function can be safely called
    /// without verifying that the aggregate signature is in the prime-order subgroup of the BLS12-381 curve.
    ///
    /// Returns `true` if the aggregate signature `aggsig` on `messages` under `public_keys` verifies (where `messages[i]`
    /// should be signed by `public_keys[i]`).
    ///
    /// Returns `false` if either:
    /// - no public keys or messages are given as input,
    /// - number of messages does not equal number of public keys
    /// - `aggsig` (1) is the identity point, or (2) is NOT a BLS12-381 elliptic curve point, or (3) is NOT a
    ///   prime-order point
    /// Does not abort.
    native fun verify_aggregate_signature_internal(
        aggsig: vector<u8>,
        public_keys: vector<PublicKeyWithPoP>,
        messages: vector<vector<u8>>,
    ): bool;

    /// CRYPTOGRAPHY WARNING: This function assumes verified proofs-of-possesion (PoP) for the public keys used in
    /// computing the aggregate public key. This prevents small-subgroup attacks and rogue-key attacks.
    ///
    /// Return `true` if the BLS `multisignature` on `message` verifies against the BLS aggregate public key `agg_public_key`.
    /// Returns `false` otherwise.
    /// Does not abort.
    native fun verify_multisignature_internal(
        multisignature: vector<u8>,
        agg_public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    /// CRYPTOGRAPHY WARNING: This function WILL check that the public key is a prime-order point, in order to prevent
    /// library users from misusing the library by forgetting to validate public keys before giving them as arguments to
    /// this function.
    ///
    /// Returns `true` if the `signature` on `message` verifies under `public key`.
    /// Returns `false` otherwise.
    /// Does not abort.
    native fun verify_normal_signature_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    /// Return `true` if the bytes in `public_key` are a valid bls12381 public key (as per `validate_pubkey`)
    /// *and* this public key has a valid proof-of-possesion (PoP).
    /// Return `false` otherwise.
    /// Does not abort.
    native fun verify_proof_of_possession_internal(
        public_key: vector<u8>,
        proof_of_possesion: vector<u8>
    ): bool;

    /// CRYPTOGRAPHY WARNING: Assumes the public key has a valid proof-of-possesion (PoP). This prevents rogue-key
    /// attacks later on during signature aggregation.
    ///
    /// Returns `true` if the `signature_share` on `message` verifies under `public key`.
    /// Returns `false` otherwise, similar to `verify_multisignature`.
    /// Does not abort.
    native fun verify_signature_share_internal(
        signature_share: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>
    ): bool;

    #[test_only]
    native fun generate_keys_internal(): (vector<u8>, vector<u8>);

    #[test_only]
    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;

    #[test_only]
    native fun generate_proof_of_possession_internal(sk: vector<u8>): vector<u8>;


    //
    // Tests
    //

    #[test_only]
    public fun maul_first_byte(bytes: &mut vector<u8>) {
        let first_sig_byte = std::vector::borrow_mut(bytes, 0);
        *first_sig_byte = *first_sig_byte ^ 0xff;
    }

    #[test_only]
    /// Generates a proof-of-possession (PoP) for the public key associated with the secret key `sk`.
    public fun generate_proof_of_possession(sk: &SecretKey): ProofOfPossession {
        ProofOfPossession {
            bytes: generate_proof_of_possession_internal(sk.bytes)
        }
    }

    #[test]
    fun test_pubkey_validation() {
        // test low order points (in group for PK)
        assert!(option::is_none(&public_key_from_bytes(x"ae3cd9403b69c20a0d455fd860e977fe6ee7140a7f091f26c860f2caccd3e0a7a7365798ac10df776675b3a67db8faa0")), 1);
        assert!(option::is_none(&public_key_from_bytes(x"928d4862a40439a67fd76a9c7560e2ff159e770dcf688ff7b2dd165792541c88ee76c82eb77dd6e9e72c89cbf1a56a68")), 1);
        assert!(option::is_some(&public_key_from_bytes(x"b3e4921277221e01ed71284be5e3045292b26c7f465a6fcdba53ee47edd39ec5160da3b229a73c75671024dcb36de091")), 1);
    }

    #[test]
    fun test_pubkey_validation_against_invalid_keys() {
        let (_sk, pk) = generate_keys();
        let pk_bytes = public_key_with_pop_to_bytes(&pk);
        assert!(option::is_some(&public_key_from_bytes(pk_bytes)), 1);

        maul_first_byte(&mut pk_bytes);
        assert!(option::is_none(&public_key_from_bytes(pk_bytes)), 1);
    }

    #[test]
    #[expected_failure(abort_code = 65537)]
    fun test_empty_pubkey_aggregation() {
        aggregate_pubkeys(std::vector::empty());
    }

    #[test]
    fun test_signature_aggregation() {
        // First, test empty aggregation
        assert!(option::is_none(&mut aggregate_signatures(vector[])), 1);

        // TODO: normal signature aggregation is covered in `test_gen_sign_verify_multi_signature()`.
        // This function should be renamed to `test_empty_signature_aggregation`.
    }

    #[test]
    fun test_pubkey_aggregation() {
        // Already covered in `test_gen_sign_verify_multi_signature()`.
    }

    #[test]
    fun test_gen_sign_verify_normal_signature_or_signature_share() {
        let (sk, pk) = generate_keys();
        let pk_unvalidated = public_key_with_pop_to_normal(&pk);

        let msg = b"hello world";
        let sig = sign_arbitrary_bytes(&sk, msg);
        assert!(verify_normal_signature(&sig, &pk_unvalidated, msg), 1);
        assert!(verify_signature_share(&sig, &pk, msg), 1);

        maul_first_byte(&mut sig.bytes);
        assert!(!verify_normal_signature(&sig, &pk_unvalidated, msg), 1);
        assert!(!verify_signature_share(&sig, &pk, msg), 1);
    }

    #[test]
    fun test_gen_sign_verify_multi_signature() {
        let (sk_a,pk_a) = generate_keys();
        let (sk_b,pk_b) = generate_keys();
        let signing_keys = vector[sk_a, sk_b];
        let aggr_pk = aggregate_pubkeys(vector[pk_a, pk_b]);

        let msg = b"hello world";

        let multisig = multi_sign_arbitrary_bytes(&signing_keys, msg);

        assert!(verify_multisignature(&multisig, &aggr_pk, msg), 1);

        // Also test signature aggregation.
        let sig_a = sign_arbitrary_bytes(&sk_a, msg);
        let sig_b = sign_arbitrary_bytes(&sk_b, msg);
        let sig_a_b = option::extract(&mut aggregate_signatures(vector[sig_a, sig_b]));
        assert!(aggr_or_multi_signature_subgroup_check(&sig_a_b), 1);
        assert!(aggr_or_multi_signature_to_bytes(&sig_a_b) == aggr_or_multi_signature_to_bytes(&multisig), 1);

        maul_first_byte(&mut multisig.bytes);
        assert!(!verify_multisignature(&multisig, &aggr_pk, msg), 1);
    }

    #[test]
    fun test_gen_sign_verify_aggregated_signature() {
        let (sk_a,pk_a) = generate_keys();
        let (sk_b,pk_b) = generate_keys();
        let signing_keys = vector[sk_a, sk_b];
        let public_keys = vector[pk_a, pk_b];

        let messages = vector[b"hello world", b"hello aptos"];
        let sig = aggr_sign_arbitrary_bytes(&signing_keys, &messages);
        assert!(verify_aggregate_signature(&sig, public_keys, messages), 1);

        maul_first_byte(&mut sig.bytes);
        assert!(!verify_aggregate_signature(&sig, public_keys, messages), 1);
    }

    #[test]
    fun test_empty_signature_aggregation() {
        assert!(option::is_none(&mut aggregate_signatures(vector[])), 1);
    }

    #[test]
    /// Tests verification of random BLS proofs-of-possession (PoPs)
    fun test_verify_pop() {
        let (sk, validated_pk) = generate_keys();
        let pk_bytes = public_key_with_pop_to_bytes(&validated_pk);
        let pop = generate_proof_of_possession(&sk);
        assert!(option::is_some(&public_key_from_bytes_with_pop(pk_bytes, &pop)), 1);
        maul_first_byte(&mut pop.bytes);
        assert!(option::is_none(&public_key_from_bytes_with_pop(pk_bytes, &pop)), 1);
    }
}
