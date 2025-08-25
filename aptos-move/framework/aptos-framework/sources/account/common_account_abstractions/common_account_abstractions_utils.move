module aptos_framework::common_account_abstractions_utils {
    use std::chain_id;
    use std::string_utils;
    use std::transaction_context::{Self, EntryFunctionPayload};

    friend aptos_framework::ethereum_derivable_account;
    friend aptos_framework::solana_derivable_account;
    friend aptos_framework::sui_derivable_account;

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
}
