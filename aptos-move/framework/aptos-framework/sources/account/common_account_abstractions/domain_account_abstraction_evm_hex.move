/// Domain account abstraction using
/// - secp256k1
/// - Ethereum specific:
///   - Append Ethereum prefix to message
///   - keccak256 hash on the appended message
///
/// Authentication takes digest, converts to hex (prefixed with 0x, with lowercase letters),
/// and then expects that to be signed.
/// authenticator is expected to be signature: vector<u8>
/// account_identity is raw public_key.
module aptos_framework::domain_account_abstraction_evm_hex {
    use std::error;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_std::aptos_hash;
    use aptos_std::option;
    use aptos_std::secp256k1;
    use aptos_std::string;
    use aptos_std::string;
    use aptos_std::string_utils;
    use aptos_std::vector;

    const EINVALID_SIGNATURE: u64 = 1;
    const EADDR_MISMATCH: u64 = 2;

    /// Authorization function for domain account abstraction.
    public fun authenticate(account: signer, aa_auth_data: AbstractionAuthData): signer {
        // Work with the digest as a hex
        let hex_digest = string_utils::to_string(aa_auth_data.digest());

        // Replicate the message prefixing that an EVM wallet will do
        let hex_digest_len = string_utils::to_string(&string::length(&hex_digest));
        let prefix = string::utf8(b"\x19Ethereum Signed Message:\n");
        let message = prefix;
        string::append(&mut message, hex_digest_len);
        string::append(&mut message, hex_digest);

        // Extract the signature r || s || v
        // EVM recovery ID is either 27 or 28. We need to map this to 0 or 1
        let signature_bytes = aa_auth_data.domain_authenticator();
        let rs = vector::slice(&signature_bytes, 0, 64);
        let v = *vector::borrow(&signature_bytes, 64) - 27;
        let signature = secp256k1::ecdsa_signature_from_bytes(rs);

        // Attempt to recover the public key
        let maybe_recovered_public_key = secp256k1::ecdsa_recover(
            aptos_hash::keccak256(message),
            v,
            &signature,
        );
        assert!(
            option::is_some(maybe_recovered_public_key),
            error::permission_denied(EINVALID_SIGNATURE)
        );

        // Ethereum address is the last 20 bytes of the keccak256(public_key)
        let recovered_public_key = option::borrow(&maybe_recovered_public_key);
        let recovered_public_key_bytes = secp256k1::ecdsa_raw_public_key_to_bytes(recovered_public_key);
        let recovered_addr = vector::slice(&aptos_hash::keccak256(recovered_public_key_bytes), 12, 32);

        // Verify that the recovered address matches the domain account identity
        let eth_addr = aa_auth_data.domain_account_identity();
        assert!(recovered_addr == eth_addr, error::permission_denied(EADDR_MISMATCH));

        account
    }
}
