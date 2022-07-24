/// This module defines a struct storing the publishing policies for the VM.
module aptos_framework::transaction_publishing_option {
    use std::error;
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::reconfiguration;

    /// Defines and holds the publishing policies for the VM. There are three possible configurations:
    /// 1. No module publishing, only allow-listed scripts are allowed.
    /// 2. No module publishing, custom scripts are allowed.
    /// 3. Both module publishing and custom scripts are allowed.
    /// We represent these as the following resource.
    struct TransactionPublishingOption has key {
        /// Only script hashes in the following list can be executed by the network. If the vector is empty, no
        /// limitation would be enforced.
        script_allow_list: vector<vector<u8>>,
        /// Anyone can publish new module if this flag is set to true.
        module_publishing_allowed: bool,
    }

    const ECONFIG: u64 = 1;

    public fun initialize(
        account: &signer,
        script_allow_list: vector<vector<u8>>,
        module_publishing_allowed: bool,
    ) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);
        assert!(!exists<TransactionPublishingOption>(@aptos_framework), error::already_exists(ECONFIG));

        move_to(
            account,
            TransactionPublishingOption{
                script_allow_list,
                module_publishing_allowed
            }
        );
    }

    public fun is_script_allowed(script_hash: &vector<u8>): bool acquires TransactionPublishingOption {
        if (vector::is_empty(script_hash)) return true;
        let publish_option = borrow_global<TransactionPublishingOption>(@aptos_framework);
        // allowlist empty = open publishing, anyone can send txes
        vector::is_empty(&publish_option.script_allow_list)
        || vector::contains(&publish_option.script_allow_list, script_hash)
    }

    public fun is_module_allowed(): bool acquires TransactionPublishingOption {
        let publish_option = borrow_global<TransactionPublishingOption>(@aptos_framework);

        publish_option.module_publishing_allowed
    }

    public entry fun set_module_publishing_allowed(account:signer, is_allowed: bool) acquires TransactionPublishingOption {
        system_addresses::assert_core_resource(&account);
        let publish_option = borrow_global_mut<TransactionPublishingOption>(@aptos_framework);
        publish_option.module_publishing_allowed = is_allowed;

        reconfiguration::reconfigure();
    }
}
