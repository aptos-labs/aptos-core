/// Derivable account abstraction that verifies a message signed by
/// SIWS from a Phantom wallet.
module aptos_experimental::daa_siws_phantom {
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes,
    };
    use std::bcs_stream::{Self};
    use std::chain_id;
    use std::error;
    use std::string_utils;
    use std::transaction_context::{Self, EntryFunctionPayload};
    use std::vector;

    /// Signature failed to verify.
    const EINVALID_SIGNATURE: u64 = 1;
    /// Non base58 character found in public key.
    const EINVALID_BASE_58_PUBLIC_KEY: u64 = 2;
    /// Entry function payload is missing.
    const EMISSING_ENTRY_FUNCTION_PAYLOAD: u64 = 3;

    // a 58-character alphabet consisting of numbers (1-9) and almost all (A-Z, a-z) letters,
    // excluding 0, O, I, and l to avoid confusion between similar-looking characters.
    const BASE_58_ALPHABET: vector<u8> = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    const HEX_ALPHABET: vector<u8> = b"0123456789abcdef";

    fun deserialize_abstract_public_key(abstract_public_key: &vector<u8>): (vector<u8>, vector<u8>) {
        let stream = bcs_stream::new(*abstract_public_key);
        let base58_public_key = *bcs_stream::deserialize_string(&mut stream).bytes();
        let domain = *bcs_stream::deserialize_string(&mut stream).bytes();
        (base58_public_key, domain)
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
            *string_utils::to_string(&chain_id).bytes()
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
        message.append(b"\n\nTo execute transaction ");
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

    fun to_public_key_bytes(base58_public_key: &vector<u8>): vector<u8> {
        let bytes = vector[0u8];
        let base = 58u16;  // Using u16 to handle multiplication without overflow

        let i = 0;
        while (i < vector::length(base58_public_key)) {
            let char = *vector::borrow(base58_public_key, i);
            let (found, char_index) = vector::index_of(&BASE_58_ALPHABET, &char);
            assert!(found, error::invalid_argument(EINVALID_BASE_58_PUBLIC_KEY));

            let mut_bytes = &mut bytes;
            let j = 0;
            let carry = (char_index as u16);

            // For each existing byte, multiply by 58 and add carry
            while (j < vector::length(mut_bytes)) {
                let current = (*vector::borrow(mut_bytes, j) as u16);
                let new_carry = current * base + carry;
                *vector::borrow_mut(mut_bytes, j) = ((new_carry & 0xff) as u8);
                carry = new_carry >> 8;
                j = j + 1;
            };

            // Add any remaining carry as new bytes
            while (carry > 0) {
                vector::push_back(mut_bytes, ((carry & 0xff) as u8));
                carry = carry >> 8;
            };

            i = i + 1;
        };

        // Handle leading zeros (1's in Base58)
        let i = 0;
        while (i < vector::length(base58_public_key) && *vector::borrow(base58_public_key, i) == 49) { // '1' is 49 in ASCII
            vector::push_back(&mut bytes, 0);
            i = i + 1;
        };

        vector::reverse(&mut bytes);
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

    fun authenticate_auth_data(
        aa_auth_data: AbstractionAuthData,
        entry_function_name: &vector<u8>
    ) {
        let abstract_public_key = aa_auth_data.derivable_abstract_public_key();
        let (base58_public_key, domain) = deserialize_abstract_public_key(abstract_public_key);
        let digest_utf8 = string_utils::to_string(aa_auth_data.digest()).bytes();
        let message = construct_message(&base58_public_key, &domain, entry_function_name, digest_utf8);

        let public_key_bytes = to_public_key_bytes(&base58_public_key);
        let public_key = new_unvalidated_public_key_from_bytes(public_key_bytes);
        let signature = new_signature_from_bytes(*aa_auth_data.derivable_abstract_signature());
        assert!(
            ed25519::signature_verify_strict(
                &signature,
                &public_key,
                message,
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );
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

    #[test]
    fun test_deserialize_abstract_public_key() {
        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let domain = b"aptos-labs.github.io";
        let abstract_public_key = create_abstract_public_key(utf8(base58_public_key), utf8(domain));
        let (public_key, domain) = deserialize_abstract_public_key(&abstract_public_key);
        assert!(public_key == base58_public_key);
        assert!(domain == domain);
    }

    #[test(framework = @0x1)]
    fun test_construct_message(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let domain = b"localhost:3000";
        let entry_function_name = b"0x1::coin::transfer";
        let digest_utf8 = b"0x9509edc861070b2848d8161c9453159139f867745dc87d32864a71e796c7d279";
        let message = construct_message(&base58_public_key, &domain, &entry_function_name, &digest_utf8);
        assert!(message == b"localhost:3000 wants you to sign in with your Solana account:\nG56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV\n\nTo execute transaction 0x1::coin::transfer on Aptos blockchain (testnet).\n\nNonce: 0x9509edc861070b2848d8161c9453159139f867745dc87d32864a71e796c7d279");
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
        assert!(network_name() == b"99");
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
        chain_id::initialize_for_test(framework, 2);

        let digest = x"026a4f93c2010cbafbac45639e995410d0902d11a3c4f0fcd1c64a1d193f4866";
        let abstract_signature = vector[129, 0, 6, 135, 53, 153, 88, 201, 243,
        227, 13, 232, 192, 42, 167, 94, 3, 120, 49, 80, 102, 193, 61, 211, 189,
        83, 37, 121, 5, 216, 30, 25, 243, 207, 172, 248, 94, 201, 123, 66, 237,
        66, 122, 201, 171, 215, 162, 187, 218, 188, 24, 165, 52, 147, 210, 39,
        128, 78, 62, 81, 73, 167, 235, 1];
        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let domain = b"localhost:3000";
        let abstract_public_key = create_abstract_public_key(utf8(base58_public_key), utf8(domain));
        let auth_data = create_derivable_auth_data(digest, abstract_signature, abstract_public_key);
        let entry_function_name = b"0x1::coin::transfer";
        authenticate_auth_data(auth_data, &entry_function_name);
    }
}
