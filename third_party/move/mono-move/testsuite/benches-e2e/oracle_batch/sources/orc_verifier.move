/// Signature checks over the framework's ed25519 and secp256k1 modules,
/// wrapped so that caller-supplied bytes can never abort.
///
/// Both framework modules assert on the length of what they are handed, and
/// `ecdsa_recover` aborts on a recovery id above three. Every input is brought
/// into range first, and a failed check is a `false` or an empty vector rather
/// than an abort, since a benchmark transaction that aborts panics the harness.
module bench::orc_verifier {
    use std::hash;
    use std::option;
    use std::vector;
    use aptos_std::ed25519;
    use aptos_std::secp256k1;

    const ED_PUBKEY_LEN: u64 = 32;
    const ED_SIG_LEN: u64 = 64;
    const SECP_SIG_LEN: u64 = 64;
    /// `ecdsa_recover` takes a 32-byte digest and applies no hashing of its own.
    const SECP_DIGEST_LEN: u64 = 32;
    /// Recovery ids run 0..3.
    const RECOVERY_IDS: u8 = 4;

    /// Truncate or zero-pad to exactly `len` bytes.
    public fun clamp(bytes: vector<u8>, len: u64): vector<u8> {
        while (vector::length(&bytes) > len) {
            vector::pop_back(&mut bytes);
        };
        while (vector::length(&bytes) < len) {
            vector::push_back(&mut bytes, 0);
        };
        bytes
    }

    /// Strict ed25519 verification. Returns `false` on a malformed key, a
    /// malformed signature, or a signature that does not verify.
    public fun verify_ed25519(
        pubkey: vector<u8>, signature: vector<u8>, message: vector<u8>
    ): bool {
        let pk = ed25519::new_unvalidated_public_key_from_bytes(
            clamp(pubkey, ED_PUBKEY_LEN));
        let sig = ed25519::new_signature_from_bytes(
            clamp(signature, ED_SIG_LEN));
        ed25519::signature_verify_strict(&sig, &pk, message)
    }

    /// The raw 64-byte secp256k1 public key recovered from `signature` over
    /// `digest`, or an empty vector when recovery failed.
    public fun recover_raw(
        digest: vector<u8>, recovery_id: u8, signature: vector<u8>
    ): vector<u8> {
        let sig = secp256k1::ecdsa_signature_from_bytes(
            clamp(signature, SECP_SIG_LEN));
        let recovered = secp256k1::ecdsa_recover(
            clamp(digest, SECP_DIGEST_LEN),
            recovery_id % RECOVERY_IDS,
            &sig,
        );
        if (option::is_some(&recovered)) {
            let pk = option::destroy_some(recovered);
            secp256k1::ecdsa_raw_public_key_to_bytes(&pk)
        } else {
            vector::empty()
        }
    }

    /// `sha3_256` of the recovered key, or an empty vector when recovery
    /// failed. Hashing the key is what makes it comparable to an authority
    /// address in `orc_queue`.
    public fun recover_secp256k1(
        digest: vector<u8>, recovery_id: u8, signature: vector<u8>
    ): vector<u8> {
        let raw = recover_raw(digest, recovery_id, signature);
        if (vector::length(&raw) == 0) raw else hash::sha3_256(raw)
    }
}
