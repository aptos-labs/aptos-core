/// Cross-cutting orchestrator that atomically pairs keyless+backup-key setup with confidential-asset DK backup.
///
/// This module is intentionally the *only* place in the framework that depends on both
/// `aptos_framework::account` (for keyless/backup-key rotation) and
/// `aptos_framework::confidential_asset` (for EK registration and encrypted-DK storage).
/// Keeping the cross-feature wiring here lets `account` and `keyless_account` stay free of
/// confidential-asset vocabulary, and lets `confidential_asset` stay free of keyless vocabulary.
module aptos_framework::keyless_confidential_backup {
    use std::error;
    use std::signer;
    use aptos_std::ed25519;
    use aptos_std::multi_key;
    use aptos_std::single_key;
    use aptos_framework::account;
    use aptos_framework::confidential_asset;
    use aptos_framework::fungible_asset;
    use aptos_framework::object::Object;

    /// The account's authentication key does not match the multi-key derived from the provided keyless public key
    /// and Ed25519 backup public key, or the keyless slot is not a Keyless / FederatedKeyless public key.
    const E_NOT_KEYLESS_BACKUP_ACCOUNT: u64 = 1;

    /// A DK has already been backed up for this account. Subsequent EK registrations for other asset types should
    /// go through `confidential_asset::register_raw` and are assumed to use the same backed-up DK.
    const E_DK_ALREADY_BACKED_UP: u64 = 2;

    /// Atomically rotates the account's auth key to a (keyless, Ed25519-backup) 1-of-2 multi-key and stores an
    /// encrypted backup of the user's confidential-asset decryption key. Currently used by Petra for keyless
    /// accounts. The DK ciphertext is opaque to the chain — see `confidential_asset::EncryptedDK`.
    entry fun upsert_ed25519_backup_key_and_encrypt_dk(
        account: &signer,
        keyless_public_key: vector<u8>,
        backup_public_key: vector<u8>,
        backup_key_proof: vector<u8>,
        dk_ciphertext: vector<u8>
    ) {
        let keyless_single_key = single_key::new_public_key_from_bytes(keyless_public_key);

        account::upsert_ed25519_backup_key_on_keyless_account_internal(
            account,
            keyless_single_key,
            backup_public_key,
            backup_key_proof
        );

        confidential_asset::upsert_encrypted_dk(account, dk_ciphertext);
    }

    /// Atomically registers an EK for the specified `asset_type` and stores a DK ciphertext. This can only be
    /// called once: i.e., when registering an EK for the 1st time. Subsequent EK registrations for other asset
    /// types should be done via `confidential_asset::register_raw` and are assumed to use the same DK that was
    /// backed up here.
    ///
    /// `keyless_public_key` and `backup_public_key` must be the (1-of-2) keyless + Ed25519-backup multi-key
    /// currently authorizing this account; otherwise the call aborts. This pins the function to keyless+backup
    /// accounts and prevents accidentally backing up a DK for an account whose auth key has a different shape.
    entry fun register_ek_and_encrypt_dk(
        owner: &signer,
        keyless_public_key: vector<u8>,
        backup_public_key: vector<u8>,
        asset_type: Object<fungible_asset::Metadata>,
        ek: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>,
        dk_ciphertext: vector<u8>
    ) {
        let owner_addr = signer::address_of(owner);

        // Verify the account is currently authorized by a 1-of-2 multi-key over the supplied keyless PK and
        // Ed25519 backup PK. Rejects calls on accounts that haven't gone through `upsert_ed25519_backup_key_*`.
        // The keyless slot must actually be a `Keyless` or `FederatedKeyless` `AnyPublicKey`; without this check,
        // a 1-of-2 multi-key built from two Ed25519 keys (or any other shape) with a matching auth key would slip past.
        let keyless_single_key = single_key::new_public_key_from_bytes(keyless_public_key);
        assert!(
            single_key::is_keyless_or_federated_keyless_public_key(&keyless_single_key),
            error::invalid_argument(E_NOT_KEYLESS_BACKUP_ACCOUNT)
        );

        let expected_auth_key = multi_key::new_multi_key_from_single_keys(
            vector[
                keyless_single_key,
                single_key::from_ed25519_public_key_unvalidated(
                    ed25519::new_unvalidated_public_key_from_bytes(backup_public_key)
                )
            ],
            1
        ).to_authentication_key();

        assert!(
            account::get_authentication_key(owner_addr) == expected_auth_key,
            error::invalid_argument(E_NOT_KEYLESS_BACKUP_ACCOUNT)
        );

        // Single-shot: only allowed at first EK registration. Subsequent assets reuse the same DK via `register_raw`.
        assert!(
            !confidential_asset::has_encrypted_dk(owner_addr),
            error::already_exists(E_DK_ALREADY_BACKED_UP)
        );

        confidential_asset::register_raw(
            owner,
            asset_type,
            ek,
            sigma_proto_comm,
            sigma_proto_resp
        );

        confidential_asset::upsert_encrypted_dk(owner, dk_ciphertext);
    }
}
