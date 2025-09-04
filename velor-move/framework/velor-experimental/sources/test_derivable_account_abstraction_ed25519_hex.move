/// Domain account abstraction using ed25519 hex for signing.
///
/// Authentication takes digest, converts to hex (prefixed with 0x, with lowercase letters),
/// and then expects that to be signed.
/// authenticator is expected to be signature: vector<u8>
/// account_identity is raw public_key.
module velor_experimental::test_derivable_account_abstraction_ed25519_hex {
    use std::error;
    use velor_std::string_utils;
    use velor_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes
    };
    use velor_framework::auth_data::AbstractionAuthData;

    const EINVALID_SIGNATURE: u64 = 1;

    /// Authorization function for domain account abstraction.
    public fun authenticate(
        account: signer, aa_auth_data: AbstractionAuthData
    ): signer {
        let hex_digest = string_utils::to_string(aa_auth_data.digest());

        let public_key =
            new_unvalidated_public_key_from_bytes(
                *aa_auth_data.derivable_abstract_public_key()
            );
        let signature =
            new_signature_from_bytes(*aa_auth_data.derivable_abstract_signature());
        assert!(
            ed25519::signature_verify_strict(
                &signature, &public_key, *hex_digest.bytes()
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );

        account
    }
}
