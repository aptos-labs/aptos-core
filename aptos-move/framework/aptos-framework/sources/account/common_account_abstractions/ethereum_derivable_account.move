/// Derivable account abstraction that verifies a message signed by
/// SIWE.
/// 1. The message format is as follows:
///
/// <domain> wants you to sign in with your Ethereum account:
/// <ethereum_address>
///
/// To execute transaction <entry_function_name> on Aptos blockchain
/// (<network_name>).
///
/// URI: <domain>
/// Version: 1
/// Chain ID: <chain_id>
/// Nonce: <digest>
/// Issued At: <issued_at>
///
/// Before signing, check that you are signing this message from <domain>
///
/// Account: <address>
/// Function: <function>
/// Aptos Network: <network_name> (<chain_id>)
/// Hash: <aptos_txn_digest>
///
/// 2. The abstract public key is a BCS serialized `SIWEAbstractPublicKey`.
/// 3. The abstract signature is a BCS serialized `SIWEAbstractSignature`.
/// 4. This module has been tested for the following wallets:
/// - Metamask
module aptos_framework::ethereum_derivable_account {
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::common_account_abstractions_utils::{network_name, entry_function_name};
    use aptos_std::secp256k1;
    use aptos_std::option;
    use aptos_std::aptos_hash;
    use aptos_std::base16::base16_utf8_to_vec_u8;
    use std::bcs_stream::{Self, deserialize_u8};
    use std::chain_id;
    use std::string_utils;
    use std::transaction_context;
    use std::vector;
    use std::string::String;

    /// Signature failed to verify.
    const EINVALID_SIGNATURE: u64 = 1;
    /// Entry function payload is missing.
    const EMISSING_ENTRY_FUNCTION_PAYLOAD: u64 = 2;
    /// Invalid signature type.
    const EINVALID_SIGNATURE_TYPE: u64 = 3;
    /// Address mismatch.
    const EADDR_MISMATCH: u64 = 4;
    /// Unexpected v value.
    const EUNEXPECTED_V: u64 = 5;

    enum SIWEAbstractSignature has drop {
        EIP1193DerivedSignature {
            signature: vector<u8>,
        },
    }

    struct SIWEAbstractPublicKey has drop {
        // The Ethereum address, with 0x prefix, in utf8 bytes
        ethereum_address: vector<u8>,
        // The domain, in utf8 bytes
        domain: vector<u8>,
    }

    /// Deserializes the abstract public key which is supposed to be a bcs
    /// serialized `SIWEAbstractPublicKey`.
    fun deserialize_abstract_public_key(abstract_public_key: &vector<u8>): SIWEAbstractPublicKey {
        let stream = bcs_stream::new(*abstract_public_key);
        let ethereum_address = *bcs_stream::deserialize_string(&mut stream).bytes();
        let domain = *bcs_stream::deserialize_string(&mut stream).bytes();
        SIWEAbstractPublicKey { ethereum_address, domain }
    }

    /// Returns a tuple of the signature type and the signature.
    /// We include the issued_at in the signature as it is a required field in the SIWE standard.
    fun deserialize_abstract_signature(abstract_signature: &vector<u8>): SIWEAbstractSignature {
        let stream = bcs_stream::new(*abstract_signature);
        let signature_type = bcs_stream::deserialize_u8(&mut stream);
        if (signature_type == 0x00) {
            let signature = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
            SIWEAbstractSignature::EIP1193DerivedSignature { signature }
        } else {
            abort(EINVALID_SIGNATURE_TYPE)
        }
    }

    // construct a message that is used to verify the signature following the SIWE standard
    // and ethers.js. ethers adds a prefix to the message, so we need to include it also
    fun construct_message(
        ethereum_address: &vector<u8>,
        domain: &vector<u8>,
        entry_function_name: &vector<u8>,
        digest_utf8: &vector<u8>,
    ): vector<u8> {
        let message = &mut vector[];
        message.append(b"Before signing, check that you are signing this message from ");
        message.append(*domain);
        message.append(b"\n\nAccount: ");
        message.append(*ethereum_address);
        message.append(b"\nFunction:");
        message.append(*entry_function_name);
        message.append(b"\nAptos Network: ");
        let network_name = network_name();
        message.append(network_name);
        message.append(b" (");
        message.append(*string_utils::to_string(&chain_id::get()).bytes());
        message.append(b")");
        message.append(b"\nHash: ");
        message.append(*digest_utf8);

        let msg_len = vector::length(message);

        let prefix = b"\x19Ethereum Signed Message:\n";
        let msg_len_string = string_utils::to_string(&msg_len); // returns string
        let msg_len_bytes = msg_len_string.bytes(); // vector<u8>

        let full_message = &mut vector[];
        full_message.append(prefix);
        full_message.append(*msg_len_bytes);
        full_message.append(*message);

        *full_message
    }

