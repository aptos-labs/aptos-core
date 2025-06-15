/// Derivable account abstraction that verifies a message signed by
/// Sui wallet.
/// 1. The message format is as follows:
///
/// <domain> wants you to sign in with your Sui account:
/// <sui_account_address>
///
/// Please confirm you explicitly initiated this request from <domain>. You are approving to execute transaction <entry_function_name> on Aptos blockchain (<network_name>).
///
/// Nonce: <digest>
///
/// 2. The abstract public key is a BCS serialized `SuiAbstractPublicKey`.
/// 3. The abstract signature is a BCS serialized `SuiAbstractSignature`.
/// 4. This module has been tested for the following wallets:
/// - Slush
/// - Phantom
/// - Nightly

module aptos_framework::sui_derivable_account {

    use aptos_framework::debug;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::common_account_abstractions_utils::{network_name, entry_function_name};
    use aptos_std::ed25519::{ Self, new_signature_from_bytes, new_validated_public_key_from_bytes, public_key_into_unvalidated };
    use std::bcs_stream::{ Self, deserialize_u8 };
    use std::bcs;
    use std::string_utils;
    use std::transaction_context;
    use std::vector;
    use aptos_std::aptos_hash;

    /// Entry function payload is missing.
    const EMISSING_ENTRY_FUNCTION_PAYLOAD: u64 = 1;
    /// Invalid signature type.
    const EINVALID_SIGNATURE_TYPE: u64 = 2;
    /// Invalid signing scheme type.
    const EINVALID_SIGNING_SCHEME_TYPE: u64 = 3;
    /// Invalid signature length.
    const EINVALID_SIGNATURE_LENGTH: u64 = 4;
    /// Invalid signature.
    const EINVALID_SIGNATURE: u64 = 5;
    /// Invalid public key.
    const EINVALID_PUBLIC_KEY: u64 = 6;
    /// Account address mismatch.
    const EACCOUNT_ADDRESS_MISMATCH: u64 = 7;

