module aptos_framework::common_account_abstractions_utils {
    use aptos_framework::auth_data::AbstractionAuthData;
    use std::chain_id;
    use std::string_utils;
    use std::transaction_context::{Self, EntryFunctionPayload};

    friend aptos_framework::ethereum_derivable_account;
    friend aptos_framework::solana_derivable_account;
    friend aptos_framework::sui_derivable_account;

    /// Entry function payload is missing.
    const EMISSING_ENTRY_FUNCTION_PAYLOAD: u64 = 1;

    public(friend) fun network_name(): vector<u8> {
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

    public(friend) fun entry_function_name(entry_function_payload: &EntryFunctionPayload): vector<u8> {
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

    public(friend) fun construct_message(
        chain_name: &vector<u8>,
        account_address: &vector<u8>,
        domain: &vector<u8>,
        entry_function_name: &vector<u8>,
        digest_utf8: &vector<u8>,
    ): vector<u8> {
        let message = &mut vector[];
        message.append(*domain);
        message.append(b" wants you to sign in with your ");
        message.append(*chain_name);
        message.append(b" account:\n");
        message.append(*account_address);
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

    public(friend) inline fun daa_authenticate(
        account: signer,
        aa_auth_data: AbstractionAuthData,
        auth_fn: |AbstractionAuthData, &vector<u8>|,
    ): signer {
        let maybe_entry_function_payload = transaction_context::entry_function_payload();
        if (maybe_entry_function_payload.is_some()) {
            let entry_function_payload = maybe_entry_function_payload.destroy_some();
            let entry_function_name = entry_function_name(&entry_function_payload);

            // call the passed-in function value
            auth_fn(aa_auth_data, &entry_function_name);
            account
        } else {
            abort(EMISSING_ENTRY_FUNCTION_PAYLOAD)
        }
    }

    #[test_only]
    use std::string::utf8;

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
    fun test_construct_message_for_sui(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);
        let sui_account_address = b"0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49";
        let domain = b"localhost:3001";
        let entry_function_name = b"0x1::coin::transfer";
        let digest_utf8 = b"0x041689ce61015dd0aa166aa2edc1cc74e63b3ed093f40e3ce4101fce067b24ad";
        let message = construct_message(&b"Sui",&sui_account_address, &domain, &entry_function_name, &digest_utf8);
        assert!(message == b"localhost:3001 wants you to sign in with your Sui account:\n0x8d6ce7a3c13617b29aaf7ec58bee5a611606a89c62c5efbea32e06d8d167bd49\n\nPlease confirm you explicitly initiated this request from localhost:3001. You are approving to execute transaction 0x1::coin::transfer on Aptos blockchain (testnet).\n\nNonce: 0x041689ce61015dd0aa166aa2edc1cc74e63b3ed093f40e3ce4101fce067b24ad");
    }

    #[test(framework = @0x1)]
    fun test_construct_message_for_solana(framework: &signer) {
        chain_id::initialize_for_test(framework, 2);

        let base58_public_key = b"G56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV";
        let domain = b"localhost:3000";
        let entry_function_name = b"0x1::coin::transfer";
        let digest_utf8 = b"0x9509edc861070b2848d8161c9453159139f867745dc87d32864a71e796c7d279";
        let message = construct_message(&b"Solana", &base58_public_key, &domain, &entry_function_name, &digest_utf8);
        assert!(message == b"localhost:3000 wants you to sign in with your Solana account:\nG56zT1K6AQab7FzwHdQ8hiHXusR14Rmddw6Vz5MFbbmV\n\nPlease confirm you explicitly initiated this request from localhost:3000. You are approving to execute transaction 0x1::coin::transfer on Aptos blockchain (testnet).\n\nNonce: 0x9509edc861070b2848d8161c9453159139f867745dc87d32864a71e796c7d279");
    }
}
