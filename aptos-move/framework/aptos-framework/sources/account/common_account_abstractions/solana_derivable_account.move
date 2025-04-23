/// Derivable account abstraction that verifies a message signed by
/// SIWS.
/// 1. The message format is as follows:
///
/// <domain> wants you to sign in with your Solana account:
/// <base58_public_key>
///
/// Please confirm you explicitly initiated this request from <domain>. You are approving to execute transaction <entry_function_name> on Aptos blockchain (<network_name>).
///
/// Nonce: <aptos_txn_digest>
///
/// 2. The abstract public key is a BCS serialized `SIWSAbstractPublicKey`.
/// 3. The abstract signature is a BCS serialized `SIWSAbstractSignature`.
/// 4. This module has been tested for the following wallets:
/// - Phantom
/// - Solflare
/// - Backpack
/// - OKX
module aptos_framework::solana_derivable_account {
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_validated_public_key_from_bytes,
        public_key_into_unvalidated,
    };
    use std::bcs_stream::{Self, deserialize_u8};
    use std::chain_id;
    use std::string_utils;
    use std::transaction_context::{Self, EntryFunctionPayload};
    use std::vector;

    /// Signature failed to verify.
    const EINVALID_SIGNATURE: u64 = 1;
    /// Non base58 character found in public key.
    const EINVALID_BASE_58_PUBLIC_KEY: u64 = 2;
    /// Entry function payload is missing.
    const EMISSING_ENTRY_FUNCTION_PAYLOAD: u64 = 3;
    /// Invalid signature type.
    const EINVALID_SIGNATURE_TYPE: u64 = 4;
    /// Invalid public key.
    const EINVALID_PUBLIC_KEY: u64 = 5;
    /// Invalid public key length.
    const EINVALID_PUBLIC_KEY_LENGTH: u64 = 6;

    // a 58-character alphabet consisting of numbers (1-9) and almost all (A-Z, a-z) letters,
    // excluding 0, O, I, and l to avoid confusion between similar-looking characters.
    const BASE_58_ALPHABET: vector<u8> = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    const HEX_ALPHABET: vector<u8> = b"0123456789abcdef";
    const PUBLIC_KEY_NUM_BYTES: u64 = 32;

    enum SIWSAbstractSignature has drop {
        MessageV1 {
            signature: vector<u8>,
        },
    }

    /// Deserializes the abstract public key which is supposed to be a bcs
    /// serialized `SIWSAbstractPublicKey`.  The base58_public_key is
    /// represented in UTF8. We prefer this format because it's computationally
    /// cheaper to decode a base58 string than to encode from raw bytes.  We
    /// require both the base58 public key in UTF8 to construct the message and
    /// the raw bytes version to do signature verification.
    fun deserialize_abstract_public_key(abstract_public_key: &vector<u8>):
    (vector<u8>, vector<u8>) {
        let stream = bcs_stream::new(*abstract_public_key);
        let base58_public_key = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
        let domain = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
        (base58_public_key, domain)
    }

    /// Returns a tuple of the signature type and the signature.
    fun deserialize_abstract_signature(abstract_signature: &vector<u8>): SIWSAbstractSignature {
        let stream = bcs_stream::new(*abstract_signature);
        let signature_type = bcs_stream::deserialize_u8(&mut stream);
        if (signature_type == 0x00) {
            let signature = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
            SIWSAbstractSignature::MessageV1 { signature }
        } else {
            abort(EINVALID_SIGNATURE_TYPE)
        }
    }

    fun network_name(): vector<u8> {
        let chain_id = chain_id::get();
        if (chain_id == 1) {
            b"mainnet"
        } else if (chain_id == 2) {
            b"testnet"
        } else if (chain_id == 4) {
            b"local"
        } else {
            let network_name = &mut vector[];
            network_name.append(b"custom network: ");
            network_name.append(*string_utils::to_string(&chain_id).bytes());
            *network_name
        }
    }

    fun construct_message(
        base58_public_key: &vector<u8>,
        domain: &vector<u8>,
        entry_function_name: &vector<u8>,
        digest_utf8: &vector<u8>,
    ): vector<u8> {
        let message = &mut vector[];
        message.append(*domain);
        message.append(b" wants you to sign in with your Solana account:\n");
        message.append(*base58_public_key);
        message.append(b"\n\nPlease confirm you explicitly initiated this request from ");
        message.append(*domain);
        message.append(b".");
        message.append(b" You are approving to execute transaction ");
        message.append(*entry_function_name);
        message.append(b" on Aptos blockchain");
        let network_name = network_name();
        message.append(b" (");
        message.append(network_name);
        message.append(b")");
        message.append(b".");
        message.append(b"\n\nNonce: ");
        message.append(*digest_utf8);
        *message
    }

    spec to_public_key_bytes {
        ensures result.length() == PUBLIC_KEY_NUM_BYTES;
    }

    fun to_public_key_bytes(base58_public_key: &vector<u8>): vector<u8> {
        let bytes = vector[0u8];
        let base = 58u16;

        let i = 0;
        while (i < base58_public_key.length()) {
            let char = base58_public_key[i];
            let (found, char_index) = BASE_58_ALPHABET.index_of(&char);
            assert!(found, EINVALID_BASE_58_PUBLIC_KEY);

            let j = 0;
            let carry = (char_index as u16);

            // For each existing byte, multiply by 58 and add carry
            while (j < bytes.length()) {
                let current = (bytes[j] as u16);
                let new_carry = current * base + carry;
                bytes[j] = ((new_carry & 0xff) as u8);
                carry = new_carry >> 8;
                j = j + 1;
            };

            // Add any remaining carry as new bytes
            while (carry > 0) {
                bytes.push_back((carry & 0xff) as u8);
                carry = carry >> 8;
            };

            i = i + 1;
        };

        // Handle leading zeros (1's in Base58)
        let i = 0;
        while (i < base58_public_key.length() && base58_public_key[i] == 49) { // '1' is 49 in ASCII
            bytes.push_back(0);
            i = i + 1;
        };

        vector::reverse(&mut bytes);
        assert!(bytes.length() == PUBLIC_KEY_NUM_BYTES, EINVALID_PUBLIC_KEY_LENGTH);
        bytes
    }

    fun entry_function_name(entry_function_payload: &EntryFunctionPayload): vector<u8> {
        let entry_function_name = &mut vector[];
        let addr_str = string_utils::to_string(
            &transaction_context::account_address(entry_function_payload)
        ).bytes();
        // .slice(1) to remove the leading '@' char
        entry_function_name.append(addr_str.slice(1, addr_str.length()));
        entry_function_name.append(b"::");
        entry_function_name.append(
            *transaction_context::module_name(entry_function_payload).bytes()
        );
        entry_function_name.append(b"::");
        entry_function_name.append(
            *transaction_context::function_name(entry_function_payload).bytes()
        );
        *entry_function_name
    }

    spec authenticate_auth_data {
        // TODO: Issue with `cannot appear in both arithmetic and bitwise
        // operation`
        pragma verify = false;
    }

    fun authenticate_auth_data(
        aa_auth_data: AbstractionAuthData,
        entry_function_name: &vector<u8>
    ) {
        let abstract_public_key = aa_auth_data.derivable_abstract_public_key();
        let (base58_public_key, domain) = deserialize_abstract_public_key(abstract_public_key);
        let digest_utf8 = string_utils::to_string(aa_auth_data.digest()).bytes();

        let public_key_bytes = to_public_key_bytes(&base58_public_key);
        let public_key = new_validated_public_key_from_bytes(public_key_bytes);
        assert!(public_key.is_some(), EINVALID_PUBLIC_KEY);
        let abstract_signature = deserialize_abstract_signature(aa_auth_data.derivable_abstract_signature());
        match (abstract_signature) {
            SIWSAbstractSignature::MessageV1 { signature: signature_bytes } => {
                let message = construct_message(&base58_public_key, &domain, entry_function_name, digest_utf8);

                let signature = new_signature_from_bytes(signature_bytes);
                assert!(
                    ed25519::signature_verify_strict(
                        &signature,
                        &public_key_into_unvalidated(public_key.destroy_some()),
                        message,
                    ),
                    EINVALID_SIGNATURE
                );
            },
        };
    }

    spec authenticate {
        // TODO: Issue with spec for authenticate_auth_data
        pragma verify = false;
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
    use std::string::{String, utf8};
    #[test_only]
    use aptos_framework::auth_data::{create_derivable_auth_data};

    #[test_only]
    struct SIWSAbstractPublicKey has drop {
        base58_public_key: String,
        domain: String,
    }

    #[test_only]
    fun create_abstract_public_key(base58_public_key: String, domain: String): vector<u8> {
        let abstract_public_key = SIWSAbstractPublicKey {
            base58_public_key,
            domain,
        };
        bcs::to_bytes(&abstract_public_key)
    }

    #[test_only]
    fun create_message_v1_signature(signature: vector<u8>): vector<u8> {
        let abstract_signature = SIWSAbstractSignature::MessageV1 { signature };
        bcs::to_bytes(&abstract_signature)
    }

    #[test]
    fun test_deserialize_abstract_public_key() {
        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let domain = b"aptos-labs.github.io";
        let abstract_public_key = create_abstract_public_key(utf8(base58_public_key), utf8(domain));
        let (public_key, domain) = deserialize_abstract_public_key(&abstract_public_key);
        assert!(public_key == base58_public_key);
        assert!(domain == domain);
    }

    #[test]
    fun test_deserialize_abstract_signature() {
        let signature_bytes = vector[129, 0, 6, 135, 53, 153, 88, 201, 243, 227, 13, 232, 192, 42, 167, 94, 3, 120, 49, 80, 102, 193, 61, 211, 189, 83, 37, 121, 5, 216, 30, 25, 243, 207, 172, 248, 94, 201, 123, 66, 237, 66, 122, 201, 171, 215, 162, 187, 218, 188, 24, 165, 52, 147, 210, 39, 128, 78, 62, 81, 73, 167, 235, 1];
        let abstract_signature = create_message_v1_signature(signature_bytes);
        let siws_abstract_signature = deserialize_abstract_signature(&abstract_signature);
        assert!(siws_abstract_signature is SIWSAbstractSignature::MessageV1);
        match (siws_abstract_signature) {
            SIWSAbstractSignature::MessageV1 { signature } => assert!(signature == signature_bytes),
        };
    }

    #[test(framework = @0x1)]
    fun test_construct_message(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let domain = b"localhost:3000";
        let entry_function_name = b"0x1::coin::transfer";
        let digest_utf8 = b"0x9509edc861070b2848d8161c9453159139f867745dc87d32864a71e796c7d279";
        let message = construct_message(&base58_public_key, &domain, &entry_function_name, &digest_utf8);
        assert!(message == b"localhost:3000 wants you to sign in with your Solana account:\nG56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV\n\nPlease confirm you explicitly initiated this request from localhost:3000. You are approving to execute transaction 0x1::coin::transfer on Aptos blockchain (testnet).\n\nNonce: 0x9509edc861070b2848d8161c9453159139f867745dc87d32864a71e796c7d279");
    }

    #[test]
    fun test_to_public_key_bytes() {
        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let base64_public_key = to_public_key_bytes(&base58_public_key);

        assert!(base64_public_key == vector[223, 236, 102, 141, 171, 166, 118,
        40, 172, 65, 89, 139, 197, 164, 172, 50, 133, 204, 100, 93, 136, 195,
        58, 158, 31, 22, 219, 93, 60, 40, 175, 12]);
    }

    #[test(framework = @0x1)]
    fun test_network_name_mainnet(framework: &signer) {
        chain_id::initialize_for_test(framework, 1);
        assert!(network_name() == b"mainnet");
    }

    #[test(framework = @0x1)]
    fun test_network_name_testnet(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);
        assert!(network_name() == b"testnet");
    }

    #[test(framework = @0x1)]
    fun test_network_name_local(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);
        assert!(network_name() == b"local");
    }

    #[test(framework = @0x1)]
    fun test_network_name_other(framework: &signer) {
        chain_id::initialize_for_test(framework, 99);
        assert!(network_name() == b"custom network: 99");
    }

    #[test(framework = @0x1)]
    fun test_entry_function_name() {
        let entry_function_payload = transaction_context::new_entry_function_payload(
            @0x1,
            utf8(b"coin"),
            utf8(b"transfer"),
            vector[],
            vector[]
        );
        let entry_function_name = entry_function_name(&entry_function_payload);
        assert!(entry_function_name == b"0x1::coin::transfer");
    }

    #[test(framework = @0x1)]
    fun test_authenticate_auth_data(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);

        let digest = x"9800ae3d949260dedd01573b2903e9de06abe914530ba5d21f068f8823bfdfa3";
        let signature = vector[70, 135, 9, 250, 23, 189, 162, 119, 77, 133, 195, 66, 102, 105, 116, 86, 29, 118, 226, 100, 94, 120, 138, 219, 252, 134, 231, 139, 47, 77, 19, 201, 4, 88, 255, 64, 185, 96, 134, 50, 27, 30, 110, 125, 251, 89, 57, 156, 17, 170, 16, 102, 107, 40, 46, 234, 15, 162, 156, 69, 132, 70, 135, 11];
        let abstract_signature = create_message_v1_signature(signature);
        let base58_public_key = b"Awrh7Cfvx5gc7Ua93hdmmni6KWvkJgH4HwMkixTxmxe";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(utf8(base58_public_key), utf8(domain));
        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);
        let entry_function_name = b"0x1::aptos_account::transfer";
        authenticate_auth_data(auth_data, &entry_function_name);
    }

    #[test(framework = @0x1)]
    #[expected_failure(abort_code = EINVALID_SIGNATURE)]
    fun test_authenticate_auth_data_invalid_signature(framework: &signer) {
        chain_id::initialize_for_test(framework, 4);

        let digest = x"9800ae3d949260dedd01573b2903e9de06abe914530ba5d21f068f8823bfdfa3";
        let signature = vector[71, 135, 9, 250, 23, 189, 162, 119, 77, 133, 195, 66, 102, 105, 116, 86, 29, 118, 226, 100, 94, 120, 138, 219, 252, 134, 231, 139, 47, 77, 19, 201, 4, 88, 255, 64, 185, 96, 134, 50, 27, 30, 110, 125, 251, 89, 57, 156, 17, 170, 16, 102, 107, 40, 46, 234, 15, 162, 156, 69, 132, 70, 135, 11];
        let abstract_signature = create_message_v1_signature(signature);
        let base58_public_key = b"Awrh7Cfvx5gc7Ua93hdmmni6KWvkJgH4HwMkixTxmxe";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(utf8(base58_public_key), utf8(domain));
        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);
        let entry_function_name = b"0x1::aptos_account::transfer";
        authenticate_auth_data(auth_data, &entry_function_name);
    }
}