    fun recover_public_key(signature_bytes: &vector<u8>, message: &vector<u8>): vector<u8> {
        let rs = vector::slice(signature_bytes, 0, 64);
        let v = *vector::borrow(signature_bytes, 64);
        assert!(v == 27 || v == 28, EUNEXPECTED_V);
        let signature = secp256k1::ecdsa_signature_from_bytes(rs);

        let maybe_recovered = secp256k1::ecdsa_recover(*message, v - 27, &signature);

        assert!(
            option::is_some(&maybe_recovered),
            EINVALID_SIGNATURE
        );

        let pubkey = option::borrow(&maybe_recovered);

        let pubkey_bytes = secp256k1::ecdsa_raw_public_key_to_bytes(pubkey);

        // Add 0x04 prefix to the public key, to match the
        // full uncompressed format from ethers.js
        let full_pubkey = &mut vector[];
        vector::push_back(full_pubkey, 4u8);
        vector::append(full_pubkey, pubkey_bytes);

        *full_pubkey
    }


    fun authenticate_auth_data(
        aa_auth_data: AbstractionAuthData,
        entry_function_name: &vector<u8>
    ) {
        let derivable_abstract_public_key = aa_auth_data.derivable_abstract_public_key();
        let abstract_public_key = deserialize_abstract_public_key(derivable_abstract_public_key);
        let digest_utf8 = string_utils::to_string(aa_auth_data.digest()).bytes();
        let abstract_signature = deserialize_abstract_signature(aa_auth_data.derivable_abstract_signature());
        let message = construct_message(&abstract_public_key.ethereum_address, &abstract_public_key.domain, entry_function_name, digest_utf8);
        let hashed_message = aptos_hash::keccak256(message);
        let public_key_bytes = recover_public_key(&abstract_signature.signature, &hashed_message);

        // 1. Skip the 0x04 prefix (take the bytes after the first byte)
        let public_key_without_prefix = vector::slice(&public_key_bytes, 1, vector::length(&public_key_bytes));
        // 2. Run Keccak256 on the public key (without the 0x04 prefix)
        let kexHash = aptos_hash::keccak256(public_key_without_prefix);
        // 3. Slice the last 20 bytes (this is the Ethereum address)
        let recovered_addr = vector::slice(&kexHash, 12, 32);
        // 4. Remove the 0x prefix from the utf8 account address
        let ethereum_address_without_prefix = vector::slice(&abstract_public_key.ethereum_address, 2, vector::length(&abstract_public_key.ethereum_address));

        let account_address_vec = base16_utf8_to_vec_u8(ethereum_address_without_prefix);
        // Verify that the recovered address matches the domain account identity
        assert!(recovered_addr == account_address_vec, EADDR_MISMATCH);
    }

    /// Authorization function for domain account abstraction.
    public fun authenticate(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let maybe_entry_function_payload = transaction_context::entry_function_payload();
        if (maybe_entry_function_payload.is_some()) {
            let entry_function_payload = maybe_entry_function_payload.destroy_some();
            let entry_function_name = entry_function_name(&entry_function_payload);
            authenticate_auth_data(aa_auth_data, &entry_function_name);
            account
        } else {
            abort(EMISSING_ENTRY_FUNCTION_PAYLOAD)
        }
    }

    #[test_only]
    use std::bcs;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use aptos_framework::auth_data::{create_derivable_auth_data};
    #[test_only]
    fun create_abstract_public_key(ethereum_address: vector<u8>, domain: vector<u8>): vector<u8> {
        let abstract_public_key = SIWEAbstractPublicKey {
            ethereum_address,
            domain,
        };
        bcs::to_bytes(&abstract_public_key)
    }

    #[test_only]
    fun create_raw_signature(signature: vector<u8>): vector<u8> {
        let abstract_signature = SIWEAbstractSignature::EIP1193DerivedSignature { signature };
        bcs::to_bytes(&abstract_signature)
    }

    #[test]
    fun test_deserialize_abstract_public_key() {
        let ethereum_address = b"0xC7B576Ead6aFb962E2DEcB35814FB29723AEC98a";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(ethereum_address, domain);
        let abstract_public_key = deserialize_abstract_public_key(&abstract_public_key);
        assert!(abstract_public_key.ethereum_address == ethereum_address);
        assert!(abstract_public_key.domain == domain);
    }

    #[test]
    fun test_deserialize_abstract_signature() {
        let signature_bytes = vector[
            249, 247, 194, 250, 31, 233, 100, 234, 109, 142, 6, 193, 203, 33, 147, 199,
            236, 117, 69, 119, 252, 219, 150, 143, 28, 112, 33, 9, 95, 53, 0, 69,
            123, 17, 207, 53, 69, 203, 213, 208, 13, 98, 225, 170, 28, 183, 181, 53,
            58, 209, 105, 56, 204, 253, 73, 82, 201, 197, 201, 139, 201, 19, 65, 215,
            28
        ];
        let abstract_signature = create_raw_signature(signature_bytes);
        let siwe_abstract_signature = deserialize_abstract_signature(&abstract_signature);
        assert!(siwe_abstract_signature is SIWEAbstractSignature::EIP1193DerivedSignature);
        match (siwe_abstract_signature) {
            SIWEAbstractSignature::EIP1193DerivedSignature { signature, issued_at } => {
                assert!(signature == signature_bytes);
            },
        };
    }

