module aptos_framework::common_domain_aa_auths {
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::string_utils;
    use aptos_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes,
    };
    use aptos_framework::auth_data::AbstractionAuthData;

    const EINVALID_SIGNATURE: u64 = 1;


    // takes digest, converts to hex (prefixed with 0x, with lowercase letters), and then expects that to be signed.
    // authenticator is expected to be signature: vector<u8>
    // account_identity is raw public_key.
    public fun authenticate_ed25519_hex(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let hex_digest = string_utils::to_string(aa_auth_data.digest());

        let public_key = new_unvalidated_public_key_from_bytes(*aa_auth_data.account_identity());
        let signature = new_signature_from_bytes(*aa_auth_data.authenticator());
        assert!(
            ed25519::signature_verify_strict(
                &signature,
                &public_key,
                *hex_digest.bytes(),
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );

        account
    }
}
