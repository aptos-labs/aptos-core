module aptos_framework::common_domain_aa_auths {
    use std::error;
    use std::option::Option;
    use std::signer;
    use aptos_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes,
    };
    use aptos_framework::auth_data::{Self, AbstractionAuthData};
    use aptos_framework::bcs_stream::{Self, deserialize_u8};

    // takes digest, converts to hex, and then expects that to be signed.
    // authenticator is expected to be struct { public_key: vector<u8>, signature: vector<u8> }
    // account_identity is raw public_key.

    public fun authenticate_ed25519_hex(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let addr = signer::address_of(&account);

        let hex_digest = bytes_to_hex(aa_auth_data.digest());
        let stream = bcs_stream::new(*aa_auth_data.authenticator());
        let public_key_bytes = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));

        assert!(
            aa_auth_data.account_identity() == public_key_bytes,
            error::permission_denied(EINVALID_ACCOUNT_IDENTITY)
        );

        let public_key = new_unvalidated_public_key_from_bytes(public_key_bytes);
        let signature = new_signature_from_bytes(
            bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x))
        );
        assert!(
            ed25519::signature_verify_strict(
                &signature,
                &public_key,
                hex_digest,
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );

        account
    }

    fun bytes_to_hex(data: &vector<u8>): vector<u8> {
        // let result = vector::empty();
        // let i = 0;
        // while (i < data.length()) {
        //     let cur = data[i];
        //     vector::push_back(&mut result, high);
        //     vector::push_back(&mut result, low);
        //     i = i + 1;
        // };
        // result
    }
}