    #[test(framework = @0x1)]
    fun test_construct_message(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);

        let ethereum_address = b"0xC7B576Ead6aFb962E2DEcB35814FB29723AEC98a";
        let domain = b"localhost:3001";
        let entry_function_name = b"0x1::aptos_account::transfer";
        let digest_utf8 = b"0x2a2f07c32382a94aa90ddfdb97076b77d779656bb9730c4f3e4d22a30df298dd";
        let message = construct_message(&ethereum_address, &domain, &entry_function_name, &digest_utf8);
        let expected_message = b"\x19Ethereum Signed Message:\n342Before signing, check that you are signing this message from localhost:3001\nAccount: 0xC7B576Ead6aFb962E2DEcB35814FB29723AEC98a\nFunction: 0x1::aptos_account::transfer\nAptos Network: local (4)\nHash: 0x2a2f07c32382a94aa90ddfdb97076b77d779656bb9730c4f3e4d22a30df298dd";
        assert!(message == expected_message);
    }

    #[test(framework = @0x1)]
    fun test_recover_public_key(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);
        let ethereum_address = b"0xC7B576Ead6aFb962E2DEcB35814FB29723AEC98a";
        let domain = b"localhost:3001";
        let entry_function_name = b"0x1::aptos_account::transfer";
        let digest = b"0x2a2f07c32382a94aa90ddfdb97076b77d779656bb9730c4f3e4d22a30df298dd";
        let message = construct_message(&ethereum_address, &domain, &entry_function_name, &digest);
        let hashed_message = aptos_hash::keccak256(message);
        let signature_bytes = vector[
            249, 247, 194, 250, 31, 233, 100, 234, 109, 142, 6, 193, 203, 33, 147, 199,
            236, 117, 69, 119, 252, 219, 150, 143, 28, 112, 33, 9, 95, 53, 0, 69,
            123, 17, 207, 53, 69, 203, 213, 208, 13, 98, 225, 170, 28, 183, 181, 53,
            58, 209, 105, 56, 204, 253, 73, 82, 201, 197, 201, 139, 201, 19, 65, 215,
            28
        ];
        let base64_public_key = recover_public_key(&signature_bytes, &hashed_message);
        assert!(base64_public_key == vector[
            4, 186, 242, 201, 107, 125, 171, 241, 239, 174, 216, 103, 198, 245, 151, 84,
            208, 238, 134, 130, 51, 223, 164, 243, 149, 234, 188, 140, 237, 189, 190, 221,
            95, 60, 172, 1, 22, 96, 232, 105, 172, 184, 198, 168, 157, 54, 230, 217,
            100, 150, 220, 31, 135, 165, 51, 83, 53, 159, 139, 98, 103, 106, 250, 194, 94
        ]
        );
    }

    #[test(framework = @0x1)]
    fun test_authenticate_auth_data(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);

        let digest = x"2a2f07c32382a94aa90ddfdb97076b77d779656bb9730c4f3e4d22a30df298dd";
        let signature = vector[
            249, 247, 194, 250, 31, 233, 100, 234, 109, 142, 6, 193, 203, 33, 147, 199,
            236, 117, 69, 119, 252, 219, 150, 143, 28, 112, 33, 9, 95, 53, 0, 69,
            123, 17, 207, 53, 69, 203, 213, 208, 13, 98, 225, 170, 28, 183, 181, 53,
            58, 209, 105, 56, 204, 253, 73, 82, 201, 197, 201, 139, 201, 19, 65, 215,
            28
        ];
        let abstract_signature = create_raw_signature(signature);
        let ethereum_address = b"0xC7B576Ead6aFb962E2DEcB35814FB29723AEC98a";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(ethereum_address, domain);
        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);
        let entry_function_name = b"0x1::aptos_account::transfer";
        authenticate_auth_data(auth_data, &entry_function_name);
    }

    #[test(framework = @0x1)]
    #[expected_failure(abort_code = EINVALID_SIGNATURE)]
    fun test_authenticate_auth_data_invalid_signature(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);

        let digest = x"2a2f07c32382a94aa90ddfdb97076b77d779656bb9730c4f3e4d22a30df298dd";
        let signature = vector[
            248, 247, 194, 250, 31, 233, 100, 234, 109, 142, 6, 193, 203, 33, 147, 199,
            236, 117, 69, 119, 252, 219, 150, 143, 28, 112, 33, 9, 95, 53, 0, 69,
            123, 17, 207, 53, 69, 203, 213, 208, 13, 98, 225, 170, 28, 183, 181, 53,
            58, 209, 105, 56, 204, 253, 73, 82, 201, 197, 201, 139, 201, 19, 65, 215,
            28
        ];
        let abstract_signature = create_raw_signature(signature);
        let ethereum_address = b"0xC7B576Ead6aFb962E2DEcB35814FB29723AEC98a";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(ethereum_address, domain);
        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);
        let entry_function_name = b"0x1::aptos_account::transfer";
        authenticate_auth_data(auth_data, &entry_function_name);
    }
}
