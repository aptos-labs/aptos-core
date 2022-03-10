/// This module defines a struct storing the publishing policies for the VM.
module CoreFramework::DiemTransactionPublishingOption {
    use Std::Capability::Cap;
    use Std::Errors;
    use Std::Vector;
    use CoreFramework::DiemTimestamp;
    use CoreFramework::SystemAddresses;
    use CoreFramework::DiemConfig;

    struct ChainMarker<phantom T> has key {}

    /// Defines and holds the publishing policies for the VM. There are three possible configurations:
    /// 1. No module publishing, only allow-listed scripts are allowed.
    /// 2. No module publishing, custom scripts are allowed.
    /// 3. Both module publishing and custom scripts are allowed.
    /// We represent these as the following resource.
    struct DiemTransactionPublishingOption has key {
        /// Only script hashes in the following list can be executed by the network. If the vector is empty, no
        /// limitation would be enforced.
        script_allow_list: vector<vector<u8>>,
        /// Anyone can publish new module if this flag is set to true.
        module_publishing_allowed: bool,
    }

    const ECHAIN_MARKER: u64 = 0;
    const ECONFIG: u64 = 1;

    public fun initialize<T>(
        core_resource_account: &signer,
        script_allow_list: vector<vector<u8>>,
        module_publishing_allowed: bool,
    ) {
        DiemTimestamp::assert_genesis();
        SystemAddresses::assert_core_resource(core_resource_account);
        assert!(!exists<ChainMarker<T>>(@CoreResources), Errors::already_published(ECHAIN_MARKER));
        assert!(!exists<DiemTransactionPublishingOption>(@CoreResources), Errors::already_published(ECONFIG));

        move_to(core_resource_account, ChainMarker<T> {});
        move_to(
            core_resource_account,
            DiemTransactionPublishingOption{
                script_allow_list,
                module_publishing_allowed
            }
        );
    }

    public fun is_script_allowed(script_hash: &vector<u8>): bool acquires DiemTransactionPublishingOption {
        if (Vector::is_empty(script_hash)) return true;
        let publish_option = borrow_global<DiemTransactionPublishingOption>(@CoreResources);
        // allowlist empty = open publishing, anyone can send txes
        Vector::is_empty(&publish_option.script_allow_list)
        || Vector::contains(&publish_option.script_allow_list, script_hash)
    }

    public fun is_module_allowed(): bool acquires DiemTransactionPublishingOption {
        let publish_option = borrow_global<DiemTransactionPublishingOption>(@CoreResources);

        publish_option.module_publishing_allowed
    }

    public fun set_module_publishing_allowed<T>(is_allowed: bool, _witness: Cap<T>) acquires DiemTransactionPublishingOption {
        assert!(exists<ChainMarker<T>>(@CoreResources), Errors::not_published(ECHAIN_MARKER));
        let publish_option = borrow_global_mut<DiemTransactionPublishingOption>(@CoreResources);
        publish_option.module_publishing_allowed = is_allowed;

        DiemConfig::reconfigure();
    }
}
