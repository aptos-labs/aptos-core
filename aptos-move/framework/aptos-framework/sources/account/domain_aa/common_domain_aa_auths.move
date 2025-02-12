module aptos_framework::common_domain_aa_auths {
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes,
    };
    use aptos_framework::auth_data::{Self, AbstractionAuthData};
    use aptos_framework::bcs_stream::{Self, deserialize_u8};

    const EINVALID_ACCOUNT_IDENTITY: u64 = 1;
    const EINVALID_SIGNATURE: u64 = 2;

    // takes digest, converts to hex, and then expects that to be signed.
    // authenticator is expected to be struct { public_key: vector<u8>, signature: vector<u8> }
    // account_identity is raw public_key.

    public fun authenticate_ed25519_hex(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let addr = signer::address_of(&account);

        let hex_digest = bytes_to_hex(aa_auth_data.digest());
        let stream = bcs_stream::new(*aa_auth_data.authenticator());
        let public_key_bytes = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));

        assert!(
            aa_auth_data.account_identity() == &public_key_bytes,
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

    // Utility function to convert a nibble (0-15) to its corresponding hex character
    fun nibble_to_char(nibble: u8): u8 {
        if (nibble < 10) {
            48 + nibble  // '0' to '9'
        } else {
            87 + nibble  // 'a' to 'f' (87 = 'a' - 10)
        }
    }

    fun bytes_to_hex(data: &vector<u8>): vector<u8> {
        let hex_chars = vector::empty();

        let i = 0;
        while (i < data.length()) {
            let cur = *data.borrow(i);
            let high_nibble = cur / 16;
            let low_nibble = cur % 16;

            hex_chars.push_back(nibble_to_char(high_nibble));
            hex_chars.push_back(nibble_to_char(low_nibble));

            i = i + 1;
        };

        hex_chars
    }
}