    fun construct_message(
        sui_public_key: &vector<u8>,
        domain: &vector<u8>,
        entry_function_name: &vector<u8>,
        digest_utf8: &vector<u8>,
    ): vector<u8> {
        let message = &mut vector[];
        message.append(*domain);
        message.append(b" wants you to sign in with your Sui account:\n");
        message.append(*sui_public_key);
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

    enum SuiAbstractSignature has drop {
        MessageV1 {
            /// The signature of the message in raw bytes
            signature: vector<u8>,
        },
    }

    /// Sui abstract public key defined with the
    struct SuiAbstractPublicKey has drop {
        // The Sui account address, in hex string format with "0x" prefix
        sui_account_address: vector<u8>,
        // The domain, in utf8 bytes
        domain: vector<u8>,
    }

    /// Sui signing scheme as defined in
    /// https://github.com/MystenLabs/ts-sdks/blob/main/packages/typescript/src/cryptography/signature-scheme.ts#L19
    enum SuiSigningScheme has drop {
        ED25519,
    }

    /// A wrapper struct that defines a message with its signing context (intent).
    /// https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L168
    struct IntentMessage has copy, drop, store {
        // The Intent metadata
        intent: Intent,
        // The raw message signed on
        value: vector<u8>,
    }

    /// Metadata specifying the scope, version, and application domain of the message.
    /// https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L86
    struct Intent has copy, drop, store {
        scope: IntentScope,
        version: IntentVersion,
        app_id: AppId,
    }

    /// https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L60
    enum IntentScope has drop, copy, store {
        TransactionData,
        TransactionEffects,
        CheckpointSummary,
        PersonalMessage,
    }

    /// https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L18
    enum IntentVersion has drop, copy, store {
        V0,
    }

    /// https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L35
    enum AppId has drop, copy, store {
        Sui,
    }

    /// Returns the signing scheme for the given value.
    fun get_signing_scheme(value: u8): SuiSigningScheme {
        if (value == 0) SuiSigningScheme::ED25519
        else abort(EINVALID_SIGNING_SCHEME_TYPE)
    }

    /// Deserializes the abstract public key which is supposed to be a bcs
    /// serialized `SuiAbstractPublicKey`.
    fun deserialize_abstract_public_key(abstract_public_key: &vector<u8>): SuiAbstractPublicKey {
        let stream = bcs_stream::new(*abstract_public_key);
        let sui_account_address = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
        let domain = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
        SuiAbstractPublicKey { sui_account_address, domain }
    }

    /// Returns a tuple of the signature.
    fun deserialize_abstract_signature(abstract_signature: &vector<u8>): SuiAbstractSignature {
        let stream = bcs_stream::new(*abstract_signature);
        let signature_type = bcs_stream::deserialize_u8(&mut stream);
        if (signature_type == 0x00) {
            let signature = bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x));
            SuiAbstractSignature::MessageV1 { signature }
        } else {
            abort(EINVALID_SIGNATURE_TYPE)
        }
    }

    /// Splits raw signature bytes containing `scheme flag (1 byte), signature (64 bytes) and public key (32 bytes)`
    /// to a tuple of (signing_scheme, signature, public_key)
    public fun split_signature_bytes(bytes: &vector<u8>): (u8, vector<u8>, vector<u8>) {
        // 1 + 64 + 32 = 97 bytes
        assert!(vector::length(bytes) == 97, EINVALID_SIGNATURE_LENGTH);

        let signing_scheme = *vector::borrow(bytes, 0);
        let abstract_signature_signature = vector::empty<u8>();
        let abstract_signature_public_key = vector::empty<u8>();

        // Extract signature (64 bytes)
        let i = 1;
        while (i < 65) {
            vector::push_back(&mut abstract_signature_signature, *vector::borrow(bytes, i));
            i = i + 1;
        };

        // Extract public key (32 bytes)
        while (i < 97) {
            vector::push_back(&mut abstract_signature_public_key, *vector::borrow(bytes, i));
            i = i + 1;
        };

        (signing_scheme, abstract_signature_signature, abstract_signature_public_key)
    }

    /// Derives the account address from the public key and returns it is a hex string with "0x" prefix
    fun derive_account_address_from_public_key(signing_scheme: u8, public_key_bytes: vector<u8>): vector<u8> {
        // Create a vector with signing scheme and public key bytes
        let data_to_hash = vector::singleton(signing_scheme);
        vector::append(&mut data_to_hash, public_key_bytes);

        // Compute blake2b hash
        let sui_account_address = aptos_hash::blake2b_256(data_to_hash);

        // Convert the address bytes to a hex string with "0x" prefix
        let sui_account_address_hex = b"0x";
        let i = 0;
        while (i < vector::length(&sui_account_address)) {
            let byte = *vector::borrow(&sui_account_address, i);
            // Convert each byte to two hex characters
            let hex_chars = vector[
                if ((byte >> 4) < 10) ((byte >> 4) + 0x30) else ((byte >> 4) - 10 + 0x61),
                if ((byte & 0xf) < 10) ((byte & 0xf) + 0x30) else ((byte & 0xf) - 10 + 0x61)
            ];
            vector::append(&mut sui_account_address_hex, hex_chars);
            i = i + 1;
        };

        // Return the account address as hex string
        sui_account_address_hex
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
        let abstract_signature = deserialize_abstract_signature(aa_auth_data.derivable_abstract_signature());
        let (signing_scheme, abstract_signature_signature, abstract_signature_public_key) = split_signature_bytes(&abstract_signature.signature);

        // Check siging scheme is ED25519 as we currently only support this scheme
        assert!(get_signing_scheme(signing_scheme) == SuiSigningScheme::ED25519, EINVALID_SIGNING_SCHEME_TYPE);

        // Derive the account address from the public key
        let sui_account_address = derive_account_address_from_public_key(signing_scheme, abstract_signature_public_key);

        let derivable_abstract_public_key = aa_auth_data.derivable_abstract_public_key();
        let abstract_public_key = deserialize_abstract_public_key(derivable_abstract_public_key);

        // Check the account address matches the abstract public key
        assert!(&sui_account_address == &abstract_public_key.sui_account_address, EACCOUNT_ADDRESS_MISMATCH);

        let public_key = new_validated_public_key_from_bytes(abstract_signature_public_key);
        assert!(public_key.is_some(), EINVALID_PUBLIC_KEY);

        let digest_utf8 = string_utils::to_string(aa_auth_data.digest()).bytes();

        // Build the raw message
        let raw_message = construct_message(&sui_account_address, &abstract_public_key.domain, entry_function_name, digest_utf8);

        // Prepend Intent to the message
        let intent = Intent {
            scope: PersonalMessage,
            version: V0,
            app_id: Sui,
        };
        let msg = IntentMessage {
            intent,
            value: raw_message,
        };
        // Serialize the whole struct
        let bcs_bytes = bcs::to_bytes<IntentMessage>(&msg);

        // Hash full_message with blake2b256
        let hash = aptos_hash::blake2b_256(bcs_bytes);

        let signature = new_signature_from_bytes(abstract_signature_signature);

        assert!(
            ed25519::signature_verify_strict(
                &signature,
                &public_key_into_unvalidated(public_key.destroy_some()),
                hash,
            ),
            EINVALID_SIGNATURE
        );
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
    use std::chain_id;
    #[test_only]
    use aptos_framework::auth_data::{create_derivable_auth_data};

    #[test_only]
    fun create_abstract_public_key(sui_account_address: vector<u8>, domain: vector<u8>): vector<u8> {
        let abstract_public_key = SuiAbstractPublicKey {
            sui_account_address,
            domain,
        };
        bcs::to_bytes(&abstract_public_key)
    }

    #[test_only]
    fun create_raw_signature(signature: vector<u8>): vector<u8> {
        let abstract_signature = SuiAbstractSignature::MessageV1 { signature };
        bcs::to_bytes(&abstract_signature)
    }

    #[test]
    fun test_derive_account_address_from_public_key() {
        let sui_public_key = vector[25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202];
        let account_address = derive_account_address_from_public_key(0x00, sui_public_key);
        assert!(account_address == b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49");
    }

    #[test]
    fun test_get_signing_scheme() {
        let signing_scheme = get_signing_scheme(0x00);
        assert!(signing_scheme == SuiSigningScheme::ED25519);
    }

    #[test]
    fun test_deserialize_abstract_signature() {
        let signature_bytes = vector[0, 151, 47, 171, 144, 115, 16, 129, 17, 202, 212, 180, 155, 213, 223, 249, 203, 195, 0, 84, 142, 121, 167, 29, 113, 159, 33, 177, 108, 137, 113, 160, 118, 41, 246, 199, 202, 79, 151, 27, 86, 235, 219, 123, 168, 152, 38, 124, 147, 146, 118, 101, 37, 187, 223, 206, 120, 101, 148, 33, 141, 80, 60, 155, 13, 25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202];
        let abstract_signature = create_raw_signature(signature_bytes);
        let sui_abstract_signature = deserialize_abstract_signature(&abstract_signature);
        assert!(sui_abstract_signature is SuiAbstractSignature::MessageV1);
        match (sui_abstract_signature) {
            SuiAbstractSignature::MessageV1 { signature } => assert!(signature == signature_bytes),
        };
    }

    #[test]
    fun test_split_signature_bytes() {
        let signature = vector[0, 151, 47, 171, 144, 115, 16, 129, 17, 202, 212, 180, 155, 213, 223, 249, 203, 195, 0, 84, 142, 121, 167, 29, 113, 159, 33, 177, 108, 137, 113, 160, 118, 41, 246, 199, 202, 79, 151, 27, 86, 235, 219, 123, 168, 152, 38, 124, 147, 146, 118, 101, 37, 187, 223, 206, 120, 101, 148, 33, 141, 80, 60, 155, 13, 25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202];
        let (signing_scheme, signature, public_key) = split_signature_bytes(&signature);
        assert!(signing_scheme == 0x00);
        assert!(signature == vector[151, 47, 171, 144, 115, 16, 129, 17, 202, 212, 180, 155, 213, 223, 249, 203, 195, 0, 84, 142, 121, 167, 29, 113, 159, 33, 177, 108, 137, 113, 160, 118, 41, 246, 199, 202, 79, 151, 27, 86, 235, 219, 123, 168, 152, 38, 124, 147, 146, 118, 101, 37, 187, 223, 206, 120, 101, 148, 33, 141, 80, 60, 155, 13]);
        assert!(public_key == vector[25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202]);
    }

    #[test(framework = @0x1)]
    fun test_construct_message(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);
        let sui_account_address = b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49";
        let domain = b"localhost:3001";
        let entry_function_name = b"0x1::coin::transfer";
        let digest_utf8 = b"0x041689ce61015dd0aa166aa2edc1cc74e63b3ed093f40e3ce4101fce067b24ad";
        let message = construct_message(&sui_account_address, &domain, &entry_function_name, &digest_utf8);
        assert!(message == b"localhost:3001 wants you to sign in with your Sui account:\n0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49\n\nPlease confirm you explicitly initiated this request from localhost:3001. You are approving to execute transaction 0x1::coin::transfer on Aptos blockchain (testnet).\n\nNonce: 0x041689ce61015dd0aa166aa2edc1cc74e63b3ed093f40e3ce4101fce067b24ad");
    }

    #[test(framework = @0x1)]
    fun test_authenticate_auth_data(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let sui_account_address = b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(sui_account_address, domain);

        let signature_bytes = vector[0, 140, 147, 99, 142, 194, 60, 242, 45, 231, 203, 175, 182, 126, 202, 88, 32, 157, 255, 80, 200, 28, 135, 142, 3, 1, 58, 192, 53, 166, 235, 171, 168, 32, 163, 200, 137, 125, 161, 149, 149, 159, 254, 116, 51, 159, 23, 11, 196, 173, 127, 7, 214, 231, 235, 171, 224, 221, 229, 219, 27, 31, 80, 173, 12, 25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202];
        let abstract_signature = create_raw_signature(signature_bytes);

        let entry_function_name = b"0x1::aptos_account::transfer";
        let digest = x"717843972d0491ba7b80fbb4e60708d87f7926c439972138062934c9dc1fc17b";

        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);

        authenticate_auth_data(auth_data, &entry_function_name);
    }

    #[test(framework = @0x1)]
    #[expected_failure(abort_code = EINVALID_SIGNATURE)]
    fun test_authenticate_auth_data_invalid_signature(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let sui_account_address = b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(sui_account_address, domain);

        let signature_bytes = vector[0, 141, 147, 99, 142, 194, 60, 242, 45, 231, 203, 175, 182, 126, 202, 88, 32, 157, 255, 80, 200, 28, 135, 142, 3, 1, 58, 192, 53, 166, 235, 171, 168, 32, 163, 200, 137, 125, 161, 149, 149, 159, 254, 116, 51, 159, 23, 11, 196, 173, 127, 7, 214, 231, 235, 171, 224, 221, 229, 219, 27, 31, 80, 173, 12, 25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202];
        let abstract_signature = create_raw_signature(signature_bytes);

        let entry_function_name = b"0x1::aptos_account::transfer";
        let digest = x"717843972d0491ba7b80fbb4e60708d87f7926c439972138062934c9dc1fc17b";

        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);

        authenticate_auth_data(auth_data, &entry_function_name);
    }

    #[test(framework = @0x1)]
    #[expected_failure(abort_code = EINVALID_SIGNING_SCHEME_TYPE)]
    fun test_authenticate_auth_data_invalid_signing_scheme_type(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let sui_account_address = b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(sui_account_address, domain);

        let signature_bytes = vector[1, 140, 147, 99, 142, 194, 60, 242, 45, 231, 203, 175, 182, 126, 202, 88, 32, 157, 255, 80, 200, 28, 135, 142, 3, 1, 58, 192, 53, 166, 235, 171, 168, 32, 163, 200, 137, 125, 161, 149, 149, 159, 254, 116, 51, 159, 23, 11, 196, 173, 127, 7, 214, 231, 235, 171, 224, 221, 229, 219, 27, 31, 80, 173, 12, 25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 202];
        let abstract_signature = create_raw_signature(signature_bytes);

        let entry_function_name = b"0x1::aptos_account::transfer";
        let digest = x"717843972d0491ba7b80fbb4e60708d87f7926c439972138062934c9dc1fc17b";

        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);

        authenticate_auth_data(auth_data, &entry_function_name);
    }


    #[test(framework = @0x1)]
    #[expected_failure(abort_code = EACCOUNT_ADDRESS_MISMATCH)]
    fun test_authenticate_auth_data_invalid_account_address_mismatch(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let sui_account_address = b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49";
        let domain = b"localhost:3001";
        let abstract_public_key = create_abstract_public_key(sui_account_address, domain);

        let signature_bytes = vector[0, 140, 147, 99, 142, 194, 60, 242, 45, 231, 203, 175, 182, 126, 202, 88, 32, 157, 255, 80, 200, 28, 135, 142, 3, 1, 58, 192, 53, 166, 235, 171, 168, 32, 163, 200, 137, 125, 161, 149, 149, 159, 254, 116, 51, 159, 23, 11, 196, 173, 127, 7, 214, 231, 235, 171, 224, 221, 229, 219, 27, 31, 80, 173, 12, 25, 200, 235, 92, 139, 72, 175, 189, 40, 0, 65, 76, 215, 148, 94, 194, 78, 134, 60, 189, 212, 116, 40, 134, 179, 104, 31, 249, 222, 84, 104, 201];
        let abstract_signature = create_raw_signature(signature_bytes);

        let entry_function_name = b"0x1::aptos_account::transfer";
        let digest = x"717843972d0491ba7b80fbb4e60708d87f7926c439972138062934c9dc1fc17b";

        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);

        authenticate_auth_data(auth_data, &entry_function_name);
    }

}
