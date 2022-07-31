/// This module defines a struct storing the publishing policies for the VM.
module aptos_framework::transaction_publishing_option {
    use std::error;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::reconfiguration;

    /// Defines and holds the publishing policies for the VM.
    struct TransactionPublishingOption has key {
        /// Anyone can publish new module if this flag is set to true.
        module_publishing_allowed: bool,
    }

    const ECONFIG: u64 = 1;

    public fun initialize(account: &signer, module_publishing_allowed: bool) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);
        assert!(!exists<TransactionPublishingOption>(@aptos_framework), error::already_exists(ECONFIG));

        move_to(
            account,
            TransactionPublishingOption{
                module_publishing_allowed
            }
        );
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
